// CPU modes
const USER_MODE: u8 = 0x10;
const FIQ_MODE: u8 = 0x11;
const IRQ_MODE: u8 = 0x12;
const SUPERVISOR_MODE: u8 = 0x13;
const ABORT_MODE: u8 = 0x17;
const UNDEFINED_MODE: u8 = 0x1B;
const SYSTEM_MODE: u8 = 0x1F;

const START_PC: u32 = 0x800_0000;

const STACK_USER_SYSTEM_START: u32 = 0x300_7F00;
const STACK_IRQ_START: u32 = 0x300_7FA0;
const STACK_SUPERVISOR_START: u32 = 0x0300_7FE0;

// Position of the bits in the CPSR register
const SIGN_FLAG: u32 = 0x8000_0000;
const ZERO_FLAG: u32 = 0x4000_0000;
const CARRY_FLAG: u32 = 0x2000_0000;
const OVERFLOW_FLAG: u32 = 0x1000_0000;
const IRQ_BIT: u32 = 0x80;
const FIQ_BIT: u32 = 0x40;
const STATE_BIT: u32 = 0x20;

pub struct Arm7 {
    bios: [u8; 0x4000],
    rom: Vec<u8>,
    registers: [u32; 16],
    // Current Program Status Register
    cpsr_register: u32,
    // Each u32 is a banked spsr (Saved Program Status Register)
    saved_psr: [u32; 5],
    // The banked out registers when switched out of user/system mode
    user_system_banked: Vec<u32>,
    fiq_banked: [u32; 7],
    supervisor_banked: [u32; 2],
    abort_banked: [u32; 2],
    irq_banked: [u32; 2],
    undefinied_banked: [u32; 2]
}

impl Arm7 {
    pub fn new() -> Arm7 {
        let mut arm7 = Arm7 { 
            bios: [0; 0x4000],
            rom: Vec::new(),
            registers: [0; 16],
            cpsr_register: SYSTEM_MODE as u32,
            saved_psr: [0; 5],
            user_system_banked: Vec::new(),
            fiq_banked: [0; 7],
            supervisor_banked: [0; 2],
            abort_banked: [0; 2],
            irq_banked: [0; 2],
            undefinied_banked: [0; 2]
        };
        arm7.registers[13] = STACK_USER_SYSTEM_START;
        arm7.irq_banked[0] = STACK_IRQ_START;
        arm7.supervisor_banked[0] = STACK_SUPERVISOR_START;
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
            self.decode_arm(opcode);
            self.registers[15] += 4;
        }
    }

    fn fetch_arm(&mut self) -> u32 {
        let pc = (self.registers[15] - START_PC) as usize;
        let opcode_array_slice = &self.rom[pc as usize..(pc + 4) as usize];
        let mut opcode_array: [u8; 4] = [0; 4];
        opcode_array.copy_from_slice(opcode_array_slice);
        u32::from_le_bytes(opcode_array)
    }

    fn fetch_thumb(&mut self) -> u16 {
        0
    }

    fn decode_arm(&mut self, opcode: u32) {
        if !self.check_codition(((opcode & 0xF000_0000) >> 28) as u8) {
            return;
        }
        match opcode & 0xFFF_FFF0 {
            0x12F_FF10 => self.branch_and_exchange(opcode & 0xF),
            _ => ()
        }
    }

    fn check_codition(&mut self, condition: u8) -> bool {
        match condition {
            0x0 => self.cpsr_register & ZERO_FLAG == ZERO_FLAG, // Z flag is set
            0x1 => self.cpsr_register & ZERO_FLAG == 0x0, // Z flag is not set
            0x2 => self.cpsr_register & CARRY_FLAG == CARRY_FLAG, // C flag is set
            0x3 => self.cpsr_register & CARRY_FLAG == 0x0, // C flag is not set
            0x4 => self.cpsr_register & SIGN_FLAG == SIGN_FLAG, // S flag is set
            0x5 => self.cpsr_register & SIGN_FLAG == 0x0, // S flag is not set
            0x6 => self.cpsr_register & OVERFLOW_FLAG == OVERFLOW_FLAG, // V flag is set
            0x7 => self.cpsr_register & OVERFLOW_FLAG == 0x0, // V flag is not set
            0x8 => self.cpsr_register & (CARRY_FLAG | ZERO_FLAG) == CARRY_FLAG, // Unsigned Higher
            0x9 => self.cpsr_register & CARRY_FLAG == 0 || self.cpsr_register & ZERO_FLAG == ZERO_FLAG, // Unsigned Lower or same
            0xA => (self.cpsr_register & SIGN_FLAG >> 31) == (self.cpsr_register & OVERFLOW_FLAG >> 28), // Signed Greater than or equal
            0xB => (self.cpsr_register & SIGN_FLAG >> 31) != (self.cpsr_register & OVERFLOW_FLAG >> 28), // Less than
            0xC => self.cpsr_register & ZERO_FLAG == 0 && (self.cpsr_register & SIGN_FLAG >> 31) != (self.cpsr_register & OVERFLOW_FLAG >> 28), //
            0xD => self.cpsr_register & ZERO_FLAG == 0 && (self.cpsr_register & SIGN_FLAG >> 31) == (self.cpsr_register & OVERFLOW_FLAG >> 28),
            0xE => true,
            0xF => true,
            _ => panic!("Condition for opcode is higher than 0xF")
        }
    }

    fn decode_thumb(&mut self, opcode: u16) {

    }

    fn branch_and_exchange(&mut self, register: u32) {
        let thumb_bit = register & 0x1 << 5;
        self.cpsr_register = self.cpsr_register | thumb_bit;
        self.registers[15] = self.registers[(register & 0xFFFF_FFFE) as usize];
    }
}