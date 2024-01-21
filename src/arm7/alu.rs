use crate::{ check_bit, get_register_number_at };

use super::constants::*;
use super::cpu::Cpu;

impl Cpu {
    pub(super) fn get_operand2(
        &mut self,
        operand2_type: Operand2Type,
        shift_type: ShiftType,
        set_condition_codes: bool,
        opcode: u32
    ) -> u32 {
        
        match operand2_type {
            Operand2Type::RegisterWithRegisterShift => {
                self.registers[15] += 4;
                self.memory.borrow_mut().add_clock_cycles(1);
                let operand = self.registers[get_register_number_at!(opcode, 0)];
                let value = self.registers[get_register_number_at!(opcode, 8)] & 0xff;
                self.barrel_shifter(value, operand, shift_type, true, set_condition_codes)
            }
            Operand2Type::RegisterWithImmediateShift => {
                let value = (opcode & 0xf80) >> 7;
                let operand = self.registers[get_register_number_at!(opcode, 0)];
                self.barrel_shifter(value, operand, shift_type, false, set_condition_codes)
            }
            Operand2Type::ImmediateWithRotation => {
                let operand = opcode & 0xff;
                let shift = ((opcode & 0xf00) >> 8) * 2;
                self.barrel_shifter(
                    shift,
                    operand,
                    ShiftType::RotateRight,
                    true,
                    set_condition_codes
                )
            }
            Operand2Type::Immediate => opcode & 0xff,
        }
    }

    fn check_carry(&mut self, operand: u32, value: u32, carry_bit: u32) {
        self.cpsr_register &= 0xdfff_ffff;
        if (operand & carry_bit) != 0 && value != 0 {
            self.cpsr_register |= CARRY_FLAG;
        }
    }

    pub fn barrel_shifter(
        &mut self,
        mut value: u32,
        mut operand: u32,
        shift_type: ShiftType,
        register_specified_shift: bool,
        set_condition_codes: bool
    ) -> u32 {
        let mut carry_bit = 0;
        let mut new_operand = 0;
        match shift_type {
            ShiftType::LogicalLeft => {
                carry_bit = (1u32).checked_shl((32u32).wrapping_sub(value)).unwrap_or(0);
                new_operand = operand.checked_shl(value).unwrap_or(0);
            }
            ShiftType::LogicalRight => {
                if value == 0 && !register_specified_shift {
                    value = 32;
                }
                carry_bit = (1u32).checked_shl(value.wrapping_sub(1)).unwrap_or(0);
                new_operand = operand.checked_shr(value).unwrap_or(0);
            }
            ShiftType::ArithmeticRight => {
                if value == 0 && !register_specified_shift {
                    value = 32;
                }
                carry_bit = (1u32).checked_shl(value.wrapping_sub(1)).unwrap_or(0);
                new_operand = (operand as i32)
                    .checked_shr(value)
                    .unwrap_or((operand as i32) >> 31) as u32;
            }
            ShiftType::RotateRight => {
                if value == 0 && !register_specified_shift {
                    let carry_in = (self.cpsr_register & CARRY_FLAG) << 2;
                    if set_condition_codes {
                        self.cpsr_register &= 0xdfff_ffff;
                        let carry_bit = (operand & 0x1) << 29;
                        self.cpsr_register |= carry_bit;
                    }
                    operand = operand >> 1;
                    return operand | carry_in;
                } else {
                    if value != 0 {
                        carry_bit = (1u32).rotate_left(value.wrapping_sub(1));
                    }
                    new_operand = operand.rotate_right(value);
                }
            }
        }
        if set_condition_codes && value != 0 {
            self.check_carry(operand, value, carry_bit);
        }
        new_operand
    }

    pub(super) fn decode_alu(
        &mut self,
        opcode: AluOpcode,
        set_condition_codes: bool,
        operand_1_register: usize,
        destination_register: usize,
        operand_2: u32
    ) {
        let operand_1 = self.registers[operand_1_register];
        let (result, carry, overflow) = match opcode {
            AluOpcode::And => self.and(operand_1, destination_register, operand_2),
            AluOpcode::ExclusiveOr => self.exclusive_or(operand_1, destination_register, operand_2),
            AluOpcode::Subtract => self.subtract(operand_1, destination_register, operand_2),
            AluOpcode::RightSubtract =>
                self.right_subtract(operand_1, destination_register, operand_2),
            AluOpcode::Add => self.add(operand_1, destination_register, operand_2),
            AluOpcode::AddCarry => self.add_carry(operand_1, destination_register, operand_2),
            AluOpcode::SubtractCarry =>
                self.subtract_carry(operand_1, destination_register, operand_2),
            AluOpcode::RightSubtractCarry =>
                self.right_subtract_carry(operand_1, destination_register, operand_2),
            AluOpcode::TestAnd => Self::tst_and(operand_1, operand_2),
            AluOpcode::TestExclusiveOr => Self::exclusive_or_teq(operand_1, operand_2),
            AluOpcode::CompareSubtract => Self::subtract_cmp(operand_1, operand_2),
            AluOpcode::CompareAdd => Self::add_cmn(operand_1, operand_2),
            AluOpcode::Or => self.orr(operand_1, destination_register, operand_2),
            AluOpcode::Move => self.mov(destination_register, operand_2),
            AluOpcode::BitClear => self.bit_clear(operand_1, destination_register, operand_2),
            AluOpcode::MoveNot => self.move_not(destination_register, operand_2)
        };
        if set_condition_codes {
            self.set_logical_operations_cpsr_flags(result);
            match opcode {
                | AluOpcode::Subtract
                | AluOpcode::RightSubtract
                | AluOpcode::Add
                | AluOpcode::AddCarry
                | AluOpcode::SubtractCarry
                | AluOpcode::RightSubtractCarry
                | AluOpcode::CompareSubtract
                | AluOpcode::CompareAdd => {
                    self.set_arithmetic_operations_cpsr_flags(carry, overflow);
                }
                _ => (),
            }
            if destination_register == 15 {
                self.restore_cpsr();
            }
        }
    }

    fn set_logical_operations_cpsr_flags(&mut self, result: u32) {
        self.cpsr_register &= 0x3fff_ffff;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        self.cpsr_register |= result & 0x8000_0000;
    }

    fn set_arithmetic_operations_cpsr_flags(&mut self, carry: bool, overflow: bool) {
        self.cpsr_register &= 0xcfff_ffff;
        if carry {
            self.cpsr_register |= CARRY_FLAG;
        }
        if overflow {
            self.cpsr_register |= OVERFLOW_FLAG;
        }
    }

    fn and(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        self.registers[destination_register] = operand_1 & operand_2;
        (self.registers[destination_register], false, false)
    }

    fn exclusive_or(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        self.registers[destination_register] = operand_1 ^ operand_2;
        (self.registers[destination_register], false, false)
    }

    fn subtract(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        let (result, carry) = operand_1.overflowing_sub(operand_2);
        self.registers[destination_register] = result;
        (
            result,
            !carry,
            check_bit!(operand_1, 31) == check_bit!(!operand_2, 31) &&
                check_bit!(operand_1, 31) != check_bit!(result, 31),
        )
    }

    fn right_subtract(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        let (result, carry) = operand_2.overflowing_sub(operand_1);
        self.registers[destination_register] = result;
        (
            result,
            !carry,
            check_bit!(operand_2, 31) == check_bit!(!operand_1, 31) &&
                check_bit!(operand_1, 31) != check_bit!(result, 31),
        )
    }

    fn add(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        let (result, carry) = operand_1.overflowing_add(operand_2);
        self.registers[destination_register as usize] = result;
        (
            result,
            carry,
            check_bit!(operand_1, 31) == check_bit!(operand_2, 31) &&
                check_bit!(operand_1, 31) != check_bit!(result, 31),
        )
    }

    fn add_carry(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        let operand_2 = operand_2.wrapping_add(check_bit!(self.cpsr_register, 29) as u32);
        self.add(operand_1, destination_register, operand_2)
    }

    fn subtract_carry(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        let operand_2 = operand_2.wrapping_sub(
            u32::MAX * (!check_bit!(self.cpsr_register, 29) as u32)
        );
        self.subtract(operand_1, destination_register, operand_2)
    }

    fn right_subtract_carry(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        let operand_1 = operand_1.wrapping_sub(
            u32::MAX * (!check_bit!(self.cpsr_register, 29) as u32)
        );
        self.right_subtract(operand_1, destination_register, operand_2)
    }

    fn tst_and(operand_1: u32, operand_2: u32) -> (u32, bool, bool) {
        (operand_1 & operand_2, false, false)
    }

    fn exclusive_or_teq(operand_1: u32, operand_2: u32) -> (u32, bool, bool) {
        (operand_1 ^ operand_2, false, false)
    }

    fn subtract_cmp(operand_1: u32, operand_2: u32) -> (u32, bool, bool) {
        let (result, carry) = operand_1.overflowing_sub(operand_2);
        (
            result,
            !carry,
            check_bit!(operand_1, 31) == check_bit!(!operand_2, 31) &&
                check_bit!(operand_1, 31) != check_bit!(result, 31),
        )
    }

    fn add_cmn(operand_1: u32, operand_2: u32) -> (u32, bool, bool) {
        let (result, carry) = operand_1.overflowing_add(operand_2);
        (
            result,
            carry,
            check_bit!(operand_1, 31) == check_bit!(operand_2, 31) &&
                check_bit!(operand_1, 31) != check_bit!(result, 31),
        )
    }

    fn orr(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        self.registers[destination_register] = operand_1 | operand_2;
        (self.registers[destination_register], false, false)
    }

    fn mov(&mut self, destination_register: usize, operand_2: u32) -> (u32, bool, bool) {
        self.registers[destination_register] = operand_2;
        (self.registers[destination_register], false, false)
    }

    fn bit_clear(
        &mut self,
        operand_1: u32,
        destination_register: usize,
        operand_2: u32
    ) -> (u32, bool, bool) {
        self.registers[destination_register] = operand_1 & !operand_2;
        (self.registers[destination_register], false, false)
    }

    fn move_not(&mut self, destination_register: usize, operand_2: u32) -> (u32, bool, bool) {
        self.registers[destination_register] = !operand_2;
        (self.registers[destination_register], false, false)
    }
}
