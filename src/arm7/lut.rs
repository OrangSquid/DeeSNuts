pub(in crate::arm7) const fn condition_lut() -> [bool; 256] {
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

