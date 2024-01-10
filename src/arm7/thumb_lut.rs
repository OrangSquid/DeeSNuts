use crate::check_bit;

use super::cpu::Cpu;

use super::constants::*;

fn dummy(cpu: &mut Cpu, foo: u32) { }

pub const fn instruction_lut() -> [InstructionHandler; 4096] {
    let dummy: InstructionHandler = dummy;
    let mut temp = [dummy; 4096];
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

const fn decode_thumb(bits27_20: u8, bits7_4: u8) -> InstructionHandler {
    dummy
}

/* fn decode_thumb(&mut self, opcode: u16) {
    match opcode & 0xE000 {
        0x0 => match  opcode & 0x1800 {
            0x1800 => (),
            _ => self.convert_move_shifted(opcode)
        },
        0x2000 => self.convert_alu_immediate(opcode),
        0x4000 => match opcode & 0x1000 {
            0x1000 => (),
            _ => match opcode & 0x800 {
                0x800 => (),
                _ => match opcode & 0x400 {
                    0x400 => self.hi_register_operation(opcode),
                    _ => ()
                }
            },
        },
        0x6000 => (),
        0x8000 => (),
        0xA000 => self.load_address(opcode),
        0xC000 => (),
        0xE000 => (),
        _ => panic!("Undefinied instruction"),
    }
}

fn convert_move_shifted(&mut self, opcode: u16) {
    let source_register = Self::get_rs_t_register_number(opcode) as u32;
    let destination_register = (Self::get_rd_t_register_number(opcode) as u32) << 12;
    let offset = ((opcode << 1) & 0xF80) as u32;
    let opcode = 0xE1B0_0000 | source_register | destination_register | offset;
    self.alu_command(opcode);
}

fn convert_alu_immediate(&mut self, opcode: u16) {
    let src_dst_register = Self::get_rb_t_register_number(opcode) as u32;
    let alu_opcode = match (opcode >> 11) & 0x2 {
        0x0 => 0xD,
        0x1 => 0xA,
        0x2 => 0x4,
        0x3 => 0x2,
        _ => panic!("Undefined opcode")
    };
    let offset = opcode & 0xFF;
    let opcode = 0xE3B0_0000 | (alu_opcode << 21) | (src_dst_register << 16) | (src_dst_register << 12) | offset as u32;
    self.alu_command(opcode);
}

fn load_address(&mut self, opcode: u16) {
    self.registers[15] += 4;
    let source_register = if check_bit!(opcode, 11) { 13 } else { 15 };
    let destination_register = Self::get_rb_t_register_number(opcode);
    let offset = ((opcode & 0xFF) << 2) as u32;
    self.registers[destination_register] = offset + self.registers[source_register];
    self.registers[15] -= 4;
}

fn hi_register_operation(&mut self, opcode: u16) {
    let destination_register = ((opcode as u32 >> 4) & 0x8) | Self::get_rd_t_register_number(opcode) as u32;
    let source_register = ((opcode as u32 >> 3) & 0x8) | Self::get_rs_t_register_number(opcode) as u32;
    let alu_opcode = match (opcode >> 8) & 0x3 {
        0x0 => 0x80_0000,
        0x1 => 0x130_0000,
        0x2 => 0x1A0_0000,
        0x3 => {
            self.branch_and_exchange(source_register);
            return;
        }
        _ => panic!("Undefined opcode")
    };
    let opcode = alu_opcode | (destination_register << 16) | (destination_register << 12) | source_register;
    self.alu_command(opcode);
} */