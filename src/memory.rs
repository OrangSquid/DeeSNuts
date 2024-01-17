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
            rom: Vec::new()
        }
    }

    pub fn load_bios(&mut self, bios: Vec<u8>) {
        self.bios.copy_from_slice(&bios);
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.rom = rom;
    }

    pub fn get_byte(&self, address: u32) -> u8 {
        self[address as usize]
    }

    pub fn get_halfword(&self, address: u32) -> u16 {
        u16::from_le_bytes(self[address as usize..address as usize + 2].try_into().unwrap())
    }

    pub fn get_word(&self, address: u32) -> u32 {
        u32::from_le_bytes(self[address as usize..address as usize + 4].try_into().unwrap())
    }

    pub fn store_byte(&mut self, address: u32, value: u8) {
        self[address as usize] = value;
    }

    pub fn store_halfword(&mut self, address: u32, value: u16) {
        let address_idx = address as usize;
        self[address_idx..address_idx + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn store_word(&mut self, address: u32, value: u32) {
        let address_idx = address as usize;
        self[address_idx..address_idx + 4].copy_from_slice(&value.to_le_bytes());
    }
}

impl Index<usize> for Memory {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        let usable_bits = index & 0xFF_FFFF;
        match index {
            BIOS_ADDRESS..=BIOS_END => &self.bios[usable_bits],
            EWRAM_ADDRESS..=EWRAM_END => &self.ewram[usable_bits],
            IWRAM_ADDRESS..=IWRAM_END => &self.iwram[usable_bits],
            IO_REGISTERS..=IO_REGISTERS_END => &self.io_registers[usable_bits],
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => &self.pallete_ram[usable_bits],
            VRAM_ADDRESS..=VRAM_END => &self.vram[usable_bits],
            OAM_ADRESS..=OAM_END =>  &self.oam[usable_bits],
            ROM_ADDRESS..=ROM_END => &self.rom[usable_bits],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index),
        }
    }
}

impl IndexMut<usize> for Memory {
    fn index_mut(&mut self, index: usize) -> &mut u8 {
        let masked_index = index & 0xFF_FFFF;
        match index {
            BIOS_ADDRESS..=BIOS_END => &mut self.bios[masked_index],
            EWRAM_ADDRESS..=EWRAM_END => &mut self.ewram[masked_index],
            IWRAM_ADDRESS..=IWRAM_END => &mut self.iwram[masked_index],
            IO_REGISTERS..=IO_REGISTERS_END => &mut self.io_registers[masked_index],
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => &mut self.pallete_ram[masked_index],
            VRAM_ADDRESS..=VRAM_END => &mut self.vram[masked_index],
            OAM_ADRESS..=OAM_END => &mut self.oam[masked_index],
            ROM_ADDRESS..=ROM_END => &mut self.rom[masked_index],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index),
        }
    }
}

impl Index<Range<usize>> for Memory {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        let min = index.start & 0xFF_FFFF;
        let max = index.end.saturating_sub(1) & 0xFF_FFFF;
        match index.start {
            BIOS_ADDRESS..=BIOS_END => &self.bios[min..=max],
            EWRAM_ADDRESS..=EWRAM_END => &self.ewram[min..=max],
            IWRAM_ADDRESS..=IWRAM_END => &self.iwram[min..=max],
            IO_REGISTERS..=IO_REGISTERS_END => &self.io_registers[min..=max],
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => &self.pallete_ram[min..=max],
            VRAM_ADDRESS..=VRAM_END => &self.vram[min..=max],
            OAM_ADRESS..=OAM_END => &self.oam[min..=max],
            ROM_ADDRESS..=ROM_END => &self.rom[min..=max],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index.start),
        }
    }
}

impl IndexMut<Range<usize>> for Memory {
    fn index_mut(&mut self, index: Range<usize>) -> &mut [u8] {
        let min = index.start & 0xFF_FFFF;
        let max = index.end.saturating_sub(1) & 0xFF_FFFF;
        match index.start {
            BIOS_ADDRESS..=BIOS_END => &mut self.bios[min..=max],
            EWRAM_ADDRESS..=EWRAM_END => &mut self.ewram[min..=max],
            IWRAM_ADDRESS..=IWRAM_END => &mut self.iwram[min..=max],
            IO_REGISTERS..=IO_REGISTERS_END => &mut self.io_registers[min..=max],
            PALLETE_RAM_ADDRESS..=PALLETE_RAM_END => &mut self.pallete_ram[min..=max],
            VRAM_ADDRESS..=VRAM_END => &mut self.vram[min..=max],
            OAM_ADRESS..=OAM_END => &mut self.oam[min..=max],
            ROM_ADDRESS..=ROM_END => &mut self.rom[min..=max],
            SRAM_ADDRESS..=SRAM_END => todo!(),
            _ => panic!("Invalid memory address: {:#X}", index.start),
        }
    }
}