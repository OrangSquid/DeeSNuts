use std::ops::Index;

use crate::alu::Alu;

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
    bios: [u8; 0x4000],
    rom: Vec<u8>,
    pub registers: [u32; 31],
    // Current Program Status Register
    pub cpsr_register: u32,
    // Each u32 is a banked spsr (Saved Program Status Register)
    saved_psr: [u32; 5],
    // The banked out registers when switched out of user/system mode
    user_system_banked: (u32, u32),
    fiq_banked: (u32, u32),
    supervisor_banked: (u32, u32),
    abort_banked: (u32, u32),
    irq_banked: (u32, u32),
    undefinied_banked: (u32, u32),
}

impl Arm7 {
    pub fn new() -> Arm7 {
        let registers = [0u32; 31];
        let mut arm7 = Arm7 {
            bios: [0; 0x4000],
            rom: Vec::new(),
            registers: registers.to_owned(),
            cpsr_register: SYSTEM_MODE as u32,
            saved_psr: [0; 5],
            user_system_banked: (16, 16),
            fiq_banked: (16, 23),
            supervisor_banked: (23, 25),
            abort_banked: (25, 27),
            irq_banked: (27, 29),
            undefinied_banked: (29, 31)
        };
        arm7.registers[13] = STACK_USER_SYSTEM_START;
        arm7.registers[(arm7.fiq_banked.1 - 1) as usize] = STACK_IRQ_START;
        arm7.registers[(arm7.supervisor_banked.1 - 1) as usize] = STACK_SUPERVISOR_START;
        arm7.registers[15] = START_PC;
        arm7
    }

    pub fn load_bios(&mut self, bios: Vec<u8>) {
        self.bios.copy_from_slice(&bios);
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.rom.clone_from(&rom)
    }

    pub fn next(&mut self) {
        // THUMB MODE
        if self.cpsr_register & STATE_BIT == STATE_BIT {
            self.registers[15] += 2;
        }
        // ARM MODE
        else {
            let opcode = self.fetch_arm();
            self.registers[15] += 4;
            self.decode_arm(opcode);
        }
    }

    fn fetch_arm(&mut self) -> u32 {
        println!("Fetching at {:#08x}", self.registers[15]);
        let pc = (self.registers[15] - START_PC) as usize;
        let opcode_array_slice = &self.rom[pc as usize..(pc + 4) as usize];
        let mut opcode_array: [u8; 4] = [0; 4];
        opcode_array.copy_from_slice(opcode_array_slice);
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
                        _ => self.sr_or_alu(opcode)
                    }
                    0x90 => match opcode & 0x60 {
                        0x0 => match opcode & 0x180_0000 {
                            0x0 => (), // Multiply
                            0x80_0000 => (), // Multiply long
                            0x100_0000 => (), // Single Data Swap
                            _ => panic!()
                        }
                        _ => match opcode & 0x40_0000 {
                            0x400_0000 => (), // Halfword Data Transfer: immediate offset
                            0x0 => (), // Halfword Data Transfer: register offset
                            _ => panic!()
                        }
                    }
                    _ => panic!()
                }
            }
            0x400_0000 => (), // Single Data Transfer or Undefined
            0x800_0000 => match opcode & 0x200_0000 {
                0x0 => (), // Block Data Transfer
                0x200_0000 => self.branch(
                    opcode & 0x100_0000 == 0x100_0000,
                    (opcode & 0xFF_FFFF) as i32,
                ),
                _ => panic!(),
            },
            0xC00_0000 => match opcode & 0x200_0000 {
                0x0 => (), // Coprocessor Data Transfer
                0x200_0000 => match opcode & 0x100_0000 {
                    0x0 => (),        // Coprocessor Data Operation or Register Transfer
                    0x100_0000 => (), // Software Interrupt
                    _ => panic!(),
                },
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    fn sr_or_alu(&mut self, opcode: u32) {
        match opcode & 0x1F0_0000 {
            0x100_0000 | 0x120_0000 | 0x140_0000 | 0x160_0000 => (),
            _ => self.alu_command(opcode & 0x3FF_FFFF)
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
        let correct_ofset = ((offset << 8) >> 6) + 4;
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
            _ => panic!("CPU is in an unrecognized mode")
        }
    }

    pub fn restore_cpsr(&mut self) {
        self.cpsr_register = *self.get_current_saved_psr();
    }

    fn sr_operation(&mut self, opcode: u32) {
        match opcode & 0x20_0000 {
            0x20_0000 => (), // MSR
            0x0 => self.mrs(opcode & 0x40_0000 == 0x40_0000, (opcode & 0xF000) >> 12), // MRS
            _ => panic!()
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
            _ => panic!()
        }
        let mut operand_2: u32 = 0;
        // Is immediate
        if opcode & 0x200_0000 == 0x200_0000 {
            operand_2 = opcode & 0xFF;
            let shift = ((opcode & 0xF00) >> 8) * 2;
            operand_2 = operand_2.rotate_right(shift);
        }
        else {
            operand_2 = self.registers[(opcode & 0xF) as usize];
        }
        if self.cpsr_register & 0x1F == USER_MODE && mask & 0xFF == 0xFF {
            panic!("Tried to set control flags in user mode")
        }
        let old_mode = self.cpsr_register & 0x1F;
        self.cpsr_register = operand_2 & mask;
    }

    fn mrs(&mut self, current_psr: bool, destination_register: u32) {
        if current_psr {
            self.registers[destination_register as usize] = self.cpsr_register;
        }
        else {
            self.registers[destination_register as usize] = *self.get_current_saved_psr();
        }
    }
}
