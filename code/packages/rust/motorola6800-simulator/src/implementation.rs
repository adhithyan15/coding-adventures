//! Checked Motorola MC6800 execution engine.

use core::fmt;

/// Size of the MC6800's sixteen-bit address space.
pub const MEMORY_SIZE: usize = 65_536;
/// Carry condition-code bit.
pub const CC_C: u8 = 0x01;
/// Overflow condition-code bit.
pub const CC_V: u8 = 0x02;
/// Zero condition-code bit.
pub const CC_Z: u8 = 0x04;
/// Negative condition-code bit.
pub const CC_N: u8 = 0x08;
/// Interrupt-mask condition-code bit.
pub const CC_I: u8 = 0x10;
/// Half-carry condition-code bit.
pub const CC_H: u8 = 0x20;
const CC_RESERVED: u8 = 0xc0;

const INVALID_OPCODES: [u8; 58] = [
    0x00, 0x02, 0x03, 0x04, 0x05, 0x12, 0x13, 0x14, 0x15, 0x18, 0x1a, 0x1c, 0x1d, 0x1e, 0x1f, 0x21,
    0x38, 0x3a, 0x3c, 0x3d, 0x41, 0x42, 0x45, 0x4b, 0x4e, 0x51, 0x52, 0x55, 0x5b, 0x5e, 0x61, 0x62,
    0x65, 0x6b, 0x71, 0x72, 0x75, 0x7b, 0x83, 0x87, 0x8f, 0x93, 0xa3, 0xb3, 0xc3, 0xc7, 0xcc, 0xcd,
    0xcf, 0xd3, 0xdc, 0xdd, 0xe3, 0xec, 0xed, 0xf3, 0xfc, 0xfd,
];

/// Return the byte width of a documented base-MC6800 opcode.
#[must_use]
pub fn instruction_width(opcode: u8) -> Option<u8> {
    if INVALID_OPCODES.contains(&opcode) {
        return None;
    }
    Some(match opcode {
        0x20..=0x2f | 0x60..=0x6f => 2,
        0x70..=0x7f => 3,
        0x80..=0x8f | 0xc0..=0xcf => {
            if matches!(opcode, 0x8c | 0x8e | 0xce) {
                3
            } else {
                2
            }
        }
        0x90..=0x9f | 0xa0..=0xaf | 0xd0..=0xdf | 0xe0..=0xef => 2,
        0xb0..=0xbf | 0xf0..=0xff => 3,
        _ => 1,
    })
}

/// Complete, owned MC6800 architectural state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Motorola6800State {
    pub a: u8,
    pub b: u8,
    pub x: u16,
    pub sp: u16,
    pub pc: u16,
    pub cc: u8,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u16,
    pub loaded_len: usize,
}

impl Default for Motorola6800State {
    fn default() -> Self {
        Self {
            a: 0,
            b: 0,
            x: 0,
            sp: 0,
            pc: 0,
            cc: CC_RESERVED,
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

/// A single successful instruction transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub raw: Vec<u8>,
    pub width: u8,
    pub mnemonic: &'static str,
    pub description: String,
    pub state_after: Motorola6800State,
}

/// Result of a bounded successful execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: Motorola6800State,
}

/// Checked lifecycle and execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Motorola6800Error {
    EmptyProgram,
    ProgramOutOfRange { origin: u16, len: usize },
    FetchOutsideProgram { pc: u16 },
    TruncatedInstruction { pc: u16, needed: u8 },
    UnknownOpcode { opcode: u8, pc: u16 },
    InvalidState { reason: &'static str },
    Halted,
    StepLimitExceeded { limit: usize },
}

impl fmt::Display for Motorola6800Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => write!(f, "program image is empty"),
            Self::ProgramOutOfRange { origin, len } => {
                write!(f, "program of {len} bytes does not fit at ${origin:04X}")
            }
            Self::FetchOutsideProgram { pc } => write!(f, "fetch at ${pc:04X} is outside program"),
            Self::TruncatedInstruction { pc, needed } => {
                write!(f, "instruction at ${pc:04X} needs {needed} bytes")
            }
            Self::UnknownOpcode { opcode, pc } => {
                write!(f, "unknown opcode ${opcode:02X} at ${pc:04X}")
            }
            Self::InvalidState { reason } => write!(f, "invalid MC6800 state: {reason}"),
            Self::Halted => write!(f, "processor is waiting"),
            Self::StepLimitExceeded { limit } => write!(f, "step limit {limit} exceeded"),
        }
    }
}

impl std::error::Error for Motorola6800Error {}

/// Functional simulator for the original Motorola MC6800.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Motorola6800Simulator {
    state: Motorola6800State,
}

#[derive(Clone, Copy)]
enum Mode {
    Immediate,
    Direct,
    Indexed,
    Extended,
}

impl Motorola6800Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset registers, memory, and installed-image metadata.
    pub fn reset(&mut self) {
        self.state = Motorola6800State::default();
    }

    /// Install a contiguous program and reset the processor.
    pub fn load(&mut self, program: &[u8], origin: u16) -> Result<(), Motorola6800Error> {
        if program.is_empty() {
            return Err(Motorola6800Error::EmptyProgram);
        }
        let start = usize::from(origin);
        let Some(end) = start.checked_add(program.len()) else {
            return Err(Motorola6800Error::ProgramOutOfRange {
                origin,
                len: program.len(),
            });
        };
        if end > MEMORY_SIZE {
            return Err(Motorola6800Error::ProgramOutOfRange {
                origin,
                len: program.len(),
            });
        }
        let mut state = Motorola6800State {
            pc: origin,
            loaded_origin: origin,
            loaded_len: program.len(),
            ..Motorola6800State::default()
        };
        state.memory[start..end].copy_from_slice(program);
        self.state = state;
        Ok(())
    }

    /// Replace the complete state after validating its owned memory and image range.
    pub fn restore(&mut self, mut state: Motorola6800State) -> Result<(), Motorola6800Error> {
        if state.memory.len() != MEMORY_SIZE {
            return Err(Motorola6800Error::InvalidState {
                reason: "memory must contain exactly 65536 bytes",
            });
        }
        let end = usize::from(state.loaded_origin).checked_add(state.loaded_len);
        if state.loaded_len == 0 || end.is_none_or(|value| value > MEMORY_SIZE) {
            return Err(Motorola6800Error::InvalidState {
                reason: "installed program range is invalid",
            });
        }
        state.cc |= CC_RESERVED;
        self.state = state;
        Ok(())
    }

    #[must_use]
    pub fn get_state(&self) -> Motorola6800State {
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
    pub fn read_x(&self) -> u16 {
        self.state.x
    }
    #[must_use]
    pub fn read_sp(&self) -> u16 {
        self.state.sp
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
    pub fn write_x(&mut self, value: u16) {
        self.state.x = value;
    }
    pub fn write_sp(&mut self, value: u16) {
        self.state.sp = value;
    }
    pub fn write_cc(&mut self, value: u8) {
        self.state.cc = value | CC_RESERVED;
    }

    pub fn read_memory(&self, address: u16, len: usize) -> Result<Vec<u8>, Motorola6800Error> {
        let start = usize::from(address);
        let Some(end) = start.checked_add(len) else {
            return Err(Motorola6800Error::InvalidState {
                reason: "memory range wraps",
            });
        };
        if end > MEMORY_SIZE {
            return Err(Motorola6800Error::InvalidState {
                reason: "memory range wraps",
            });
        }
        Ok(self.state.memory[start..end].to_vec())
    }

    pub fn write_memory(&mut self, address: u16, bytes: &[u8]) -> Result<(), Motorola6800Error> {
        let start = usize::from(address);
        let Some(end) = start.checked_add(bytes.len()) else {
            return Err(Motorola6800Error::InvalidState {
                reason: "memory range wraps",
            });
        };
        if end > MEMORY_SIZE {
            return Err(Motorola6800Error::InvalidState {
                reason: "memory range wraps",
            });
        }
        self.state.memory[start..end].copy_from_slice(bytes);
        Ok(())
    }

    fn installed_contains(&self, address: u16) -> bool {
        let value = usize::from(address);
        let start = usize::from(self.state.loaded_origin);
        value >= start && value < start + self.state.loaded_len
    }

    fn byte(&self, address: u16) -> u8 {
        self.state.memory[usize::from(address)]
    }
    fn set_byte(&mut self, address: u16, value: u8) {
        self.state.memory[usize::from(address)] = value;
    }
    fn word(&self, address: u16) -> u16 {
        u16::from_be_bytes([self.byte(address), self.byte(address.wrapping_add(1))])
    }
    fn set_word(&mut self, address: u16, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.set_byte(address, high);
        self.set_byte(address.wrapping_add(1), low);
    }
    fn flag(&self, flag: u8) -> bool {
        self.state.cc & flag != 0
    }
    fn set_flag(&mut self, flag: u8, set: bool) {
        if set {
            self.state.cc |= flag;
        } else {
            self.state.cc &= !flag;
        }
        self.state.cc |= CC_RESERVED;
    }
    fn set_nz8(&mut self, value: u8) {
        self.set_flag(CC_N, value & 0x80 != 0);
        self.set_flag(CC_Z, value == 0);
    }
    fn set_nz16(&mut self, value: u16) {
        self.set_flag(CC_N, value & 0x8000 != 0);
        self.set_flag(CC_Z, value == 0);
    }
    fn logic8(&mut self, value: u8) {
        self.set_nz8(value);
        self.set_flag(CC_V, false);
    }
    fn add8(&mut self, left: u8, right: u8, carry: bool) -> u8 {
        let carry_in = u16::from(carry);
        let sum = u16::from(left) + u16::from(right) + carry_in;
        let result = sum as u8;
        self.set_nz8(result);
        self.set_flag(CC_C, sum > 0xff);
        self.set_flag(CC_H, (left & 0x0f) + (right & 0x0f) + carry_in as u8 > 0x0f);
        self.set_flag(CC_V, (!(left ^ right) & (left ^ result) & 0x80) != 0);
        result
    }
    fn sub8(&mut self, left: u8, right: u8, borrow: bool) -> u8 {
        let amount = u16::from(right) + u16::from(borrow);
        let result = left.wrapping_sub(amount as u8);
        self.set_nz8(result);
        self.set_flag(CC_C, u16::from(left) < amount);
        self.set_flag(CC_V, ((left ^ right) & (left ^ result) & 0x80) != 0);
        result
    }
    fn compare16(&mut self, left: u16, right: u16) {
        let result = left.wrapping_sub(right);
        self.set_nz16(result);
        self.set_flag(CC_V, ((left ^ right) & (left ^ result) & 0x8000) != 0);
    }
    fn push8(&mut self, value: u8) {
        self.set_byte(self.state.sp, value);
        self.state.sp = self.state.sp.wrapping_sub(1);
    }
    fn pull8(&mut self) -> u8 {
        self.state.sp = self.state.sp.wrapping_add(1);
        self.byte(self.state.sp)
    }
    fn push16(&mut self, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.push8(low);
        self.push8(high);
    }
    fn pull16(&mut self) -> u16 {
        let high = self.pull8();
        let low = self.pull8();
        u16::from_be_bytes([high, low])
    }
    fn push_frame(&mut self, pc: u16) {
        self.push16(pc);
        self.push16(self.state.x);
        self.push8(self.state.a);
        self.push8(self.state.b);
        self.push8(self.state.cc);
    }
    fn effective(&self, mode: Mode, raw: &[u8]) -> u16 {
        match mode {
            Mode::Direct => u16::from(raw[1]),
            Mode::Indexed => self.state.x.wrapping_add(u16::from(raw[1])),
            Mode::Extended => u16::from_be_bytes([raw[1], raw[2]]),
            Mode::Immediate => unreachable!("immediate operands have no effective address"),
        }
    }
    fn operand8(&self, mode: Mode, raw: &[u8]) -> u8 {
        match mode {
            Mode::Immediate => raw[1],
            _ => self.byte(self.effective(mode, raw)),
        }
    }
    fn operand16(&self, mode: Mode, raw: &[u8]) -> u16 {
        match mode {
            Mode::Immediate => u16::from_be_bytes([raw[1], raw[2]]),
            _ => self.word(self.effective(mode, raw)),
        }
    }

    /// Execute one instruction atomically.
    pub fn step(&mut self) -> Result<StepTrace, Motorola6800Error> {
        if self.state.halted {
            return Err(Motorola6800Error::Halted);
        }
        let pc = self.state.pc;
        if !self.installed_contains(pc) {
            return Err(Motorola6800Error::FetchOutsideProgram { pc });
        }
        let opcode = self.byte(pc);
        let Some(width) = instruction_width(opcode) else {
            return Err(Motorola6800Error::UnknownOpcode { opcode, pc });
        };
        for offset in 0..width {
            if !self.installed_contains(pc.wrapping_add(u16::from(offset))) {
                return Err(Motorola6800Error::TruncatedInstruction { pc, needed: width });
            }
        }
        let raw: Vec<u8> = (0..width)
            .map(|offset| self.byte(pc.wrapping_add(u16::from(offset))))
            .collect();
        let before = self.state.clone();
        self.state.pc = pc.wrapping_add(u16::from(width));
        let result = self.execute_opcode(opcode, &raw);
        let mnemonic = match result {
            Ok(value) => value,
            Err(error) => {
                self.state = before;
                return Err(error);
            }
        };
        Ok(StepTrace {
            pc_before: pc,
            pc_after: self.state.pc,
            raw,
            width,
            mnemonic,
            description: mnemonic.to_string(),
            state_after: self.state.clone(),
        })
    }

    /// Run until WAI or a transactional step limit failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Motorola6800Error> {
        let before = self.state.clone();
        let mut traces = Vec::new();
        while traces.len() < max_steps {
            match self.step() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: self.state.clone(),
                        });
                    }
                }
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
        }
        self.state = before;
        Err(Motorola6800Error::StepLimitExceeded { limit: max_steps })
    }

    /// Load and execute a program transactionally.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u16,
        max_steps: usize,
    ) -> Result<ExecutionResult, Motorola6800Error> {
        let before = self.state.clone();
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }

    fn execute_opcode(
        &mut self,
        opcode: u8,
        raw: &[u8],
    ) -> Result<&'static str, Motorola6800Error> {
        match opcode {
            0x01 => Ok("NOP"),
            0x06 => {
                self.state.cc = self.state.a | CC_RESERVED;
                Ok("TAP")
            }
            0x07 => {
                self.state.a = self.state.cc;
                Ok("TPA")
            }
            0x08 => {
                self.state.x = self.state.x.wrapping_add(1);
                self.set_flag(CC_Z, self.state.x == 0);
                Ok("INX")
            }
            0x09 => {
                self.state.x = self.state.x.wrapping_sub(1);
                self.set_flag(CC_Z, self.state.x == 0);
                Ok("DEX")
            }
            0x0a => {
                self.set_flag(CC_V, false);
                Ok("CLV")
            }
            0x0b => {
                self.set_flag(CC_V, true);
                Ok("SEV")
            }
            0x0c => {
                self.set_flag(CC_C, false);
                Ok("CLC")
            }
            0x0d => {
                self.set_flag(CC_C, true);
                Ok("SEC")
            }
            0x0e => {
                self.set_flag(CC_I, false);
                Ok("CLI")
            }
            0x0f => {
                self.set_flag(CC_I, true);
                Ok("SEI")
            }
            0x10 => {
                self.state.a = self.sub8(self.state.a, self.state.b, false);
                Ok("SBA")
            }
            0x11 => {
                self.sub8(self.state.a, self.state.b, false);
                Ok("CBA")
            }
            0x16 => {
                self.state.b = self.state.a;
                self.logic8(self.state.b);
                Ok("TAB")
            }
            0x17 => {
                self.state.a = self.state.b;
                self.logic8(self.state.a);
                Ok("TBA")
            }
            0x19 => {
                self.decimal_adjust();
                Ok("DAA")
            }
            0x1b => {
                self.state.a = self.add8(self.state.a, self.state.b, false);
                Ok("ABA")
            }
            0x20..=0x2f => {
                self.branch(opcode, raw[1] as i8);
                Ok(branch_mnemonic(opcode))
            }
            0x30 => {
                self.state.x = self.state.sp.wrapping_add(1);
                Ok("TSX")
            }
            0x31 => {
                self.state.sp = self.state.sp.wrapping_add(1);
                Ok("INS")
            }
            0x32 => {
                self.state.a = self.pull8();
                Ok("PULA")
            }
            0x33 => {
                self.state.b = self.pull8();
                Ok("PULB")
            }
            0x34 => {
                self.state.sp = self.state.sp.wrapping_sub(1);
                Ok("DES")
            }
            0x35 => {
                self.state.sp = self.state.x.wrapping_sub(1);
                Ok("TXS")
            }
            0x36 => {
                self.push8(self.state.a);
                Ok("PSHA")
            }
            0x37 => {
                self.push8(self.state.b);
                Ok("PSHB")
            }
            0x39 => {
                self.state.pc = self.pull16();
                Ok("RTS")
            }
            0x3b => {
                self.state.cc = self.pull8() | CC_RESERVED;
                self.state.b = self.pull8();
                self.state.a = self.pull8();
                self.state.x = self.pull16();
                self.state.pc = self.pull16();
                Ok("RTI")
            }
            0x3e => {
                let pc = self.state.pc;
                self.push_frame(pc);
                self.state.halted = true;
                Ok("WAI")
            }
            0x3f => {
                let pc = self.state.pc;
                self.push_frame(pc);
                self.set_flag(CC_I, true);
                self.state.pc = self.word(0xfffa);
                Ok("SWI")
            }
            0x40..=0x5f => self.accumulator_unary(opcode),
            0x60..=0x7f => self.memory_unary(opcode, raw),
            0x80..=0xbf => self.group_a(opcode, raw),
            0xc0..=0xff => self.group_b(opcode, raw),
            _ => Err(Motorola6800Error::UnknownOpcode {
                opcode,
                pc: self.state.pc.wrapping_sub(raw.len() as u16),
            }),
        }
    }

    fn decimal_adjust(&mut self) {
        let original = self.state.a;
        let mut correction = 0;
        if original & 0x0f > 9 || self.flag(CC_H) {
            correction |= 0x06;
        }
        if original > 0x99 || self.flag(CC_C) {
            correction |= 0x60;
        }
        let (result, carry) = original.overflowing_add(correction);
        self.state.a = result;
        self.set_nz8(result);
        self.set_flag(CC_C, carry || correction & 0x60 != 0);
    }

    fn branch(&mut self, opcode: u8, displacement: i8) {
        let n = self.flag(CC_N);
        let z = self.flag(CC_Z);
        let v = self.flag(CC_V);
        let c = self.flag(CC_C);
        let take = match opcode {
            0x20 => true,
            0x22 => !c && !z,
            0x23 => c || z,
            0x24 => !c,
            0x25 => c,
            0x26 => !z,
            0x27 => z,
            0x28 => !v,
            0x29 => v,
            0x2a => !n,
            0x2b => n,
            0x2c => n == v,
            0x2d => n != v,
            0x2e => !z && n == v,
            0x2f => z || n != v,
            _ => false,
        };
        if take {
            self.state.pc = self.state.pc.wrapping_add_signed(i16::from(displacement));
        }
    }

    fn unary_value(&mut self, operation: u8, value: u8) -> u8 {
        match operation {
            0x0 => {
                let result = 0_u8.wrapping_sub(value);
                self.set_nz8(result);
                self.set_flag(CC_V, value == 0x80);
                self.set_flag(CC_C, value != 0);
                result
            }
            0x3 => {
                let result = !value;
                self.logic8(result);
                self.set_flag(CC_C, true);
                result
            }
            0x4 => {
                self.set_flag(CC_C, value & 1 != 0);
                let result = value >> 1;
                self.set_flag(CC_N, false);
                self.set_flag(CC_Z, result == 0);
                self.set_flag(CC_V, self.flag(CC_C));
                result
            }
            0x6 => {
                let carry = self.flag(CC_C);
                self.set_flag(CC_C, value & 1 != 0);
                let result = (value >> 1) | if carry { 0x80 } else { 0 };
                self.set_nz8(result);
                self.set_flag(CC_V, self.flag(CC_N) ^ self.flag(CC_C));
                result
            }
            0x7 => {
                self.set_flag(CC_C, value & 1 != 0);
                let result = (value >> 1) | (value & 0x80);
                self.set_nz8(result);
                self.set_flag(CC_V, self.flag(CC_N) ^ self.flag(CC_C));
                result
            }
            0x8 => {
                self.set_flag(CC_C, value & 0x80 != 0);
                let result = value << 1;
                self.set_nz8(result);
                self.set_flag(CC_V, self.flag(CC_N) ^ self.flag(CC_C));
                result
            }
            0x9 => {
                let carry = self.flag(CC_C);
                self.set_flag(CC_C, value & 0x80 != 0);
                let result = (value << 1) | u8::from(carry);
                self.set_nz8(result);
                self.set_flag(CC_V, self.flag(CC_N) ^ self.flag(CC_C));
                result
            }
            0xa => {
                let result = value.wrapping_sub(1);
                self.set_nz8(result);
                self.set_flag(CC_V, value == 0x80);
                result
            }
            0xc => {
                let result = value.wrapping_add(1);
                self.set_nz8(result);
                self.set_flag(CC_V, value == 0x7f);
                result
            }
            0xd => {
                self.set_nz8(value);
                self.set_flag(CC_V, false);
                value
            }
            0xf => {
                self.set_flag(CC_N, false);
                self.set_flag(CC_Z, true);
                self.set_flag(CC_V, false);
                self.set_flag(CC_C, false);
                0
            }
            _ => value,
        }
    }

    fn accumulator_unary(&mut self, opcode: u8) -> Result<&'static str, Motorola6800Error> {
        let operation = opcode & 0x0f;
        let use_b = opcode & 0x10 != 0;
        let value = if use_b { self.state.b } else { self.state.a };
        let result = self.unary_value(operation, value);
        if operation != 0xd {
            if use_b {
                self.state.b = result;
            } else {
                self.state.a = result;
            }
        }
        Ok(unary_mnemonic(operation, use_b))
    }

    fn memory_unary(&mut self, opcode: u8, raw: &[u8]) -> Result<&'static str, Motorola6800Error> {
        let operation = opcode & 0x0f;
        let mode = if opcode < 0x70 {
            Mode::Indexed
        } else {
            Mode::Extended
        };
        let address = self.effective(mode, raw);
        if operation == 0x0e {
            self.state.pc = address;
            return Ok("JMP");
        }
        let value = self.byte(address);
        let result = self.unary_value(operation, value);
        if operation != 0x0d {
            self.set_byte(address, result);
        }
        Ok(memory_unary_mnemonic(operation))
    }

    fn group_mode(opcode: u8) -> Mode {
        match opcode >> 4 {
            0x8 | 0xc => Mode::Immediate,
            0x9 | 0xd => Mode::Direct,
            0xa | 0xe => Mode::Indexed,
            _ => Mode::Extended,
        }
    }

    fn group_a(&mut self, opcode: u8, raw: &[u8]) -> Result<&'static str, Motorola6800Error> {
        let mode = Self::group_mode(opcode);
        let low = opcode & 0x0f;
        match low {
            0x0 => {
                let value = self.operand8(mode, raw);
                self.state.a = self.sub8(self.state.a, value, false);
                Ok("SUBA")
            }
            0x1 => {
                let value = self.operand8(mode, raw);
                self.sub8(self.state.a, value, false);
                Ok("CMPA")
            }
            0x2 => {
                let value = self.operand8(mode, raw);
                self.state.a = self.sub8(self.state.a, value, self.flag(CC_C));
                Ok("SBCA")
            }
            0x4 => {
                self.state.a &= self.operand8(mode, raw);
                self.logic8(self.state.a);
                Ok("ANDA")
            }
            0x5 => {
                let value = self.state.a & self.operand8(mode, raw);
                self.logic8(value);
                Ok("BITA")
            }
            0x6 => {
                self.state.a = self.operand8(mode, raw);
                self.logic8(self.state.a);
                Ok("LDAA")
            }
            0x7 => {
                let address = self.effective(mode, raw);
                self.set_byte(address, self.state.a);
                self.logic8(self.state.a);
                Ok("STAA")
            }
            0x8 => {
                self.state.a ^= self.operand8(mode, raw);
                self.logic8(self.state.a);
                Ok("EORA")
            }
            0x9 => {
                let value = self.operand8(mode, raw);
                self.state.a = self.add8(self.state.a, value, self.flag(CC_C));
                Ok("ADCA")
            }
            0xa => {
                self.state.a |= self.operand8(mode, raw);
                self.logic8(self.state.a);
                Ok("ORAA")
            }
            0xb => {
                let value = self.operand8(mode, raw);
                self.state.a = self.add8(self.state.a, value, false);
                Ok("ADDA")
            }
            0xc => {
                let value = self.operand16(mode, raw);
                self.compare16(self.state.x, value);
                Ok("CPX")
            }
            0xd => {
                let target = if opcode == 0x8d {
                    self.state.pc.wrapping_add_signed(i16::from(raw[1] as i8))
                } else {
                    self.effective(mode, raw)
                };
                self.push16(self.state.pc);
                self.state.pc = target;
                Ok(if opcode == 0x8d { "BSR" } else { "JSR" })
            }
            0xe => {
                self.state.sp = self.operand16(mode, raw);
                self.set_nz16(self.state.sp);
                self.set_flag(CC_V, false);
                Ok("LDS")
            }
            0xf => {
                let address = self.effective(mode, raw);
                self.set_word(address, self.state.sp);
                self.set_nz16(self.state.sp);
                self.set_flag(CC_V, false);
                Ok("STS")
            }
            _ => Err(Motorola6800Error::UnknownOpcode {
                opcode,
                pc: self.state.pc.wrapping_sub(raw.len() as u16),
            }),
        }
    }

    fn group_b(&mut self, opcode: u8, raw: &[u8]) -> Result<&'static str, Motorola6800Error> {
        let mode = Self::group_mode(opcode);
        let low = opcode & 0x0f;
        match low {
            0x0 => {
                let value = self.operand8(mode, raw);
                self.state.b = self.sub8(self.state.b, value, false);
                Ok("SUBB")
            }
            0x1 => {
                let value = self.operand8(mode, raw);
                self.sub8(self.state.b, value, false);
                Ok("CMPB")
            }
            0x2 => {
                let value = self.operand8(mode, raw);
                self.state.b = self.sub8(self.state.b, value, self.flag(CC_C));
                Ok("SBCB")
            }
            0x4 => {
                self.state.b &= self.operand8(mode, raw);
                self.logic8(self.state.b);
                Ok("ANDB")
            }
            0x5 => {
                let value = self.state.b & self.operand8(mode, raw);
                self.logic8(value);
                Ok("BITB")
            }
            0x6 => {
                self.state.b = self.operand8(mode, raw);
                self.logic8(self.state.b);
                Ok("LDAB")
            }
            0x7 => {
                let address = self.effective(mode, raw);
                self.set_byte(address, self.state.b);
                self.logic8(self.state.b);
                Ok("STAB")
            }
            0x8 => {
                self.state.b ^= self.operand8(mode, raw);
                self.logic8(self.state.b);
                Ok("EORB")
            }
            0x9 => {
                let value = self.operand8(mode, raw);
                self.state.b = self.add8(self.state.b, value, self.flag(CC_C));
                Ok("ADCB")
            }
            0xa => {
                self.state.b |= self.operand8(mode, raw);
                self.logic8(self.state.b);
                Ok("ORAB")
            }
            0xb => {
                let value = self.operand8(mode, raw);
                self.state.b = self.add8(self.state.b, value, false);
                Ok("ADDB")
            }
            0xe => {
                self.state.x = self.operand16(mode, raw);
                self.set_nz16(self.state.x);
                self.set_flag(CC_V, false);
                Ok("LDX")
            }
            0xf => {
                let address = self.effective(mode, raw);
                self.set_word(address, self.state.x);
                self.set_nz16(self.state.x);
                self.set_flag(CC_V, false);
                Ok("STX")
            }
            _ => Err(Motorola6800Error::UnknownOpcode {
                opcode,
                pc: self.state.pc.wrapping_sub(raw.len() as u16),
            }),
        }
    }
}

fn branch_mnemonic(opcode: u8) -> &'static str {
    match opcode {
        0x20 => "BRA",
        0x22 => "BHI",
        0x23 => "BLS",
        0x24 => "BCC",
        0x25 => "BCS",
        0x26 => "BNE",
        0x27 => "BEQ",
        0x28 => "BVC",
        0x29 => "BVS",
        0x2a => "BPL",
        0x2b => "BMI",
        0x2c => "BGE",
        0x2d => "BLT",
        0x2e => "BGT",
        _ => "BLE",
    }
}

fn unary_mnemonic(operation: u8, use_b: bool) -> &'static str {
    match (operation, use_b) {
        (0x0, false) => "NEGA",
        (0x0, true) => "NEGB",
        (0x3, false) => "COMA",
        (0x3, true) => "COMB",
        (0x4, false) => "LSRA",
        (0x4, true) => "LSRB",
        (0x6, false) => "RORA",
        (0x6, true) => "RORB",
        (0x7, false) => "ASRA",
        (0x7, true) => "ASRB",
        (0x8, false) => "ASLA",
        (0x8, true) => "ASLB",
        (0x9, false) => "ROLA",
        (0x9, true) => "ROLB",
        (0xa, false) => "DECA",
        (0xa, true) => "DECB",
        (0xc, false) => "INCA",
        (0xc, true) => "INCB",
        (0xd, false) => "TSTA",
        (0xd, true) => "TSTB",
        (0xf, false) => "CLRA",
        (0xf, true) => "CLRB",
        _ => "CLR",
    }
}

fn memory_unary_mnemonic(operation: u8) -> &'static str {
    match operation {
        0x0 => "NEG",
        0x3 => "COM",
        0x4 => "LSR",
        0x6 => "ROR",
        0x7 => "ASR",
        0x8 => "ASL",
        0x9 => "ROL",
        0xa => "DEC",
        0xc => "INC",
        0xd => "TST",
        _ => "CLR",
    }
}
