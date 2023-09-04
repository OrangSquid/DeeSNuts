use crate::arm7::Arm7;
use crate::arm7::CARRY_FLAG;

pub trait Alu {
    fn alu_command(&mut self, opcode: u32);

    fn barrel_shifter(&mut self, value: u32, operand: u32, shift_type: u32, register_specified_shift: bool) -> u32;

    fn decode_alu(&mut self, opcode: u32, set_condition_codes: bool, operand_1: u32, destination_register: u32, operand_2: u32);
}

impl Alu for Arm7 {
    fn alu_command(&mut self, opcode: u32) {
        let operand_1 = self.registers[((opcode & 0xF_0000) >> 16) as usize];
        let mut operand_2 = 0;
        let destination_register = (opcode & 0xF000) >> 12;
        // Operand 2 is an immediate value
        if opcode & 0x10_0000 == 0x10_0000 {
            operand_2 = opcode & 0xFF;
            let shift = ((opcode & 0xF00) >> 8) * 2;
            operand_2 = operand_2.rotate_right(shift);
        }
        // Operand 2 is a value in a register
        else {
            operand_2 = self.registers[(opcode & 0xF) as usize];
            // Shift is in a register
            if opcode & 0x10 == 0x10 {
                // Shift is only done using the least significant byte in the register
                let value = self.registers[((opcode & 0xF00) >> 8) as usize] & 0xFF;
                let shift_type = 0x60;
                operand_2 = self.barrel_shifter(value, operand_2, shift_type, true);
            }
            // Shift is an immediate value
            else {
                let value = (opcode & 0xF80) >> 7;
                let shift_type = (opcode & 0x60) >> 5;
                operand_2 = self.barrel_shifter(value, operand_2, shift_type, false);
            }
        }
        let alu_opcode = (opcode & 0x1E0_0000) >> 21;
        let set_condition_codes = (opcode & 0x10_0000) == 0x10_0000;
        self.decode_alu(alu_opcode, set_condition_codes, operand_1, destination_register, operand_2)
    }

    fn barrel_shifter(&mut self, mut value: u32, mut operand: u32, shift_type: u32, register_specified_shift: bool) -> u32 {
        match shift_type {
            0x0 => {
                let carry_bit = (1 as u32) << (32 - value);
                if operand & carry_bit != 0 {
                    self.cpsr_register |= CARRY_FLAG;
                }
                operand << value
            } // LSL
            0x1 => {
                if value == 0 && !register_specified_shift {
                    value = 32;
                }
                let carry_bit = (1 as u32) << value - 1;
                if operand & carry_bit != 0 {
                    self.cpsr_register |= CARRY_FLAG;
                }
                operand >> value
            } // LSR
            0x2 => {
                if value == 0 && !register_specified_shift {
                    value = 32;
                }
                let carry_bit = (1 as u32) << value - 1;
                if operand & carry_bit != 0 {
                    self.cpsr_register |= CARRY_FLAG;
                }
                (operand as i32 >> value) as u32
            } // ASR
            0x3 => {
                if value == 0 && !register_specified_shift {
                    let carry_bit = (operand & 0x1) << 29;
                    operand = operand >> 1;
                    let carry_in = (self.cpsr_register & CARRY_FLAG) << 2;
                    self.cpsr_register |= carry_bit;
                    operand | carry_in
                }
                else {
                    operand.rotate_right(value)
                }
            } // RRS
            _ => panic!()
        }
    }

    fn decode_alu(&mut self, opcode: u32, set_condition_codes: bool, operand_1: u32, destination_register: u32, operand_2: u32) {
        match opcode {
            _ => ()
        }
    }
}