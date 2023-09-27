use std::{cell::RefCell, rc::Rc, time::Instant};

use crate::memory::Memory;

// CPU modes
const USER_MODE: u32 = 0x10;
const FIQ_MODE: u32 = 0x11;
const IRQ_MODE: u32 = 0x12;
const SUPERVISOR_MODE: u32 = 0x13;
const ABORT_MODE: u32 = 0x17;
const UNDEFINED_MODE: u32 = 0x1B;
const SYSTEM_MODE: u32 = 0x1F;

const START_PC: u32 = 0x800_0000;

const STACK_USER_SYSTEM_START: u32 = 0x300_7F00;
const STACK_IRQ_START: u32 = 0x300_7FA0;
const STACK_SUPERVISOR_START: u32 = 0x0300_7FE0;

// Position of the bits in the CPSR register
pub const SIGN_FLAG: u32 = 0x8000_0000;
pub const ZERO_FLAG: u32 = 0x4000_0000;
pub const CARRY_FLAG: u32 = 0x2000_0000;
pub const OVERFLOW_FLAG: u32 = 0x1000_0000;
const IRQ_BIT: u32 = 0x80;
const FIQ_BIT: u32 = 0x40;
const STATE_BIT: u32 = 0x20;
const CONDITION_LUT: [bool; 256] = build_condition_lut();

const fn build_condition_lut() -> [bool; 256] {
    const SIGN_FLAG: u8 = 0x8;
    const ZERO_FLAG: u8 = 0x4;
    const CARRY_FLAG: u8 = 0x2;
    const OVERFLOW_FLAG: u8 = 0x1;

    let mut temp = [false; 256];
    let mut last_index = 0;

    while last_index != 256 {
        let condition_code = ((last_index & 0xF0) >> 4) as u8;
        let flag_set = (last_index & 0xF) as u8;
        temp[last_index] = match condition_code {
            0x0 => flag_set & ZERO_FLAG != 0,
            0x1 => flag_set & ZERO_FLAG == 0,
            0x2 => flag_set & CARRY_FLAG != 0,
            0x3 => flag_set & CARRY_FLAG == 0,
            0x4 => flag_set & SIGN_FLAG != 0,
            0x5 => flag_set & SIGN_FLAG == 0,
            0x6 => flag_set & OVERFLOW_FLAG != 0,
            0x7 => flag_set & OVERFLOW_FLAG == 0,
            0x8 => flag_set & (CARRY_FLAG | ZERO_FLAG) == CARRY_FLAG,
            0x9 => flag_set & CARRY_FLAG == 0 || flag_set & ZERO_FLAG != 0,
            0xA => (flag_set >> 3) == (flag_set & OVERFLOW_FLAG),
            0xB => (flag_set >> 3) != (flag_set & OVERFLOW_FLAG),
            0xC => flag_set & ZERO_FLAG == 0 && (flag_set >> 3) != (flag_set & OVERFLOW_FLAG),
            0xD => flag_set & ZERO_FLAG == 0 && (flag_set >> 3) == (flag_set & OVERFLOW_FLAG),
            0xE => true,
            0xF => true,
            _ => panic!("Condition for opcode is higher than 0xF"),
        };
        last_index += 1;
    }
    temp
}

 #[macro_export]
macro_rules! check_bit {
    ($opcode:expr, $bit:expr) => {
        $opcode & (1 << $bit) == (1 << $bit)
    };
}

// TODO Fix visibility problems
pub struct Arm7 {
    memory: Rc<RefCell<Memory>>,
    pub registers: [u32; 16],
    // Current Program Status Register
    pub cpsr_register: u32,
    // Each u32 is a banked spsr (Saved Program Status Register)
    saved_psr: [u32; 5],
    // The banked out registers when switched out of user/system mode
    fiq_lo_banked: [u32; 5],
    user_banked: [u32; 2],
    fiq_hi_banked: [u32; 2],
    supervisor_banked: [u32; 2],
    abort_banked: [u32; 2],
    irq_banked: [u32; 2],
    undefinied_banked: [u32; 2],
}

impl Arm7 {
    pub fn new(memory: Rc<RefCell<Memory>>) -> Arm7 {
        let mut arm7 = Arm7 {
            memory,
            registers: [0; 16],
            cpsr_register: SYSTEM_MODE as u32,
            saved_psr: [0; 5],
            fiq_lo_banked: [0; 5],
            user_banked: [0; 2],
            fiq_hi_banked: [0; 2],
            supervisor_banked: [0; 2],
            abort_banked: [0; 2],
            irq_banked: [0; 2],
            undefinied_banked: [0; 2],
        };
        arm7.registers[13] = STACK_USER_SYSTEM_START;
        arm7.registers[13] = STACK_IRQ_START;
        arm7.registers[13] = STACK_SUPERVISOR_START;
        arm7.registers[15] = START_PC;
        arm7
    }

    pub fn next(&mut self) {
        // THUMB MODE
        if self.cpsr_register & STATE_BIT == STATE_BIT {
            self.registers[15] += 2;
        }
        // ARM MODE
        else {
            let opcode = self.fetch_arm();
            // Clear the bottom 2 bits
            self.decode_arm(opcode);
            self.registers[15] += 4;
        }
    }

    fn fetch_arm(&mut self) -> u32 {
        //println!("Fetching at {:#08x}", self.registers[15]);
        let address = self.registers[15] & 0xFFFF_FFFC;
        self
            .memory
            .borrow()
            .get_word(self.registers[15] & 0xFFFF_FFFC)
    }

    fn fetch_thumb(&mut self) -> u16 {
        0
    }

    // TODO: a single data transfer opcode might be an undefinied instruction, should take care
    // of it at a later date
    fn decode_arm(&mut self, opcode: u32) {
        //println!("Decoding {:#08x}", opcode);
        if !CONDITION_LUT[(((opcode >> 24) & 0xF0) | self.cpsr_register >> 28) as usize] {
            return;
        }
        //let lmao = Instant::now();
        match opcode & (0x3 << 26) {
            0x0 => match opcode & 0x90 {
                0x0 | 0x80 => self.sr_or_alu(opcode),
                0x10 => match opcode & 0x12F_FF10 {
                    0x12F_FF10 => self.branch_and_exchange(opcode),
                    _ => self.sr_or_alu(opcode),
                },
                0x90 => match opcode & 0x60 {
                    0x0 => match opcode & 0x180_0000 {
                        0x0 => self.multiply(opcode),
                        0x80_0000 => self.multiply_long(opcode),
                        0x100_0000 => self.single_data_swap(opcode),
                        _ => panic!(),
                    },
                    _ => {
                        if opcode & 0x40_0000 == 0x40_0000 {
                            self.halfword_data_transfer(opcode);
                        }
                    }
                },
                _ => panic!("Undefinied instruction"),
            },
            0x400_0000 => self.single_data_transfer(opcode),
            0x800_0000 => match opcode & 0x200_0000 {
                0x0 => self.block_data_transfer(opcode),
                0x200_0000 => self.branch(opcode),
                _ => panic!("Undefinied instruction"),
            },
            _ => panic!("Undefinied instruction"),
        }
        /* println!(
            "execute: {}",
            Instant::now().duration_since(lmao).as_secs_f64()
        ); */
    }
    
    fn decode_thumb(&mut self, opcode: u16) {}

    fn switch_modes(&mut self, old_mode: u32) {
        match old_mode {
            USER_MODE | SYSTEM_MODE => self.user_banked.copy_from_slice(&mut self.registers[13..15]),
            FIQ_MODE => {
                // Swap beacuse the other modes all share R8 through R12
                self.registers[8..13].swap_with_slice(&mut self.fiq_lo_banked);
                self.fiq_hi_banked.copy_from_slice(&mut self.registers[13..15]);
            }
            SUPERVISOR_MODE => self.supervisor_banked.copy_from_slice(&mut self.registers[13..15]),
            ABORT_MODE => self.abort_banked.copy_from_slice(&mut self.registers[13..15]),
            IRQ_MODE => self.irq_banked.copy_from_slice(&mut self.registers[13..15]),
            UNDEFINED_MODE => self.undefinied_banked.copy_from_slice(&mut self.registers[13..15]),
            _ => panic!("Unrecognized mode"),
        }
        match self.cpsr_register & 0x1F {
            USER_MODE | SYSTEM_MODE => self.registers[13..15].copy_from_slice(&self.user_banked),
            FIQ_MODE => {
                // Swap beacuse the other modes all share R8 through R12
                self.fiq_lo_banked.swap_with_slice(&mut self.registers[8..13]);
                self.registers[13..15].copy_from_slice(&self.fiq_hi_banked);
            }
            SUPERVISOR_MODE => self.registers[13..15].copy_from_slice(&self.supervisor_banked),
            ABORT_MODE => self.registers[13..15].copy_from_slice(&self.abort_banked),
            IRQ_MODE => self.registers[13..15].copy_from_slice(&self.irq_banked),
            UNDEFINED_MODE => self.registers[13..15].copy_from_slice(&self.undefinied_banked),
            _ => panic!("Unrecognized mode")
        }
    }

    fn sr_or_alu(&mut self, opcode: u32) {
        match opcode & 0x1F0_0000 {
            0x100_0000 | 0x120_0000 | 0x140_0000 | 0x160_0000 => self.sr_operation(opcode),
            _ => self.alu_command(opcode & 0x3FF_FFFF),
        }
    }

    #[inline(always)]
    fn get_rn_register_number(opcode: u32) -> usize {
        ((opcode >> 16) & 0xF) as usize
    }

    #[inline(always)]
    fn get_rd_register_number(opcode: u32) -> usize {
        ((opcode >> 12) & 0xF) as usize
    }

    #[inline(always)]
    fn get_rs_register_number(opcode: u32) -> usize {
        ((opcode >> 8) & 0xF) as usize
    }

    #[inline(always)]
    fn get_rm_register_number(opcode: u32) -> usize {
        (opcode & 0xF) as usize
    }

    #[inline(always)]
    fn get_rn_register_value(&self, opcode: u32) -> u32 {
        self.registers[Self::get_rn_register_number(opcode)]
    }

    #[inline(always)]
    fn get_rd_register_value(&self, opcode: u32) -> u32 {
        self.registers[Self::get_rd_register_number(opcode)]
    }

    #[inline(always)]
    fn get_rs_register_value(&self, opcode: u32) -> u32 {
        self.registers[Self::get_rs_register_number(opcode)]
    }

    #[inline(always)]
    fn get_rm_register_value(&self, opcode: u32) -> u32 {
        self.registers[Self::get_rm_register_number(opcode)]
    }

    fn branch_and_exchange(&mut self, opcode: u32) {
        let address = self.get_rm_register_value(opcode);
        let thumb_bit = address & 0x1;
        self.cpsr_register = self.cpsr_register | (thumb_bit << 5);
        self.registers[15] = address;
    }

    fn branch(&mut self, opcode: u32) {
        // Due to prefetching, the PC should be 8 bytes ahead
        let offset = (opcode & 0xFF_FFFF) as i32;
        let correct_ofset = ((offset << 8) >> 6) + 4;
        if check_bit!(opcode, 24) {
            self.registers[14] = self.registers[15]
        }
        self.registers[15] = ((self.registers[15] as i32) + correct_ofset) as u32;
    }

    fn get_current_saved_psr(&mut self) -> &mut u32 {
        match self.cpsr_register & 0x1F {
            USER_MODE => &mut self.cpsr_register,
            FIQ_MODE => &mut self.saved_psr[0],
            IRQ_MODE => &mut self.saved_psr[1],
            SUPERVISOR_MODE => &mut self.saved_psr[2],
            ABORT_MODE => &mut self.saved_psr[3],
            UNDEFINED_MODE => &mut self.saved_psr[4],
            SYSTEM_MODE => &mut self.cpsr_register,
            _ => panic!("CPU is in an unrecognized mode"),
        }
    }

    pub fn restore_cpsr(&mut self) {
        let old_mode = self.cpsr_register & 0x1F;
        self.cpsr_register = *self.get_current_saved_psr();
        if old_mode != self.cpsr_register & 0x1F {
            self.switch_modes(old_mode);
        }
        self.switch_modes(old_mode)
    }

    fn sr_operation(&mut self, opcode: u32) {
        match opcode & 0x20_0000 {
            0x20_0000 => self.msr(opcode), // MSR
            0x0 => self.mrs(opcode & 0x40_0000 == 0x40_0000, (opcode & 0xF000) >> 12), // MRS
            _ => panic!(),
        }
    }

    fn msr(&mut self, opcode: u32) {
        let mask: u32 = match opcode & (0xF << 16) {
            0x1_0000 => 0xFF,
            0x2_0000 => 0xFF00,
            0x3_0000 => 0xFFFF,
            0x4_0000 => 0xFF0000,
            0x5_0000 => 0xFF00FF,
            0x6_0000 => 0xFFFF00,
            0x7_0000 => 0xFFFFFF,
            0x8_0000 => 0xFF000000,
            0x9_0000 => 0xFF0000FF,
            0xA_0000 => 0xFF00FF00,
            0xB_0000 => 0xFF00FFFF,
            0xC_0000 => 0xFFFF0000,
            0xD_0000 => 0xFFFF00FF,
            0xE_0000 => 0xFFFFFF00,
            0xF_0000 => 0xFFFFFFFF,
            _ => panic!(),
        };
        let operand_2: u32 = 
        // Is immediate
        if check_bit!(opcode, 25) {
            let shift = ((opcode & 0xF00) >> 8) * 2;
            (opcode & 0xFF).rotate_right(shift).rotate_right(shift)
        }
        // Is in register
        else {
            self.get_rm_register_value(opcode)
        };
        
        if self.cpsr_register & 0x1F == USER_MODE && mask & 0xFF == 0xFF {
            panic!("Tried to set control flags in user mode")
        }
        let old_mode = self.cpsr_register & 0x1F;
        self.cpsr_register = (operand_2 & mask) | (self.cpsr_register & !mask);
        if old_mode != self.cpsr_register & 0x1F {
            self.switch_modes(old_mode);
        }
    }

    fn mrs(&mut self, current_psr: bool, destination_register: u32) {
        if current_psr {
            self.registers[destination_register as usize] = self.cpsr_register;
        } else {
            self.registers[destination_register as usize] = *self.get_current_saved_psr();
        }
    }

    // TODO NEXT BUG TO SQUASH
    fn multiply(&mut self, opcode: u32) {
        let mut operand_1 = 0;
        // If accumulate
        if opcode & 0x20_0000 == 0x20_0000 {
            operand_1 = self.registers[(opcode & 0xF000) as usize];
        }
        let operand_2 = self.registers[(opcode & 0xF00) as usize];
        let operand_3 = self.registers[(opcode & 0xF) as usize];
        self.registers[(opcode & 0xF_0000) as usize] =
            operand_3.wrapping_mul(operand_2).wrapping_add(operand_1);
        // If set condition
        if opcode & 0x10_0000 == 0x10_0000 {
            self.set_multiply_flags(self.registers[(opcode & 0xF_0000) as usize]);
        }
    }

    fn multiply_long(&mut self, opcode: u32) {
        let register_hi = opcode & 0xF_0000;
        let register_lo = opcode & 0xF000;
        let operand_1 = self.registers[(opcode & 0xF00) as usize];
        let operand_2 = self.registers[(opcode & 0xF) as usize];
        let mut operand_3 = 0;
        // If accumulate
        if opcode & 0x20_0000 == 0x20_0000 {
            operand_3 = ((self.registers[register_hi as usize] as u64) << 32)
                | self.registers[register_lo as usize] as u64;
        }
        let result =
        // If Signed
        if opcode & 0x40_0000 == 0x40_0000 {
            (operand_2 as i64 * operand_1 as i64).wrapping_add(operand_3 as i64) as u64
        } else {
            (operand_2 as u64 * operand_1 as u64).wrapping_add(operand_3)
        };

        self.registers[register_hi as usize] = (result & 0xFFFF_FFFF_0000_0000 >> 32) as u32;
        self.registers[register_lo as usize] = (result & 0xFFFF_FFFF) as u32;
        if opcode & 0x10_0000 == 0x10_0000 {
            self.set_long_multiply_flags(result);
        }
    }

    fn set_long_multiply_flags(&mut self, result: u64) {
        self.cpsr_register &= 0x1FFF_FFFF;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        if check_bit!(result, 63) {
            self.cpsr_register |= SIGN_FLAG;
        }
    }

    fn set_multiply_flags(&mut self, result: u32) {
        self.cpsr_register &= 0x1FFF_FFFF;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        if check_bit!(result, 31) {
            self.cpsr_register |= SIGN_FLAG;
        }
    }

    fn single_data_transfer(&mut self, opcode: u32) {
        self.registers[15] += 8;
        let mut address = self.get_rn_register_value(opcode);
        self.registers[15] += 4;
        let src_dst_register = Self::get_rd_register_number(opcode);
        let mut offset = 0;
        let register_offset = check_bit!(opcode, 25);
        let pre_indexing = check_bit!(opcode, 24);
        let up = check_bit!(opcode, 23);
        let byte = check_bit!(opcode, 22);
        let write_back = check_bit!(opcode, 21);
        let load = check_bit!(opcode, 20);
        // TODO repeated code with alu
        // Is register offset
        if register_offset {
            offset = self.get_rm_register_value(opcode);
            // Shift is in a register
            if check_bit!(opcode, 4) {
                // Shift is only done using the least significant byte in the register
                let value = self.get_rs_register_value(opcode);
                let shift_type = 0x60;
                offset = self.barrel_shifter(value, offset, shift_type, true);
            }
            // Shift is an immediate value
            else {
                let value = (opcode & 0xF80) >> 7;
                let shift_type = (opcode & 0x60) >> 5;
                offset = self.barrel_shifter(value, offset, shift_type, false);
            }
        }
        // Is immediate value
        else {
            offset = opcode & 0xFFF;
        }
        // Pre indexing
        if pre_indexing {
            if up {
                address += offset;
            } else {
                address -= offset;
            }
        }
        // Load from memory
        if load {
            self.load_memory(
                address,
                src_dst_register,
                byte,
            );
        } else {
            self.store_memory(
                address,
                src_dst_register.clone(),
                byte,
            )
        }
        // Post Indexing
        if !pre_indexing {
            if up {
                address += offset;
            } else {
                address -= offset;
            }
        }
        // Write Back
        if write_back {
            self.registers[Self::get_rn_register_number(opcode)] = address;
        }
        self.registers[15] -= 12;
    }

    fn load_memory(&mut self, address: u32, dst_register: usize, is_byte: bool) {
        let value_to_load = if is_byte {
            self.memory.borrow()[address as usize] as u32
        } else {
            let value = self.memory.borrow().get_word(address);
            if address % 4 != 0 {
                self.barrel_shifter(value, (address & 3) * 8, 3, false)
            } else {
                value
            }
        };
        self.registers[dst_register] = value_to_load;
    }

    fn store_memory(&mut self, address: u32, src_register: usize, is_byte: bool) {
        let value_to_store = self.registers[src_register];
        if !is_byte {
            self.memory.borrow_mut().store_word(address, value_to_store);
        } else {
            self.memory
                .borrow_mut()
                .store_byte(address, value_to_store as u8);
        }
    }

    fn halfword_data_transfer(&mut self, opcode: u32) {
        self.registers[15] += 8;
        let mut address = self.get_rn_register_value(opcode);
        self.registers[15] += 4;
        let src_dst_register = Self::get_rd_register_number(opcode);
        let offset = match opcode & 0x40_0000 {
            0x40_0000 => opcode & 0xF | (opcode & 0xF00) >> 4,
            0x0 => self.registers[(opcode & 0xF) as usize],
            _ => panic!(
                "Something went terribly wrong while deducing offset type in halfword transfer"
            ),
        };
        
        // Pre indexing
        if opcode & 0x100_0000 == 0x100_0000 {
            if opcode & 0x80_0000 == 0x80_0000 {
                address += offset;
            } else {
                address -= offset;
            }
        }
        // Load from memory
        if opcode & 0x10_0000 == 0x10_0000 {
            self.load_halfword(address, src_dst_register, opcode & 0x60);
        } else {
            self.store_halfword(address, src_dst_register, opcode & 0x60)
        }
        // Post Indexing
        if opcode & 0x100_0000 == 0x0 {
            if opcode & 0x80_0000 == 0x80_0000 {
                address += offset;
            } else {
                address -= offset;
            }
        }
        // Write Back
        if opcode & 0x20_0000 == 0x20_0000 {
            self.registers[Self::get_rn_register_number(opcode)] = address;
        }
        self.registers[15] -= 12;
    }

    fn load_halfword(&mut self, address: u32, dst_register: usize, sh: u32) {
        let value = self.memory.borrow().get_halfword(address);
        match sh {
            0x20 => self.registers[dst_register] = value as u32,
            0x40 => self.registers[dst_register] = (((value & 0xFF) as i8) as i32) as u32,
            0x60 => self.registers[dst_register] = ((value as i16) as i32) as u32,
            _ => panic!("Something went terribly wrong while loading a halfword"),
        }
    }

    fn store_halfword(&mut self, address: u32, src_register: usize, sh: u32) {
        let value = self.registers[src_register] as u16;
        if sh & 0x20 == 0x20 {
            self.memory
                .borrow_mut()
                .store_halfword(address, value);
        } else {
            panic!("Something went terribly wrong while storing a halfword");
        }
    }

    fn block_data_transfer(&mut self, opcode: u32) {
        let mut address = self.get_rn_register_value(opcode);
        let mut register_mask = opcode & 0xFFFF;
        let mut registers = Vec::new();
        let number_registers = register_mask.count_ones();
        registers.reserve(number_registers as usize);
        let pre_indexing = check_bit!(opcode, 24);
        let up = check_bit!(opcode, 23);
        let psr_user_bit = check_bit!(opcode, 22);
        let write_back = check_bit!(opcode, 21);
        let load = check_bit!(opcode, 20);
        let mut final_address = 0;
        for i in 0u8..16u8 {
            if register_mask & 0x1 == 0x1 {
                registers.push(i);
            }
            register_mask = register_mask >> 1;
        }
        if pre_indexing {
            if up {
                final_address += address + 4 * number_registers;
                address += 4
            } else {
                final_address += address - 4 * number_registers;
                address = final_address
            }
        } else {
            if up {
                final_address += address + 4 * number_registers;
            } else {
                final_address += address - 4 * number_registers;
                address = final_address + 4;
            }
        }
        // Store old mode if S bit is set to transfer user mode registers
        let old_mode = self.cpsr_register & 0x1F;
        if psr_user_bit && self.cpsr_register & USER_MODE != USER_MODE {
            self.cpsr_register = self.cpsr_register & 0xFFFF_FFE0 | USER_MODE;
            self.switch_modes(old_mode)
        }
        // Load Multiple
        if load {
            self.load_multiple(address, &registers, old_mode);
        }
        // Store Multiple
        else {
            self.store_multiple(address, &registers);
        }
        // Restore old mode
        if psr_user_bit {
            self.cpsr_register = self.cpsr_register & 0xFFFF_FFE0 | old_mode;
            self.switch_modes(USER_MODE)
        }
        // Write Back
        if write_back {
            if write_back {
                self.registers[Self::get_rn_register_number(opcode)] = final_address;
            } else {
                self.registers[Self::get_rn_register_number(opcode)] = final_address;
            }
        }
    }

    fn load_multiple(&mut self, mut address: u32, registers: &[u8], old_mode: u32) {
        for register in registers {
            if *register == 15 {
                self.cpsr_register = self.cpsr_register & 0xFFFF_FFE0 | old_mode;
                self.cpsr_register = *self.get_current_saved_psr();
                self.cpsr_register = self.cpsr_register & 0xFFFF_FFE0 | USER_MODE;
            }
            let value = self.memory.borrow().get_word(address);
            self.registers[*register as usize] = value;
            address += 4;
        }
    }

    fn store_multiple(&mut self, mut address: u32, registers: &[u8]) {
        for register in registers {
            let value_to_store = self.registers[*register as usize];
            self.memory.borrow_mut().store_word(address, value_to_store);
            address += 4;
        }
    }

    fn single_data_swap(&mut self, opcode: u32) {
        let address = self.get_rn_register_value(opcode);
        let dst_register = Self::get_rd_register_number(opcode);
        let src_register = Self::get_rm_register_number(opcode);
        let is_byte = check_bit!(opcode, 22);
        self.load_memory(address, dst_register, is_byte);
        self.store_memory(address, src_register, is_byte);
    }
}
