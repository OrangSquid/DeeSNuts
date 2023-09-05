pub struct Memory {
    bios: [u8; 0x4000],
    rom: Vec<u8>,
    ewram: [u8; 0x40000],
    iwram: [u8; 0x8000],
    
}