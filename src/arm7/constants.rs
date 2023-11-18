// CPU modes
pub(in crate::arm7) const USER_MODE: u32 = 0x10;
pub(in crate::arm7) const FIQ_MODE: u32 = 0x11;
pub(in crate::arm7) const IRQ_MODE: u32 = 0x12;
pub(in crate::arm7) const SUPERVISOR_MODE: u32 = 0x13;
pub(in crate::arm7) const ABORT_MODE: u32 = 0x17;
pub(in crate::arm7) const UNDEFINED_MODE: u32 = 0x1B;
pub(in crate::arm7) const SYSTEM_MODE: u32 = 0x1F;

// Memory locations
pub(in crate::arm7) const START_PC: u32 = 0x800_0000;
pub(in crate::arm7) const STACK_USER_SYSTEM_START: u32 = 0x300_7F00;
pub(in crate::arm7) const STACK_IRQ_START: u32 = 0x300_7FA0;
pub(in crate::arm7) const STACK_SUPERVISOR_START: u32 = 0x0300_7FE0;

// Position of the bits in the CPSR register
pub(in crate::arm7) const SIGN_FLAG: u32 = 0x8000_0000;
pub(in crate::arm7) const ZERO_FLAG: u32 = 0x4000_0000;
pub(in crate::arm7) const CARRY_FLAG: u32 = 0x2000_0000;
pub(in crate::arm7) const OVERFLOW_FLAG: u32 = 0x1000_0000;
pub(in crate::arm7) const IRQ_BIT: u32 = 0x80;
pub(in crate::arm7) const FIQ_BIT: u32 = 0x40;
pub(in crate::arm7) const STATE_BIT: u32 = 0x20;