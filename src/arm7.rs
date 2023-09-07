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

pub struct Arm7 {
    memory: Memory,
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
    pub fn new() -> Arm7 {
        // TODO: THE BANKED REGISTERS I HATE IT HERE
        let mut arm7 = Arm7 {
            memory: Memory::new(),
            registers: [0; 16],
            cpsr_register: SYSTEM_MODE as u32,
            saved_psr: [0; 5],
            fiq_lo_banked: [0; 5],
            user_banked: [0; 2],
            fiq_hi_banked: [0; 2],
            supervisor_banked: [0; 2],
            abort_banked: [0; 2],
            irq_banked: [0; 2],
            undefinied_banked: [0; 2]
        };
        arm7.registers[13] = STACK_USER_SYSTEM_START;
        arm7.registers[13] = STACK_IRQ_START;
        arm7.registers[13] = STACK_SUPERVISOR_START;
        arm7.registers[15] = START_PC;
        arm7
    }

    pub fn load_bios(&mut self, bios: Vec<u8>) {
        self.memory.load_bios(bios);
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.memory.load_rom(rom)
    }

    pub fn next(&mut self) {
        // THUMB MODE
        if self.cpsr_register & STATE_BIT == STATE_BIT {
            self.registers[15] += 2;
        }
        // ARM MODE
        else {
            let opcode = self.fetch_arm();
            self.decode_arm(opcode);
            self.registers[15] += 4;
        }
    }

    fn fetch_arm(&mut self) -> u32 {
        println!("Fetching at {:#08x}", self.registers[15]);
        let pc = self.registers[15] as usize;
        let mut opcode_array: [u8; 4] = [0; 4];
        for i in 0..4 {
            opcode_array[i] = self.memory[pc + i];
        }
        u32::from_le_bytes(opcode_array)
    }

    fn fetch_thumb(&mut self) -> u16 {
        0
    }

    // TODO: a single data transfer opcode might be an undefinied instruction, should take care
    // of it at a later date
    fn decode_arm(&mut self, opcode: u32) {
        println!("Decoding {:#08x}", opcode);
        if !self.check_codition(((opcode & 0xF000_0000) >> 28) as u8) {
            return;
        }
        match opcode & 0xC00_0000 {
            0x0 => {
                match opcode & 0x90 {
                    0x0 | 0x80 => self.sr_or_alu(opcode),
                    0x10 => match opcode & 0x12F_FF10 {
                        0x12F_FF10 => self.branch_and_exchange(opcode & 0xF),
                        _ => self.sr_or_alu(opcode),
                    },
                    0x90 => match opcode & 0x60 {
                        0x0 => match opcode & 0x180_0000 {
                            0x0 => self.multiply(opcode),
                            0x80_0000 => self.multiply_long(opcode),
                            0x100_0000 => (), // Single Data Swap
                            _ => panic!(),
                        },
                        _ => match opcode & 0x40_0000 {
                            0x400_0000 => (), // Halfword Data Transfer: immediate offset
                            0x0 => (),        // Halfword Data Transfer: register offset
                            _ => panic!("Undefinied instruction"),
                        },
                    },
                    _ => panic!("Undefinied instruction"),
                }
            }
            0x400_0000 => self.single_data_transfer(opcode),
            0x800_0000 => match opcode & 0x200_0000 {
                0x0 => (), // Block Data Transfer
                0x200_0000 => self.branch(
                    opcode & 0x100_0000 == 0x100_0000,
                    (opcode & 0xFF_FFFF) as i32,
                ),
                _ => panic!("Undefinied instruction"),
            },
            _ => panic!("Undefinied instruction"),
        }
    }

    fn switch_modes(&mut self, old_mode: u32) {
        match old_mode {
            USER_MODE | SYSTEM_MODE => self.user_banked.copy_from_slice(&mut self.registers[13..15]),
            FIQ_MODE => {
                // Swap beacuse the other modes all share R8 through R12
                &self.registers[8..13].swap_with_slice(&mut self.fiq_lo_banked);
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

    fn check_codition(&mut self, condition: u8) -> bool {
        match condition {
            0x0 => self.cpsr_register & ZERO_FLAG == ZERO_FLAG, // Z flag is set
            0x1 => self.cpsr_register & ZERO_FLAG == 0x0,       // Z flag is not set
            0x2 => self.cpsr_register & CARRY_FLAG == CARRY_FLAG, // C flag is set
            0x3 => self.cpsr_register & CARRY_FLAG == 0x0,      // C flag is not set
            0x4 => self.cpsr_register & SIGN_FLAG == SIGN_FLAG, // S flag is set
            0x5 => self.cpsr_register & SIGN_FLAG == 0x0,       // S flag is not set
            0x6 => self.cpsr_register & OVERFLOW_FLAG == OVERFLOW_FLAG, // V flag is set
            0x7 => self.cpsr_register & OVERFLOW_FLAG == 0x0,   // V flag is not set
            0x8 => self.cpsr_register & (CARRY_FLAG | ZERO_FLAG) == CARRY_FLAG, // Unsigned Higher
            0x9 => {
                self.cpsr_register & CARRY_FLAG == 0 || self.cpsr_register & ZERO_FLAG == ZERO_FLAG
            } // Unsigned Lower or same
            0xA => {
                (self.cpsr_register & SIGN_FLAG >> 31) == (self.cpsr_register & OVERFLOW_FLAG >> 28)
            } // Signed Greater than or equal
            0xB => {
                (self.cpsr_register & SIGN_FLAG >> 31) != (self.cpsr_register & OVERFLOW_FLAG >> 28)
            } // Less than
            0xC => {
                self.cpsr_register & ZERO_FLAG == 0
                    && (self.cpsr_register & SIGN_FLAG >> 31)
                        != (self.cpsr_register & OVERFLOW_FLAG >> 28)
            } //
            0xD => {
                self.cpsr_register & ZERO_FLAG == 0
                    && (self.cpsr_register & SIGN_FLAG >> 31)
                        == (self.cpsr_register & OVERFLOW_FLAG >> 28)
            }
            0xE => true,
            0xF => true,
            _ => panic!("Condition for opcode is higher than 0xF"),
        }
    }

    fn decode_thumb(&mut self, opcode: u16) {}

    fn branch_and_exchange(&mut self, register: u32) {
        let thumb_bit = register & 0x1 << 5;
        self.cpsr_register = self.cpsr_register | thumb_bit;
        self.registers[15] = self.registers[(register & 0xFFFF_FFFE) as usize];
    }

    fn branch(&mut self, link: bool, offset: i32) {
        // Due to prefetching, the PC should be 8 bytes ahead
        let correct_ofset = ((offset << 8) >> 6) + 8;
        if link {
            self.registers[14] = self.registers[15]
        }
        let mut temp_pc = self.registers[15] as i32;
        temp_pc += correct_ofset;
        self.registers[15] = temp_pc as u32;
    }

    fn get_current_saved_psr(&mut self) -> &mut u32 {
        match self.cpsr_register & 0x1F {
            USER_MODE => panic!("No saved PSR in user mode"),
            FIQ_MODE => &mut self.saved_psr[0],
            IRQ_MODE => &mut self.saved_psr[1],
            SUPERVISOR_MODE => &mut self.saved_psr[2],
            ABORT_MODE => &mut self.saved_psr[3],
            UNDEFINED_MODE => &mut self.saved_psr[4],
            SYSTEM_MODE => panic!("No saved PSR in system mode"),
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
        let mut mask: u32 = 0;
        match opcode & 0xF_0000 {
            0x1_0000 => mask = 0xFF,
            0x2_0000 => mask = 0xFF00,
            0x3_0000 => mask = 0xFFFF,
            0x4_0000 => mask = 0xFF0000,
            0x5_0000 => mask = 0xFF00FF,
            0x6_0000 => mask = 0xFFFF00,
            0x7_0000 => mask = 0xFFFFFF,
            0x8_0000 => mask = 0xFF000000,
            0x9_0000 => mask = 0xFF0000FF,
            0xA_0000 => mask = 0xFF00FF00,
            0xB_0000 => mask = 0xFF00FFFF,
            0xC_0000 => mask = 0xFFFF0000,
            0xD_0000 => mask = 0xFFFF00FF,
            0xE_0000 => mask = 0xFFFFFF00,
            0xF_0000 => mask = 0xFFFFFFFF,
            _ => panic!(),
        }
        let mut operand_2: u32 = 0;
        // Is immediate
        if opcode & 0x200_0000 == 0x200_0000 {
            operand_2 = opcode & 0xFF;
            let shift = ((opcode & 0xF00) >> 8) * 2;
            operand_2 = operand_2.rotate_right(shift);
        }
        // Is in register 
        else {
            operand_2 = self.registers[(opcode & 0xF) as usize];
        }
        if self.cpsr_register & 0x1F == USER_MODE && mask & 0xFF == 0xFF {
            panic!("Tried to set control flags in user mode")
        }
        let old_mode = self.cpsr_register & 0x1F;
        self.cpsr_register = operand_2 & mask;
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

    fn multiply(&mut self, opcode: u32) {
        let mut operand_1 = 0;
        // If accumulate
        if opcode & 0x20_0000 == 0x20_0000 {
            operand_1 = self.registers[(opcode & 0xF000) as usize];
        }
        let operand_2 = self.registers[(opcode & 0xF00) as usize];
        let operand_3 = self.registers[(opcode & 0xF) as usize];
        self.registers[(opcode & 0xF_0000) as usize] = operand_3.wrapping_mul(operand_2).wrapping_add(operand_1);
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
            operand_3 = ((self.registers[register_hi as usize] as u64) << 32) | self.registers[register_lo as usize] as u64;
        }
        let mut result = 0;
        // If Signed
        if opcode & 0x40_0000 == 0x40_0000 {
            result = (operand_2 as i64 * operand_1 as i64).wrapping_add(operand_3 as i64) as u64;
        }
        else {
            result = (operand_2 as u64 * operand_1 as u64).wrapping_add(operand_3);
        }
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
        if result & 0x8000_0000_0000_0000 == 0x8000_0000_0000_0000 {
            self.cpsr_register |= SIGN_FLAG;
        }
    }

    fn set_multiply_flags(&mut self, result: u32) {
        self.cpsr_register &= 0x1FFF_FFFF;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        if result & 0x8000_0000 == 0x8000_0000 {
            self.cpsr_register |= SIGN_FLAG;
        }
    }
    
    fn single_data_transfer(&mut self, opcode: u32) {
        self.registers[15] += 8;
        let base_register = self.registers[((opcode & 0xF_0000) >> 16) as usize];
        self.registers[15] += 4;
        let mut src_dst_register = self.registers[((opcode & 0xF000) >> 12) as usize];
        let mut offset = 0;
        self.registers[15] -= 4;
        // TODO repeated code with alu
        // Is register offset
        if opcode & 0x200_0000 == 0x200_0000 {
            offset = self.registers[(opcode & 0xF) as usize];
            // Shift is in a register
            if opcode & 0x10 == 0x10 {
                self.registers[15] += 4;
                // Shift is only done using the least significant byte in the register
                let value = self.registers[((opcode & 0xF00) >> 8) as usize] & 0xFF;
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
        if opcode & 0x100_0000 == 0x100_0000 {
            if opcode & 0x80_0000 == 0x80_0000 {
                src_dst_register += offset;
            }
            else {
                src_dst_register -= offset;
            }
        }
        // Load from memory
        if opcode & 0x10_0000 == 0x10_0000 {
            self.load_memory(base_register, src_dst_register, opcode & 0x40_0000 == 0x40_0000);
        }
        else {
            self.store_memory(base_register, src_dst_register, opcode & 0x40_0000 == 0x40_0000)
        }
        // Post Indexing
        if opcode & 0x100_0000 == 0x0 {
            if opcode & 0x80_0000 == 0x80_0000 {
                src_dst_register += offset;
            }
            else {
                src_dst_register -= offset;
            }
        }
        // Write Back
        if opcode & 0x20_0000 == 0x20_0000 {
            self.registers[((opcode & 0xF000) >> 12) as usize] = src_dst_register;
        }
    }

    
    
    fn load_memory(&mut self, base_register: u32, src_register: u32, is_byte: bool) {
        let mut value = self.memory[src_register as usize] as u32;
        if !is_byte {
            // Is Word aligned
            if src_register % 4 == 0 {
                for i in 1..4 {
                    value |= (self.memory[(src_register + i) as usize] as u32) << (8 * i);
                }
            }
            // Is halfword aligned
            else {
                value |= (self.memory[(src_register + 1) as usize] as u32) << 8;
                value |= (self.memory[(src_register - 2) as usize] as u32) << 16;
                value |= (self.memory[(src_register - 1) as usize] as u32) << 24;
            }
        }
        self.registers[base_register as usize] = value;
    }

    fn store_memory(&mut self, base_register: u32, dst_register: u32, is_byte: bool) {
        let mut value = self.registers[base_register as usize] as u32;
        if !is_byte {
            self.memory[dst_register as usize..(dst_register + 4) as usize].copy_from_slice(&value.to_le_bytes());
        }
        else {
            self.memory[dst_register as usize] = (value & 0xFF) as u8;
        }
    }
}
