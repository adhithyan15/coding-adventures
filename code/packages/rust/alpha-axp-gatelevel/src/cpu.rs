//! Complete DFF-backed Alpha machine lifecycle.

use crate::register_file::RegisterFile64;
use crate::state::{clock_bit, DffMemory};
use crate::{alu, bits, decoder};
use alpha_axp_simulator::{
    AlphaError, AlphaSimulator, AlphaState, ExecutionResult, StepTrace, MEMORY_SIZE,
};
use logic_gates::gates::{and_gate, not_gate, or_gate};

/// Exact persistent topology: 524,288 memory bits, 2,048 GPR bits, 128 PC/nPC
/// bits, and one halt bit.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 32 * 64 + 2 * 64 + 1;

/// DFF-backed DEC Alpha AXP 21064 gate machine.
#[derive(Clone, Debug)]
pub struct AlphaGateSimulator {
    rf: RegisterFile64,
    memory: DffMemory,
    halt_q: u8,
    loaded_origin: u64,
    loaded_len: usize,
}

impl Default for AlphaGateSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl AlphaGateSimulator {
    pub fn new() -> Self {
        Self {
            rf: RegisterFile64::new(),
            memory: DffMemory::new(MEMORY_SIZE),
            halt_q: 0,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    pub fn reset(&mut self) {
        self.rf = RegisterFile64::new();
        self.memory.clear();
        clock_bit(&mut self.halt_q, false);
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    pub fn get_state(&self) -> AlphaState {
        AlphaState {
            pc: self.rf.pc(),
            npc: self.rf.npc(),
            regs: self.rf.snapshot(),
            memory: self.memory.snapshot(),
            halted: self.halt_q != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    pub fn restore(&mut self, state: &AlphaState) -> Result<(), AlphaError> {
        let mut validator = AlphaSimulator::new();
        validator.restore(state)?;
        self.rf.restore(&state.regs, state.pc, state.npc);
        self.memory.restore_snapshot(&state.memory);
        clock_bit(&mut self.halt_q, state.halted);
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), AlphaError> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), AlphaError> {
        let mut validator = AlphaSimulator::new();
        validator.load_at_checked(program, origin)?;
        self.restore(&validator.get_state())
    }

    pub fn load(&mut self, program: &[u8]) -> Result<(), AlphaError> {
        self.load_checked(program)
    }

    pub fn read_register_checked(&self, index: usize) -> Result<u64, AlphaError> {
        if index >= 32 {
            return Err(AlphaError::InvalidRegister { index });
        }
        Ok(self.rf.read(index))
    }

    pub fn write_register_checked(&mut self, index: usize, value: u64) -> Result<(), AlphaError> {
        if index >= 32 {
            return Err(AlphaError::InvalidRegister { index });
        }
        self.rf.write(index, value);
        Ok(())
    }

    pub fn read_byte_checked(&self, address: u64) -> Result<u8, AlphaError> {
        let index = direct_range(address, 1)?;
        Ok(self.memory[index])
    }

    pub fn write_byte_checked(&mut self, address: u64, value: u8) -> Result<(), AlphaError> {
        let index = direct_range(address, 1)?;
        self.memory.write(index, value);
        Ok(())
    }

    pub fn read_word_checked(&self, address: u64) -> Result<u16, AlphaError> {
        Ok(u16::from_le_bytes(self.read_array::<2>(address)?))
    }

    pub fn write_word_checked(&mut self, address: u64, value: u16) -> Result<(), AlphaError> {
        self.write_array(address, &value.to_le_bytes())
    }

    pub fn read_long_checked(&self, address: u64) -> Result<u32, AlphaError> {
        Ok(u32::from_le_bytes(self.read_array::<4>(address)?))
    }

    pub fn write_long_checked(&mut self, address: u64, value: u32) -> Result<(), AlphaError> {
        self.write_array(address, &value.to_le_bytes())
    }

    pub fn read_quad_checked(&self, address: u64) -> Result<u64, AlphaError> {
        Ok(u64::from_le_bytes(self.read_array::<8>(address)?))
    }

    pub fn write_quad_checked(&mut self, address: u64, value: u64) -> Result<(), AlphaError> {
        self.write_array(address, &value.to_le_bytes())
    }

    pub fn step_checked(&mut self) -> Result<StepTrace, AlphaError> {
        let before = self.get_state();
        if before.halted {
            return Err(AlphaError::Halted);
        }
        let pc = before.pc;
        self.validate_fetch(pc)?;
        let raw = self.fetch_word(pc);
        match self.execute_raw(raw, pc) {
            Ok(mnemonic) => Ok(StepTrace {
                pc_before: pc,
                pc_after: self.rf.pc(),
                raw,
                mnemonic: mnemonic.to_string(),
                state_before: before,
                state_after: self.get_state(),
            }),
            Err(error) => {
                self.restore(&before).expect("previous state is valid");
                Err(error)
            }
        }
    }

    pub fn step(&mut self) -> Result<StepTrace, AlphaError> {
        self.step_checked()
    }

    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, AlphaError> {
        let before = self.get_state();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step_checked() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: self.get_state(),
                        });
                    }
                }
                Err(error) => {
                    self.restore(&before).expect("previous state is valid");
                    return Err(error);
                }
            }
        }
        self.restore(&before).expect("previous state is valid");
        Err(AlphaError::StepLimitExceeded { max_steps })
    }

    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, AlphaError> {
        let before = self.get_state();
        self.load_checked(program)?;
        match self.run_loaded_checked(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore(&before).expect("previous state is valid");
                Err(error)
            }
        }
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, AlphaError> {
        self.run_checked(program, max_steps)
    }

    fn read_array<const WIDTH: usize>(&self, address: u64) -> Result<[u8; WIDTH], AlphaError> {
        direct_alignment(address, WIDTH)?;
        let start = direct_range(address, WIDTH)?;
        Ok(self.memory[start..start + WIDTH]
            .try_into()
            .expect("validated width"))
    }

    fn write_array(&mut self, address: u64, bytes: &[u8]) -> Result<(), AlphaError> {
        direct_alignment(address, bytes.len())?;
        let start = direct_range(address, bytes.len())?;
        for (offset, byte) in bytes.iter().copied().enumerate() {
            self.memory.write(start + offset, byte);
        }
        Ok(())
    }

    fn reg(&self, index: usize) -> u64 {
        self.rf.read(index)
    }

    fn set_reg(&mut self, index: usize, value: u64) {
        self.rf.write(index, value);
    }

    fn validate_fetch(&self, pc: u64) -> Result<(), AlphaError> {
        if pc & 3 != 0 {
            return Err(AlphaError::MisalignedAccess {
                address: pc,
                width: 4,
            });
        }
        let end = self.loaded_origin + self.loaded_len as u64;
        if pc < self.loaded_origin || pc.checked_add(4).is_none_or(|next| next > end) {
            return Err(AlphaError::TruncatedInstruction { pc });
        }
        Ok(())
    }

    fn fetch_word(&self, pc: u64) -> u32 {
        let start = pc as usize;
        u32::from_le_bytes(
            self.memory[start..start + 4]
                .try_into()
                .expect("validated fetch"),
        )
    }

    fn advance(&mut self) {
        let next = self.rf.npc() & 0xffff;
        self.rf.write_pc(next);
        self.rf.write_npc(alu::addq(next, 4).result & 0xffff);
    }

    fn execute_raw(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        self.advance();
        match decoder::decode(raw).op {
            0x00 => self.exec_pal(raw, pc),
            0x10 => self.exec_inta(raw, pc),
            0x11 => self.exec_intl(raw, pc),
            0x12 => self.exec_ints(raw, pc),
            0x13 => self.exec_intm(raw, pc),
            0x1a => Ok(self.exec_jump(raw, pc)),
            0x0a | 0x0c | 0x0d | 0x0e | 0x28..=0x2d => self.exec_memory(raw, pc),
            0x30 | 0x34 | 0x38..=0x3f => Ok(self.exec_branch(raw, pc)),
            _ => Err(AlphaError::UnknownInstruction { raw, pc }),
        }
    }

    fn operands(&self, raw: u32) -> (u64, u64, u8, usize) {
        let decoded = decoder::decode(raw);
        let b = decoded
            .literal
            .map_or_else(|| self.reg(decoded.rb), u64::from);
        (self.reg(decoded.ra), b, decoded.function, decoded.rc)
    }

    fn exec_pal(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let palcode = decoder::decode(raw).palcode;
        if palcode != 0 {
            return Err(AlphaError::UnsupportedPalcode { palcode, pc });
        }
        clock_bit(&mut self.halt_q, true);
        Ok("HALT")
    }

    fn exec_inta(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let (a, b, function, rc) = self.operands(raw);
        let (value, mnemonic) = match function {
            0x00 | 0x40 => (alu::addl(a, b).result, "ADDL"),
            0x20 | 0x60 => (alu::addq(a, b).result, "ADDQ"),
            0x09 | 0x49 => (alu::subl(a, b).result, "SUBL"),
            0x29 | 0x69 => (alu::subq(a, b).result, "SUBQ"),
            0x18 | 0x58 => (alu::mull(a, b), "MULL"),
            0x38 | 0x78 => (alu::mulq(a, b), "MULQ"),
            0x02 => (alu::addl(bits::shl_64(a, 2), b).result, "S4ADDL"),
            0x22 => (alu::addq(bits::shl_64(a, 2), b).result, "S4ADDQ"),
            0x0b => (alu::subl(bits::shl_64(a, 2), b).result, "S4SUBL"),
            0x2b => (alu::subq(bits::shl_64(a, 2), b).result, "S4SUBQ"),
            0x12 => (alu::addl(bits::shl_64(a, 3), b).result, "S8ADDL"),
            0x32 => (alu::addq(bits::shl_64(a, 3), b).result, "S8ADDQ"),
            0x1b => (alu::subl(bits::shl_64(a, 3), b).result, "S8SUBL"),
            0x3b => (alu::subq(bits::shl_64(a, 3), b).result, "S8SUBQ"),
            0x2d => (alu::cmpeq(a, b), "CMPEQ"),
            0x4d => (alu::cmplt(a, b), "CMPLT"),
            0x6d => (alu::cmple(a, b), "CMPLE"),
            0x3d => (alu::cmpult(a, b), "CMPULT"),
            0x7d => (alu::cmpule(a, b), "CMPULE"),
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        self.set_reg(rc, value);
        Ok(mnemonic)
    }

    fn exec_intl(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let (a, b, function, rc) = self.operands(raw);
        let current = self.reg(rc);
        let a_bits = bits::u64_to_bits(a);
        let zero = bits::zero_64(a);
        let (value, mnemonic) = match function {
            0x00 => (alu::andq(a, b).result, "AND"),
            0x08 => (alu::bic(a, b).result, "BIC"),
            0x20 => (alu::bis(a, b).result, "BIS"),
            0x28 => (alu::ornot(a, b).result, "ORNOT"),
            0x40 => (alu::xorq(a, b).result, "XOR"),
            0x48 => (alu::eqv(a, b).result, "EQV"),
            0x14 => (select(a_bits[0], b, current), "CMOVLBS"),
            0x16 => (select(not_gate(a_bits[0]), b, current), "CMOVLBC"),
            0x24 => (select(zero, b, current), "CMOVEQ"),
            0x26 => (select(not_gate(zero), b, current), "CMOVNE"),
            0x44 => (select(a_bits[63], b, current), "CMOVLT"),
            0x46 => (select(not_gate(a_bits[63]), b, current), "CMOVGE"),
            0x64 => (select(or_gate(a_bits[63], zero), b, current), "CMOVLE"),
            0x66 => (
                select(and_gate(not_gate(a_bits[63]), not_gate(zero)), b, current),
                "CMOVGT",
            ),
            0x61 => (alu::bic(a, b).result, "AMASK"),
            0x6c => (0, "IMPLVER"),
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        self.set_reg(rc, value);
        Ok(mnemonic)
    }

    fn exec_ints(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let (a, b, function, rc) = self.operands(raw);
        let shift = (b & 63) as u32;
        let offset = bits::shl_64(b & 7, 3) as u32;
        let (value, mnemonic) = match function {
            0x39 => (bits::shl_64(a, shift), "SLL"),
            0x34 => (bits::shr_64(a, shift), "SRL"),
            0x3c => (bits::sar_64(a, shift), "SRA"),
            0x06 => (alu::andq(bits::shr_64(a, offset), 0xff).result, "EXTBL"),
            0x16 => (alu::andq(bits::shr_64(a, offset), 0xffff).result, "EXTWL"),
            0x26 => (
                alu::andq(bits::shr_64(a, offset), 0xffff_ffff).result,
                "EXTLL",
            ),
            0x36 => (bits::shr_64(a, offset), "EXTQL"),
            0x0b => (bits::shl_64(alu::andq(a, 0xff).result, offset), "INSBL"),
            0x1b => (bits::shl_64(alu::andq(a, 0xffff).result, offset), "INSWL"),
            0x2b => (
                bits::shl_64(alu::andq(a, 0xffff_ffff).result, offset),
                "INSLL",
            ),
            0x3b => (bits::shl_64(a, offset), "INSQL"),
            0x02 => (alu::bic(a, bits::shl_64(0xff, offset)).result, "MSKBL"),
            0x12 => (alu::bic(a, bits::shl_64(0xffff, offset)).result, "MSKWL"),
            0x22 => (
                alu::bic(a, bits::shl_64(0xffff_ffff, offset)).result,
                "MSKLL",
            ),
            0x32 => (alu::bic(a, bits::shl_64(u64::MAX, offset)).result, "MSKQL"),
            0x30 => (zap(a, b as u8, false), "ZAP"),
            0x31 => (zap(a, b as u8, true), "ZAPNOT"),
            0x00 => (bits::sext_8(a), "SEXTB"),
            0x01 => (bits::sext_16(a), "SEXTW"),
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        self.set_reg(rc, value);
        Ok(mnemonic)
    }

    fn exec_intm(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let (a, b, function, rc) = self.operands(raw);
        let (value, mnemonic) = match function {
            0x00 | 0x40 => (alu::mull(a, b), "MULL"),
            0x20 | 0x60 => (alu::mulq(a, b), "MULQ"),
            0x30 => (alu::umulh(a, b), "UMULH"),
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        self.set_reg(rc, value);
        Ok(mnemonic)
    }

    fn exec_memory(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let decoded = decoder::decode(raw);
        let displacement = bits::sext_16(decoded.displacement16 as u16 as u64);
        let address = alu::addq(self.reg(decoded.rb), displacement).result & 0xffff;
        let mnemonic = match decoded.op {
            0x28 | 0x2a => {
                let value = bits::sext_32(self.load_width(address, 4)?);
                self.set_reg(decoded.ra, value);
                "LDL"
            }
            0x29 | 0x2b => {
                let value = self.load_width(address, 8)?;
                self.set_reg(decoded.ra, value);
                "LDQ"
            }
            0x0a => {
                let value = self.load_width(address, 1)?;
                self.set_reg(decoded.ra, value);
                "LDBU"
            }
            0x0c => {
                let value = self.load_width(address, 2)?;
                self.set_reg(decoded.ra, value);
                "LDWU"
            }
            0x2c => {
                self.store_width(address, 4, self.reg(decoded.ra))?;
                "STL"
            }
            0x2d => {
                self.store_width(address, 8, self.reg(decoded.ra))?;
                "STQ"
            }
            0x0e => {
                self.store_width(address, 1, self.reg(decoded.ra))?;
                "STB"
            }
            0x0d => {
                self.store_width(address, 2, self.reg(decoded.ra))?;
                "STW"
            }
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        Ok(mnemonic)
    }

    fn exec_branch(&mut self, raw: u32, pc: u64) -> &'static str {
        let decoded = decoder::decode(raw);
        let value = self.reg(decoded.ra);
        let displacement = bits::shl_64(decoded.displacement21 as i64 as u64, 2);
        let link = alu::addq(pc, 4).result;
        let target = alu::addq(link, displacement).result & 0xffff;
        let value_bits = bits::u64_to_bits(value);
        let zero = bits::zero_64(value);
        let (taken, mnemonic) = match decoded.op {
            0x30 => (1, "BR"),
            0x34 => {
                self.set_reg(decoded.ra, link);
                (1, "BSR")
            }
            0x39 => (zero, "BEQ"),
            0x3d => (not_gate(zero), "BNE"),
            0x3a => (value_bits[63], "BLT"),
            0x3b => (or_gate(value_bits[63], zero), "BLE"),
            0x3f => (and_gate(not_gate(value_bits[63]), not_gate(zero)), "BGT"),
            0x3e => (not_gate(value_bits[63]), "BGE"),
            0x38 => (not_gate(value_bits[0]), "BLBC"),
            0x3c => (value_bits[0], "BLBS"),
            _ => unreachable!("dispatch restricts branch opcodes"),
        };
        if taken != 0 {
            self.rf.write_pc(target);
            self.rf.write_npc(alu::addq(target, 4).result & 0xffff);
        }
        mnemonic
    }

    fn exec_jump(&mut self, raw: u32, pc: u64) -> &'static str {
        let decoded = decoder::decode(raw);
        let target = alu::andq(self.reg(decoded.rb), 0xffff_fffc).result & 0xffff;
        self.rf.write_pc(target);
        self.rf.write_npc(alu::addq(target, 4).result & 0xffff);
        self.set_reg(decoded.ra, alu::addq(pc, 4).result);
        ["JMP", "JSR", "RET", "JSR_COROUTINE"][decoded.jump_function as usize]
    }

    fn load_width(&self, address: u64, width: usize) -> Result<u64, AlphaError> {
        direct_alignment(address, width)?;
        let mut value = 0;
        for offset in 0..width {
            value |= u64::from(self.memory[(address as usize + offset) & 0xffff]) << (8 * offset);
        }
        Ok(value)
    }

    fn store_width(&mut self, address: u64, width: usize, value: u64) -> Result<(), AlphaError> {
        direct_alignment(address, width)?;
        for offset in 0..width {
            self.memory.write(
                (address as usize + offset) & 0xffff,
                (value >> (8 * offset)) as u8,
            );
        }
        Ok(())
    }
}

fn select(condition: u8, when_true: u64, when_false: u64) -> u64 {
    let true_bits = bits::u64_to_bits(when_true);
    let false_bits = bits::u64_to_bits(when_false);
    bits::bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        or_gate(
            and_gate(condition, true_bits[bit]),
            and_gate(not_gate(condition), false_bits[bit]),
        )
    }))
}

fn zap(value: u64, mask: u8, inverted: bool) -> u64 {
    let value = bits::u64_to_bits(value);
    let output: [u8; 64] = std::array::from_fn(|bit| {
        let mask_bit = (mask >> (bit / 8)) & 1;
        let keep = if inverted {
            mask_bit
        } else {
            not_gate(mask_bit)
        };
        and_gate(value[bit], keep)
    });
    bits::bits_to_u64(&output)
}

fn direct_alignment(address: u64, width: usize) -> Result<(), AlphaError> {
    if !address.is_multiple_of(width as u64) {
        return Err(AlphaError::MisalignedAccess { address, width });
    }
    Ok(())
}

fn direct_range(address: u64, width: usize) -> Result<usize, AlphaError> {
    let start =
        usize::try_from(address).map_err(|_| AlphaError::MemoryOutOfRange { address, width })?;
    start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .map(|_| start)
        .ok_or(AlphaError::MemoryOutOfRange { address, width })
}
