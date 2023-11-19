use std::{ cell::RefCell, rc::Rc, fs::{ File, OpenOptions }, io::{ Write, BufWriter } };

use crate::{ memory::Memory, check_bit, get_register_number_at };

use super::{constants::*, lut};
use super::lut::{
    ShiftType,
    HalfwordTransferType,
    Instruction,
    Operand2Type,
    condition_lut,
    instruction_lut,
};

const CONDITION_LUT: [bool; 256] = condition_lut();
const INSTRUCTION_LUT: [Instruction; 4096] = instruction_lut();

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
    log: File,
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
            log: lmao,
        };
        arm7.registers[13] = STACK_USER_SYSTEM_START;
        arm7.irq_banked[0] = STACK_IRQ_START;
        arm7.supervisor_banked[0] = STACK_SUPERVISOR_START;
        arm7.registers[15] = START_PC;
        arm7
    }

    pub fn next(&mut self) {
        self.output_registers();
        // THUMB MODE
        if (self.cpsr_register & STATE_BIT) == STATE_BIT {
            let opcode = self.fetch_thumb();
            //self.decode_thumb(opcode);
            self.registers[15] += 2;
        } else {
            // ARM MODE
            let opcode = self.fetch_arm();
            self.decode_arm(opcode);
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
        self.memory.borrow().get_word(self.registers[15] & 0xffff_fffc)
    }

    fn fetch_thumb(&mut self) -> u16 {
        //println!("Fetching at {:#08x}", self.registers[15]);
        self.memory.borrow().get_halfword(self.registers[15] & 0xffff_fffe)
    }

    fn decode_arm(&mut self, opcode: u32) {
        //println!("Decoding {:#08x}", opcode);
        if !CONDITION_LUT[(((opcode >> 24) & 0xf0) | (self.cpsr_register >> 28)) as usize] {
            return;
        }
        let bits27_20 = (opcode >> 20) & 0xff;
        let bits7_4 = (opcode >> 4) & 0xf;
        let instruction = INSTRUCTION_LUT[((bits27_20 << 4) | bits7_4) as usize];
        match instruction {
            Instruction::BranchAndExchange => self.branch_and_exchange(opcode),
            Instruction::Alu { operand2_type, opcode: alu_opcode, set_conditions, shift_type } =>
                self.alu_command(operand2_type, alu_opcode, set_conditions, shift_type, opcode),
            Instruction::Branch { link } => self.branch(link, opcode),
            Instruction::MRSTransfer { source_is_spsr } =>
                self.mrs(source_is_spsr, get_register_number_at!(opcode, 12)),
            Instruction::MSRTransfer { operand2_type, destination_is_spsr } =>
                self.msr(operand2_type, destination_is_spsr, opcode),
            Instruction::Multiply { accumulate, set_conditions } =>
                self.multiply(accumulate, set_conditions, opcode),
            Instruction::MultiplyLong { signed, accumulate, set_conditions } =>
                self.multiply_long(signed, accumulate, set_conditions, opcode),
            Instruction::SingleDataTransfer {
                operand2_type,
                pre_indexing,
                add_offset,
                transfer_byte,
                write_back,
                load,
                shift_type,
            } =>
                self.single_data_transfer(
                    operand2_type,
                    pre_indexing,
                    add_offset,
                    transfer_byte,
                    write_back,
                    load,
                    shift_type,
                    opcode
                ),
            Instruction::HalfowrdTransfer {
                immediate,
                pre_indexing,
                add_offset,
                write_back,
                load,
                halfword_transfer_type,
            } =>
                self.halfword_data_transfer(
                    immediate,
                    pre_indexing,
                    add_offset,
                    write_back,
                    load,
                    halfword_transfer_type,
                    opcode
                ),
            Instruction::BlockDataTransfer {
                pre_indexing,
                add_offset,
                load_psr,
                write_back,
                load,
            } =>
                self.block_data_transfer(
                    pre_indexing,
                    add_offset,
                    load_psr,
                    write_back,
                    load,
                    opcode
                ),
            Instruction::SingleDataSwap { transfer_byte } =>
                self.single_data_swap(transfer_byte, opcode),
            Instruction::SoftwareInterrupt => todo!(),
            Instruction::Undefined => todo!(),
            Instruction::NoOp => panic!(),
        }
    }

    /* fn decode_thumb(&mut self, opcode: u16) {
        match opcode & 0xE000 {
            0x0 => match  opcode & 0x1800 {
                0x1800 => (),
                _ => self.convert_move_shifted(opcode)
            },
            0x2000 => self.convert_alu_immediate(opcode),
            0x4000 => match opcode & 0x1000 {
                0x1000 => (),
                _ => match opcode & 0x800 {
                    0x800 => (),
                    _ => match opcode & 0x400 {
                        0x400 => self.hi_register_operation(opcode),
                        _ => ()
                    }
                },
            },
            0x6000 => (),
            0x8000 => (),
            0xA000 => self.load_address(opcode),
            0xC000 => (),
            0xE000 => (),
            _ => panic!("Undefinied instruction"),
        }
    }

    fn convert_move_shifted(&mut self, opcode: u16) {
        let source_register = Self::get_rs_t_register_number(opcode) as u32;
        let destination_register = (Self::get_rd_t_register_number(opcode) as u32) << 12;
        let offset = ((opcode << 1) & 0xF80) as u32;
        let opcode = 0xE1B0_0000 | source_register | destination_register | offset;
        self.alu_command(opcode);
    }

    fn convert_alu_immediate(&mut self, opcode: u16) {
        let src_dst_register = Self::get_rb_t_register_number(opcode) as u32;
        let alu_opcode = match (opcode >> 11) & 0x2 {
            0x0 => 0xD,
            0x1 => 0xA,
            0x2 => 0x4,
            0x3 => 0x2,
            _ => panic!("Undefined opcode")
        };
        let offset = opcode & 0xFF;
        let opcode = 0xE3B0_0000 | (alu_opcode << 21) | (src_dst_register << 16) | (src_dst_register << 12) | offset as u32;
        self.alu_command(opcode);
    }

    fn load_address(&mut self, opcode: u16) {
        self.registers[15] += 4;
        let source_register = if check_bit!(opcode, 11) { 13 } else { 15 };
        let destination_register = Self::get_rb_t_register_number(opcode);
        let offset = ((opcode & 0xFF) << 2) as u32;
        self.registers[destination_register] = offset + self.registers[source_register];
        self.registers[15] -= 4;
    }

    fn hi_register_operation(&mut self, opcode: u16) {
        let destination_register = ((opcode as u32 >> 4) & 0x8) | Self::get_rd_t_register_number(opcode) as u32;
        let source_register = ((opcode as u32 >> 3) & 0x8) | Self::get_rs_t_register_number(opcode) as u32;
        let alu_opcode = match (opcode >> 8) & 0x3 {
            0x0 => 0x80_0000,
            0x1 => 0x130_0000,
            0x2 => 0x1A0_0000,
            0x3 => {
                self.branch_and_exchange(source_register);
                return;
            }
            _ => panic!("Undefined opcode")
        };
        let opcode = alu_opcode | (destination_register << 16) | (destination_register << 12) | source_register;
        self.alu_command(opcode);
    } */

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

    fn branch_and_exchange(&mut self, opcode: u32) {
        let mut address = self.registers[get_register_number_at!(opcode, 0)];
        // Compensation needs to be done because PC is incremented after instruction is executed
        // THUMB MODE
        if (self.cpsr_register & STATE_BIT) != 0 {
            address -= 2;
        } else {
            // ARM MODE
            address -= 4;
        }
        let thumb_bit = address & 0x1;
        self.cpsr_register = (self.cpsr_register & !STATE_BIT) | (thumb_bit << 5);
        self.registers[15] = address & !0x1;
    }

    fn branch(&mut self, link: bool, opcode: u32) {
        // Due to prefetching, the PC should be 8 bytes ahead
        let offset = (opcode & 0xff_ffff) as i32;
        let correct_ofset = ((offset << 8) >> 6) + 4;
        if link {
            self.registers[14] = self.registers[15];
        }
        self.registers[15] = ((self.registers[15] as i32) + correct_ofset) as u32;
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
        self.switch_modes(old_mode)
    }

    fn msr(&mut self, operand2_type: Operand2Type, destination_is_spsr: bool, opcode: u32) {
        let mask: u32 = match opcode & (0xf << 16) {
            0x1_0000 => 0xff,
            0x2_0000 => 0xff00,
            0x3_0000 => 0xffff,
            0x4_0000 => 0xff0000,
            0x5_0000 => 0xff00ff,
            0x6_0000 => 0xffff00,
            0x7_0000 => 0xffffff,
            0x8_0000 => 0xff000000,
            0x9_0000 => 0xff0000ff,
            0xa_0000 => 0xff00ff00,
            0xb_0000 => 0xff00ffff,
            0xc_0000 => 0xffff0000,
            0xd_0000 => 0xffff00ff,
            0xe_0000 => 0xffffff00,
            0xf_0000 => 0xffffffff,
            _ => panic!(),
        };
        let operand_2: u32 = self.get_operand2(
            operand2_type,
            super::lut::ShiftType::LogicalLeft,
            false,
            opcode
        );

        if (self.cpsr_register & 0x1f) == USER_MODE && (mask & 0xff) == 0xff {
            panic!("Tried to set control flags in user mode");
        }

        let old_mode = self.cpsr_register & 0x1f;
        self.cpsr_register = (operand_2 & mask) | (self.cpsr_register & !mask);
        if old_mode != (self.cpsr_register & 0x1f) {
            self.switch_modes(old_mode);
        }
    }

    fn mrs(&mut self, source_is_spsr: bool, destination_register: usize) {
        if source_is_spsr {
            self.registers[destination_register] = self.cpsr_register;
        } else {
            self.registers[destination_register] = *self.get_current_saved_psr();
        }
    }

    fn multiply(&mut self, accumulate: bool, set_conditions: bool, opcode: u32) {
        let operand_1 = if accumulate {
            self.registers[get_register_number_at!(opcode, 12)]
        } else {
            0
        };
        let operand_2 = self.registers[get_register_number_at!(opcode, 8)];
        let operand_3 = self.registers[get_register_number_at!(opcode, 0)];
        let destination_register = get_register_number_at!(opcode, 16);
        let result = operand_3.wrapping_mul(operand_2).wrapping_add(operand_1);
        if set_conditions {
            self.set_multiply_flags(result);
        }
        self.registers[destination_register] = result;
    }

    fn multiply_long(&mut self, signed: bool, accumulate: bool, set_conditions: bool, opcode: u32) {
        let register_hi = self.registers[get_register_number_at!(opcode, 16)] as u64;
        let register_lo = self.registers[get_register_number_at!(opcode, 12)] as u64;
        let operand_1 = self.registers[get_register_number_at!(opcode, 8)];
        let operand_2 = self.registers[get_register_number_at!(opcode, 0)];

        let operand_3 = if accumulate { (register_hi << 32) | register_lo } else { 0 };

        let result = if signed {
            ((operand_2 as i64) * (operand_1 as i64)).wrapping_add(operand_3 as i64) as u64
        } else {
            ((operand_2 as u64) * (operand_1 as u64)).wrapping_add(operand_3)
        };

        self.registers[register_hi as usize] = (result & (0xffff_ffff_0000_0000 >> 32)) as u32;
        self.registers[register_lo as usize] = (result & 0xffff_ffff) as u32;
        if set_conditions {
            self.set_long_multiply_flags(result);
        }
    }

    fn set_long_multiply_flags(&mut self, result: u64) {
        self.cpsr_register &= 0x1fff_ffff;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        if check_bit!(result, 63) {
            self.cpsr_register |= SIGN_FLAG;
        }
    }

    fn set_multiply_flags(&mut self, result: u32) {
        self.cpsr_register &= 0x1fff_ffff;
        if result == 0 {
            self.cpsr_register |= ZERO_FLAG;
        }
        if check_bit!(result, 31) {
            self.cpsr_register |= SIGN_FLAG;
        }
    }

    fn single_data_transfer(
        &mut self,
        operand2_type: Operand2Type,
        pre_indexing: bool,
        add_offset: bool,
        transfer_byte: bool,
        write_back: bool,
        load: bool,
        shift_type: ShiftType,
        opcode: u32
    ) {
        self.registers[15] += 8;
        let base_register = get_register_number_at!(opcode, 16);
        let mut address = self.registers[base_register];
        self.registers[15] += 4;
        let src_dst_register = get_register_number_at!(opcode, 12);
        let offset = self.get_operand2(operand2_type, shift_type, true, opcode);

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

        if write_back {
            self.registers[base_register] = address;
        }
        self.registers[15] -= 12;
    }

    fn load_memory(&mut self, address: u32, dst_register: usize, is_byte: bool) {
        self.registers[dst_register] = if is_byte {
            self.memory.borrow()[address as usize] as u32
        } else {
            let value = self.memory.borrow().get_word(address);
            if address % 4 != 0 {
                self.barrel_shifter(value, (address & 3) * 8, ShiftType::RotateRight, false, true)
            } else {
                value
            }
        };
    }

    fn store_memory(&mut self, address: u32, src_register: usize, is_byte: bool) {
        let value_to_store = self.registers[src_register];
        if !is_byte {
            self.memory.borrow_mut().store_word(address, value_to_store);
        } else {
            self.memory.borrow_mut().store_byte(address, value_to_store as u8);
        }
    }

    fn halfword_data_transfer(
        &mut self,
        immediate: bool,
        pre_indexing: bool,
        add_offset: bool,
        write_back: bool,
        load: bool,
        halfword_transfer_type: HalfwordTransferType,
        opcode: u32
    ) {
        self.registers[15] += 8;
        let base_register = get_register_number_at!(opcode, 16);
        let mut address = self.registers[base_register];
        self.registers[15] += 4;
        let src_dst_register = get_register_number_at!(opcode, 12);
        let offset = match immediate {
            true => (opcode & 0xf) | ((opcode & 0xf00) >> 4),
            false => self.registers[get_register_number_at!(opcode, 0)],
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
            if (opcode & 0x80_0000) == 0x80_0000 {
                address += offset;
            } else {
                address -= offset;
            }
        }

        if write_back {
            self.registers[base_register] = address;
        }
        self.registers[15] -= 12;
    }

    fn load_halfword(
        &mut self,
        address: u32,
        dst_register: usize,
        halfword_transfer_type: HalfwordTransferType
    ) {
        let value = self.memory.borrow().get_halfword(address);
        match halfword_transfer_type {
            HalfwordTransferType::UnsignedHalfwords => {
                self.registers[dst_register] = value as u32;
            }
            HalfwordTransferType::SignedByte => {
                self.registers[dst_register] = (value & 0xff) as i8 as i32 as u32;
            }
            HalfwordTransferType::SignedHalfwords => {
                self.registers[dst_register] = value as i16 as i32 as u32;
            }
            HalfwordTransferType::NoOp =>
                panic!("Something went terribly wrong while loading a halfword"),
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
            self.memory.borrow_mut().store_halfword(address, value);
        } else {
            panic!("Something went terribly wrong while storing a halfword");
        }
    }

    fn block_data_transfer(
        &mut self,
        pre_indexing: bool,
        add_offset: bool,
        load_psr: bool,
        write_back: bool,
        load: bool,
        opcode: u32
    ) {
        let base_register = get_register_number_at!(opcode, 16);
        let mut address = self.registers[base_register];
        let mut register_mask = opcode & 0xffff;
        let number_registers = register_mask.count_ones();
        let mut registers = Vec::new();
        registers.reserve(number_registers as usize);
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
            if *register == 15 && psr_bit {
                self.cpsr_register = (self.cpsr_register & 0xffff_ffe0) | old_mode;
                self.cpsr_register = *self.get_current_saved_psr();
                self.cpsr_register = (self.cpsr_register & 0xffff_ffe0) | USER_MODE;
            }
            self.registers[*register as usize] = self.memory.borrow().get_word(address);
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

    fn single_data_swap(&mut self, transfer_byte: bool, opcode: u32) {
        let address = self.registers[get_register_number_at!(opcode, 16)];
        let dst_register = get_register_number_at!(opcode, 12);
        let src_register = get_register_number_at!(opcode, 0);
        self.load_memory(address, dst_register, transfer_byte);
        self.store_memory(address, src_register, transfer_byte);
    }
}
