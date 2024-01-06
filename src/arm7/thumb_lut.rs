use crate::check_bit;

use super::constants::*;

const fn decode_thumb(j: u8, i: u8) -> Instruction {
    Instruction::NoOp
}

pub const fn instruction_lut() -> [Instruction; 4096] {
    let mut temp = [Instruction::NoOp; 4096];
    let mut i = 0;
    while i <= 0xF {
        let mut j = 0;
        while j <= 0xFF {
            temp[j << 4 | i] = decode_thumb(j as u8, i as u8);
            j += 1;
        }
        i += 1;
    }
    temp
}