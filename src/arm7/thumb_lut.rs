use crate::{check_bit, get_thumb_register_number_at};

use super::cpu::Cpu;

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
            0x18 => dummy,
            _ => move_shifted_register_handler
        },
        0x20 => alu_immeddiate_handler,
        0x40 => match bits15_8 & 0x10 {
            0x10 => dummy,
            _ => match bits15_8 & 0x8 {
                0x8 => dummy,
                _ => match bits15_8 & 0x4 {
                    0x4 => hi_register_operation_handler,
                    _ => dummy
                }
            },
        },
        0x60 => dummy,
        0x80 => dummy,
        0xA0 => load_address_handler,
        0xC0 => dummy,
        0xE0 => dummy,
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

fn load_address_handler(cpu: &mut Cpu, opcode: u32) {
    let destination_register = get_thumb_register_number_at!(opcode, 8);
    let source_register = if check_bit!(opcode, 11) {
        13
    } else { 15 };
    let operand_2 = (opcode & 0xFF) << 2;

    cpu.decode_alu(AluOpcode::Add, false, source_register, destination_register, operand_2);
}
