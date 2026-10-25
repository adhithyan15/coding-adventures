//! Independent MC6809 transition engine built from fixed-width gate networks.
//!
//! The functional crate is deliberately absent from this module.  It supplies
//! the public state and error vocabulary at the package boundary, while this
//! engine fetches, decodes, addresses, and executes from an owned state copy.

use crate::{
    gates, indexed_postbyte_is_valid, opcode_is_documented, Motorola6809Error, Motorola6809State,
    StepTrace, CC_C, CC_E, CC_F, CC_H, CC_I, CC_N, CC_V, CC_Z,
};

#[derive(Clone, Copy)]
enum Mode {
    Immediate,
    Direct,
    Indexed,
    Extended,
}

struct GateEngine {
    state: Motorola6809State,
    fetched: usize,
    fetched_bytes: [u8; 8],
}

pub(crate) fn execute(before: &Motorola6809State) -> Result<StepTrace, Motorola6809Error> {
    if before.halted {
        return Err(Motorola6809Error::Halted);
    }
    let mut engine = GateEngine {
        state: before.clone(),
        fetched: 0,
        fetched_bytes: [0; 8],
    };
    engine.step_inner()
}

impl GateEngine {
    fn set_d(&mut self, value: u16) {
        [self.state.a, self.state.b] = value.to_be_bytes();
    }

    fn step_inner(&mut self) -> Result<StepTrace, Motorola6809Error> {
        let pc_before = self.state.pc;
        self.fetched = 0;
        self.fetched_bytes = [0; 8];
        let first = gates::decode_value(self.fetch_byte(pc_before)?)?;
        let (page, opcode) = if matches!(first, 0x10 | 0x11) {
            (first, gates::decode_value(self.fetch_byte(pc_before)?)?)
        } else {
            (0, first)
        };
        if !opcode_is_documented(page, opcode) {
            let full = if page == 0 {
                u16::from(opcode)
            } else {
                u16::from_be_bytes([page, opcode])
            };
            return Err(Motorola6809Error::UnknownOpcode {
                opcode: full,
                pc: pc_before,
            });
        }
        let mnemonic = match page {
            0 => self.execute_base(opcode, pc_before)?,
            0x10 => self.execute_page2(opcode, pc_before)?,
            0x11 => self.execute_page3(opcode, pc_before)?,
            _ => unreachable!(),
        };
        let raw = self.fetched_bytes[..self.fetched].to_vec();
        Ok(StepTrace {
            pc_before,
            pc_after: self.state.pc,
            raw,
            width: u8::try_from(self.fetched).expect("MC6809 instruction width fits u8"),
            mnemonic,
            state_after: self.state.clone(),
        })
    }

    fn fetch_byte(&mut self, start_pc: u16) -> Result<u8, Motorola6809Error> {
        let pc = self.state.pc;
        let start = usize::from(self.state.loaded_origin);
        let end = start + self.state.loaded_len;
        let index = usize::from(pc);
        if index < start || index >= end {
            return if self.fetched == 0 {
                Err(Motorola6809Error::FetchOutsideProgram { pc })
            } else {
                Err(Motorola6809Error::TruncatedInstruction {
                    pc: start_pc,
                    needed: self.fetched + 1,
                })
            };
        }
        let value = gates::decode_value(self.state.memory[index])?;
        let Some(slot) = self.fetched_bytes.get_mut(self.fetched) else {
            return Err(Motorola6809Error::InvalidState {
                reason: "instruction exceeds the MC6809 maximum encoded width",
            });
        };
        *slot = value;
        self.state.pc = gates::add_16(pc, 1, 0).0;
        self.fetched += 1;
        Ok(value)
    }

    fn fetch_word(&mut self, start_pc: u16) -> Result<u16, Motorola6809Error> {
        Ok(u16::from_be_bytes([
            self.fetch_byte(start_pc)?,
            self.fetch_byte(start_pc)?,
        ]))
    }

    fn read16(&self, address: u16) -> u16 {
        u16::from_be_bytes([
            self.state.memory[usize::from(address)],
            self.state.memory[usize::from(gates::add_16(address, 1, 0).0)],
        ])
    }

    fn write16(&mut self, address: u16, value: u16) {
        let [hi, lo] = value.to_be_bytes();
        self.state.memory[usize::from(address)] = hi;
        self.state.memory[usize::from(gates::add_16(address, 1, 0).0)] = lo;
    }

    fn mode_for(opcode: u8) -> Mode {
        match opcode >> 4 {
            0x8 | 0xc => Mode::Immediate,
            0x9 | 0xd => Mode::Direct,
            0xa | 0xe => Mode::Indexed,
            0xb | 0xf => Mode::Extended,
            _ => unreachable!(),
        }
    }

    fn address(&mut self, mode: Mode, pc: u16) -> Result<u16, Motorola6809Error> {
        match mode {
            Mode::Direct => Ok(u16::from_be_bytes([self.state.dp, self.fetch_byte(pc)?])),
            Mode::Indexed => self.indexed_address(pc),
            Mode::Extended => self.fetch_word(pc),
            Mode::Immediate => unreachable!(),
        }
    }

    fn operand8(&mut self, mode: Mode, pc: u16) -> Result<u8, Motorola6809Error> {
        if matches!(mode, Mode::Immediate) {
            self.fetch_byte(pc)
        } else {
            let address = self.address(mode, pc)?;
            Ok(self.state.memory[usize::from(address)])
        }
    }

    fn operand16(&mut self, mode: Mode, pc: u16) -> Result<u16, Motorola6809Error> {
        if matches!(mode, Mode::Immediate) {
            self.fetch_word(pc)
        } else {
            let address = self.address(mode, pc)?;
            Ok(self.read16(address))
        }
    }

    fn index_register(&self, code: u8) -> u16 {
        match code {
            0 => self.state.x,
            1 => self.state.y,
            2 => self.state.u,
            _ => self.state.s,
        }
    }

    fn set_index_register(&mut self, code: u8, value: u16) {
        match code {
            0 => self.state.x = value,
            1 => self.state.y = value,
            2 => self.state.u = value,
            _ => self.state.s = value,
        }
    }

    fn indexed_address(&mut self, pc: u16) -> Result<u16, Motorola6809Error> {
        let postbyte = gates::decode_value(self.fetch_byte(pc)?)?;
        if !indexed_postbyte_is_valid(postbyte) {
            return Err(Motorola6809Error::InvalidIndexedPostbyte { postbyte, pc });
        }
        if postbyte & 0x80 == 0 {
            let register = (postbyte >> 5) & 3;
            let raw = i16::from(postbyte & 0x1f);
            let offset = if raw & 0x10 != 0 { raw - 0x20 } else { raw };
            return Ok(gates::add_signed_16(self.index_register(register), offset));
        }
        let register = (postbyte >> 5) & 3;
        let indirect = postbyte & 0x10 != 0;
        let mode = postbyte & 0x0f;
        let mut address = match mode {
            0 => {
                let value = self.index_register(register);
                self.set_index_register(register, gates::add_16(value, 1, 0).0);
                value
            }
            1 => {
                let value = self.index_register(register);
                self.set_index_register(register, gates::add_16(value, 2, 0).0);
                value
            }
            2 => {
                let value = gates::sub_16(self.index_register(register), 1).0;
                self.set_index_register(register, value);
                value
            }
            3 => {
                let value = gates::sub_16(self.index_register(register), 2).0;
                self.set_index_register(register, value);
                value
            }
            4 => self.index_register(register),
            5 => gates::add_signed_16(self.index_register(register), i16::from(self.state.b as i8)),
            6 => gates::add_signed_16(self.index_register(register), i16::from(self.state.a as i8)),
            8 => {
                let offset = self.fetch_byte(pc)? as i8;
                gates::add_signed_16(self.index_register(register), i16::from(offset))
            }
            9 => {
                let offset = self.fetch_word(pc)? as i16;
                gates::add_signed_16(self.index_register(register), offset)
            }
            0x0b => gates::add_signed_16(self.index_register(register), self.state.d() as i16),
            0x0c => {
                let offset = self.fetch_byte(pc)? as i8;
                gates::add_signed_16(self.state.pc, i16::from(offset))
            }
            0x0d => {
                let offset = self.fetch_word(pc)? as i16;
                gates::add_signed_16(self.state.pc, offset)
            }
            0x0f => self.fetch_word(pc)?,
            _ => unreachable!(),
        };
        if indirect {
            address = self.read16(address);
        }
        Ok(address)
    }

    fn set_flag(&mut self, flag: u8, set: bool) {
        let retained = gates::bit_and_8(self.state.cc, gates::bit_not_8(flag));
        self.state.cc = gates::bit_or_8(retained, gates::select_8(0, flag, u8::from(set)));
    }

    fn set_nz8(&mut self, value: u8) {
        self.set_flag(CC_N, gates::negative_8(value) == 1);
        self.set_flag(CC_Z, gates::is_zero_8(value) == 1);
    }

    fn set_nz16(&mut self, value: u16) {
        self.set_flag(CC_N, gates::negative_16(value) == 1);
        self.set_flag(CC_Z, gates::is_zero_16(value) == 1);
    }

    fn load8_flags(&mut self, value: u8) {
        self.set_nz8(value);
        self.set_flag(CC_V, false);
    }

    fn load16_flags(&mut self, value: u16) {
        self.set_nz16(value);
        self.set_flag(CC_V, false);
    }

    fn add8(&mut self, left: u8, right: u8, carry: bool) -> u8 {
        let (result, carry_out, overflow, half_carry) = gates::add_8(left, right, u8::from(carry));
        self.set_flag(CC_H, half_carry == 1);
        self.set_nz8(result);
        self.set_flag(CC_V, overflow == 1);
        self.set_flag(CC_C, carry_out == 1);
        result
    }

    fn sub8(&mut self, left: u8, right: u8, borrow: bool) -> u8 {
        let (result, borrow_out, overflow) = gates::sub_8(left, right, u8::from(borrow));
        self.set_nz8(result);
        self.set_flag(CC_V, overflow == 1);
        self.set_flag(CC_C, borrow_out == 1);
        result
    }

    fn add16(&mut self, left: u16, right: u16) -> u16 {
        let (result, carry_out, overflow) = gates::add_16(left, right, 0);
        self.set_nz16(result);
        self.set_flag(CC_V, overflow == 1);
        self.set_flag(CC_C, carry_out == 1);
        result
    }

    fn sub16(&mut self, left: u16, right: u16) -> u16 {
        let (result, borrow_out, overflow) = gates::sub_16(left, right);
        self.set_nz16(result);
        self.set_flag(CC_V, overflow == 1);
        self.set_flag(CC_C, borrow_out == 1);
        result
    }

    fn branch_condition(&self, code: u8) -> bool {
        gates::branch_selected(0x20 | gates::bit_and_8(code, 0x0f), self.state.cc) == 1
    }

    fn execute_base(&mut self, opcode: u8, pc: u16) -> Result<&'static str, Motorola6809Error> {
        match opcode {
            0x00..=0x0f => self.execute_memory_rmw(opcode, Mode::Direct, pc),
            0x10 | 0x11 => unreachable!(),
            0x12 => Ok("NOP"),
            0x13 => {
                self.state.halted = true;
                Ok("SYNC")
            }
            0x16 => {
                let offset = self.fetch_word(pc)? as i16;
                self.state.pc = gates::add_signed_16(self.state.pc, offset);
                Ok("LBRA")
            }
            0x17 => {
                let offset = self.fetch_word(pc)? as i16;
                let target = gates::add_signed_16(self.state.pc, offset);
                self.push16_s(self.state.pc);
                self.state.pc = target;
                Ok("LBSR")
            }
            0x19 => {
                self.daa();
                Ok("DAA")
            }
            0x1a => {
                self.state.cc = gates::bit_or_8(self.state.cc, self.fetch_byte(pc)?);
                Ok("ORCC")
            }
            0x1c => {
                self.state.cc = gates::bit_and_8(self.state.cc, self.fetch_byte(pc)?);
                Ok("ANDCC")
            }
            0x1d => {
                self.state.a = gates::select_8(0, 0xff, gates::negative_8(self.state.b));
                self.set_nz16(self.state.d());
                Ok("SEX")
            }
            0x1e => {
                let postbyte = self.fetch_byte(pc)?;
                self.transfer(postbyte, true, pc)?;
                Ok("EXG")
            }
            0x1f => {
                let postbyte = self.fetch_byte(pc)?;
                self.transfer(postbyte, false, pc)?;
                Ok("TFR")
            }
            0x20..=0x2f => {
                let offset = self.fetch_byte(pc)? as i8;
                if self.branch_condition(opcode) {
                    self.state.pc = gates::add_signed_16(self.state.pc, i16::from(offset));
                }
                Ok(branch_mnemonic(opcode, false))
            }
            0x30..=0x33 => {
                let address = self.indexed_address(pc)?;
                match opcode {
                    0x30 => {
                        self.state.x = address;
                        self.set_flag(CC_Z, gates::is_zero_16(address) == 1);
                        Ok("LEAX")
                    }
                    0x31 => {
                        self.state.y = address;
                        self.set_flag(CC_Z, gates::is_zero_16(address) == 1);
                        Ok("LEAY")
                    }
                    0x32 => {
                        self.state.s = address;
                        Ok("LEAS")
                    }
                    _ => {
                        self.state.u = address;
                        Ok("LEAU")
                    }
                }
            }
            0x34..=0x37 => {
                let mask = self.fetch_byte(pc)?;
                match opcode {
                    0x34 => {
                        self.push_mask_s(mask);
                        Ok("PSHS")
                    }
                    0x35 => {
                        self.pull_mask_s(mask);
                        Ok("PULS")
                    }
                    0x36 => {
                        self.push_mask_u(mask);
                        Ok("PSHU")
                    }
                    _ => {
                        self.pull_mask_u(mask);
                        Ok("PULU")
                    }
                }
            }
            0x39 => {
                self.state.pc = self.pull16_s();
                Ok("RTS")
            }
            0x3a => {
                self.state.x = gates::add_16(self.state.x, u16::from(self.state.b), 0).0;
                Ok("ABX")
            }
            0x3b => {
                self.rti();
                Ok("RTI")
            }
            0x3c => {
                self.state.cc = gates::bit_and_8(self.state.cc, self.fetch_byte(pc)?);
                self.state.cc = gates::bit_or_8(self.state.cc, CC_E);
                self.push_mask_s(0xff);
                self.state.halted = true;
                Ok("CWAI")
            }
            0x3d => {
                let product = gates::multiply_8(self.state.a, self.state.b);
                self.set_d(product);
                self.set_flag(CC_Z, gates::is_zero_16(product) == 1);
                self.set_flag(CC_C, gates::flag(product as u8, 0x80) == 1);
                Ok("MUL")
            }
            0x3f => {
                self.software_interrupt(0xfffa, true);
                Ok("SWI")
            }
            0x40..=0x5f => self.execute_accumulator_rmw(opcode),
            0x60..=0x6f => self.execute_memory_rmw(opcode, Mode::Indexed, pc),
            0x70..=0x7f => self.execute_memory_rmw(opcode, Mode::Extended, pc),
            0x80..=0xff => self.execute_main_group(opcode, pc),
            _ => unreachable!(),
        }
    }

    fn execute_page2(&mut self, opcode: u8, pc: u16) -> Result<&'static str, Motorola6809Error> {
        match opcode {
            0x21..=0x2f => {
                let offset = self.fetch_word(pc)? as i16;
                if self.branch_condition(opcode) {
                    self.state.pc = gates::add_signed_16(self.state.pc, offset);
                }
                Ok(branch_mnemonic(opcode, true))
            }
            0x3f => {
                self.software_interrupt(0xfff4, false);
                Ok("SWI2")
            }
            _ => self.execute_page2_data(opcode, pc),
        }
    }

    fn execute_page3(&mut self, opcode: u8, pc: u16) -> Result<&'static str, Motorola6809Error> {
        if opcode == 0x3f {
            self.software_interrupt(0xfff2, false);
            return Ok("SWI3");
        }
        let mode = Self::mode_for(opcode);
        let value = self.operand16(mode, pc)?;
        if opcode & 0x0f == 3 {
            let _ = self.sub16(self.state.u, value);
            Ok("CMPU")
        } else {
            let _ = self.sub16(self.state.s, value);
            Ok("CMPS")
        }
    }

    fn execute_page2_data(
        &mut self,
        opcode: u8,
        pc: u16,
    ) -> Result<&'static str, Motorola6809Error> {
        let mode = Self::mode_for(opcode);
        match opcode & 0x0f {
            3 => {
                let value = self.operand16(mode, pc)?;
                let _ = self.sub16(self.state.d(), value);
                Ok("CMPD")
            }
            0x0c => {
                let value = self.operand16(mode, pc)?;
                let _ = self.sub16(self.state.y, value);
                Ok("CMPY")
            }
            0x0e => {
                let value = self.operand16(mode, pc)?;
                if opcode & 0x40 == 0 {
                    self.state.y = value;
                } else {
                    self.state.s = value;
                }
                self.load16_flags(value);
                Ok(if opcode & 0x40 == 0 { "LDY" } else { "LDS" })
            }
            0x0f => {
                let address = self.address(mode, pc)?;
                let (value, mnemonic) = if opcode & 0x40 == 0 {
                    (self.state.y, "STY")
                } else {
                    (self.state.s, "STS")
                };
                self.write16(address, value);
                self.load16_flags(value);
                Ok(mnemonic)
            }
            _ => unreachable!(),
        }
    }

    fn execute_main_group(
        &mut self,
        opcode: u8,
        pc: u16,
    ) -> Result<&'static str, Motorola6809Error> {
        if opcode == 0x8d {
            let offset = self.fetch_byte(pc)? as i8;
            let target = gates::add_signed_16(self.state.pc, i16::from(offset));
            self.push16_s(self.state.pc);
            self.state.pc = target;
            return Ok("BSR");
        }
        let mode = Self::mode_for(opcode);
        let low = opcode & 0x0f;
        let group_b = opcode & 0x40 != 0;
        if !group_b {
            self.execute_a_group(low, mode, pc)
        } else {
            self.execute_b_group(low, mode, pc)
        }
    }

    fn execute_a_group(
        &mut self,
        low: u8,
        mode: Mode,
        pc: u16,
    ) -> Result<&'static str, Motorola6809Error> {
        match low {
            0 => {
                let value = self.operand8(mode, pc)?;
                self.state.a = self.sub8(self.state.a, value, false);
                Ok("SUBA")
            }
            1 => {
                let value = self.operand8(mode, pc)?;
                let _ = self.sub8(self.state.a, value, false);
                Ok("CMPA")
            }
            2 => {
                let value = self.operand8(mode, pc)?;
                self.state.a =
                    self.sub8(self.state.a, value, gates::flag(self.state.cc, CC_C) == 1);
                Ok("SBCA")
            }
            3 => {
                let value = self.operand16(mode, pc)?;
                let result = self.sub16(self.state.d(), value);
                self.set_d(result);
                Ok("SUBD")
            }
            4 => {
                let value = self.operand8(mode, pc)?;
                self.state.a = gates::bit_and_8(self.state.a, value);
                self.load8_flags(self.state.a);
                Ok("ANDA")
            }
            5 => {
                let value = self.operand8(mode, pc)?;
                self.load8_flags(gates::bit_and_8(self.state.a, value));
                Ok("BITA")
            }
            6 => {
                let value = self.operand8(mode, pc)?;
                self.state.a = value;
                self.load8_flags(value);
                Ok("LDA")
            }
            7 => {
                let address = self.address(mode, pc)?;
                self.state.memory[usize::from(address)] = self.state.a;
                self.load8_flags(self.state.a);
                Ok("STA")
            }
            8 => {
                let value = self.operand8(mode, pc)?;
                self.state.a = gates::bit_xor_8(self.state.a, value);
                self.load8_flags(self.state.a);
                Ok("EORA")
            }
            9 => {
                let value = self.operand8(mode, pc)?;
                self.state.a =
                    self.add8(self.state.a, value, gates::flag(self.state.cc, CC_C) == 1);
                Ok("ADCA")
            }
            0x0a => {
                let value = self.operand8(mode, pc)?;
                self.state.a = gates::bit_or_8(self.state.a, value);
                self.load8_flags(self.state.a);
                Ok("ORA")
            }
            0x0b => {
                let value = self.operand8(mode, pc)?;
                self.state.a = self.add8(self.state.a, value, false);
                Ok("ADDA")
            }
            0x0c => {
                let value = self.operand16(mode, pc)?;
                let _ = self.sub16(self.state.x, value);
                Ok("CMPX")
            }
            0x0d => {
                let address = self.address(mode, pc)?;
                self.push16_s(self.state.pc);
                self.state.pc = address;
                Ok("JSR")
            }
            0x0e => {
                let value = self.operand16(mode, pc)?;
                self.state.x = value;
                self.load16_flags(value);
                Ok("LDX")
            }
            _ => {
                let address = self.address(mode, pc)?;
                self.write16(address, self.state.x);
                self.load16_flags(self.state.x);
                Ok("STX")
            }
        }
    }

    fn execute_b_group(
        &mut self,
        low: u8,
        mode: Mode,
        pc: u16,
    ) -> Result<&'static str, Motorola6809Error> {
        match low {
            0 => {
                let value = self.operand8(mode, pc)?;
                self.state.b = self.sub8(self.state.b, value, false);
                Ok("SUBB")
            }
            1 => {
                let value = self.operand8(mode, pc)?;
                let _ = self.sub8(self.state.b, value, false);
                Ok("CMPB")
            }
            2 => {
                let value = self.operand8(mode, pc)?;
                self.state.b =
                    self.sub8(self.state.b, value, gates::flag(self.state.cc, CC_C) == 1);
                Ok("SBCB")
            }
            3 => {
                let value = self.operand16(mode, pc)?;
                let result = self.add16(self.state.d(), value);
                self.set_d(result);
                Ok("ADDD")
            }
            4 => {
                let value = self.operand8(mode, pc)?;
                self.state.b = gates::bit_and_8(self.state.b, value);
                self.load8_flags(self.state.b);
                Ok("ANDB")
            }
            5 => {
                let value = self.operand8(mode, pc)?;
                self.load8_flags(gates::bit_and_8(self.state.b, value));
                Ok("BITB")
            }
            6 => {
                let value = self.operand8(mode, pc)?;
                self.state.b = value;
                self.load8_flags(value);
                Ok("LDB")
            }
            7 => {
                let address = self.address(mode, pc)?;
                self.state.memory[usize::from(address)] = self.state.b;
                self.load8_flags(self.state.b);
                Ok("STB")
            }
            8 => {
                let value = self.operand8(mode, pc)?;
                self.state.b = gates::bit_xor_8(self.state.b, value);
                self.load8_flags(self.state.b);
                Ok("EORB")
            }
            9 => {
                let value = self.operand8(mode, pc)?;
                self.state.b =
                    self.add8(self.state.b, value, gates::flag(self.state.cc, CC_C) == 1);
                Ok("ADCB")
            }
            0x0a => {
                let value = self.operand8(mode, pc)?;
                self.state.b = gates::bit_or_8(self.state.b, value);
                self.load8_flags(self.state.b);
                Ok("ORB")
            }
            0x0b => {
                let value = self.operand8(mode, pc)?;
                self.state.b = self.add8(self.state.b, value, false);
                Ok("ADDB")
            }
            0x0c => {
                let value = self.operand16(mode, pc)?;
                self.set_d(value);
                self.load16_flags(value);
                Ok("LDD")
            }
            0x0d => {
                let address = self.address(mode, pc)?;
                self.write16(address, self.state.d());
                self.load16_flags(self.state.d());
                Ok("STD")
            }
            0x0e => {
                let value = self.operand16(mode, pc)?;
                self.state.u = value;
                self.load16_flags(value);
                Ok("LDU")
            }
            _ => {
                let address = self.address(mode, pc)?;
                self.write16(address, self.state.u);
                self.load16_flags(self.state.u);
                Ok("STU")
            }
        }
    }

    fn execute_accumulator_rmw(&mut self, opcode: u8) -> Result<&'static str, Motorola6809Error> {
        let use_b = opcode & 0x10 != 0;
        let value = if use_b { self.state.b } else { self.state.a };
        let (result, mnemonic, write) = self.rmw_value(opcode & 0x0f, value);
        if write {
            if use_b {
                self.state.b = result;
            } else {
                self.state.a = result;
            }
        }
        Ok(match (opcode & 0x0f, use_b) {
            (0, false) => "NEGA",
            (0, true) => "NEGB",
            (3, false) => "COMA",
            (3, true) => "COMB",
            (4, false) => "LSRA",
            (4, true) => "LSRB",
            (6, false) => "RORA",
            (6, true) => "RORB",
            (7, false) => "ASRA",
            (7, true) => "ASRB",
            (8, false) => "ASLA",
            (8, true) => "ASLB",
            (9, false) => "ROLA",
            (9, true) => "ROLB",
            (0x0a, false) => "DECA",
            (0x0a, true) => "DECB",
            (0x0c, false) => "INCA",
            (0x0c, true) => "INCB",
            (0x0d, false) => "TSTA",
            (0x0d, true) => "TSTB",
            (0x0f, false) => "CLRA",
            (0x0f, true) => "CLRB",
            _ => mnemonic,
        })
    }

    fn execute_memory_rmw(
        &mut self,
        opcode: u8,
        mode: Mode,
        pc: u16,
    ) -> Result<&'static str, Motorola6809Error> {
        let address = self.address(mode, pc)?;
        if opcode & 0x0f == 0x0e {
            self.state.pc = address;
            return Ok("JMP");
        }
        let value = self.state.memory[usize::from(address)];
        let (result, mnemonic, write) = self.rmw_value(opcode & 0x0f, value);
        if write {
            self.state.memory[usize::from(address)] = result;
        }
        Ok(mnemonic)
    }

    fn rmw_value(&mut self, operation: u8, value: u8) -> (u8, &'static str, bool) {
        match operation {
            0 => {
                let result = gates::sub_8(0, value, 0).0;
                self.set_nz8(result);
                self.set_flag(CC_V, gates::equal_8(value, 0x80) == 1);
                self.set_flag(CC_C, gates::is_zero_8(value) == 0);
                (result, "NEG", true)
            }
            3 => {
                let result = gates::bit_not_8(value);
                self.set_nz8(result);
                self.set_flag(CC_V, false);
                self.set_flag(CC_C, true);
                (result, "COM", true)
            }
            4 => {
                let carry = gates::u8_to_bits(value)[0];
                let result = gates::shift_right_8(value, 0);
                self.set_flag(CC_C, carry == 1);
                self.set_flag(CC_N, false);
                self.set_flag(CC_Z, gates::is_zero_8(result) == 1);
                self.set_flag(CC_V, carry == 1);
                (result, "LSR", true)
            }
            6 => {
                let carry = gates::flag(self.state.cc, CC_C);
                let carry_out = gates::u8_to_bits(value)[0];
                let result = gates::shift_right_8(value, carry);
                self.set_flag(CC_C, carry_out == 1);
                self.set_nz8(result);
                self.set_flag(
                    CC_V,
                    gates::flag(self.state.cc, CC_N) != gates::flag(self.state.cc, CC_C),
                );
                (result, "ROR", true)
            }
            7 => {
                let carry_out = gates::u8_to_bits(value)[0];
                let result = gates::shift_right_8(value, gates::negative_8(value));
                self.set_flag(CC_C, carry_out == 1);
                self.set_nz8(result);
                self.set_flag(
                    CC_V,
                    gates::flag(self.state.cc, CC_N) != gates::flag(self.state.cc, CC_C),
                );
                (result, "ASR", true)
            }
            8 => {
                let carry_out = gates::negative_8(value);
                let result = gates::shift_left_8(value, 0);
                self.set_flag(CC_C, carry_out == 1);
                self.set_nz8(result);
                self.set_flag(
                    CC_V,
                    gates::flag(self.state.cc, CC_N) != gates::flag(self.state.cc, CC_C),
                );
                (result, "ASL", true)
            }
            9 => {
                let carry = gates::flag(self.state.cc, CC_C);
                let carry_out = gates::negative_8(value);
                let result = gates::shift_left_8(value, carry);
                self.set_flag(CC_C, carry_out == 1);
                self.set_nz8(result);
                self.set_flag(
                    CC_V,
                    gates::flag(self.state.cc, CC_N) != gates::flag(self.state.cc, CC_C),
                );
                (result, "ROL", true)
            }
            0x0a => {
                let result = gates::sub_8(value, 1, 0).0;
                self.set_nz8(result);
                self.set_flag(CC_V, gates::equal_8(value, 0x80) == 1);
                (result, "DEC", true)
            }
            0x0c => {
                let result = gates::add_8(value, 1, 0).0;
                self.set_nz8(result);
                self.set_flag(CC_V, gates::equal_8(value, 0x7f) == 1);
                (result, "INC", true)
            }
            0x0d => {
                self.set_nz8(value);
                self.set_flag(CC_V, false);
                (value, "TST", false)
            }
            0x0f => {
                self.set_flag(CC_N, false);
                self.set_flag(CC_Z, true);
                self.set_flag(CC_V, false);
                self.set_flag(CC_C, false);
                (0, "CLR", true)
            }
            _ => unreachable!(),
        }
    }

    fn transfer(&mut self, postbyte: u8, exchange: bool, pc: u16) -> Result<(), Motorola6809Error> {
        let source = postbyte >> 4;
        let destination = postbyte & 0x0f;
        let source_width = register_width(source);
        let destination_width = register_width(destination);
        if source_width.is_none() || source_width != destination_width {
            return Err(Motorola6809Error::InvalidTransfer { postbyte, pc });
        }
        let source_value = self.register_value(source);
        let destination_value = self.register_value(destination);
        self.set_register_value(destination, source_value);
        if exchange {
            self.set_register_value(source, destination_value);
        }
        Ok(())
    }

    fn register_value(&self, code: u8) -> u16 {
        match code {
            0 => self.state.d(),
            1 => self.state.x,
            2 => self.state.y,
            3 => self.state.u,
            4 => self.state.s,
            5 => self.state.pc,
            8 => u16::from(self.state.a),
            9 => u16::from(self.state.b),
            0x0a => u16::from(self.state.cc),
            0x0b => u16::from(self.state.dp),
            _ => unreachable!(),
        }
    }

    fn set_register_value(&mut self, code: u8, value: u16) {
        match code {
            0 => self.set_d(value),
            1 => self.state.x = value,
            2 => self.state.y = value,
            3 => self.state.u = value,
            4 => self.state.s = value,
            5 => self.state.pc = value,
            8 => self.state.a = value as u8,
            9 => self.state.b = value as u8,
            0x0a => self.state.cc = value as u8,
            0x0b => self.state.dp = value as u8,
            _ => unreachable!(),
        }
    }

    fn push8_s(&mut self, value: u8) {
        self.state.s = gates::sub_16(self.state.s, 1).0;
        self.state.memory[usize::from(self.state.s)] = value;
    }
    fn pull8_s(&mut self) -> u8 {
        let value = self.state.memory[usize::from(self.state.s)];
        self.state.s = gates::add_16(self.state.s, 1, 0).0;
        value
    }
    fn push16_s(&mut self, value: u16) {
        let [hi, lo] = value.to_be_bytes();
        self.push8_s(lo);
        self.push8_s(hi);
    }
    fn pull16_s(&mut self) -> u16 {
        u16::from_be_bytes([self.pull8_s(), self.pull8_s()])
    }
    fn push8_u(&mut self, value: u8) {
        self.state.u = gates::sub_16(self.state.u, 1).0;
        self.state.memory[usize::from(self.state.u)] = value;
    }
    fn pull8_u(&mut self) -> u8 {
        let value = self.state.memory[usize::from(self.state.u)];
        self.state.u = gates::add_16(self.state.u, 1, 0).0;
        value
    }
    fn push16_u(&mut self, value: u16) {
        let [hi, lo] = value.to_be_bytes();
        self.push8_u(lo);
        self.push8_u(hi);
    }
    fn pull16_u(&mut self) -> u16 {
        u16::from_be_bytes([self.pull8_u(), self.pull8_u()])
    }

    fn push_mask_s(&mut self, mask: u8) {
        if gates::flag(mask, 0x80) == 1 {
            self.push16_s(self.state.pc);
        }
        if gates::flag(mask, 0x40) == 1 {
            self.push16_s(self.state.u);
        }
        if gates::flag(mask, 0x20) == 1 {
            self.push16_s(self.state.y);
        }
        if gates::flag(mask, 0x10) == 1 {
            self.push16_s(self.state.x);
        }
        if gates::flag(mask, 0x08) == 1 {
            self.push8_s(self.state.dp);
        }
        if gates::flag(mask, 0x04) == 1 {
            self.push8_s(self.state.b);
        }
        if gates::flag(mask, 0x02) == 1 {
            self.push8_s(self.state.a);
        }
        if gates::flag(mask, 0x01) == 1 {
            self.push8_s(self.state.cc);
        }
    }

    fn pull_mask_s(&mut self, mask: u8) {
        if gates::flag(mask, 0x01) == 1 {
            self.state.cc = self.pull8_s();
        }
        if gates::flag(mask, 0x02) == 1 {
            self.state.a = self.pull8_s();
        }
        if gates::flag(mask, 0x04) == 1 {
            self.state.b = self.pull8_s();
        }
        if gates::flag(mask, 0x08) == 1 {
            self.state.dp = self.pull8_s();
        }
        if gates::flag(mask, 0x10) == 1 {
            self.state.x = self.pull16_s();
        }
        if gates::flag(mask, 0x20) == 1 {
            self.state.y = self.pull16_s();
        }
        if gates::flag(mask, 0x40) == 1 {
            self.state.u = self.pull16_s();
        }
        if gates::flag(mask, 0x80) == 1 {
            self.state.pc = self.pull16_s();
        }
    }

    fn push_mask_u(&mut self, mask: u8) {
        if gates::flag(mask, 0x80) == 1 {
            self.push16_u(self.state.pc);
        }
        if gates::flag(mask, 0x40) == 1 {
            self.push16_u(self.state.s);
        }
        if gates::flag(mask, 0x20) == 1 {
            self.push16_u(self.state.y);
        }
        if gates::flag(mask, 0x10) == 1 {
            self.push16_u(self.state.x);
        }
        if gates::flag(mask, 0x08) == 1 {
            self.push8_u(self.state.dp);
        }
        if gates::flag(mask, 0x04) == 1 {
            self.push8_u(self.state.b);
        }
        if gates::flag(mask, 0x02) == 1 {
            self.push8_u(self.state.a);
        }
        if gates::flag(mask, 0x01) == 1 {
            self.push8_u(self.state.cc);
        }
    }

    fn pull_mask_u(&mut self, mask: u8) {
        if gates::flag(mask, 0x01) == 1 {
            self.state.cc = self.pull8_u();
        }
        if gates::flag(mask, 0x02) == 1 {
            self.state.a = self.pull8_u();
        }
        if gates::flag(mask, 0x04) == 1 {
            self.state.b = self.pull8_u();
        }
        if gates::flag(mask, 0x08) == 1 {
            self.state.dp = self.pull8_u();
        }
        if gates::flag(mask, 0x10) == 1 {
            self.state.x = self.pull16_u();
        }
        if gates::flag(mask, 0x20) == 1 {
            self.state.y = self.pull16_u();
        }
        if gates::flag(mask, 0x40) == 1 {
            self.state.s = self.pull16_u();
        }
        if gates::flag(mask, 0x80) == 1 {
            self.state.pc = self.pull16_u();
        }
    }

    fn software_interrupt(&mut self, vector: u16, mask_interrupts: bool) {
        self.state.cc = gates::bit_or_8(self.state.cc, CC_E);
        self.push_mask_s(0xff);
        if mask_interrupts {
            self.state.cc = gates::bit_or_8(self.state.cc, CC_I | CC_F);
        }
        self.state.pc = self.read16(vector);
    }

    fn rti(&mut self) {
        self.state.cc = self.pull8_s();
        if gates::flag(self.state.cc, CC_E) == 1 {
            self.state.a = self.pull8_s();
            self.state.b = self.pull8_s();
            self.state.dp = self.pull8_s();
            self.state.x = self.pull16_s();
            self.state.y = self.pull16_s();
            self.state.u = self.pull16_s();
        }
        self.state.pc = self.pull16_s();
    }

    fn daa(&mut self) {
        let original = self.state.a;
        let carry = gates::flag(self.state.cc, CC_C);
        let half = gates::flag(self.state.cc, CC_H);
        let low = gates::bit_and_8(original, 0x0f);
        let low_adjust = logic_gates::gates::or_gate(gates::greater_than_8(low, 9), half);
        let high_adjust = logic_gates::gates::or_gate(gates::greater_than_8(original, 0x99), carry);
        let correction = gates::bit_or_8(
            gates::select_8(0, 0x06, low_adjust),
            gates::select_8(0, 0x60, high_adjust),
        );
        let (result, carry_out, _, _) = gates::add_8(original, correction, 0);
        self.state.a = result;
        self.set_nz8(self.state.a);
        self.set_flag(CC_V, false);
        self.set_flag(CC_C, logic_gates::gates::or_gate(carry, carry_out) == 1);
    }
}

fn register_width(code: u8) -> Option<u8> {
    match code {
        0..=5 => Some(16),
        8..=0x0b => Some(8),
        _ => None,
    }
}

fn branch_mnemonic(opcode: u8, long: bool) -> &'static str {
    const SHORT: [&str; 16] = [
        "BRA", "BRN", "BHI", "BLS", "BHS", "BLO", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI", "BGE",
        "BLT", "BGT", "BLE",
    ];
    const LONG: [&str; 16] = [
        "LBRA", "LBRN", "LBHI", "LBLS", "LBHS", "LBLO", "LBNE", "LBEQ", "LBVC", "LBVS", "LBPL",
        "LBMI", "LBGE", "LBLT", "LBGT", "LBLE",
    ];
    if long {
        LONG[usize::from(opcode & 0x0f)]
    } else {
        SHORT[usize::from(opcode & 0x0f)]
    }
}
