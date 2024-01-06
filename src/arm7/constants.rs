// CPU modes
pub const USER_MODE: u32 = 0x10;
pub const FIQ_MODE: u32 = 0x11;
pub const IRQ_MODE: u32 = 0x12;
pub const SUPERVISOR_MODE: u32 = 0x13;
pub const ABORT_MODE: u32 = 0x17;
pub const UNDEFINED_MODE: u32 = 0x1B;
pub const SYSTEM_MODE: u32 = 0x1F;

// Memory locations
pub const START_PC: u32 = 0x800_0000;
pub const STACK_USER_SYSTEM_START: u32 = 0x300_7F00;
pub const STACK_IRQ_START: u32 = 0x300_7FA0;
pub const STACK_SUPERVISOR_START: u32 = 0x0300_7FE0;

// Position of the bits in the CPSR register
pub const SIGN_FLAG: u32 = 0x8000_0000;
pub const ZERO_FLAG: u32 = 0x4000_0000;
pub const CARRY_FLAG: u32 = 0x2000_0000;
pub const OVERFLOW_FLAG: u32 = 0x1000_0000;
pub const IRQ_BIT: u32 = 0x80;
pub const FIQ_BIT: u32 = 0x40;
pub const STATE_BIT: u32 = 0x20;

#[derive(Clone, Copy)]
pub enum Instruction {
    BranchAndExchange,
    Alu {
        operand2_type: Operand2Type,
        opcode: AluOpcode,
        set_conditions: bool,
        shift_type: ShiftType
    },
    Branch {
        link: bool
    },
    MRSTransfer {
        source_is_spsr: bool
    },
    MSRTransfer {
        operand2_type: Operand2Type,
        destination_is_spsr: bool
    },
    Multiply {
        accumulate: bool,
        set_conditions: bool
    },
    MultiplyLong {
        signed: bool,
        accumulate: bool,
        set_conditions: bool
    },
    SingleDataTransfer {
        operand2_type: Operand2Type,
        pre_indexing: bool,
        add_offset: bool,
        transfer_byte: bool,
        write_back: bool,
        load: bool,
        shift_type: ShiftType
    },
    HalfowrdTransfer {
        immediate: bool,
        pre_indexing: bool,
        add_offset: bool,
        write_back: bool,
        load: bool,
        halfword_transfer_type: HalfwordTransferType
    },
    BlockDataTransfer {
        pre_indexing: bool,
        add_offset: bool,
        load_psr: bool,
        write_back: bool,
        load: bool
    },
    SingleDataSwap {
        transfer_byte: bool
    },
    SoftwareInterrupt,
    Undefined,
    NoOp
}

#[derive(Clone, Copy)]
pub enum AluOpcode {
    And,
    ExclusiveOr,
    Subtract,
    RightSubtract,
    Add,
    AddCarry,
    SubtractCarry,
    RightSubtractCarry,
    TestAnd,
    TestExclusiveOr,
    CompareSubtract,
    CompareAdd,
    Or,
    Move,
    BitClear,
    MoveNot
}

pub const fn to_alu_opcode(value: u8) -> AluOpcode {
    match value {
        0x0 => AluOpcode::And,
        0x1 => AluOpcode::ExclusiveOr,
        0x2 => AluOpcode::Subtract,
        0x3 => AluOpcode::RightSubtract,
        0x4 => AluOpcode::Add,
        0x5 => AluOpcode::AddCarry,
        0x6 => AluOpcode::SubtractCarry,
        0x7 => AluOpcode::RightSubtractCarry,
        0x8 => AluOpcode::TestAnd,
        0x9 => AluOpcode::TestExclusiveOr,
        0xA => AluOpcode::CompareSubtract,
        0xB => AluOpcode::CompareAdd,
        0xC => AluOpcode::Or,
        0xD => AluOpcode::Move,
        0xE => AluOpcode::BitClear,
        0xF => AluOpcode::MoveNot,
        _ => panic!("Invalid ALU opcode")
    }
}

#[derive(Clone, Copy)]
pub enum ShiftType {
    LogicalLeft,
    LogicalRight,
    ArithmeticRight,
    RotateRight
}

pub const fn to_shift_type(value: u8) -> ShiftType {
    match value {
        0x0 => ShiftType::LogicalLeft,
        0x1 => ShiftType::LogicalRight,
        0x2 => ShiftType::ArithmeticRight,
        0x3 => ShiftType::RotateRight,
        _ => panic!("Invalid shift type")
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum HalfwordTransferType {
    NoOp,
    UnsignedHalfwords,
    SignedByte,
    SignedHalfwords
}

pub const fn to_halfword_transfer_type(value: u8) -> HalfwordTransferType {
    match value {
        0x0 => HalfwordTransferType::NoOp,
        0x1 => HalfwordTransferType::UnsignedHalfwords,
        0x2 => HalfwordTransferType::SignedByte,
        0x3 => HalfwordTransferType::SignedHalfwords,
        _ => panic!("Invalid halfword transfer type")
    }
}

#[derive(Clone, Copy)]
pub enum Operand2Type {
    RegisterWithRegisterShift,
    RegisterWithImmediateShift,
    ImmediateWithRotation,
    Immediate
}
