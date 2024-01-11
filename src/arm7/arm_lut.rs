use core::panic;

use crate::{check_bit, get_register_number_at};

use super::{constants::*, cpu::Cpu};

fn dummy(cpu: &mut Cpu, foo: u32) { }

pub const fn arm_instruction_lut() -> [InstructionHandler; 4096] {
    let dummy: InstructionHandler = dummy;
    let mut temp = [dummy; 4096];
    let mut i = 0;
    while i <= 0xF {
        let mut j = 0;
        while j <= 0xFF {
            temp[j << 4 | i] = decode_arm(j as u8, i as u8);
            j += 1;
        }
        i += 1;
    }
    temp
}

const fn decode_sr_alu(bits27_20: u8) -> InstructionHandler {
    match bits27_20 & 0x1F {
        0x12 | 0x16 => msr_transfer_handler,
        0x10 | 0x14 => mrs_transfer_handler,
        _ => alu_handler
    }
}

const fn decode_arm_0x0_start(bits27_20: u8, bits7_4: u8) -> InstructionHandler {
    match bits7_4 & 0x9 {
        0x0 | 0x8 => decode_sr_alu(bits27_20),
        0x1 => {
            if bits27_20 == 0x12 && bits7_4 == 0x1 {
                branch_and_exchange_handler
            } else {
                decode_sr_alu(bits27_20)
            }
        },
        0x9 =>
        match bits27_20 & 0x20 {
            0x20 => decode_sr_alu(bits27_20),
            _ =>
            match bits7_4 & 0x6 {
                0x0 =>
                match bits27_20 & 0x18 {
                    0x0 =>  multiply_handler,
                    0x8 =>  multiply_long_hanlder,
                    0x10 => single_data_swap,
                    _ => undefinied_handler
                },
                _ => halfword_data_transfer_handler
            }
        },
        _ => panic!()
    }
}

const fn decode_arm_0x40_start(bits27_20: u8, bits7_4: u8) -> InstructionHandler {
    if check_bit!(bits27_20, 5) && check_bit!(bits7_4, 0) {
        undefinied_handler
    } else {
        single_data_transfer
    }
}

const fn decode_arm_0x80_start(bits27_20: u8) -> InstructionHandler {
    match bits27_20 & 0x20 {
        0x0 => block_data_transfer_handler,
        0x20 => branch_handler,
        _ => panic!(),
    }
}

pub(super) const fn decode_arm(bits27_20: u8, bits7_4: u8) -> InstructionHandler {
    match bits27_20 & 0xC0 {
        0x0 => decode_arm_0x0_start(bits27_20, bits7_4),
        0x40 => decode_arm_0x40_start(bits27_20, bits7_4),
        0x80 => decode_arm_0x80_start(bits27_20),
        0xC0 => software_interrupt_handler,
        _ => panic!()
    }
}

fn branch_handler(cpu: &mut Cpu, opcode: u32) {
    let link = check_bit!(opcode, 24);
    let offset = (opcode & 0xff_ffff) as i32;

    cpu.branch(link, offset);
}

fn branch_and_exchange_handler(cpu: &mut Cpu, opcode: u32) {
    let source_register = get_register_number_at!(opcode, 0);

    cpu.branch_and_exchange(source_register);
}

fn msr_transfer_handler(cpu: &mut Cpu, opcode: u32) {
    let operand2_type = 
        if check_bit!(opcode, 24) {
            Operand2Type::ImmediateWithRotation
        } else {
            Operand2Type::RegisterWithImmediateShift
        };
    let destination_is_spsr = check_bit!(opcode, 22);
    let mask: u32 = match opcode & (0xf << 16) {
        0x1_0000 => 0xff,
        0x2_0000 => 0xff00,
        0x3_0000 => 0xffff,
        0x4_0000 => 0xff0000,
        0x5_0000 => 0xff00ff,
        0x6_0000 => 0xffff00,
        0x7_0000 => 0xffffff,
        0x8_0000 => 0xff000000,
        0x9_0000 => 0xff0000ff,
        0xa_0000 => 0xff00ff00,
        0xb_0000 => 0xff00ffff,
        0xc_0000 => 0xffff0000,
        0xd_0000 => 0xffff00ff,
        0xe_0000 => 0xffffff00,
        0xf_0000 => 0xffffffff,
        _ => panic!(),
    };
    let operand_2: u32 = cpu.get_operand2(
        operand2_type,
        super::constants::ShiftType::LogicalLeft,
        false,
        opcode
    );

    cpu.msr(operand2_type, destination_is_spsr, mask, operand_2);
}

fn mrs_transfer_handler(cpu: &mut Cpu, opcode: u32) {
    let source_is_spsr = check_bit!(opcode, 22);
    let destination_register = get_register_number_at!(opcode, 12);

    cpu.mrs(source_is_spsr, destination_register);
}

fn alu_handler(cpu: &mut Cpu, opcode: u32) {
    let operand2_type = 
        if check_bit!(opcode, 25) {
            Operand2Type::ImmediateWithRotation
        } else if check_bit!(opcode, 4) {
            Operand2Type::RegisterWithRegisterShift
        } else {
            Operand2Type::RegisterWithImmediateShift
        };
    let alu_opcode = to_alu_opcode((opcode >> 21) & 0xF);
    let set_condition_codes =  check_bit!(opcode, 20);
    let shift_type = to_shift_type((opcode >> 5) & 0x3);
    let operand_1_regsiter = get_register_number_at!(opcode, 16);
    let destination_register = get_register_number_at!(opcode, 12);
    let old_r15 = cpu.registers[15];
    let operand_2 = cpu.get_operand2(operand2_type, shift_type, set_condition_codes, opcode);

    cpu.decode_alu(alu_opcode, set_condition_codes, operand_1_regsiter, destination_register, operand_2);
    if destination_register != 15 {
        cpu.registers[15] = old_r15;
    }
}

fn multiply_handler(cpu: &mut Cpu, opcode: u32) {
    let accumulate = check_bit!(opcode, 21);
    let set_conditions =  check_bit!(opcode, 20);
    let operand_1_register = get_register_number_at!(opcode, 12);
    let operand_2_register = get_register_number_at!(opcode, 8);
    let operand_3_register = get_register_number_at!(opcode, 0);
    let destination_register = get_register_number_at!(opcode, 16);

    cpu.multiply(accumulate, set_conditions, operand_1_register, operand_2_register, operand_3_register, destination_register);
}

fn multiply_long_hanlder(cpu: &mut Cpu, opcode: u32) {
    let signed = check_bit!(opcode, 22);
    let accumulate = check_bit!(opcode, 21);
    let set_conditions = check_bit!(opcode, 20);

    let register_hi = get_register_number_at!(opcode, 16);
    let register_lo = get_register_number_at!(opcode, 12);
    let operand_1_register = get_register_number_at!(opcode, 8);
    let operand_2_register = get_register_number_at!(opcode, 0);

    cpu.multiply_long(signed, accumulate, set_conditions, register_hi, register_lo, operand_1_register, operand_2_register);
}

fn single_data_transfer(cpu: &mut Cpu, opcode: u32) {
    let operand2_type =  
        if !check_bit!(opcode, 25) {
            Operand2Type::Immediate
        } else if check_bit!(opcode, 4) {
            Operand2Type::RegisterWithRegisterShift
        } else {
            Operand2Type::RegisterWithImmediateShift
        };
    let pre_indexing = check_bit!(opcode, 24);
    let add_offset = check_bit!(opcode, 23);
    let transfer_byte = check_bit!(opcode, 22);
    let write_back = check_bit!(opcode, 21);
    let load = check_bit!(opcode, 20);
    let shift_type = to_shift_type((opcode >> 5) & 0x3);

    let base_register = get_register_number_at!(opcode, 16);
    let offset = cpu.get_operand2(operand2_type, shift_type, true, opcode);
    let src_dst_register = get_register_number_at!(opcode, 12);

    cpu.single_data_transfer(pre_indexing, add_offset, transfer_byte, write_back, load, base_register, offset, src_dst_register);
}

fn halfword_data_transfer_handler(cpu: &mut Cpu, opcode: u32) {
    let immediate = check_bit!(opcode, 22);
    let pre_indexing = check_bit!(opcode, 24); 
    let add_offset = check_bit!(opcode, 23); 
    let write_back = check_bit!(opcode, 21); 
    let load = check_bit!(opcode, 20); 
    let halfword_transfer_type = to_halfword_transfer_type((opcode >> 5) & 0x3);
    let offset_value = (opcode & 0xf) | ((opcode & 0xf00) >> 4);
    let offset_register = get_register_number_at!(opcode, 0);
    let base_register = get_register_number_at!(opcode, 16);
    let src_dst_register = get_register_number_at!(opcode, 12);

    cpu.halfword_data_transfer(immediate, pre_indexing, add_offset, write_back, load, halfword_transfer_type, base_register, src_dst_register, offset_value, offset_register);
}

fn block_data_transfer_handler(cpu: &mut Cpu, opcode: u32) {
    let pre_indexing = check_bit!(opcode, 24);
    let add_offset = check_bit!(opcode, 23);
    let load_psr = check_bit!(opcode, 22);
    let write_back = check_bit!(opcode, 21);
    let load = check_bit!(opcode, 20);
    let base_register = get_register_number_at!(opcode, 16);
    let register_mask = opcode & 0xffff;

    cpu.block_data_transfer(pre_indexing, add_offset, load_psr, write_back, load, base_register, register_mask);
}

fn single_data_swap(cpu: &mut Cpu, opcode: u32) {
    let transfer_byte = check_bit!(opcode, 22);
    let address_register = get_register_number_at!(opcode, 16);
    let dst_register = get_register_number_at!(opcode, 12);
    let src_register = get_register_number_at!(opcode, 0);

    cpu.single_data_swap(transfer_byte, address_register, dst_register, src_register);
}

fn software_interrupt_handler(cpu: &mut Cpu, opcode: u32) { }

fn undefinied_handler(cpu: &mut Cpu, opcode: u32) { }

pub(super) const fn condition_lut() -> [bool; 256] {
    const SIGN_FLAG: u8 = 0x8;
    const ZERO_FLAG: u8 = 0x4;
    const CARRY_FLAG: u8 = 0x2;
    const OVERFLOW_FLAG: u8 = 0x1;

    let mut temp = [false; 256];
    let mut last_index = 0;

    while last_index != 256 {
        let condition_code = ((last_index & 0xF0) >> 4) as u8;
        let flag_set = (last_index & 0xF) as u8;
        temp[last_index] = match condition_code {
            0x0 => flag_set & ZERO_FLAG != 0,
            0x1 => flag_set & ZERO_FLAG == 0,
            0x2 => flag_set & CARRY_FLAG != 0,
            0x3 => flag_set & CARRY_FLAG == 0,
            0x4 => flag_set & SIGN_FLAG != 0,
            0x5 => flag_set & SIGN_FLAG == 0,
            0x6 => flag_set & OVERFLOW_FLAG != 0,
            0x7 => flag_set & OVERFLOW_FLAG == 0,
            0x8 => flag_set & (CARRY_FLAG | ZERO_FLAG) == CARRY_FLAG,
            0x9 => flag_set & CARRY_FLAG == 0 || flag_set & ZERO_FLAG != 0,
            0xA => (flag_set >> 3) == (flag_set & OVERFLOW_FLAG),
            0xB => (flag_set >> 3) != (flag_set & OVERFLOW_FLAG),
            0xC => flag_set & ZERO_FLAG == 0 && (flag_set >> 3) == (flag_set & OVERFLOW_FLAG),
            0xD => flag_set & ZERO_FLAG == ZERO_FLAG || (flag_set >> 3) != (flag_set & OVERFLOW_FLAG),
            0xE => true,
            0xF => true,
            _ => panic!("Condition for opcode is higher than 0xF"),
        };
        last_index += 1;
    }
    temp
}
