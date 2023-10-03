use crate::arm7::Arm7;
use crate::arm7::CARRY_FLAG;
use crate::arm7::OVERFLOW_FLAG;
use crate::arm7::ZERO_FLAG;
use crate::check_bit;

impl Arm7 {
    pub fn alu_command(&mut self, opcode: u32) {
        // prefetch compensation
        let current_pc = self.registers[15];
        self.registers[15] += 8;
        let mut operand_2 = 0;
        let destination_register = Self::get_rd_register_number(opcode) as u32;
        let set_condition_codes = check_bit!(opcode, 20);
        // Operand 2 is an immediate value
        if opcode & 0x200_0000 == 0x200_0000 {
            operand_2 = opcode & 0xFF;
            let shift = ((opcode & 0xF00) >> 8) * 2;
            operand_2 = self.barrel_shifter(shift, operand_2, 0x3, true, set_condition_codes);
        }
        // Operand 2 is a value in a register
        // Either bits 4 and 7 are 1 and 0, respectively or bit 4 is 0
        else if opcode & 0x90 == 0x10 || opcode & 0x10 == 0 {
            operand_2 = self.get_rm_register_value(opcode);
            // Shift is in a register
            if opcode & 0x10 == 0x10 {
                self.registers[15] += 4;
                // Shift is only done using the least significant byte in the register
                let value = self.get_rs_register_value(opcode) & 0xFF;
                let shift_type = 0x60;
                operand_2 = self.barrel_shifter(value, operand_2, shift_type, true, set_condition_codes);
            }
            // Shift is an immediate value
            else {
                let value = (opcode & 0xF80) >> 7;
                let shift_type = (opcode & 0x60) >> 5;
                operand_2 = self.barrel_shifter(value, operand_2, shift_type, false, set_condition_codes);
            }
        } else {
            panic!();
        }
        let operand_1 = self.get_rn_register_value(opcode);
        let alu_opcode = (opcode & 0x1E0_0000) >> 21;
        self.registers[15] = current_pc;
        self.decode_alu(
            alu_opcode,
            set_condition_codes,
            operand_1,
            destination_register,
            operand_2,
        )
    }

    #[inline(always)]
    fn check_carry(&mut self, operand: u32, value: u32, carry_bit: u32) {
        if (operand & carry_bit != 0) && value != 0 {
            self.cpsr_register |= CARRY_FLAG;
        }
    }

    pub fn barrel_shifter(&mut self, mut value: u32, mut operand: u32, shift_type: u32, register_specified_shift: bool, set_condition_codes: bool) -> u32 {
        let mut carry_bit = 0;
        let mut new_operand = 0;
        match shift_type {
            0x0 => {
                carry_bit = (1u32) << (31 - value);
                new_operand = operand << value
            } // LSL
            0x1 => {
                if value == 0 && !register_specified_shift {
                    value = 32;
                }
                carry_bit = (1u32) << (value - 1);
                new_operand = operand >> value
            } // LSR
            0x2 => {
                if value == 0 && !register_specified_shift {
                    value = 32;
                }
                carry_bit = (1u32) << (value - 1);
                new_operand = (operand as i32 >> value) as u32;
            } // ASR
            0x3 => {
                if value == 0 && !register_specified_shift {
                    let carry_in = (self.cpsr_register & CARRY_FLAG) << 2;
                    if set_condition_codes {
                        let carry_bit = (operand & 0x1) << 29;
                        self.cpsr_register |= carry_bit;
                    }
                    operand = operand >> 1;
                    return operand | carry_in;
                } else {
                    if value != 0 {
                        carry_bit = (1u32) << (value - 1);
                    }
                    new_operand = operand.rotate_right(value)
                }
            } // RRS
            _ => panic!(),
        }
        if set_condition_codes {
            self.check_carry(operand, value, carry_bit);
        }
        new_operand
    }

    fn decode_alu(
        &mut self,
        opcode: u32,
        set_condition_codes: bool,
        operand_1: u32,
        destination_register: u32,
        operand_2: u32,
    ) {
        let (result, carry, overflow) = match opcode {
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
            self.set_logical_operations_cpsr_flags(result);
            match opcode {
                0x2 | 0x3 | 0x4 | 0x5 | 0x6 | 0x7 | 0xA | 0xB => {
                    self.set_arithmetic_operations_cpsr_flags(carry, overflow);
                }
                _ => (),
            }
        } else if set_condition_codes && destination_register == 15 {
            self.restore_cpsr();
        }
    }

    fn set_logical_operations_cpsr_flags(&mut self, result: u32) {
        self.cpsr_register &= 0x3FFF_FFFF;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        self.cpsr_register |= result & 0x8000_0000;
    }

    fn set_arithmetic_operations_cpsr_flags(&mut self, carry: bool, overflow: bool) {
        self.cpsr_register &= 0xCFFF_FFFF;
        if carry {
            self.cpsr_register |= CARRY_FLAG;
        }
        if overflow {
            self.cpsr_register |= OVERFLOW_FLAG;
        }
    }

    fn and(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        self.registers[destination_register as usize] = operand_1 & operand_2;
        (self.registers[destination_register as usize], false, false)
    }

    fn exclusive_or(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        self.registers[destination_register as usize] = operand_1 ^ operand_2;
        (self.registers[destination_register as usize], false, false)
    }

    fn subtract(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        let (result, carry) = operand_1.overflowing_sub(operand_2);
        self.registers[destination_register as usize] = result;
        (result, !carry, (check_bit!(operand_1, 31) == check_bit!(-(operand_2 as i32), 31)) && (check_bit!(operand_1, 31)) != check_bit!(result, 31))
    }

    fn right_subtract(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        let (result, carry) = operand_2.overflowing_sub(operand_1);
        self.registers[destination_register as usize] = result;
        (result, !carry, (check_bit!(operand_1, 31) == check_bit!(-(operand_2 as i32), 31)) && (check_bit!(operand_1, 31)) != check_bit!(result, 31))
    }

    fn add(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        let (result, carry) = operand_1.overflowing_add(operand_2);
        self.registers[destination_register as usize] = result;
        (result, carry, (check_bit!(operand_1, 31) == check_bit!(operand_2, 31)) && (check_bit!(operand_1, 31)) != check_bit!(result, 31))
    }

    fn add_carry(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        let operand_2 = operand_2.wrapping_add((check_bit!(self.cpsr_register, 29)) as u32);
        self.add(operand_1, destination_register, operand_2)
    }

    fn subtract_carry(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        let operand_2 = (!operand_2 + 1).wrapping_sub(1 * (check_bit!(self.cpsr_register, 29)) as u32);
        self.add(operand_1, destination_register, operand_2)
    }

    fn right_subtract_carry(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        let operand_1 = (!operand_1 + 1).wrapping_sub(1 * (check_bit!(self.cpsr_register, 29)) as u32);
        self.add(operand_1, destination_register, operand_2)
    }

    fn tst_and(operand_1: u32, operand_2: u32) -> (u32, bool, bool) {
        (operand_1 & operand_2, false, false)
    }

    fn exclusive_or_teq(operand_1: u32, operand_2: u32) -> (u32, bool, bool) {
        (operand_1 ^ operand_2, false, false)
    }

    fn subtract_cmp(operand_1: u32, operand_2: u32) -> (u32, bool, bool) {
        let (result, carry) = operand_2.overflowing_sub(operand_1);
        (result, !carry, (check_bit!(operand_1, 31) == check_bit!(-(operand_2 as i32), 31)) && (check_bit!(operand_1, 31)) != check_bit!(result, 31))
    }

    fn add_cmn(operand_1: u32, operand_2: u32) -> (u32, bool, bool) {
        let (result, carry) = operand_1.overflowing_add(operand_2);
        (result, carry, (check_bit!(operand_1, 31) == check_bit!(operand_2, 31)) && (check_bit!(operand_1, 31)) != check_bit!(result, 31))
    }

    fn orr(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        self.registers[destination_register as usize] = operand_1 | operand_2;
        (self.registers[destination_register as usize], false, false)
    }

    fn mov(&mut self, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        self.registers[destination_register as usize] = operand_2;
        (self.registers[destination_register as usize], false, false)
    }

    fn bit_clear(&mut self, operand_1: u32, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        self.registers[destination_register as usize] = operand_1 & !operand_2;
        (self.registers[destination_register as usize], false, false)
    }

    fn move_not(&mut self, destination_register: u32, operand_2: u32) -> (u32, bool, bool) {
        self.registers[destination_register as usize] = !operand_2;
        (self.registers[destination_register as usize], false, false)
    }
}
