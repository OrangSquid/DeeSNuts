use core::panic;

use crate::check_bit;

use super::{constants::*, cpu::Cpu};

pub const fn instruction_lut() -> [Instruction; 4096] {
    let mut temp = [Instruction::NoOp; 4096];
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

const fn decode_sr_alu(bits27_20: u8, bits7_4: u8) -> Instruction {
    match bits27_20 & 0x1F {
        0x12 | 0x16 => Instruction::MSRTransfer {
            operand2_type: 
                if check_bit!(bits27_20, 5) {
                    Operand2Type::ImmediateWithRotation
                } else {
                    Operand2Type::RegisterWithImmediateShift
                },
            destination_is_spsr: check_bit!(bits27_20, 2)
        },
        0x10 | 0x14 => Instruction::MRSTransfer {
            source_is_spsr: check_bit!(bits27_20, 2)
        },
        _ => Instruction::Alu {
            operand2_type: 
                if check_bit!(bits27_20, 5) {
                    Operand2Type::ImmediateWithRotation
                } else if check_bit!(bits7_4, 0) {
                    Operand2Type::RegisterWithRegisterShift
                } else {
                    Operand2Type::RegisterWithImmediateShift
                },
            opcode: to_alu_opcode((bits27_20 >> 1) & 0xF),
            set_conditions: check_bit!(bits27_20, 0),
            shift_type: to_shift_type((bits7_4 >> 1) & 0x3)
        },
    }
}

const fn decode_arm_0x0_start(bits27_20: u8, bits7_4: u8) -> Instruction {
    match bits7_4 & 0x9 {
        0x0 | 0x8 => decode_sr_alu(bits27_20, bits7_4),
        0x1 => {
            if bits27_20 == 0x12 && bits7_4 == 0x1 {
                Instruction::BranchAndExchange
            } else {
                decode_sr_alu(bits27_20, bits7_4)
            }
        },
        0x9 =>
        match bits27_20 & 0x20 {
            0x20 => decode_sr_alu(bits27_20, bits7_4),
            _ =>
            match bits7_4 & 0x6 {
                0x0 =>
                match bits27_20 & 0x18 {
                    0x0 => Instruction::Multiply {
                        accumulate: check_bit!(bits27_20, 1),
                        set_conditions: check_bit!(bits27_20, 0)
                    },
                    0x8 => Instruction::MultiplyLong {
                        signed: check_bit!(bits27_20, 2),
                        accumulate: check_bit!(bits27_20, 1),
                        set_conditions: check_bit!(bits27_20, 0)
                    },
                    0x10 => Instruction::SingleDataSwap {
                        transfer_byte: check_bit!(bits27_20, 2)
                    },
                    _ => Instruction::NoOp
                },
                _ => Instruction::HalfowrdTransfer { 
                    immediate: check_bit!(bits27_20, 2),
                    pre_indexing: check_bit!(bits27_20, 4), 
                    add_offset: check_bit!(bits27_20, 3), 
                    write_back: check_bit!(bits27_20, 1), 
                    load: check_bit!(bits27_20, 0), 
                    halfword_transfer_type: to_halfword_transfer_type((bits7_4 >> 1) & 0x3) 
                }
            }
        },
        _ => panic!()
    }
}

const fn decode_arm_0x40_start(bits27_20: u8, bits7_4: u8) -> Instruction {
    if check_bit!(bits27_20, 5) && check_bit!(bits7_4, 0) {
        Instruction::Undefined
    } else {
        Instruction::SingleDataTransfer {
            operand2_type: 
                if !check_bit!(bits27_20, 5) {
                    Operand2Type::Immediate
                } else if check_bit!(bits7_4, 0) {
                    Operand2Type::RegisterWithRegisterShift
                } else {
                    Operand2Type::RegisterWithImmediateShift
                },
            pre_indexing: check_bit!(bits27_20, 4),
            add_offset: check_bit!(bits27_20, 3),
            transfer_byte: check_bit!(bits27_20, 2),
            write_back: check_bit!(bits27_20, 1),
            load: check_bit!(bits27_20, 0),
            shift_type: to_shift_type((bits7_4 >> 1) & 0x3)
        }
    }
}

const fn decode_arm_0x80_start(bits27_20: u8) -> Instruction {
    match bits27_20 & 0x20 {
        0x0 => Instruction::BlockDataTransfer {
            pre_indexing: check_bit!(bits27_20, 4),
            add_offset: check_bit!(bits27_20, 3),
            load_psr: check_bit!(bits27_20, 2),
            write_back: check_bit!(bits27_20, 1),
            load: check_bit!(bits27_20, 0)
        },
        0x20 => Instruction::Branch { 
            link: check_bit!(bits27_20, 4) 
        },
        _ => panic!(),
    }
}

pub const fn decode_arm(bits27_20: u8, bits7_4: u8) -> Instruction {
    match bits27_20 & 0xC0 {
        0x0 => decode_arm_0x0_start(bits27_20, bits7_4),
        0x40 => decode_arm_0x40_start(bits27_20, bits7_4),
        0x80 => decode_arm_0x80_start(bits27_20),
        0xC0 => Instruction::SoftwareInterrupt,
        _ => panic!()
    }
}

/* type ARMHandler = fn(&mut Cpu, u32);

pub const fn lmao() -> ARMHandler {
    Cpu::branch_and_exchange
} */

pub(in super) const fn condition_lut() -> [bool; 256] {
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
