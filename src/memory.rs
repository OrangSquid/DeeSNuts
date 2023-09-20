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
    bios: Vec<u8>,
    ewram: Vec<u8>,
    iwram: Vec<u8>,
    io_registers: Vec<u8>,
    pallete_ram: Vec<u8>,
    vram: Vec<u8>,
    oam: Vec<u8>,
    rom: Vec<u8>
}

impl Memory {
    pub fn new() -> Self {
        Self {
            bios: vec![0; 0x4000],
            ewram: vec![0; 0x40000],
            iwram: vec![0; 0x8000],
            io_registers: vec![0; 0x3FF],
            pallete_ram: vec![0; 0x400],
            vram: vec![0; 0x18000],
            oam: vec![0; 0x400],
            rom: Vec::new(),
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
            BIOS_ADDRESS..=BIOS_END => &self.bios[index & 0xFF_FFFF],
            EWRAM_ADDRESS..=EWRAM_END => &self.ewram[index & 0xFF_FFFF],
            IWRAM_ADDRESS..=IWRAM_END => &self.iwram[index & 0xFF_FFFF],
            IO_REGISTERS..=IO_REGISTERS_END => &self.io_registers[index & 0xFF_FFFF],
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => &self.pallete_ram[index & 0xFF_FFFF],
            VRAM_ADDRESS..=VRAM_END => &self.vram[index & 0xFF_FFFF],
            OAM_ADRESS..=OAM_END =>  &self.oam[index & 0xFF_FFFF],
            ROM_ADDRESS..=ROM_END => &self.rom[index & 0xFF_FFFF],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index),
        }
    }
}

impl IndexMut<usize> for Memory {
    fn index_mut(&mut self, index: usize) -> &mut u8 {
        match index {
            BIOS_ADDRESS..=BIOS_END => &mut self.bios[index & 0xFF_FFFF],
            EWRAM_ADDRESS..=EWRAM_END => &mut self.ewram[index & 0xFF_FFFF],
            IWRAM_ADDRESS..=IWRAM_END => &mut self.iwram[index & 0xFF_FFFF],
            IO_REGISTERS..=IO_REGISTERS_END => &mut self.io_registers[index & 0xFF_FFFF],
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => &mut self.pallete_ram[index & 0xFF_FFFF],
            VRAM_ADDRESS..=VRAM_END => &mut self.vram[index & 0xFF_FFFF],
            OAM_ADRESS..=OAM_END => &mut self.oam[index & 0xFF_FFFF],
            ROM_ADDRESS..=ROM_END => &mut self.rom[index & 0xFF_FFFF],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index),
        }
    }
}

impl Index<Range<usize>> for Memory {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        match index.clone().min().unwrap() {
            BIOS_ADDRESS..=BIOS_END => &self.bios[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            EWRAM_ADDRESS..=EWRAM_END => &self.ewram[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            IWRAM_ADDRESS..=IWRAM_END => &self.iwram[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            IO_REGISTERS..=IO_REGISTERS_END => &self.io_registers[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => &self.pallete_ram[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            VRAM_ADDRESS..=VRAM_END => &self.vram[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            OAM_ADRESS..=OAM_END => &self.oam[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            ROM_ADDRESS..=ROM_END => &self.rom[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index.min().unwrap()),
        }
    }
}

impl IndexMut<Range<usize>> for Memory {
    fn index_mut(&mut self, index: Range<usize>) -> &mut [u8] {
        match index.clone().min().unwrap() {
            BIOS_ADDRESS..=BIOS_END => &mut self.bios[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            EWRAM_ADDRESS..=EWRAM_END => &mut self.ewram[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            IWRAM_ADDRESS..=IWRAM_END => &mut self.iwram[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            IO_REGISTERS..=IO_REGISTERS_END => &mut self.io_registers[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => &mut self.pallete_ram[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            VRAM_ADDRESS..=VRAM_END => &mut self.vram[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            OAM_ADRESS..=OAM_END => &mut self.oam[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            ROM_ADDRESS..=ROM_END => &mut self.rom[index.clone().min().unwrap() & 0xFF_FFFF..index.max().unwrap() & 0xFF_FFFF],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index.min().unwrap()),
        }
    }
}