use crate::arm7::Arm7;
use crate::arm7::CARRY_FLAG;
use crate::arm7::OVERFLOW_FLAG;
use crate::arm7::ZERO_FLAG;

pub trait Alu {
    fn alu_command(&mut self, opcode: u32);

    fn barrel_shifter(
        &mut self,
        value: u32,
        operand: u32,
        shift_type: u32,
        register_specified_shift: bool,
    ) -> u32;

    fn decode_alu(
        &mut self,
        opcode: u32,
        set_condition_codes: bool,
        operand_1: u32,
        destination_register: u32,
        operand_2: u32,
    );

    fn and(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32;

    fn exclusive_or(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32;

    fn subtract(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32;

    fn right_subtract(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32;

    fn add(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32;

    fn add_carry(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32;

    fn subtract_carry(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32;

    fn right_subtract_carry(
        &mut self,
        operand_1: u32,
        destination_register: u32,
        operand_2: u32,
    ) -> u32;

    fn tst_and(operand_1: u32, operand_2: u32) -> u32;

    fn exclusive_or_teq(operand_1: u32, operand_2: u32) -> u32;

    fn subtract_cmp(operand_1: u32, operand_2: u32) -> u32;

    fn add_cmn(operand_1: u32, operand_2: u32) -> u32;

    fn orr(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32;

    fn mov(&mut self, destination_register: u32, operand_2: u32) -> u32;

    fn bit_clear(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32;

    fn move_not(&mut self, destination_register: u32, operand_2: u32) -> u32;

    fn set_logical_operations_cpsr_flags(&mut self, result: u32);

    fn set_arithmetic_operations_cpsr_flags(&mut self, operand_1: u32, result: u32, operand_2: u32);
}

impl Arm7 {
    
}

impl Alu for Arm7 {
    fn alu_command(&mut self, opcode: u32) {
        // prefetch compensation
        let current_pc = self.registers[15];
        self.registers[15] += 8;
        let mut operand_2 = 0;
        let destination_register = (opcode & 0xF000) >> 12;
        // Operand 2 is an immediate value
        if opcode & 0x200_0000 == 0x200_0000 {
            operand_2 = opcode & 0xFF;
            let shift = ((opcode & 0xF00) >> 8) * 2;
            operand_2 = operand_2.rotate_right(shift);
        }
        // Operand 2 is a value in a register
        // Either bits 4 and 7 are 1 and 0, respectively or bit 4 is 0
        else if opcode & 0x90 == 0x10 || opcode & 0x10 == 0 {
            operand_2 = self.registers[(opcode & 0xF) as usize];
            // Shift is in a register
            if opcode & 0x10 == 0x10 {
                self.registers[15] += 4;
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
        else {
            panic!();
        }
        let operand_1 = self.registers[((opcode & 0xF_0000) >> 16) as usize];
        let alu_opcode = (opcode & 0x1E0_0000) >> 21;
        let set_condition_codes = (opcode & 0x10_0000) == 0x10_0000;
        self.registers[15] = current_pc;
        self.decode_alu(
            alu_opcode,
            set_condition_codes,
            operand_1,
            destination_register,
            operand_2,
        )
    }

    // TODO make the checks for carry out a seperate functiion
    fn barrel_shifter(
        &mut self,
        mut value: u32,
        mut operand: u32,
        shift_type: u32,
        register_specified_shift: bool,
    ) -> u32 {
        match shift_type {
            0x0 => {
                let carry_bit = (1 as u32) << (32 - value);
                if (operand & carry_bit != 0) && value != 0 {
                    self.cpsr_register |= CARRY_FLAG;
                }
                operand << value
            } // LSL
            0x1 => {
                if value == 0 && !register_specified_shift {
                    value = 32;
                }
                let carry_bit = (1 as u32) << (value - 1);
                if operand & carry_bit != 0 && value != 0 {
                    self.cpsr_register |= CARRY_FLAG;
                }
                operand >> value
            } // LSR
            0x2 => {
                if value == 0 && !register_specified_shift {
                    value = 32;
                }
                let carry_bit = (1 as u32) << (value - 1);
                if operand & carry_bit != 0 && value != 0 {
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
                } else {
                    if value != 0 {
                        let carry_bit = (1 as u32) << (value - 1);
                        if operand & carry_bit != 0 {
                            self.cpsr_register |= CARRY_FLAG;
                        }
                    }
                    operand.rotate_right(value)
                }
            } // RRS
            _ => panic!(),
        }
    }

    fn decode_alu(
        &mut self,
        opcode: u32,
        set_condition_codes: bool,
        operand_1: u32,
        destination_register: u32,
        operand_2: u32,
    ) {
        let result: u32 = match opcode {
            0x0 => self.and(operand_1, destination_register, operand_2),
            0x1 => self.exclusive_or(operand_1, destination_register, operand_2),
            0x2 => self.subtract(operand_1, destination_register, operand_2),
            0x3 => self.right_subtract(operand_1, destination_register, operand_2),
            0x4 => self.add(operand_1, destination_register, operand_2),
            0x5 => self.add_carry(operand_1, destination_register, operand_2),
            0x6 => self.subtract_carry(operand_1, destination_register, operand_2),
            0x7 => self.right_subtract_carry(operand_1, destination_register, operand_2),
            0x8 => Self::tst_and(operand_1, operand_2),
            0x9 => Self::exclusive_or_teq(operand_1, operand_2),
            0xA => Self::subtract_cmp(operand_1, operand_2),
            0xB => Self::add_cmn(operand_1, operand_2),
            0xC => self.orr(operand_1, destination_register, operand_2),
            0xD => self.mov(destination_register, operand_2),
            0xE => self.bit_clear(operand_1, destination_register, operand_2),
            0xF => self.move_not(destination_register, operand_2),
            _ => panic!(),
        };
        if set_condition_codes && destination_register != 15 {
            match opcode {
                0x0 | 0x1 | 0x8 | 0x9 | 0xC | 0xD | 0xE | 0xF => {
                    self.set_logical_operations_cpsr_flags(result)
                }
                0x2 | 0x3 | 0x4 | 0x5 | 0x6 | 0x7 | 0xA | 0xB => {
                    self.set_arithmetic_operations_cpsr_flags(operand_1, result, operand_2)
                }
                _ => (),
            }
        } else if set_condition_codes && destination_register == 15 {
            self.restore_cpsr();
        }
    }

    fn set_logical_operations_cpsr_flags(&mut self, result: u32) {
        self.cpsr_register &= 0xCFFF_FFFF;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        self.cpsr_register |= result & 0x8000_0000;
    }

    fn set_arithmetic_operations_cpsr_flags(
        &mut self,
        operand_1: u32,
        result: u32,
        operand_2: u32,
    ) {
        self.cpsr_register &= 0xFFF_FFFF;
        self.set_logical_operations_cpsr_flags(result);
        if (operand_1 | operand_2) & 0x8000_0000 == 0x8000_0000 && result & 0x8000_0000 == 0 {
            self.cpsr_register |= CARRY_FLAG;
        }
        if (operand_1 & operand_2) & 0x8000_0000 == 0 && result & 0x8000_0000 == 0x8000_0000
            || (operand_1 & operand_2) & 0x8000_0000 == 0x8000_0000 && result & 0x8000_0000 == 0
        {
            self.cpsr_register |= OVERFLOW_FLAG;
        }
    }

    fn and(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] = operand_1 & operand_2;
        self.registers[destination_register as usize]
    }

    fn exclusive_or(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] = operand_1 ^ operand_2;
        self.registers[destination_register as usize]
    }

    fn subtract(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] = operand_1.overflowing_sub(operand_2).0;
        self.registers[destination_register as usize]
    }

    fn right_subtract(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] = operand_2.overflowing_sub(operand_1).0;
        self.registers[destination_register as usize]
    }

    fn add(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] = operand_2.overflowing_add(operand_1).0;
        self.registers[destination_register as usize]
    }

    fn add_carry(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] =
            operand_1.overflowing_add(operand_2.overflowing_add((self.cpsr_register & CARRY_FLAG) >> 29).0).0;
        self.registers[destination_register as usize]
    }

    fn subtract_carry(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] =
            operand_1.overflowing_sub(operand_2.overflowing_add((self.cpsr_register & CARRY_FLAG) >> 29).0).0.overflowing_sub(1).0;
        self.registers[destination_register as usize]
    }

    fn right_subtract_carry(
        &mut self,
        operand_1: u32,
        destination_register: u32,
        operand_2: u32,
    ) -> u32 {
        self.registers[destination_register as usize] =
        operand_2.overflowing_sub(operand_1.overflowing_add((self.cpsr_register & CARRY_FLAG) >> 29).0).0.overflowing_sub(1).0;
        self.registers[destination_register as usize]
    }

    fn tst_and(operand_1: u32, operand_2: u32) -> u32 {
        operand_1 & operand_2
    }

    fn exclusive_or_teq(operand_1: u32, operand_2: u32) -> u32 {
        operand_1 ^ operand_2
    }

    fn subtract_cmp(operand_1: u32, operand_2: u32) -> u32 {
        operand_1 - operand_2
    }

    fn add_cmn(operand_1: u32, operand_2: u32) -> u32 {
        operand_1 + operand_2
    }

    fn orr(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] = operand_1 & operand_2;
        self.registers[destination_register as usize]
    }

    fn mov(&mut self, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] = operand_2;
        self.registers[destination_register as usize]
    }

    fn bit_clear(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] = operand_1 & !operand_2;
        self.registers[destination_register as usize]
    }

    fn move_not(&mut self, destination_register: u32, operand_2: u32) -> u32 {
        self.registers[destination_register as usize] = !operand_2;
        self.registers[destination_register as usize]
    }
}
