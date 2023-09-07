use std::ops::{Index, Range, IndexMut};

const BIOS_ADDRESS: usize = 0x00000000;
const BIOS_END: usize = 0x00003FFF;
const EWRAM_ADDRESS: usize = 0x02000000;
const EWRAM_END: usize = 0x0203FFFF;
const IWRAM_ADDRESS: usize = 0x03000000;
const IWRAM_END: usize = 0x03007FFF;
const IO_REGISTERS: usize = 0x04000000;
const IO_REGISTERS_END: usize = 0x040003FE;
const PALLETE_RAM_ADDRESS: usize = 0x05000000;
const PALLETE_RAM_END: usize = 0x050003FF;
const VRAM_ADDRESS: usize = 0x06000000;
const VRAM_END: usize = 0x06017FFF;
const OAM_ADRESS: usize = 0x07000000;
const OAM_END: usize = 0x070003FF;
const ROM_ADDRESS: usize = 0x08000000;
const ROM_END: usize = 0x09FFFFFF;
const SRAM_ADDRESS: usize = 0x0E000000;
const SRAM_END: usize = 0x0E00FFFF;

pub struct Memory {
    bios: [u8; 0x4000],
    rom: Vec<u8>,
    ewram: [u8; 0x40000],
    iwram: [u8; 0x8000],
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            bios: [0; 0x4000],
            rom: Vec::new(),
            ewram: [0; 0x40000],
            iwram: [0; 0x8000],
        }
    }

    pub fn load_bios(&mut self, bios: Vec<u8>) {
        self.bios.copy_from_slice(&bios);
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.rom = rom;
    }
}

impl Index<usize> for Memory {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            BIOS_ADDRESS..=BIOS_END => &self.bios[index],
            EWRAM_ADDRESS..=EWRAM_END => &self.ewram[index],
            IWRAM_ADDRESS..=IWRAM_END => &self.iwram[index],
            IO_REGISTERS..=IO_REGISTERS_END => todo!(),
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => todo!(),
            VRAM_ADDRESS..=VRAM_END => todo!(),
            OAM_ADRESS..=OAM_END => todo!(),
            ROM_ADDRESS..=ROM_END => &self.rom[index],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index),
        }
    }
}

impl IndexMut<usize> for Memory {
    fn index_mut(&mut self, index: usize) -> &mut u8 {
        match index {
            BIOS_ADDRESS..=BIOS_END => &mut self.bios[index],
            EWRAM_ADDRESS..=EWRAM_END => &mut self.ewram[index],
            IWRAM_ADDRESS..=IWRAM_END => &mut self.iwram[index],
            IO_REGISTERS..=IO_REGISTERS_END => todo!(),
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => todo!(),
            VRAM_ADDRESS..=VRAM_END => todo!(),
            OAM_ADRESS..=OAM_END => todo!(),
            ROM_ADDRESS..=ROM_END => &mut self.rom[index],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index),
        }
    }
}

impl Index<Range<usize>> for Memory {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        match index.clone().min().unwrap() {
            BIOS_ADDRESS..=BIOS_END => &self.bios[index],
            EWRAM_ADDRESS..=EWRAM_END => &self.ewram[index],
            IWRAM_ADDRESS..=IWRAM_END => &self.iwram[index],
            IO_REGISTERS..=IO_REGISTERS_END => todo!(),
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => todo!(),
            VRAM_ADDRESS..=VRAM_END => todo!(),
            OAM_ADRESS..=OAM_END => todo!(),
            ROM_ADDRESS..=ROM_END => &self.rom[index],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index.min().unwrap()),
        }
    }
}

impl IndexMut<Range<usize>> for Memory {
    fn index_mut(&mut self, index: Range<usize>) -> &mut [u8] {
        match index.clone().min().unwrap() {
            BIOS_ADDRESS..=BIOS_END => &mut self.bios[index],
            EWRAM_ADDRESS..=EWRAM_END => &mut self.ewram[index],
            IWRAM_ADDRESS..=IWRAM_END => &mut self.iwram[index],
            IO_REGISTERS..=IO_REGISTERS_END => todo!(),
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => todo!(),
            VRAM_ADDRESS..=VRAM_END => todo!(),
            OAM_ADRESS..=OAM_END => todo!(),
            ROM_ADDRESS..=ROM_END => &mut self.rom[index],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index.min().unwrap()),
        }
    }
}