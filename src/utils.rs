#[macro_export]
macro_rules! check_bit {
    ($opcode:expr, $bit:expr) => {
        $opcode & (1 << $bit) == (1 << $bit)
    };
}