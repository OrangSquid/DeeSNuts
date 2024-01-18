use std::{ cell::RefCell, rc::Rc, fs::{ File, OpenOptions }, io::{ Write, BufWriter } };

use crate::{ memory::Memory, check_bit };

use super::{constants::*, thumb_lut::thumb_instruction_lut};
use super::arm_lut::{
    condition_lut,
    arm_instruction_lut
};

const CONDITION_LUT: [bool; 256] = condition_lut();
const ARM_INSTRUCTION_LUT: [InstructionHandler; 4096] = arm_instruction_lut();
const THUMB_INSTRUCTION_LUT: [InstructionHandler; 256] = thumb_instruction_lut();

struct PipelineStage2 {
    handler: InstructionHandler,
    opcode: u32
}

pub struct Cpu {
    memory: Rc<RefCell<Memory>>,
    pub(super) registers: [u32; 16],
    // Current Program Status Register
    pub(super) cpsr_register: u32,
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
    pipeline_stage_1: Option<u32>,
    pipeline_stage_2: Option<PipelineStage2>,
    pub(super) flush: bool,
    log: File,
    last_data_bus_read: u32,
}

impl Cpu {
    pub fn new(memory: Rc<RefCell<Memory>>) -> Cpu {
        let lmao = OpenOptions::new().write(true).create(true).open("log.bin").unwrap();
        let mut arm7 = Cpu {
            memory,
            registers: [0; 16],
            cpsr_register: SYSTEM_MODE | IRQ_BIT | FIQ_BIT,
            saved_psr: [0; 5],
            fiq_lo_banked: [0; 5],
            user_banked: [0; 2],
            fiq_hi_banked: [0; 2],
            supervisor_banked: [0; 2],
            abort_banked: [0; 2],
            irq_banked: [0; 2],
            undefinied_banked: [0; 2],
            pipeline_stage_1: None,
            pipeline_stage_2: None,
            flush: false,
            log: lmao,
            last_data_bus_read: 0
        };
        arm7.registers[13] = STACK_USER_SYSTEM_START;
        arm7.irq_banked[0] = STACK_IRQ_START;
        arm7.supervisor_banked[0] = STACK_SUPERVISOR_START;
        arm7.registers[15] = START_PC;
        arm7
    }

    pub fn next(&mut self) {
        self.output_registers();
        if (self.cpsr_register & STATE_BIT) == STATE_BIT {
            // THUMB MODE
            if self.pipeline_stage_2.is_some() {
                let instruction = self.pipeline_stage_2.as_ref().unwrap();
                (instruction.handler)(self, instruction.opcode);
                if self.flush {
                    self.pipeline_flush();
                    return;
                }
            }
            if self.pipeline_stage_1.is_some() {
                let opcode = self.pipeline_stage_1.unwrap();
                let bits15_8 = opcode >> 8;
                self.pipeline_stage_2 = Some(PipelineStage2 { handler: THUMB_INSTRUCTION_LUT[bits15_8 as usize], opcode });
            }
            self.pipeline_stage_1 = Some(self.fetch_thumb());
            self.registers[15] += 2;
        } else {
            // ARM MODE
            if self.registers[15] == 0x8001540 {
                println!("aiushdgasiydgh");
            }
            let temp_pipeline_1 = Some(self.fetch_arm());
            if self.pipeline_stage_2.is_some() {
                let instruction = self.pipeline_stage_2.as_ref().unwrap();
                if CONDITION_LUT[(((instruction.opcode >> 24) & 0xf0) | (self.cpsr_register >> 28)) as usize] {
                    (instruction.handler)(self, instruction.opcode);
                }
                if self.flush {
                    self.pipeline_flush();
                    return;
                }
            }
            if self.pipeline_stage_1.is_some() {
                let opcode = self.pipeline_stage_1.unwrap();
                let bits27_20 = (opcode >> 20) & 0xff;
                let bits7_4 = (opcode >> 4) & 0xf;
                self.pipeline_stage_2 = Some(PipelineStage2 { handler: ARM_INSTRUCTION_LUT[((bits27_20 << 4) | bits7_4) as usize], opcode });
            }
            self.pipeline_stage_1 = temp_pipeline_1;
            self.registers[15] += 4;
        }
    }

    fn output_registers(&mut self) {
        let spsr = self.get_current_saved_psr().to_owned();
        let mut lmao = BufWriter::new(&self.log);
        for i in 0..16 {
            lmao.write(&self.registers[i].to_le_bytes()).unwrap();
        }
        lmao.write(&self.cpsr_register.to_le_bytes()).unwrap();
        lmao.write(&spsr.to_le_bytes()).unwrap();
    }

    fn fetch_arm(&mut self) -> u32 {
        //println!("Fetching at {:#08x}", self.registers[15]);
        let instruction = self.memory.borrow().get_word(self.registers[15] & 0xffff_fffc);
        self.last_data_bus_read = instruction;
        instruction
    }

    fn fetch_thumb(&mut self) -> u32 {
        //println!("Fetching at {:#08x}", self.registers[15]);
        self.memory.borrow().get_halfword(self.registers[15] & 0xffff_fffe) as u32
    }

    pub(super) fn pipeline_flush(&mut self) {
        if (self.cpsr_register & STATE_BIT) == STATE_BIT {
            let opcode = self.fetch_thumb();
            let bits15_8 = opcode >> 8;
            self.pipeline_stage_2 = Some(PipelineStage2 { handler: THUMB_INSTRUCTION_LUT[bits15_8 as usize], opcode });
            self.registers[15] += 2;
            self.pipeline_stage_1 = Some(self.fetch_thumb());
            self.registers[15] += 2;
        } else {
            let opcode = self.fetch_arm();
            let bits27_20 = (opcode >> 20) & 0xff;
            let bits7_4 = (opcode >> 4) & 0xf;
            self.pipeline_stage_2 = Some(PipelineStage2 { handler: ARM_INSTRUCTION_LUT[((bits27_20 << 4) | bits7_4) as usize], opcode });
            self.registers[15] += 4;
            self.pipeline_stage_1 = Some(self.fetch_arm());
            self.registers[15] += 4;
        }
        self.flush = false;
    }

    fn switch_modes(&mut self, old_mode: u32) {
        match old_mode {
            USER_MODE | SYSTEM_MODE =>
                self.user_banked.copy_from_slice(&mut self.registers[13..15]),
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
        match self.cpsr_register & 0x1f {
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
            _ => panic!("Unrecognized mode"),
        }
    }

    pub(super) fn branch_and_exchange(&mut self, source_register: usize) {
        let address = self.registers[source_register];
        let thumb_bit = address & 0x1;
        self.cpsr_register = (self.cpsr_register & !STATE_BIT) | (thumb_bit << 5);
        self.registers[15] = address & !0x1;
        self.flush = true;
    }

    pub(super) fn branch(&mut self, link: bool, mut offset: i32) {
        offset = (offset << 8) >> 6;
        if link {
            self.registers[14] = self.registers[15] - 4;
        }
        self.registers[15] = ((self.registers[15] as i32) + offset) as u32;
        self.flush = true;
    }

    fn get_current_saved_psr(&mut self) -> &mut u32 {
        match self.cpsr_register & 0x1f {
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

    pub(super) fn restore_cpsr(&mut self) {
        let old_mode = self.cpsr_register & 0x1f;
        self.cpsr_register = *self.get_current_saved_psr();
        if old_mode != (self.cpsr_register & 0x1f) {
            self.switch_modes(old_mode);
        }
    }

    pub(super) fn msr(&mut self, destination_is_spsr: bool, mask: u32, operand_2: u32) {
        if (self.cpsr_register & 0x1f) == USER_MODE && (mask & 0xff) == 0xff {
            panic!("Tried to set control flags in user mode");
        }

        if destination_is_spsr {
            *self.get_current_saved_psr() = (operand_2 & mask) | (*self.get_current_saved_psr() & !mask);
        } else {
            let old_mode = self.cpsr_register & 0x1f;
            self.cpsr_register = (operand_2 & mask) | (self.cpsr_register & !mask);
            if old_mode != (self.cpsr_register & 0x1f) {
                self.switch_modes(old_mode);
            }
        }
    }

    pub(super) fn mrs(&mut self, source_is_spsr: bool, destination_register: usize) {
        if source_is_spsr {
            self.registers[destination_register] = *self.get_current_saved_psr();
        } else {
            self.registers[destination_register] = self.cpsr_register;
        }
    }

    pub(super) fn multiply(&mut self, accumulate: bool, set_conditions: bool, operand_1_register: usize, operand_2_register: usize, operand_3_register: usize, destination_register: usize) {
        let operand_1 = if accumulate {
            self.registers[operand_1_register]
        } else {
            0
        };
        let operand_2 = self.registers[operand_2_register];
        let operand_3 = self.registers[operand_3_register];
        let result = operand_3.wrapping_mul(operand_2).wrapping_add(operand_1);
        if set_conditions {
            self.set_multiply_flags(result);
        }
        self.registers[destination_register] = result;
    }

    pub(super) fn multiply_long(&mut self, signed: bool, accumulate: bool, set_conditions: bool, register_hi: usize, register_lo: usize, operand_1_register: usize, operand_2_register: usize) {
        let operand_1 = self.registers[operand_1_register];
        let operand_2 = self.registers[operand_2_register];

        let operand_3 = if accumulate { ((self.registers[register_hi] as u64) << 32) | self.registers[register_lo] as u64 } else { 0 };

        let result = if signed {
            (((operand_2 as i32) as i64) * ((operand_1 as i32) as i64)).wrapping_add(operand_3 as i64) as u64
        } else {
            ((operand_2 as u64) * (operand_1 as u64)).wrapping_add(operand_3)
        };

        self.registers[register_hi as usize] = ((result & 0xffff_ffff_0000_0000) >> 32) as u32;
        self.registers[register_lo as usize] = (result & 0xffff_ffff) as u32;
        if set_conditions {
            self.set_long_multiply_flags(result);
        }
    }

    fn set_long_multiply_flags(&mut self, result: u64) {
        self.cpsr_register &= 0x2fff_ffff;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        if check_bit!(result, 63) {
            self.cpsr_register |= SIGN_FLAG;
        }
    }

    fn set_multiply_flags(&mut self, result: u32) {
        self.cpsr_register &= 0x2fff_ffff;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        if check_bit!(result, 31) {
            self.cpsr_register |= SIGN_FLAG;
        }
    }

    pub(super) fn single_data_transfer(
        &mut self,
        pre_indexing: bool,
        add_offset: bool,
        transfer_byte: bool,
        write_back: bool,
        load: bool,
        base_register: usize,
        offset: u32,
        src_dst_register: usize
    ) {
        let mut address = self.registers[base_register];
        self.registers[15] += 4;

        if pre_indexing {
            if add_offset {
                address += offset;
            } else {
                address -= offset;
            }
        }

        if load {
            self.load_memory(address, src_dst_register, transfer_byte);
        } else {
            self.store_memory(address, src_dst_register, transfer_byte);
        }

        if !pre_indexing {
            if add_offset {
                address += offset;
            } else {
                address -= offset;
            }
        } 
        if (write_back || !pre_indexing) && (!load || (load && src_dst_register != base_register)) {
            self.registers[base_register] = address;
        }
    }

    fn load_memory(&mut self, address: u32, dst_register: usize, is_byte: bool) {
        if address > 0x1000_0000 {
            self.registers[dst_register] = self.last_data_bus_read;
            return;
        }
        let memory = self.memory.borrow();
        self.registers[dst_register] = if is_byte {
            memory[address as usize] as u32
        } else {
            let mut value = memory.get_byte(address) as u32;
            let mut address_rotate = (address & 0xFFFF_FFFC) + (address + 1) % 4;
            value |= (memory.get_byte(address_rotate) as u32) << 8;
            address_rotate = (address & 0xFFFF_FFFC) + (address + 2) % 4;
            value |= (memory.get_byte(address_rotate) as u32) << 16;
            address_rotate = (address & 0xFFFF_FFFC) + (address + 3) % 4;
            value |= (memory.get_byte(address_rotate) as u32) << 24;
            value
        };
        self.last_data_bus_read = self.registers[dst_register];
    }

    fn store_memory(&mut self, address: u32, src_register: usize, is_byte: bool) {
        let value_to_store = self.registers[src_register];
        if !is_byte {
            self.memory.borrow_mut().store_word(address & 0xFFFF_FFFC, value_to_store);
        } else {
            self.memory.borrow_mut().store_byte(address, value_to_store as u8);
        }
    }

    pub(super) fn halfword_data_transfer(
        &mut self,
        immediate: bool,
        pre_indexing: bool,
        add_offset: bool,
        write_back: bool,
        load: bool,
        halfword_transfer_type: HalfwordTransferType,
        base_register: usize,
        src_dst_register: usize,
        offset_value: u32,
        offset_register: usize
    ) {
        let mut address = self.registers[base_register];
        self.registers[15] += 4;
        let offset = match immediate {
            true => offset_value,
            false => self.registers[offset_register],
        };

        if pre_indexing {
            if add_offset {
                address += offset;
            } else {
                address -= offset;
            }
        }

        if load {
            self.load_halfword(address, src_dst_register, halfword_transfer_type);
        } else {
            self.store_halfword(address, src_dst_register, halfword_transfer_type);
        }

        if !pre_indexing {
            if add_offset {
                address += offset;
            } else {
                address -= offset;
            }
        } 
        if (write_back || !pre_indexing) && (!load || (load && src_dst_register != base_register)) {
            self.registers[base_register] = address;
        }
    }

    fn load_halfword(
        &mut self,
        address: u32,
        dst_register: usize,
        halfword_transfer_type: HalfwordTransferType
    ) {
        if address > 0x1000_0000 {
            match halfword_transfer_type {
                HalfwordTransferType::UnsignedHalfwords => {
                    self.registers[dst_register] = self.last_data_bus_read & 0xFFFF;
                }
                HalfwordTransferType::SignedByte => {
                    self.registers[dst_register] = (self.last_data_bus_read & 0xFF) as i8 as i32 as u32;
                }
                HalfwordTransferType::SignedHalfwords => {
                    self.registers[dst_register] = (self.last_data_bus_read & 0xFFFF) as i16 as i32 as u32;
                }
                HalfwordTransferType::NoOp =>
                    panic!("Something went terribly wrong while loading a halfword"),
            }
        } else {
            let value = self.memory.borrow().get_halfword(address & !1);
            match (halfword_transfer_type, check_bit!(address, 0)) {
                (HalfwordTransferType::UnsignedHalfwords, false) => {
                    self.registers[dst_register] = value as u32;
                }
                (HalfwordTransferType::UnsignedHalfwords, true) => {
                    self.registers[dst_register] = (value as u32).rotate_right(8);
                }
                (HalfwordTransferType::SignedByte, _) => {
                    self.registers[dst_register] = (value & 0xff) as i8 as i32 as u32;
                }
                (HalfwordTransferType::SignedHalfwords, false) => {
                    self.registers[dst_register] = value as i16 as i32 as u32;
                }
                (HalfwordTransferType::SignedHalfwords, true) => {
                    self.registers[dst_register] = (value as i16 as i32 >> 8) as u32;
                }
                (HalfwordTransferType::NoOp, _) =>
                    panic!("Something went terribly wrong while loading a halfword"),
            }
            self.last_data_bus_read = self.memory.borrow().get_word(address & 0xFFFF_FFFC);
        }
    }

    fn store_halfword(
        &mut self,
        address: u32,
        src_register: usize,
        halfword_transfer_type: HalfwordTransferType
    ) {
        let value = self.registers[src_register] as u16;
        if halfword_transfer_type == HalfwordTransferType::UnsignedHalfwords {
            self.memory.borrow_mut().store_halfword(address / 2 * 2, value);
        } else {
            panic!("Something went terribly wrong while storing a halfword");
        }
    }

    pub(super) fn block_data_transfer(
        &mut self,
        pre_indexing: bool,
        add_offset: bool,
        load_psr: bool,
        write_back: bool,
        load: bool,
        base_register: usize,
        mut register_mask: u32
    ) {
        let mut address = self.registers[base_register];
        let number_registers = register_mask.count_ones();
        let mut registers = Vec::with_capacity(number_registers as usize);
        let mut final_address = 0;
        for i in 0u8..16u8 {
            if (register_mask & 0x1) == 0x1 {
                registers.push(i);
            }
            register_mask = register_mask >> 1;
        }

        if pre_indexing {
            if add_offset {
                final_address += address + 4 * number_registers;
                address += 4;
            } else {
                final_address += address - 4 * number_registers;
                address = final_address;
            }
        } else {
            if add_offset {
                final_address += address + 4 * number_registers;
            } else {
                final_address += address - 4 * number_registers;
                address = final_address + 4;
            }
        }
        // Store old mode if S bit is set to transfer user mode registers
        let old_mode = self.cpsr_register & 0x1f;
        if load_psr && (self.cpsr_register & USER_MODE) != USER_MODE {
            self.cpsr_register = (self.cpsr_register & 0xffff_ffe0) | USER_MODE;
            self.switch_modes(old_mode);
        }

        if load {
            self.load_multiple(address, &registers, old_mode, load_psr);
        } else {
            self.store_multiple(address, &registers);
        }

        if load_psr {
            self.cpsr_register = (self.cpsr_register & 0xffff_ffe0) | old_mode;
            self.switch_modes(USER_MODE);
        }

        if write_back {
            if write_back {
                self.registers[base_register] = final_address;
            } else {
                self.registers[base_register] = final_address;
            }
        }
    }

    fn load_multiple(&mut self, mut address: u32, registers: &[u8], old_mode: u32, psr_bit: bool) {
        for register in registers {
            if *register == 15 {
                self.flush = true;
                if psr_bit {
                    self.cpsr_register = (self.cpsr_register & 0xffff_ffe0) | old_mode;
                    self.cpsr_register = *self.get_current_saved_psr();
                    self.cpsr_register = (self.cpsr_register & 0xffff_ffe0) | USER_MODE;
                }
            }
            self.registers[*register as usize] = self.memory.borrow().get_word(address);
            address += 4;
        }
    }

    fn store_multiple(&mut self, mut address: u32, registers: &[u8]) {
        for register in registers {
            if *register == 15 {
                self.flush = true;
            }
            let value_to_store = self.registers[*register as usize];
            self.memory.borrow_mut().store_word(address, value_to_store);
            address += 4;
        }
    }

    pub(super) fn single_data_swap(&mut self, transfer_byte: bool, address_register: usize, dst_register: usize, src_register: usize) {
        let address = self.registers[address_register];
        let old_dst_register = self.registers[dst_register];
        self.load_memory(address, dst_register, transfer_byte);
        let new_dst_register = self.registers[dst_register];
        self.registers[dst_register] = old_dst_register;
        self.store_memory(address, src_register, transfer_byte);
        self.registers[dst_register] = new_dst_register;
    }

    fn software_interrupt(&mut self) {

    }
}
