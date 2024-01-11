#[macro_export]
macro_rules! check_bit {
    ($opcode:expr, $bit:expr) => {
        $opcode & (1 << $bit) == (1 << $bit)
    };
}

#[macro_export]
macro_rules! get_register_number_at {
    ($opcode:expr, $bits:expr) => {
        (($opcode >> $bits) & 0xF) as usize
    };
}

#[macro_export]
macro_rules! get_thumb_register_number_at {
    ($opcode:expr, $bits:expr) => {
        (($opcode >> $bits) & 0x7) as usize
    };
}