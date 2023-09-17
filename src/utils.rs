#[inline]
fn check_bit_mask(opcode: u32, mask: u32) -> bool {
    opcode & mask == mask
}