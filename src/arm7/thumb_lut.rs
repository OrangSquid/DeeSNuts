use crate::{check_bit, get_thumb_register_number_at};

use super::cpu::{self, Cpu, CONDITION_LUT};

use super::constants::*;

fn dummy(cpu: &mut Cpu, foo: u32) { }

pub const fn thumb_instruction_lut() -> [InstructionHandler; 256] {
    let dummy: InstructionHandler = dummy;
    let mut temp = [dummy; 256];
    let mut i = 0;
    while i < 256 {
        temp[i] = decode_thumb(i as u8);
        i += 1;
    }
    temp
}

const fn decode_thumb(bits15_8: u8) -> InstructionHandler {
    match bits15_8 & 0xE0 {
        0x0 => match bits15_8 & 0x18 {
            0x18 => add_subtract_handler,
            _ => move_shifted_register_handler
        },
        0x20 => alu_immeddiate_handler,
        0x40 => match bits15_8 & 0x10 {
            0x10 => load_store_with_register_offset_handler,
            _ => match bits15_8 & 0x8 {
                0x8 => pc_relative_handler,
                _ => match bits15_8 & 0x4 {
                    0x4 => hi_register_operation_handler,
                    _ => alu_operations_handler
                }
            },
        },
        0x60 => load_store_with_immediate_offset_handler,
        0x80 => match bits15_8 & 0x10 {
            0x10 => sp_relative_load_handler,
            _ => load_store_halfword_handler
        },
        0xA0 => match bits15_8 & 0x10 {
            0x10 => match bits15_8 & 0x6 {
                0x0 => add_offset_to_stack_pointer_handler,
                0x4 => push_pop_register_handler,
                _ => dummy
            },
            _ => load_address_handler,
        }
        0xC0 => match bits15_8 & 0x10 {
            0x10 => match bits15_8 & 0xF {
                0xF => software_interrupt_handler,
                _ => conditional_branch_handler
            },
            _ => multiple_load_store_handler
        },
        0xE0 => match bits15_8 & 0x18 {
            0x0 => unconditional_branch_handler,
            _ => long_branch_with_link_handler,
        },
        _ => panic!(),
    }
}

fn move_shifted_register_handler(cpu: &mut Cpu, opcode: u32) {
    let source_register = get_thumb_register_number_at!(opcode, 3);
    let destination_register = get_thumb_register_number_at!(opcode, 0);
    let shift_type = match opcode & 0x1800 {
        0x0 => ShiftType::LogicalLeft,
        0x800 => ShiftType::LogicalRight,
        0x1000 => ShiftType::ArithmeticRight,
        _ => panic!()
    };

    let operand_2 = cpu.get_operand2(Operand2Type::RegisterWithImmediateShift, shift_type, true, ((opcode & 0x7C0) << 1) | source_register as u32);

    cpu.decode_alu(AluOpcode::Move, true, 0, destination_register, operand_2)
}

fn add_subtract_handler(cpu: &mut Cpu, opcode: u32) {
    let operand_type = if check_bit!(opcode, 10) {
        Operand2Type::Immediate
    } else {
        Operand2Type::RegisterWithImmediateShift
    };
    let alu_opcode = if check_bit!(opcode, 9) {
        AluOpcode::Add
    } else {
        AluOpcode::Subtract
    };
    let operand_2 = cpu.get_operand2(operand_type, ShiftType::LogicalLeft, false, opcode & 0x7);
    let operand_1_register = get_thumb_register_number_at!(opcode, 3);
    let destination_register = get_thumb_register_number_at!(opcode, 0);
    
    cpu.decode_alu(alu_opcode, true, operand_1_register, destination_register, operand_2);
}

fn alu_immeddiate_handler(cpu: &mut Cpu, opcode: u32) {
    let alu_opcode = match opcode & 0x1800 {
        0x0 => AluOpcode::Move,
        0x800 => AluOpcode::CompareSubtract,
        0x1000 => AluOpcode::Add,
        0x1800 => AluOpcode::Subtract,
        _ => panic!()
    };
    let src_dst_register = get_thumb_register_number_at!(opcode, 8);
    let operand_2 = opcode & 0xFF;

    cpu.decode_alu(alu_opcode, true, src_dst_register, src_dst_register, operand_2);
}

fn alu_operations_handler(cpu: &mut Cpu, opcode: u32) {
    let alu_opcode = to_alu_opcode((opcode & 0x3C0) >> 6);
    let operand_1_register = get_thumb_register_number_at!(opcode, 3);
    let operand_2_register = get_thumb_register_number_at!(opcode, 0);
    let operand_2 = cpu.get_operand2(Operand2Type::RegisterWithImmediateShift, ShiftType::LogicalLeft, false, operand_2_register as u32);

    cpu.decode_alu(alu_opcode, true, operand_1_register, operand_2_register, operand_2);
}

fn hi_register_operation_handler(cpu: &mut Cpu, opcode: u32) {
    let source_register = ((opcode as usize & 0x40) >> 3) | get_thumb_register_number_at!(opcode, 3);
    let destination_register = ((opcode as usize & 0x80) >> 4) | get_thumb_register_number_at!(opcode, 0);
    if opcode & 0x300 == 0x300 {
        cpu.branch_and_exchange(source_register);
    } else {
        let (alu_opcode, set_condition_codes) = match opcode & 0x300 {
            0x0 => (AluOpcode::Add, false),
            0x100 => (AluOpcode::CompareSubtract, true),
            0x200 => (AluOpcode::Move, false),
            _ => panic!()
        };
        let operand_2 = cpu.get_operand2(Operand2Type::RegisterWithImmediateShift, ShiftType::LogicalLeft, false, source_register as u32);

        cpu.decode_alu(alu_opcode, set_condition_codes, destination_register, destination_register, operand_2)
    }
}

fn pc_relative_handler(cpu: &mut Cpu, opcode: u32) {
    let destination_register = get_thumb_register_number_at!(opcode, 8);
    let offset = (opcode & 0xFF) << 2;

    cpu.single_data_transfer(true, true, false, false, true, 15, offset, destination_register, false);
}

fn load_store_with_register_offset_handler(cpu: &mut Cpu, opcode: u32) {
    let halfword = check_bit!(opcode, 9);
    let register_offset = get_thumb_register_number_at!(opcode, 6);
    let offset = cpu.get_operand2(Operand2Type::RegisterWithImmediateShift, ShiftType::LogicalLeft, false, register_offset as u32);
    let base_register = get_thumb_register_number_at!(opcode, 3);
    let src_dst_register = get_thumb_register_number_at!(opcode, 0);

    if halfword {
        let load = (opcode >> 10) & 0x3 != 0x0;
        let halfword_transfer_type = to_halfword_transfer_type((opcode >> 10) & 0x3);
        cpu.halfword_data_transfer(false, true, true, false, load, halfword_transfer_type, base_register, src_dst_register, offset, offset as usize);
    } else {
        let load = check_bit!(opcode, 11);
        let transfer_byte = check_bit!(opcode, 10);
        cpu.single_data_transfer(true, true, transfer_byte, false, load, base_register, offset, src_dst_register, false);
    }
}

fn load_store_with_immediate_offset_handler(cpu: &mut Cpu, opcode: u32) {
    let transfer_byte = check_bit!(opcode, 12);
    let load = check_bit!(opcode, 11);
    let offset = ((opcode >> 6) & 0x1F) << 2;
    let base_register = get_thumb_register_number_at!(opcode, 3);
    let src_dst_register = get_thumb_register_number_at!(opcode, 0);

    cpu.single_data_transfer(true, true, transfer_byte, false, load, base_register, offset, src_dst_register, false);
}

fn load_store_halfword_handler(cpu: &mut Cpu, opcode: u32) {
    let load = check_bit!(opcode, 11);
    let offset = (opcode >> 6) & 0x1F;
    let base_register = get_thumb_register_number_at!(opcode, 3);
    let src_dst_register = get_thumb_register_number_at!(opcode, 0);
    let halfword_transfer_type = HalfwordTransferType::UnsignedHalfwords;

    cpu.halfword_data_transfer(true, true, true, false, load, halfword_transfer_type, base_register, src_dst_register, offset, offset as usize);
}

fn sp_relative_load_handler(cpu: &mut Cpu, opcode: u32) {
    let load = check_bit!(opcode, 11);
    let destination_register = get_thumb_register_number_at!(opcode, 8);
    let offset = opcode & 0xFF;

    cpu.single_data_transfer(true, true, false, false, load, 13, offset, destination_register, false);
}

fn load_address_handler(cpu: &mut Cpu, opcode: u32) {
    let destination_register = get_thumb_register_number_at!(opcode, 8);
    let source_register = if check_bit!(opcode, 11) {
        13
    } else { 15 };
    let operand_2 = (opcode & 0xFF) << 2;

    cpu.decode_alu(AluOpcode::Add, false, source_register, destination_register, operand_2);
}

fn add_offset_to_stack_pointer_handler(cpu: &mut Cpu, opcode: u32) {
    let operand_2 = (opcode & 0x3F) << 2;
    let alu_opcode = if check_bit!(opcode, 7) {
        AluOpcode::Subtract
    } else {
        AluOpcode::Add
    };

    cpu.decode_alu(alu_opcode, false, 13, 13, operand_2);
}

fn push_pop_register_handler(cpu: &mut Cpu, opcode: u32) {
    let load = check_bit!(opcode, 11);
    let r_bit = check_bit!(opcode, 8);
    let mut register_mask = opcode << 8;
    if load {
        register_mask |= r_bit as u32;
    } else {
        register_mask |= (r_bit as u32) << 1;
    }

    cpu.block_data_transfer(!load, !load, false, true, load, 13, register_mask)
}

fn multiple_load_store_handler(cpu: &mut Cpu, opcode: u32) {
    let load = check_bit!(opcode, 11);
    let base_register = get_thumb_register_number_at!(opcode, 8);
    let register_mask = opcode << 8;

    cpu.block_data_transfer(false, true, false, true, load, base_register, register_mask);
}

fn conditional_branch_handler(cpu: &mut Cpu, opcode: u32) {
    let offset = ((opcode as i32 & 0xFF) << 24) >> 24;
    let condition = (opcode as usize & 0xF00) >> 4;

    if CONDITION_LUT[condition | (cpu.cpsr_register >> 28) as usize] {
        cpu.branch(false, offset, 1, 8);
    }
}

fn software_interrupt_handler(cpu: &mut Cpu, opcode: u32) {
    cpu.software_interrupt();
}

fn unconditional_branch_handler(cpu: &mut Cpu, opcode: u32) {
    let offset = ((opcode as i32 & 0x7FF) << 21) >> 21;

    cpu.branch(false, offset, 1, 11);
}

fn long_branch_with_link_handler(cpu: &mut Cpu, opcode: u32) {
    let offset = (opcode as i32) & 0x7FF;
    if check_bit!(opcode, 11) {
        let temp = cpu.registers[15];
        cpu.registers[15] = (cpu.registers[14] as i32 + (offset << 1)) as u32;
        cpu.registers[14] = temp | 1;
        
    } else {
        cpu.registers[14] = (cpu.registers[15] as i32 + ((offset << 21) >> 9)) as u32;
    }
}
