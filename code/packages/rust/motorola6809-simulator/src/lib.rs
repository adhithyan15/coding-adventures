//! Checked, deterministic Motorola MC6809 functional simulator.

use core::fmt;

/// Size of the MC6809's sixteen-bit address space.
pub const MEMORY_SIZE: usize = 65_536;
/// Maximum number of traces retained by one bounded execution.
pub const MAX_RETAINED_STEPS: usize = 65_536;

pub const CC_C: u8 = 0x01;
pub const CC_V: u8 = 0x02;
pub const CC_Z: u8 = 0x04;
pub const CC_N: u8 = 0x08;
pub const CC_I: u8 = 0x10;
pub const CC_H: u8 = 0x20;
pub const CC_F: u8 = 0x40;
pub const CC_E: u8 = 0x80;

/// Report whether an opcode is documented on the base, page-2, or page-3 map.
#[must_use]
pub fn opcode_is_documented(page: u8, opcode: u8) -> bool {
    match page {
        0 => !matches!(
            opcode,
            0x01 | 0x02
                | 0x05
                | 0x0b
                | 0x14
                | 0x15
                | 0x18
                | 0x1b
                | 0x38
                | 0x3e
                | 0x41
                | 0x42
                | 0x45
                | 0x4b
                | 0x4e
                | 0x51
                | 0x52
                | 0x55
                | 0x5b
                | 0x5e
                | 0x61
                | 0x62
                | 0x65
                | 0x6b
                | 0x71
                | 0x72
                | 0x75
                | 0x7b
                | 0x87
                | 0x8f
                | 0xc7
                | 0xcd
                | 0xcf
        ),
        0x10 => matches!(
            opcode,
            0x21..=0x2f
                | 0x3f
                | 0x83
                | 0x8c
                | 0x8e
                | 0x93
                | 0x9c
                | 0x9e
                | 0x9f
                | 0xa3
                | 0xac
                | 0xae
                | 0xaf
                | 0xb3
                | 0xbc
                | 0xbe
                | 0xbf
                | 0xce
                | 0xde
                | 0xdf
                | 0xee
                | 0xef
                | 0xfe
                | 0xff
        ),
        0x11 => matches!(
            opcode,
            0x3f | 0x83 | 0x8c | 0x93 | 0x9c | 0xa3 | 0xac | 0xb3 | 0xbc
        ),
        _ => false,
    }
}

/// Report whether an indexed-addressing postbyte has a documented meaning.
#[must_use]
pub fn indexed_postbyte_is_valid(postbyte: u8) -> bool {
    if postbyte & 0x80 == 0 {
        return true;
    }
    let indirect = postbyte & 0x10 != 0;
    let mode = postbyte & 0x0f;
    match mode {
        0 | 2 => !indirect,
        1 | 3 | 4 | 5 | 6 | 8 | 9 | 0x0b | 0x0c | 0x0d => true,
        0x0f => postbyte == 0x9f,
        _ => false,
    }
}

/// Complete owned architectural state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Motorola6809State {
    pub a: u8,
    pub b: u8,
    pub x: u16,
    pub y: u16,
    pub u: u16,
    pub s: u16,
    pub pc: u16,
    pub dp: u8,
    pub cc: u8,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u16,
    pub loaded_len: usize,
}

impl Motorola6809State {
    #[must_use]
    pub const fn d(&self) -> u16 {
        u16::from_be_bytes([self.a, self.b])
    }

    fn set_d(&mut self, value: u16) {
        [self.a, self.b] = value.to_be_bytes();
    }
}

impl Default for Motorola6809State {
    fn default() -> Self {
        Self {
            a: 0,
            b: 0,
            x: 0,
            y: 0,
            u: 0,
            s: 0,
            pc: 0,
            dp: 0,
            cc: CC_I | CC_F,
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

/// One successful instruction transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub raw: Vec<u8>,
    pub width: u8,
    pub mnemonic: &'static str,
    pub state_after: Motorola6809State,
}

/// Result of a successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: Motorola6809State,
}

/// Checked lifecycle and decode failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Motorola6809Error {
    EmptyProgram,
    ProgramOutOfRange { origin: u16, len: usize },
    MemoryOutOfRange { address: u16, len: usize },
    FetchOutsideProgram { pc: u16 },
    TruncatedInstruction { pc: u16, needed: usize },
    UnknownOpcode { opcode: u16, pc: u16 },
    InvalidIndexedPostbyte { postbyte: u8, pc: u16 },
    InvalidTransfer { postbyte: u8, pc: u16 },
    InvalidState { reason: &'static str },
    Halted,
    RetentionLimitExceeded { requested: usize, limit: usize },
    AllocationFailed,
    StepLimitExceeded { limit: usize },
}

impl fmt::Display for Motorola6809Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => write!(f, "program image is empty"),
            Self::ProgramOutOfRange { origin, len } => {
                write!(f, "program of {len} bytes does not fit at ${origin:04X}")
            }
            Self::MemoryOutOfRange { address, len } => {
                write!(
                    f,
                    "memory access of {len} bytes does not fit at ${address:04X}"
                )
            }
            Self::FetchOutsideProgram { pc } => write!(f, "fetch at ${pc:04X} is outside program"),
            Self::TruncatedInstruction { pc, needed } => {
                write!(f, "instruction at ${pc:04X} needs {needed} bytes")
            }
            Self::UnknownOpcode { opcode, pc } => {
                write!(f, "unknown opcode ${opcode:04X} at ${pc:04X}")
            }
            Self::InvalidIndexedPostbyte { postbyte, pc } => {
                write!(f, "invalid indexed postbyte ${postbyte:02X} at ${pc:04X}")
            }
            Self::InvalidTransfer { postbyte, pc } => {
                write!(f, "invalid transfer postbyte ${postbyte:02X} at ${pc:04X}")
            }
            Self::InvalidState { reason } => write!(f, "invalid MC6809 state: {reason}"),
            Self::Halted => write!(f, "processor is waiting"),
            Self::RetentionLimitExceeded { requested, limit } => {
                write!(f, "requested {requested} traces exceeds limit {limit}")
            }
            Self::AllocationFailed => write!(f, "trace allocation failed"),
            Self::StepLimitExceeded { limit } => write!(f, "step limit {limit} exceeded"),
        }
    }
}

impl std::error::Error for Motorola6809Error {}

#[derive(Clone, Copy)]
enum Mode {
    Immediate,
    Direct,
    Indexed,
    Extended,
}

/// Functional simulator for the Motorola MC6809.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Motorola6809Simulator {
    state: Motorola6809State,
    fetched: usize,
}

impl Motorola6809Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = Motorola6809State::default();
        self.fetched = 0;
    }

    pub fn load(&mut self, program: &[u8], origin: u16) -> Result<(), Motorola6809Error> {
        if program.is_empty() {
            return Err(Motorola6809Error::EmptyProgram);
        }
        let start = usize::from(origin);
        let Some(end) = start.checked_add(program.len()) else {
            return Err(Motorola6809Error::ProgramOutOfRange {
                origin,
                len: program.len(),
            });
        };
        if end > MEMORY_SIZE {
            return Err(Motorola6809Error::ProgramOutOfRange {
                origin,
                len: program.len(),
            });
        }
        let mut state = Motorola6809State {
            pc: origin,
            loaded_origin: origin,
            loaded_len: program.len(),
            ..Motorola6809State::default()
        };
        state.memory[start..end].copy_from_slice(program);
        self.state = state;
        self.fetched = 0;
        Ok(())
    }

    pub fn restore(&mut self, state: Motorola6809State) -> Result<(), Motorola6809Error> {
        if state.memory.len() != MEMORY_SIZE {
            return Err(Motorola6809Error::InvalidState {
                reason: "memory must contain exactly 65536 bytes",
            });
        }
        let end = usize::from(state.loaded_origin).checked_add(state.loaded_len);
        if state.loaded_len == 0 || end.is_none_or(|value| value > MEMORY_SIZE) {
            return Err(Motorola6809Error::InvalidState {
                reason: "installed program range is invalid",
            });
        }
        self.state = state;
        self.fetched = 0;
        Ok(())
    }

    #[must_use]
    pub fn get_state(&self) -> Motorola6809State {
        self.state.clone()
    }
    #[must_use]
    pub fn read_a(&self) -> u8 {
        self.state.a
    }
    #[must_use]
    pub fn read_b(&self) -> u8 {
        self.state.b
    }
    #[must_use]
    pub fn read_d(&self) -> u16 {
        self.state.d()
    }
    #[must_use]
    pub fn read_x(&self) -> u16 {
        self.state.x
    }
    #[must_use]
    pub fn read_y(&self) -> u16 {
        self.state.y
    }
    #[must_use]
    pub fn read_u(&self) -> u16 {
        self.state.u
    }
    #[must_use]
    pub fn read_s(&self) -> u16 {
        self.state.s
    }
    #[must_use]
    pub fn read_dp(&self) -> u8 {
        self.state.dp
    }
    #[must_use]
    pub fn read_cc(&self) -> u8 {
        self.state.cc
    }
    pub fn write_a(&mut self, value: u8) {
        self.state.a = value;
    }
    pub fn write_b(&mut self, value: u8) {
        self.state.b = value;
    }
    pub fn write_d(&mut self, value: u16) {
        self.state.set_d(value);
    }
    pub fn write_x(&mut self, value: u16) {
        self.state.x = value;
    }
    pub fn write_y(&mut self, value: u16) {
        self.state.y = value;
    }
    pub fn write_u(&mut self, value: u16) {
        self.state.u = value;
    }
    pub fn write_s(&mut self, value: u16) {
        self.state.s = value;
    }
    pub fn write_dp(&mut self, value: u8) {
        self.state.dp = value;
    }
    pub fn write_cc(&mut self, value: u8) {
        self.state.cc = value;
    }

    pub fn read_memory(&self, address: u16, len: usize) -> Result<Vec<u8>, Motorola6809Error> {
        let start = usize::from(address);
        let Some(end) = start.checked_add(len) else {
            return Err(Motorola6809Error::MemoryOutOfRange { address, len });
        };
        if end > MEMORY_SIZE {
            return Err(Motorola6809Error::MemoryOutOfRange { address, len });
        }
        Ok(self.state.memory[start..end].to_vec())
    }

    pub fn write_memory(&mut self, address: u16, bytes: &[u8]) -> Result<(), Motorola6809Error> {
        let start = usize::from(address);
        let Some(end) = start.checked_add(bytes.len()) else {
            return Err(Motorola6809Error::MemoryOutOfRange {
                address,
                len: bytes.len(),
            });
        };
        if end > MEMORY_SIZE {
            return Err(Motorola6809Error::MemoryOutOfRange {
                address,
                len: bytes.len(),
            });
        }
        self.state.memory[start..end].copy_from_slice(bytes);
        Ok(())
    }

    pub fn step(&mut self) -> Result<StepTrace, Motorola6809Error> {
        if self.state.halted {
            return Err(Motorola6809Error::Halted);
        }
        let checkpoint = self.clone();
        match self.step_inner() {
            Ok(trace) => Ok(trace),
            Err(error) => {
                *self = checkpoint;
                Err(error)
            }
        }
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Motorola6809Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(Motorola6809Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let checkpoint = self.clone();
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| Motorola6809Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
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
                    *self = checkpoint;
                    return Err(error);
                }
            }
        }
        *self = checkpoint;
        Err(Motorola6809Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u16,
        max_steps: usize,
    ) -> Result<ExecutionResult, Motorola6809Error> {
        let checkpoint = self.clone();
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                *self = checkpoint;
                Err(error)
            }
        }
    }

    fn step_inner(&mut self) -> Result<StepTrace, Motorola6809Error> {
        let pc_before = self.state.pc;
        self.fetched = 0;
        let first = self.fetch_byte(pc_before)?;
        let (page, opcode) = if matches!(first, 0x10 | 0x11) {
            (first, self.fetch_byte(pc_before)?)
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
        let start = usize::from(pc_before);
        let raw = self.state.memory[start..start + self.fetched].to_vec();
        Ok(StepTrace {
            pc_before,
            pc_after: self.state.pc,
            raw,
            width: u8::try_from(self.fetched).expect("MC6809 instruction width fits u8"),
            mnemonic,
            state_after: self.get_state(),
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
        let value = self.state.memory[index];
        self.state.pc = pc.wrapping_add(1);
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
            self.state.memory[usize::from(address.wrapping_add(1))],
        ])
    }

    fn write16(&mut self, address: u16, value: u16) {
        let [hi, lo] = value.to_be_bytes();
        self.state.memory[usize::from(address)] = hi;
        self.state.memory[usize::from(address.wrapping_add(1))] = lo;
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
        let postbyte = self.fetch_byte(pc)?;
        if !indexed_postbyte_is_valid(postbyte) {
            return Err(Motorola6809Error::InvalidIndexedPostbyte { postbyte, pc });
        }
        if postbyte & 0x80 == 0 {
            let register = (postbyte >> 5) & 3;
            let raw = i16::from(postbyte & 0x1f);
            let offset = if raw & 0x10 != 0 { raw - 0x20 } else { raw };
            return Ok(self.index_register(register).wrapping_add_signed(offset));
        }
        let register = (postbyte >> 5) & 3;
        let indirect = postbyte & 0x10 != 0;
        let mode = postbyte & 0x0f;
        let mut address = match mode {
            0 => {
                let value = self.index_register(register);
                self.set_index_register(register, value.wrapping_add(1));
                value
            }
            1 => {
                let value = self.index_register(register);
                self.set_index_register(register, value.wrapping_add(2));
                value
            }
            2 => {
                let value = self.index_register(register).wrapping_sub(1);
                self.set_index_register(register, value);
                value
            }
            3 => {
                let value = self.index_register(register).wrapping_sub(2);
                self.set_index_register(register, value);
                value
            }
            4 => self.index_register(register),
            5 => self
                .index_register(register)
                .wrapping_add_signed(i16::from(self.state.b as i8)),
            6 => self
                .index_register(register)
                .wrapping_add_signed(i16::from(self.state.a as i8)),
            8 => {
                let offset = self.fetch_byte(pc)? as i8;
                self.index_register(register)
                    .wrapping_add_signed(i16::from(offset))
            }
            9 => {
                let offset = self.fetch_word(pc)? as i16;
                self.index_register(register).wrapping_add_signed(offset)
            }
            0x0b => self
                .index_register(register)
                .wrapping_add_signed(self.state.d() as i16),
            0x0c => {
                let offset = self.fetch_byte(pc)? as i8;
                self.state.pc.wrapping_add_signed(i16::from(offset))
            }
            0x0d => {
                let offset = self.fetch_word(pc)? as i16;
                self.state.pc.wrapping_add_signed(offset)
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
        if set {
            self.state.cc |= flag;
        } else {
            self.state.cc &= !flag;
        }
    }

    fn set_nz8(&mut self, value: u8) {
        self.set_flag(CC_N, value & 0x80 != 0);
        self.set_flag(CC_Z, value == 0);
    }

    fn set_nz16(&mut self, value: u16) {
        self.set_flag(CC_N, value & 0x8000 != 0);
        self.set_flag(CC_Z, value == 0);
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
        let cin = u16::from(carry);
        let sum = u16::from(left) + u16::from(right) + cin;
        let result = sum as u8;
        self.set_flag(
            CC_H,
            u16::from(left & 0x0f) + u16::from(right & 0x0f) + cin > 0x0f,
        );
        self.set_nz8(result);
        self.set_flag(CC_V, (!(left ^ right) & (left ^ result) & 0x80) != 0);
        self.set_flag(CC_C, sum > 0xff);
        result
    }

    fn sub8(&mut self, left: u8, right: u8, borrow: bool) -> u8 {
        let subtrahend = u16::from(right) + u16::from(borrow);
        let result = left.wrapping_sub(right).wrapping_sub(u8::from(borrow));
        self.set_nz8(result);
        self.set_flag(CC_V, ((left ^ right) & (left ^ result) & 0x80) != 0);
        self.set_flag(CC_C, u16::from(left) < subtrahend);
        result
    }

    fn add16(&mut self, left: u16, right: u16) -> u16 {
        let sum = u32::from(left) + u32::from(right);
        let result = sum as u16;
        self.set_nz16(result);
        self.set_flag(CC_V, (!(left ^ right) & (left ^ result) & 0x8000) != 0);
        self.set_flag(CC_C, sum > 0xffff);
        result
    }

    fn sub16(&mut self, left: u16, right: u16) -> u16 {
        let result = left.wrapping_sub(right);
        self.set_nz16(result);
        self.set_flag(CC_V, ((left ^ right) & (left ^ result) & 0x8000) != 0);
        self.set_flag(CC_C, left < right);
        result
    }

    fn branch_condition(&self, code: u8) -> bool {
        let c = self.state.cc & CC_C != 0;
        let v = self.state.cc & CC_V != 0;
        let z = self.state.cc & CC_Z != 0;
        let n = self.state.cc & CC_N != 0;
        match code & 0x0f {
            0 => true,
            1 => false,
            2 => !c && !z,
            3 => c || z,
            4 => !c,
            5 => c,
            6 => !z,
            7 => z,
            8 => !v,
            9 => v,
            0x0a => !n,
            0x0b => n,
            0x0c => n == v,
            0x0d => n != v,
            0x0e => !z && n == v,
            _ => z || n != v,
        }
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
                self.state.pc = self.state.pc.wrapping_add_signed(offset);
                Ok("LBRA")
            }
            0x17 => {
                let offset = self.fetch_word(pc)? as i16;
                let target = self.state.pc.wrapping_add_signed(offset);
                self.push16_s(self.state.pc);
                self.state.pc = target;
                Ok("LBSR")
            }
            0x19 => {
                self.daa();
                Ok("DAA")
            }
            0x1a => {
                self.state.cc |= self.fetch_byte(pc)?;
                Ok("ORCC")
            }
            0x1c => {
                self.state.cc &= self.fetch_byte(pc)?;
                Ok("ANDCC")
            }
            0x1d => {
                self.state.a = if self.state.b & 0x80 != 0 { 0xff } else { 0 };
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
                    self.state.pc = self.state.pc.wrapping_add_signed(i16::from(offset));
                }
                Ok(branch_mnemonic(opcode, false))
            }
            0x30..=0x33 => {
                let address = self.indexed_address(pc)?;
                match opcode {
                    0x30 => {
                        self.state.x = address;
                        self.set_flag(CC_Z, address == 0);
                        Ok("LEAX")
                    }
                    0x31 => {
                        self.state.y = address;
                        self.set_flag(CC_Z, address == 0);
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
                self.state.x = self.state.x.wrapping_add(u16::from(self.state.b));
                Ok("ABX")
            }
            0x3b => {
                self.rti();
                Ok("RTI")
            }
            0x3c => {
                self.state.cc &= self.fetch_byte(pc)?;
                self.state.cc |= CC_E;
                self.push_mask_s(0xff);
                self.state.halted = true;
                Ok("CWAI")
            }
            0x3d => {
                let product = u16::from(self.state.a) * u16::from(self.state.b);
                self.state.set_d(product);
                self.set_flag(CC_Z, product == 0);
                self.set_flag(CC_C, product & 0x80 != 0);
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
                    self.state.pc = self.state.pc.wrapping_add_signed(offset);
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
            let target = self.state.pc.wrapping_add_signed(i16::from(offset));
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
                self.state.a = self.sub8(self.state.a, value, self.state.cc & CC_C != 0);
                Ok("SBCA")
            }
            3 => {
                let value = self.operand16(mode, pc)?;
                let result = self.sub16(self.state.d(), value);
                self.state.set_d(result);
                Ok("SUBD")
            }
            4 => {
                let value = self.operand8(mode, pc)?;
                self.state.a &= value;
                self.load8_flags(self.state.a);
                Ok("ANDA")
            }
            5 => {
                let value = self.operand8(mode, pc)?;
                self.load8_flags(self.state.a & value);
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
                self.state.a ^= value;
                self.load8_flags(self.state.a);
                Ok("EORA")
            }
            9 => {
                let value = self.operand8(mode, pc)?;
                self.state.a = self.add8(self.state.a, value, self.state.cc & CC_C != 0);
                Ok("ADCA")
            }
            0x0a => {
                let value = self.operand8(mode, pc)?;
                self.state.a |= value;
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
                self.state.b = self.sub8(self.state.b, value, self.state.cc & CC_C != 0);
                Ok("SBCB")
            }
            3 => {
                let value = self.operand16(mode, pc)?;
                let result = self.add16(self.state.d(), value);
                self.state.set_d(result);
                Ok("ADDD")
            }
            4 => {
                let value = self.operand8(mode, pc)?;
                self.state.b &= value;
                self.load8_flags(self.state.b);
                Ok("ANDB")
            }
            5 => {
                let value = self.operand8(mode, pc)?;
                self.load8_flags(self.state.b & value);
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
                self.state.b ^= value;
                self.load8_flags(self.state.b);
                Ok("EORB")
            }
            9 => {
                let value = self.operand8(mode, pc)?;
                self.state.b = self.add8(self.state.b, value, self.state.cc & CC_C != 0);
                Ok("ADCB")
            }
            0x0a => {
                let value = self.operand8(mode, pc)?;
                self.state.b |= value;
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
                self.state.set_d(value);
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
                let result = 0_u8.wrapping_sub(value);
                self.set_nz8(result);
                self.set_flag(CC_V, value == 0x80);
                self.set_flag(CC_C, value != 0);
                (result, "NEG", true)
            }
            3 => {
                let result = !value;
                self.set_nz8(result);
                self.set_flag(CC_V, false);
                self.set_flag(CC_C, true);
                (result, "COM", true)
            }
            4 => {
                let result = value >> 1;
                self.set_flag(CC_C, value & 1 != 0);
                self.set_flag(CC_N, false);
                self.set_flag(CC_Z, result == 0);
                self.set_flag(CC_V, self.state.cc & CC_C != 0);
                (result, "LSR", true)
            }
            6 => {
                let carry = self.state.cc & CC_C != 0;
                let result = (value >> 1) | if carry { 0x80 } else { 0 };
                self.set_flag(CC_C, value & 1 != 0);
                self.set_nz8(result);
                self.set_flag(
                    CC_V,
                    (self.state.cc & CC_N != 0) != (self.state.cc & CC_C != 0),
                );
                (result, "ROR", true)
            }
            7 => {
                let result = (value >> 1) | (value & 0x80);
                self.set_flag(CC_C, value & 1 != 0);
                self.set_nz8(result);
                self.set_flag(
                    CC_V,
                    (self.state.cc & CC_N != 0) != (self.state.cc & CC_C != 0),
                );
                (result, "ASR", true)
            }
            8 => {
                let result = value << 1;
                self.set_flag(CC_C, value & 0x80 != 0);
                self.set_nz8(result);
                self.set_flag(
                    CC_V,
                    (self.state.cc & CC_N != 0) != (self.state.cc & CC_C != 0),
                );
                (result, "ASL", true)
            }
            9 => {
                let carry = self.state.cc & CC_C != 0;
                let result = (value << 1) | u8::from(carry);
                self.set_flag(CC_C, value & 0x80 != 0);
                self.set_nz8(result);
                self.set_flag(
                    CC_V,
                    (self.state.cc & CC_N != 0) != (self.state.cc & CC_C != 0),
                );
                (result, "ROL", true)
            }
            0x0a => {
                let result = value.wrapping_sub(1);
                self.set_nz8(result);
                self.set_flag(CC_V, value == 0x80);
                (result, "DEC", true)
            }
            0x0c => {
                let result = value.wrapping_add(1);
                self.set_nz8(result);
                self.set_flag(CC_V, value == 0x7f);
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
            0 => self.state.set_d(value),
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
        self.state.s = self.state.s.wrapping_sub(1);
        self.state.memory[usize::from(self.state.s)] = value;
    }
    fn pull8_s(&mut self) -> u8 {
        let value = self.state.memory[usize::from(self.state.s)];
        self.state.s = self.state.s.wrapping_add(1);
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
        self.state.u = self.state.u.wrapping_sub(1);
        self.state.memory[usize::from(self.state.u)] = value;
    }
    fn pull8_u(&mut self) -> u8 {
        let value = self.state.memory[usize::from(self.state.u)];
        self.state.u = self.state.u.wrapping_add(1);
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
        if mask & 0x80 != 0 {
            self.push16_s(self.state.pc);
        }
        if mask & 0x40 != 0 {
            self.push16_s(self.state.u);
        }
        if mask & 0x20 != 0 {
            self.push16_s(self.state.y);
        }
        if mask & 0x10 != 0 {
            self.push16_s(self.state.x);
        }
        if mask & 0x08 != 0 {
            self.push8_s(self.state.dp);
        }
        if mask & 0x04 != 0 {
            self.push8_s(self.state.b);
        }
        if mask & 0x02 != 0 {
            self.push8_s(self.state.a);
        }
        if mask & 0x01 != 0 {
            self.push8_s(self.state.cc);
        }
    }

    fn pull_mask_s(&mut self, mask: u8) {
        if mask & 0x01 != 0 {
            self.state.cc = self.pull8_s();
        }
        if mask & 0x02 != 0 {
            self.state.a = self.pull8_s();
        }
        if mask & 0x04 != 0 {
            self.state.b = self.pull8_s();
        }
        if mask & 0x08 != 0 {
            self.state.dp = self.pull8_s();
        }
        if mask & 0x10 != 0 {
            self.state.x = self.pull16_s();
        }
        if mask & 0x20 != 0 {
            self.state.y = self.pull16_s();
        }
        if mask & 0x40 != 0 {
            self.state.u = self.pull16_s();
        }
        if mask & 0x80 != 0 {
            self.state.pc = self.pull16_s();
        }
    }

    fn push_mask_u(&mut self, mask: u8) {
        if mask & 0x80 != 0 {
            self.push16_u(self.state.pc);
        }
        if mask & 0x40 != 0 {
            self.push16_u(self.state.s);
        }
        if mask & 0x20 != 0 {
            self.push16_u(self.state.y);
        }
        if mask & 0x10 != 0 {
            self.push16_u(self.state.x);
        }
        if mask & 0x08 != 0 {
            self.push8_u(self.state.dp);
        }
        if mask & 0x04 != 0 {
            self.push8_u(self.state.b);
        }
        if mask & 0x02 != 0 {
            self.push8_u(self.state.a);
        }
        if mask & 0x01 != 0 {
            self.push8_u(self.state.cc);
        }
    }

    fn pull_mask_u(&mut self, mask: u8) {
        if mask & 0x01 != 0 {
            self.state.cc = self.pull8_u();
        }
        if mask & 0x02 != 0 {
            self.state.a = self.pull8_u();
        }
        if mask & 0x04 != 0 {
            self.state.b = self.pull8_u();
        }
        if mask & 0x08 != 0 {
            self.state.dp = self.pull8_u();
        }
        if mask & 0x10 != 0 {
            self.state.x = self.pull16_u();
        }
        if mask & 0x20 != 0 {
            self.state.y = self.pull16_u();
        }
        if mask & 0x40 != 0 {
            self.state.s = self.pull16_u();
        }
        if mask & 0x80 != 0 {
            self.state.pc = self.pull16_u();
        }
    }

    fn software_interrupt(&mut self, vector: u16, mask_interrupts: bool) {
        self.state.cc |= CC_E;
        self.push_mask_s(0xff);
        if mask_interrupts {
            self.state.cc |= CC_I | CC_F;
        }
        self.state.pc = self.read16(vector);
    }

    fn rti(&mut self) {
        self.state.cc = self.pull8_s();
        if self.state.cc & CC_E != 0 {
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
        let carry = self.state.cc & CC_C != 0;
        let half = self.state.cc & CC_H != 0;
        let mut correction = 0_u16;
        if original & 0x0f > 9 || half {
            correction |= 0x06;
        }
        if original > 0x99 || carry {
            correction |= 0x60;
        }
        let result = u16::from(original) + correction;
        self.state.a = result as u8;
        self.set_nz8(self.state.a);
        self.set_flag(CC_V, false);
        self.set_flag(CC_C, carry || result > 0xff);
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
