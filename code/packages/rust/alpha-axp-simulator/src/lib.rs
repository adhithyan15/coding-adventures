//! DEC Alpha AXP 21064 integer simulator for Layer 07s.
//!
//! The educational machine has 32 64-bit integer registers (`r31` is zero),
//! PC/nPC control flow, and 64 KiB of wrapping little-endian memory. Checked
//! operations expose complete snapshots, typed atomic faults, instruction
//! traces, and transactional bounded runs.

pub mod encoding;

/// Exact memory size of the educational Alpha machine.
pub const MEMORY_SIZE: usize = 65_536;
/// Architectural zero register.
pub const ZERO_REGISTER: usize = 31;
/// `call_pal 0`, the repository halt sentinel.
pub const HALT_WORD: u32 = 0;

/// Complete owned architectural and lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlphaState {
    pub pc: u64,
    pub npc: u64,
    pub regs: [u64; 32],
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u64,
    pub loaded_len: usize,
}

/// Typed fail-closed lifecycle and execution errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlphaError {
    Halted,
    InvalidRegister {
        index: usize,
    },
    InvalidState(String),
    MisalignedProgram {
        origin: u64,
    },
    ProgramOutOfRange {
        origin: u64,
        length: usize,
        memory_size: usize,
    },
    TruncatedInstruction {
        pc: u64,
    },
    MemoryOutOfRange {
        address: u64,
        width: usize,
    },
    MisalignedAccess {
        address: u64,
        width: usize,
    },
    UnsupportedPalcode {
        palcode: u32,
        pc: u64,
    },
    UnknownInstruction {
        raw: u32,
        pc: u64,
    },
    StepLimitExceeded {
        max_steps: usize,
    },
}

impl std::fmt::Display for AlphaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Halted => f.write_str("CPU is halted"),
            Self::InvalidRegister { index } => {
                write!(f, "register index {index} is outside r0-r31")
            }
            Self::InvalidState(message) => f.write_str(message),
            Self::MisalignedProgram { origin } => {
                write!(f, "program origin {origin:#018x} is not word-aligned")
            }
            Self::ProgramOutOfRange {
                origin,
                length,
                memory_size,
            } => write!(
                f,
                "program of {length} bytes at {origin:#018x} exceeds {memory_size}-byte memory"
            ),
            Self::TruncatedInstruction { pc } => {
                write!(
                    f,
                    "instruction at {pc:#018x} crosses the installed program boundary"
                )
            }
            Self::MemoryOutOfRange { address, width } => {
                write!(f, "{width}-byte access at {address:#018x} exceeds memory")
            }
            Self::MisalignedAccess { address, width } => {
                write!(f, "misaligned {width}-byte access at {address:#018x}")
            }
            Self::UnsupportedPalcode { palcode, pc } => {
                write!(f, "unsupported PALcode {palcode:#09x} at {pc:#018x}")
            }
            Self::UnknownInstruction { raw, pc } => {
                write!(f, "unknown instruction {raw:#010x} at {pc:#018x}")
            }
            Self::StepLimitExceeded { max_steps } => {
                write!(f, "step limit {max_steps} exceeded")
            }
        }
    }
}

impl std::error::Error for AlphaError {}

/// Complete checked instruction transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u64,
    pub pc_after: u64,
    pub raw: u32,
    pub mnemonic: String,
    pub state_before: AlphaState,
    pub state_after: AlphaState,
}

/// Observable outcome of a bounded checked run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: AlphaState,
}

/// Functional DEC Alpha AXP 21064 simulator.
#[derive(Debug, Clone)]
pub struct AlphaSimulator {
    regs: [u64; 32],
    memory: Vec<u8>,
    pc: u64,
    npc: u64,
    halted: bool,
    loaded_origin: u64,
    loaded_len: usize,
}

impl Default for AlphaSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl AlphaSimulator {
    /// Construct the exact 64 KiB architectural machine.
    pub fn new() -> Self {
        Self {
            regs: [0; 32],
            memory: vec![0; MEMORY_SIZE],
            pc: 0,
            npc: 4,
            halted: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    /// Reset every architectural and lifecycle field deterministically.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Return a complete owned snapshot.
    pub fn get_state(&self) -> AlphaState {
        AlphaState {
            pc: self.pc,
            npc: self.npc,
            regs: self.regs,
            memory: self.memory.clone(),
            halted: self.halted,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    /// Atomically validate and restore a complete snapshot.
    pub fn restore(&mut self, state: &AlphaState) -> Result<(), AlphaError> {
        validate_state(state)?;
        self.pc = state.pc;
        self.npc = state.npc;
        self.regs = state.regs;
        self.memory.clone_from(&state.memory);
        self.halted = state.halted;
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
        Ok(())
    }

    /// Install a program at address zero after validating it.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), AlphaError> {
        self.load_at_checked(program, 0)
    }

    /// Install a program at a word-aligned origin after deterministic reset.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), AlphaError> {
        if origin & 3 != 0 {
            return Err(AlphaError::MisalignedProgram { origin });
        }
        let start = usize::try_from(origin).map_err(|_| AlphaError::ProgramOutOfRange {
            origin,
            length: program.len(),
            memory_size: MEMORY_SIZE,
        })?;
        let end = start
            .checked_add(program.len())
            .filter(|end| *end <= MEMORY_SIZE)
            .ok_or(AlphaError::ProgramOutOfRange {
                origin,
                length: program.len(),
                memory_size: MEMORY_SIZE,
            })?;
        self.reset();
        self.memory[start..end].copy_from_slice(program);
        self.pc = origin;
        self.npc = (origin + 4) & 0xffff;
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        Ok(())
    }

    /// Compatibility load at address zero.
    pub fn load(&mut self, program: &[u8]) -> Result<(), AlphaError> {
        self.load_checked(program)
    }

    pub fn read_register_checked(&self, index: usize) -> Result<u64, AlphaError> {
        if index >= 32 {
            return Err(AlphaError::InvalidRegister { index });
        }
        Ok(self.reg(index))
    }

    pub fn write_register_checked(&mut self, index: usize, value: u64) -> Result<(), AlphaError> {
        if index >= 32 {
            return Err(AlphaError::InvalidRegister { index });
        }
        self.set_reg(index, value);
        Ok(())
    }

    pub fn read_byte_checked(&self, address: u64) -> Result<u8, AlphaError> {
        Ok(self.memory[direct_range(address, 1)?])
    }

    pub fn write_byte_checked(&mut self, address: u64, value: u8) -> Result<(), AlphaError> {
        let index = direct_range(address, 1)?;
        self.memory[index] = value;
        Ok(())
    }

    pub fn read_word_checked(&self, address: u64) -> Result<u16, AlphaError> {
        direct_alignment(address, 2)?;
        let start = direct_range(address, 2)?;
        Ok(u16::from_le_bytes(
            self.memory[start..start + 2]
                .try_into()
                .expect("validated width"),
        ))
    }

    pub fn write_word_checked(&mut self, address: u64, value: u16) -> Result<(), AlphaError> {
        direct_alignment(address, 2)?;
        let start = direct_range(address, 2)?;
        self.memory[start..start + 2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn read_long_checked(&self, address: u64) -> Result<u32, AlphaError> {
        direct_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        Ok(u32::from_le_bytes(
            self.memory[start..start + 4]
                .try_into()
                .expect("validated width"),
        ))
    }

    pub fn write_long_checked(&mut self, address: u64, value: u32) -> Result<(), AlphaError> {
        direct_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        self.memory[start..start + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn read_quad_checked(&self, address: u64) -> Result<u64, AlphaError> {
        direct_alignment(address, 8)?;
        let start = direct_range(address, 8)?;
        Ok(u64::from_le_bytes(
            self.memory[start..start + 8]
                .try_into()
                .expect("validated width"),
        ))
    }

    pub fn write_quad_checked(&mut self, address: u64, value: u64) -> Result<(), AlphaError> {
        direct_alignment(address, 8)?;
        let start = direct_range(address, 8)?;
        self.memory[start..start + 8].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Execute one checked, atomic instruction transition.
    pub fn step_checked(&mut self) -> Result<StepTrace, AlphaError> {
        if self.halted {
            return Err(AlphaError::Halted);
        }
        let pc = self.pc;
        self.validate_fetch(pc)?;
        let raw = self.fetch_word(pc);
        let before = self.get_state();
        let outcome = self.execute_raw(raw, pc);
        match outcome {
            Ok(mnemonic) => {
                let after = self.get_state();
                Ok(StepTrace {
                    pc_before: pc,
                    pc_after: self.pc,
                    raw,
                    mnemonic: mnemonic.to_string(),
                    state_before: before,
                    state_after: after,
                })
            }
            Err(error) => {
                self.restore(&before).expect("previous state is valid");
                Err(error)
            }
        }
    }

    /// Compatibility checked step.
    pub fn step(&mut self) -> Result<StepTrace, AlphaError> {
        self.step_checked()
    }

    /// Run an installed program transactionally.
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

    /// Load and run a program transactionally.
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

    /// Compatibility bounded execution.
    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, AlphaError> {
        self.run_checked(program, max_steps)
    }

    fn reg(&self, index: usize) -> u64 {
        if index == ZERO_REGISTER {
            0
        } else {
            self.regs[index]
        }
    }

    fn set_reg(&mut self, index: usize, value: u64) {
        if index != ZERO_REGISTER {
            self.regs[index] = value;
        }
    }

    fn validate_fetch(&self, pc: u64) -> Result<(), AlphaError> {
        if pc & 3 != 0 {
            return Err(AlphaError::MisalignedAccess {
                address: pc,
                width: 4,
            });
        }
        let start = self.loaded_origin;
        let end = start + self.loaded_len as u64;
        if pc < start || pc.checked_add(4).is_none_or(|next| next > end) {
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
        self.pc = self.npc & 0xffff;
        self.npc = (self.npc + 4) & 0xffff;
    }

    fn execute_raw(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        self.advance();
        let op = raw >> 26;
        match op {
            0x00 => self.exec_pal(raw, pc),
            0x10 => self.exec_inta(raw, pc),
            0x11 => self.exec_intl(raw, pc),
            0x12 => self.exec_ints(raw, pc),
            0x13 => self.exec_intm(raw, pc),
            0x1a => Ok(self.exec_jump(raw, pc)),
            0x0a | 0x0c | 0x0d | 0x0e | 0x28..=0x2d => self.exec_memory(raw, op, pc),
            0x30 | 0x34 | 0x38..=0x3f => Ok(self.exec_branch(raw, op, pc)),
            _ => Err(AlphaError::UnknownInstruction { raw, pc }),
        }
    }

    fn decode_operate(&self, raw: u32) -> (u64, u64, u32, usize) {
        let ra = ((raw >> 21) & 0x1f) as usize;
        let source = if raw & (1 << 12) != 0 {
            ((raw >> 13) & 0xff) as u64
        } else {
            self.reg(((raw >> 16) & 0x1f) as usize)
        };
        (
            self.reg(ra),
            source,
            (raw >> 5) & 0x7f,
            (raw & 0x1f) as usize,
        )
    }

    fn exec_pal(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let palcode = raw & 0x03ff_ffff;
        if palcode == 0 {
            self.halted = true;
            Ok("HALT")
        } else {
            Err(AlphaError::UnsupportedPalcode { palcode, pc })
        }
    }

    fn exec_inta(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let (a, b, func, rc) = self.decode_operate(raw);
        let (value, mnemonic) = match func {
            0x00 | 0x40 => (sext32((a as u32).wrapping_add(b as u32)), "ADDL"),
            0x20 | 0x60 => (a.wrapping_add(b), "ADDQ"),
            0x09 | 0x49 => (sext32((a as u32).wrapping_sub(b as u32)), "SUBL"),
            0x29 | 0x69 => (a.wrapping_sub(b), "SUBQ"),
            0x18 | 0x58 => (sext32((a as u32).wrapping_mul(b as u32)), "MULL"),
            0x38 | 0x78 => (a.wrapping_mul(b), "MULQ"),
            0x02 => (
                sext32((a as u32).wrapping_mul(4).wrapping_add(b as u32)),
                "S4ADDL",
            ),
            0x22 => (a.wrapping_mul(4).wrapping_add(b), "S4ADDQ"),
            0x0b => (
                sext32((a as u32).wrapping_mul(4).wrapping_sub(b as u32)),
                "S4SUBL",
            ),
            0x2b => (a.wrapping_mul(4).wrapping_sub(b), "S4SUBQ"),
            0x12 => (
                sext32((a as u32).wrapping_mul(8).wrapping_add(b as u32)),
                "S8ADDL",
            ),
            0x32 => (a.wrapping_mul(8).wrapping_add(b), "S8ADDQ"),
            0x1b => (
                sext32((a as u32).wrapping_mul(8).wrapping_sub(b as u32)),
                "S8SUBL",
            ),
            0x3b => (a.wrapping_mul(8).wrapping_sub(b), "S8SUBQ"),
            0x2d => (u64::from(a == b), "CMPEQ"),
            0x4d => (u64::from((a as i64) < (b as i64)), "CMPLT"),
            0x6d => (u64::from((a as i64) <= (b as i64)), "CMPLE"),
            0x3d => (u64::from(a < b), "CMPULT"),
            0x7d => (u64::from(a <= b), "CMPULE"),
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        self.set_reg(rc, value);
        Ok(mnemonic)
    }

    fn exec_intl(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let (a, b, func, rc) = self.decode_operate(raw);
        let current = self.reg(rc);
        let (value, mnemonic) = match func {
            0x00 => (a & b, "AND"),
            0x08 => (a & !b, "BIC"),
            0x20 => (a | b, "BIS"),
            0x28 => (a | !b, "ORNOT"),
            0x40 => (a ^ b, "XOR"),
            0x48 => (a ^ !b, "EQV"),
            0x14 => (if a & 1 != 0 { b } else { current }, "CMOVLBS"),
            0x16 => (if a & 1 == 0 { b } else { current }, "CMOVLBC"),
            0x24 => (if a == 0 { b } else { current }, "CMOVEQ"),
            0x26 => (if a != 0 { b } else { current }, "CMOVNE"),
            0x44 => (if (a as i64) < 0 { b } else { current }, "CMOVLT"),
            0x46 => (if (a as i64) >= 0 { b } else { current }, "CMOVGE"),
            0x64 => (if (a as i64) <= 0 { b } else { current }, "CMOVLE"),
            0x66 => (if (a as i64) > 0 { b } else { current }, "CMOVGT"),
            0x61 => (a & !b, "AMASK"),
            0x6c => (0, "IMPLVER"),
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        self.set_reg(rc, value);
        Ok(mnemonic)
    }

    fn exec_ints(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let (a, b, func, rc) = self.decode_operate(raw);
        let shift = (b & 63) as u32;
        let offset = ((b & 7) * 8) as u32;
        let (value, mnemonic) = match func {
            0x39 => (a.wrapping_shl(shift), "SLL"),
            0x34 => (a >> shift, "SRL"),
            0x3c => (((a as i64) >> shift) as u64, "SRA"),
            0x06 => ((a >> offset) & 0xff, "EXTBL"),
            0x16 => ((a >> offset) & 0xffff, "EXTWL"),
            0x26 => ((a >> offset) & 0xffff_ffff, "EXTLL"),
            0x36 => (a >> offset, "EXTQL"),
            0x0b => ((a & 0xff).wrapping_shl(offset), "INSBL"),
            0x1b => ((a & 0xffff).wrapping_shl(offset), "INSWL"),
            0x2b => ((a & 0xffff_ffff).wrapping_shl(offset), "INSLL"),
            0x3b => (a.wrapping_shl(offset), "INSQL"),
            0x02 => (a & !(0xffu64.wrapping_shl(offset)), "MSKBL"),
            0x12 => (a & !(0xffffu64.wrapping_shl(offset)), "MSKWL"),
            0x22 => (a & !(0xffff_ffffu64.wrapping_shl(offset)), "MSKLL"),
            0x32 => (a & !(u64::MAX.wrapping_shl(offset)), "MSKQL"),
            0x30 => (zap(a, b as u8, false), "ZAP"),
            0x31 => (zap(a, b as u8, true), "ZAPNOT"),
            0x00 => ((a as u8 as i8 as i64) as u64, "SEXTB"),
            0x01 => ((a as u16 as i16 as i64) as u64, "SEXTW"),
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        self.set_reg(rc, value);
        Ok(mnemonic)
    }

    fn exec_intm(&mut self, raw: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let (a, b, func, rc) = self.decode_operate(raw);
        let (value, mnemonic) = match func {
            0x00 | 0x40 => (sext32((a as u32).wrapping_mul(b as u32)), "MULL"),
            0x20 | 0x60 => (a.wrapping_mul(b), "MULQ"),
            0x30 => (((u128::from(a) * u128::from(b)) >> 64) as u64, "UMULH"),
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        self.set_reg(rc, value);
        Ok(mnemonic)
    }

    fn exec_memory(&mut self, raw: u32, op: u32, pc: u64) -> Result<&'static str, AlphaError> {
        let ra = ((raw >> 21) & 0x1f) as usize;
        let rb = ((raw >> 16) & 0x1f) as usize;
        let displacement = raw as u16 as i16 as i64;
        let address = self.reg(rb).wrapping_add(displacement as u64) & 0xffff;
        let mnemonic = match op {
            0x28 | 0x2a => {
                let value = sext32(self.load_width(address, 4)? as u32);
                self.set_reg(ra, value);
                "LDL"
            }
            0x29 | 0x2b => {
                let value = self.load_width(address, 8)?;
                self.set_reg(ra, value);
                "LDQ"
            }
            0x0a => {
                let value = self.load_width(address, 1)?;
                self.set_reg(ra, value);
                "LDBU"
            }
            0x0c => {
                let value = self.load_width(address, 2)?;
                self.set_reg(ra, value);
                "LDWU"
            }
            0x2c => {
                self.store_width(address, 4, self.reg(ra))?;
                "STL"
            }
            0x2d => {
                self.store_width(address, 8, self.reg(ra))?;
                "STQ"
            }
            0x0e => {
                self.store_width(address, 1, self.reg(ra))?;
                "STB"
            }
            0x0d => {
                self.store_width(address, 2, self.reg(ra))?;
                "STW"
            }
            _ => return Err(AlphaError::UnknownInstruction { raw, pc }),
        };
        Ok(mnemonic)
    }

    fn exec_branch(&mut self, raw: u32, op: u32, pc: u64) -> &'static str {
        let ra = ((raw >> 21) & 0x1f) as usize;
        let value = self.reg(ra);
        let displacement = (((raw & 0x1f_ffff) << 11) as i32 >> 11) as i64;
        let target = (pc + 4).wrapping_add((displacement * 4) as u64) & 0xffff;
        let (taken, mnemonic) = match op {
            0x30 => (true, "BR"),
            0x34 => {
                self.set_reg(ra, pc + 4);
                (true, "BSR")
            }
            0x39 => (value == 0, "BEQ"),
            0x3d => (value != 0, "BNE"),
            0x3a => ((value as i64) < 0, "BLT"),
            0x3b => ((value as i64) <= 0, "BLE"),
            0x3f => ((value as i64) > 0, "BGT"),
            0x3e => ((value as i64) >= 0, "BGE"),
            0x38 => (value & 1 == 0, "BLBC"),
            0x3c => (value & 1 != 0, "BLBS"),
            _ => unreachable!("dispatch restricts branch opcodes"),
        };
        if taken {
            self.pc = target;
            self.npc = (target + 4) & 0xffff;
        }
        mnemonic
    }

    fn exec_jump(&mut self, raw: u32, pc: u64) -> &'static str {
        let ra = ((raw >> 21) & 0x1f) as usize;
        let rb = ((raw >> 16) & 0x1f) as usize;
        let function = (raw >> 14) & 3;
        let target = self.reg(rb) & !3 & 0xffff;
        self.pc = target;
        self.npc = (target + 4) & 0xffff;
        self.set_reg(ra, pc + 4);
        ["JMP", "JSR", "RET", "JSR_COROUTINE"][function as usize]
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
            self.memory[(address as usize + offset) & 0xffff] = (value >> (8 * offset)) as u8;
        }
        Ok(())
    }
}

fn validate_state(state: &AlphaState) -> Result<(), AlphaError> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(AlphaError::InvalidState(format!(
            "memory must contain exactly {MEMORY_SIZE} bytes"
        )));
    }
    if state.regs[ZERO_REGISTER] != 0 {
        return Err(AlphaError::InvalidState("r31 must remain zero".into()));
    }
    if state.pc >= MEMORY_SIZE as u64 || state.pc & 3 != 0 {
        return Err(AlphaError::InvalidState(
            "PC must be aligned inside memory".into(),
        ));
    }
    if state.npc >= MEMORY_SIZE as u64 || state.npc & 3 != 0 {
        return Err(AlphaError::InvalidState(
            "nPC must be aligned inside memory".into(),
        ));
    }
    if state.loaded_origin & 3 != 0
        || state.loaded_origin >= MEMORY_SIZE as u64
        || state.loaded_origin + state.loaded_len as u64 > MEMORY_SIZE as u64
    {
        return Err(AlphaError::InvalidState(
            "installed program range is invalid".into(),
        ));
    }
    Ok(())
}

fn direct_range(address: u64, width: usize) -> Result<usize, AlphaError> {
    let start =
        usize::try_from(address).map_err(|_| AlphaError::MemoryOutOfRange { address, width })?;
    if start.checked_add(width).is_none_or(|end| end > MEMORY_SIZE) {
        return Err(AlphaError::MemoryOutOfRange { address, width });
    }
    Ok(start)
}

fn direct_alignment(address: u64, width: usize) -> Result<(), AlphaError> {
    if !address.is_multiple_of(width as u64) {
        Err(AlphaError::MisalignedAccess { address, width })
    } else {
        Ok(())
    }
}

fn sext32(value: u32) -> u64 {
    value as i32 as i64 as u64
}

fn zap(value: u64, mask: u8, keep_set: bool) -> u64 {
    let mut result = 0;
    for byte in 0..8 {
        let set = mask & (1 << byte) != 0;
        if set == keep_set {
            result |= value & (0xffu64 << (byte * 8));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operate(op: u32, ra: u32, literal: u32, func: u32, rc: u32) -> u32 {
        (op << 26) | (ra << 21) | ((literal & 0xff) << 13) | (1 << 12) | (func << 5) | rc
    }

    fn assemble(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    #[test]
    fn bis_immediate_and_halt() {
        let mut cpu = AlphaSimulator::new();
        let result = cpu
            .run_checked(&assemble(&[operate(0x11, 31, 42, 0x20, 1), 0]), 4)
            .unwrap();
        assert_eq!(result.final_state.regs[1], 42);
        assert!(result.halted);
    }

    #[test]
    fn typed_faults_are_atomic() {
        let mut cpu = AlphaSimulator::new();
        cpu.load_checked(&assemble(&[1])).unwrap();
        let before = cpu.get_state();
        assert!(matches!(
            cpu.step_checked(),
            Err(AlphaError::UnsupportedPalcode { .. })
        ));
        assert_eq!(cpu.get_state(), before);
    }

    #[test]
    fn restore_validates_zero_register_and_memory() {
        let mut cpu = AlphaSimulator::new();
        let before = cpu.get_state();
        let mut invalid = before.clone();
        invalid.regs[31] = 1;
        assert!(matches!(
            cpu.restore(&invalid),
            Err(AlphaError::InvalidState(_))
        ));
        assert_eq!(cpu.get_state(), before);
    }
}
