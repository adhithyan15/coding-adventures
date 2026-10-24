//! Programmer-visible System/360 state and checked instruction execution.
//!
//! The original machine separated architecture from implementation. This file
//! deliberately stays on the architecture side of that boundary; the paired
//! gate-level crate owns physical structure and timing.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const MEMORY_BYTES: usize = 1 << 20;
const ADDRESS_MASK: u32 = 0x00ff_ffff;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct System360State {
    pub gprs: [u32; 16],
    /// Raw long hexadecimal-floating values for architectural FPRs 0, 2, 4, 6.
    pub fprs: [u64; 4],
    pub pc: u32,
    pub condition_code: u8,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: Vec<u8>,
    pub width: u8,
    pub mnemonic: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: System360State,
    pub traces: Vec<StepTrace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum System360Error {
    EmptyProgram,
    MisalignedProgram { origin: u32 },
    ProgramOutOfRange { origin: u32, bytes: usize },
    FetchOutsideProgram { pc: u32 },
    TruncatedInstruction { pc: u32, needed: usize },
    UnknownOpcode { opcode: u8, pc: u32 },
    UnsupportedSupervisorCall { number: u8 },
    MisalignedAccess { address: u32, alignment: usize },
    MemoryOutOfRange { address: u32, bytes: usize },
    InvalidRegister { bank: &'static str, index: usize },
    InvalidState { reason: &'static str },
    Specification { reason: &'static str },
    FixedPointOverflow,
    FixedPointDivide,
    FloatingExponent,
    FloatingDivide,
    Halted,
    MaxStepsExceeded { max_steps: usize },
}

impl Display for System360Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => write!(f, "System/360 program is empty"),
            Self::MisalignedProgram { origin } => {
                write!(f, "program origin {origin:#08x} is not halfword aligned")
            }
            Self::ProgramOutOfRange { origin, bytes } => write!(
                f,
                "program at {origin:#08x} with {bytes} bytes exceeds installed storage"
            ),
            Self::FetchOutsideProgram { pc } => write!(
                f,
                "instruction fetch at {pc:#08x} is outside the installed program"
            ),
            Self::TruncatedInstruction { pc, needed } => write!(
                f,
                "truncated instruction at {pc:#08x}: {needed} bytes required"
            ),
            Self::UnknownOpcode { opcode, pc } => {
                write!(f, "unknown opcode {opcode:#04x} at {pc:#08x}")
            }
            Self::UnsupportedSupervisorCall { number } => {
                write!(f, "unsupported supervisor call {number}")
            }
            Self::MisalignedAccess { address, alignment } => write!(
                f,
                "storage address {address:#08x} is not aligned to {alignment} bytes"
            ),
            Self::MemoryOutOfRange { address, bytes } => write!(
                f,
                "storage range {address:#08x}..+{bytes} exceeds installed storage"
            ),
            Self::InvalidRegister { bank, index } => {
                write!(f, "{bank} register {index} is invalid")
            }
            Self::InvalidState { reason } => write!(f, "invalid System/360 state: {reason}"),
            Self::Specification { reason } => {
                write!(f, "System/360 specification exception: {reason}")
            }
            Self::FixedPointOverflow => write!(f, "System/360 fixed-point overflow"),
            Self::FixedPointDivide => write!(f, "System/360 fixed-point divide exception"),
            Self::FloatingExponent => write!(f, "System/360 floating-point exponent exception"),
            Self::FloatingDivide => write!(f, "System/360 floating-point divide exception"),
            Self::Halted => write!(f, "System/360 CPU is halted"),
            Self::MaxStepsExceeded { max_steps } => {
                write!(f, "execution exceeded {max_steps} steps")
            }
        }
    }
}

impl Error for System360Error {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct System360Simulator {
    state: System360State,
}

impl Default for System360Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl System360Simulator {
    pub fn new() -> Self {
        Self {
            state: System360State {
                gprs: [0; 16],
                fprs: [0; 4],
                pc: 0,
                condition_code: 0,
                memory: vec![0; MEMORY_BYTES],
                halted: false,
                loaded_origin: 0,
                loaded_len: 0,
            },
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
    pub fn get_state(&self) -> System360State {
        self.state.clone()
    }

    pub fn restore(&mut self, state: System360State) -> Result<(), System360Error> {
        if state.memory.len() != MEMORY_BYTES {
            return Err(System360Error::InvalidState {
                reason: "memory has the wrong installed size",
            });
        }
        if state.condition_code > 3 {
            return Err(System360Error::InvalidState {
                reason: "condition code is outside 0..3",
            });
        }
        if state.pc > ADDRESS_MASK || !state.pc.is_multiple_of(2) {
            return Err(System360Error::InvalidState {
                reason: "instruction address is invalid",
            });
        }
        let end = (state.loaded_origin as usize).checked_add(state.loaded_len);
        if state.loaded_origin > ADDRESS_MASK || end.is_none_or(|end| end > MEMORY_BYTES) {
            return Err(System360Error::InvalidState {
                reason: "installed program range is invalid",
            });
        }
        self.state = state;
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<usize, System360Error> {
        if program.is_empty() {
            return Err(System360Error::EmptyProgram);
        }
        if !origin.is_multiple_of(2) {
            return Err(System360Error::MisalignedProgram { origin });
        }
        let end = (origin as usize).checked_add(program.len()).ok_or(
            System360Error::ProgramOutOfRange {
                origin,
                bytes: program.len(),
            },
        )?;
        if origin > ADDRESS_MASK || end > MEMORY_BYTES {
            return Err(System360Error::ProgramOutOfRange {
                origin,
                bytes: program.len(),
            });
        }
        let mut next = Self::new();
        next.state.pc = origin;
        next.state.loaded_origin = origin;
        next.state.loaded_len = program.len();
        next.state.memory[origin as usize..end].copy_from_slice(program);
        *self = next;
        Ok(program.len())
    }

    pub fn read_gpr(&self, index: usize) -> Result<u32, System360Error> {
        self.state
            .gprs
            .get(index)
            .copied()
            .ok_or(System360Error::InvalidRegister {
                bank: "general",
                index,
            })
    }
    pub fn write_gpr(&mut self, index: usize, value: u32) -> Result<(), System360Error> {
        *self
            .state
            .gprs
            .get_mut(index)
            .ok_or(System360Error::InvalidRegister {
                bank: "general",
                index,
            })? = value;
        Ok(())
    }
    fn fpr_slot(index: usize) -> Result<usize, System360Error> {
        if index <= 6 && index.is_multiple_of(2) {
            Ok(index / 2)
        } else {
            Err(System360Error::InvalidRegister {
                bank: "floating-point",
                index,
            })
        }
    }
    pub fn read_fpr(&self, index: usize) -> Result<u64, System360Error> {
        Ok(self.state.fprs[Self::fpr_slot(index)?])
    }
    pub fn write_fpr(&mut self, index: usize, value: u64) -> Result<(), System360Error> {
        let slot = Self::fpr_slot(index)?;
        self.state.fprs[slot] = value;
        Ok(())
    }
    pub fn set_program_counter(&mut self, pc: u32) -> Result<(), System360Error> {
        if pc > ADDRESS_MASK || !pc.is_multiple_of(2) {
            return Err(System360Error::InvalidState {
                reason: "instruction address is invalid",
            });
        }
        self.state.pc = pc;
        Ok(())
    }

    fn range(address: u32, bytes: usize) -> Result<std::ops::Range<usize>, System360Error> {
        let end = (address as usize)
            .checked_add(bytes)
            .ok_or(System360Error::MemoryOutOfRange { address, bytes })?;
        if address > ADDRESS_MASK || end > MEMORY_BYTES {
            return Err(System360Error::MemoryOutOfRange { address, bytes });
        }
        Ok(address as usize..end)
    }
    fn aligned(
        address: u32,
        bytes: usize,
        alignment: usize,
    ) -> Result<std::ops::Range<usize>, System360Error> {
        if !(address as usize).is_multiple_of(alignment) {
            return Err(System360Error::MisalignedAccess { address, alignment });
        }
        Self::range(address, bytes)
    }
    pub fn read_memory(&self, address: u32, bytes: usize) -> Result<Vec<u8>, System360Error> {
        Ok(self.state.memory[Self::range(address, bytes)?].to_vec())
    }
    pub fn write_memory(&mut self, address: u32, data: &[u8]) -> Result<(), System360Error> {
        let range = Self::range(address, data.len())?;
        self.state.memory[range].copy_from_slice(data);
        Ok(())
    }
    fn read16(&self, a: u32) -> Result<u16, System360Error> {
        Ok(u16::from_be_bytes(
            self.state.memory[Self::aligned(a, 2, 2)?]
                .try_into()
                .expect("two bytes"),
        ))
    }
    fn read32(&self, a: u32) -> Result<u32, System360Error> {
        Ok(u32::from_be_bytes(
            self.state.memory[Self::aligned(a, 4, 4)?]
                .try_into()
                .expect("four bytes"),
        ))
    }
    fn read64(&self, a: u32) -> Result<u64, System360Error> {
        Ok(u64::from_be_bytes(
            self.state.memory[Self::aligned(a, 8, 8)?]
                .try_into()
                .expect("eight bytes"),
        ))
    }
    fn write16(&mut self, a: u32, v: u16) -> Result<(), System360Error> {
        let r = Self::aligned(a, 2, 2)?;
        self.state.memory[r].copy_from_slice(&v.to_be_bytes());
        Ok(())
    }
    fn write32(&mut self, a: u32, v: u32) -> Result<(), System360Error> {
        let r = Self::aligned(a, 4, 4)?;
        self.state.memory[r].copy_from_slice(&v.to_be_bytes());
        Ok(())
    }
    fn write64(&mut self, a: u32, v: u64) -> Result<(), System360Error> {
        let r = Self::aligned(a, 8, 8)?;
        self.state.memory[r].copy_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn fetch(&self) -> Result<Vec<u8>, System360Error> {
        let pc = self.state.pc;
        if !pc.is_multiple_of(2) {
            return Err(System360Error::InvalidState {
                reason: "instruction address is not halfword aligned",
            });
        }
        let start = pc as usize;
        let lo = self.state.loaded_origin as usize;
        let hi = lo + self.state.loaded_len;
        if start < lo || start >= hi {
            return Err(System360Error::FetchOutsideProgram { pc });
        }
        let opcode = self.state.memory[start];
        let width =
            instruction_width(opcode).ok_or(System360Error::UnknownOpcode { opcode, pc })? as usize;
        if start + width > hi {
            return Err(System360Error::TruncatedInstruction { pc, needed: width });
        }
        Ok(self.state.memory[start..start + width].to_vec())
    }

    pub fn step(&mut self) -> Result<StepTrace, System360Error> {
        if self.state.halted {
            return Err(System360Error::Halted);
        }
        let before = self.state.clone();
        match self.step_inner() {
            Ok(trace) => Ok(trace),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }
    fn step_inner(&mut self) -> Result<StepTrace, System360Error> {
        let raw = self.fetch()?;
        let pc_before = self.state.pc;
        let width = raw.len() as u8;
        self.state.pc = self.state.pc.wrapping_add(u32::from(width)) & ADDRESS_MASK;
        let (mnemonic, description) = match raw.len() {
            2 => self.exec_rr(&raw)?,
            4 => self.exec_four(&raw)?,
            6 => self.exec_ss(&raw)?,
            _ => unreachable!(),
        };
        Ok(StepTrace {
            pc_before,
            pc_after: self.state.pc,
            raw,
            width,
            mnemonic: mnemonic.into(),
            description,
        })
    }
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, System360Error> {
        let before = self.state.clone();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => {
                    traces.push(trace);
                    if self.state.halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            final_state: self.get_state(),
                            traces,
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
        Err(System360Error::MaxStepsExceeded { max_steps })
    }
    pub fn execute(
        mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, System360Error> {
        self.load(program, origin)?;
        self.run(max_steps)
    }

    fn ea_rx(&self, i: &[u8]) -> u32 {
        let x = usize::from(i[1] & 15);
        let b = usize::from(i[2] >> 4);
        let d = (u32::from(i[2] & 15) << 8) | u32::from(i[3]);
        (if x == 0 { 0 } else { self.state.gprs[x] })
            .wrapping_add(if b == 0 { 0 } else { self.state.gprs[b] })
            .wrapping_add(d)
            & ADDRESS_MASK
    }
    fn ea_base(i: &[u8], offset: usize, g: &[u32; 16]) -> u32 {
        let b = usize::from(i[offset] >> 4);
        let d = (u32::from(i[offset] & 15) << 8) | u32::from(i[offset + 1]);
        (if b == 0 { 0 } else { g[b] }).wrapping_add(d) & ADDRESS_MASK
    }
    fn signed_cc(&mut self, value: i32) {
        self.state.condition_code = ord(value.cmp(&0));
    }
    fn logical_cc(&mut self, value: u32) {
        self.state.condition_code = u8::from(value != 0);
    }
    fn add(&mut self, a: u32, b: u32) -> Result<u32, System360Error> {
        let v = (a as i32)
            .checked_add(b as i32)
            .ok_or(System360Error::FixedPointOverflow)?;
        self.signed_cc(v);
        Ok(v as u32)
    }
    fn sub(&mut self, a: u32, b: u32) -> Result<u32, System360Error> {
        let v = (a as i32)
            .checked_sub(b as i32)
            .ok_or(System360Error::FixedPointOverflow)?;
        self.signed_cc(v);
        Ok(v as u32)
    }
    fn pair(&self, r: usize) -> Result<(), System360Error> {
        if r >= 15 || !r.is_multiple_of(2) {
            Err(System360Error::Specification {
                reason: "register pair must start at an even register below R15",
            })
        } else {
            Ok(())
        }
    }
    fn divide(&mut self, r: usize, divisor: i32) -> Result<(), System360Error> {
        self.pair(r)?;
        if divisor == 0 {
            return Err(System360Error::FixedPointDivide);
        }
        let dividend =
            ((u64::from(self.state.gprs[r]) << 32) | u64::from(self.state.gprs[r + 1])) as i64;
        let q = i128::from(dividend) / i128::from(divisor);
        let rem = i128::from(dividend) % i128::from(divisor);
        if q < i128::from(i32::MIN) || q > i128::from(i32::MAX) {
            return Err(System360Error::FixedPointDivide);
        }
        self.state.gprs[r] = rem as i32 as u32;
        self.state.gprs[r + 1] = q as i32 as u32;
        Ok(())
    }
    fn branch(&mut self, mask: u8, target: u32) {
        if mask & (8 >> self.state.condition_code) != 0 {
            self.state.pc = target & ADDRESS_MASK;
        }
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn hexadecimal_float_edge_paths_are_deterministic() {
        let zero = Hfp::decode(0, false);
        let canonical_zero = Hfp {
            negative: false,
            exponent: 0,
            fraction: 0,
        };
        let positive = Hfp::decode(0x4110_0000_0000_0000, true);
        let negative = positive.sign(true);
        assert_eq!(zero.encode(true), Ok(0));
        assert_eq!(zero.cc(), 0);
        assert_eq!(negative.cc(), 1);
        assert_eq!(positive.compare(negative), 2);
        assert_eq!(zero.compare(zero), 0);
        assert_eq!(negative.compare(positive), 1);
        assert_eq!(Hfp::multiply(zero, positive, 14), Ok(canonical_zero));
        assert_eq!(Hfp::divide(zero, positive, 14), Ok(canonical_zero));
        assert_eq!(Hfp::halve(zero, 14), Ok(canonical_zero));
        assert_eq!(Hfp::add(zero, positive, 14), Ok(positive));
        assert!(matches!(
            Hfp::divide(positive, zero, 14),
            Err(System360Error::FloatingDivide)
        ));
        assert!(matches!(
            Hfp {
                exponent: 64,
                ..positive
            }
            .encode(true),
            Err(System360Error::FloatingExponent)
        ));
    }

    #[test]
    fn valid_state_and_program_counter_updates_round_trip() {
        let mut cpu = System360Simulator::new();
        cpu.set_program_counter(2).unwrap();
        let state = cpu.get_state();
        cpu.reset();
        cpu.restore(state.clone()).unwrap();
        assert_eq!(cpu.get_state(), state);
    }
}

fn ord(value: std::cmp::Ordering) -> u8 {
    match value {
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Less => 1,
        std::cmp::Ordering::Greater => 2,
    }
}
fn span(first: usize, last: usize) -> usize {
    if last >= first {
        last - first + 1
    } else {
        17 - first + last
    }
}
fn instruction_width(op: u8) -> Option<u8> {
    match op {
        0x05..=0x07 | 0x0a | 0x10..=0x3f => Some(2),
        0x40..=0x7f | 0x86..=0x98 => Some(4),
        0xd2 | 0xd4..=0xd7 => Some(6),
        _ => None,
    }
}

impl System360Simulator {
    fn exec_rr(&mut self, raw: &[u8]) -> Result<(&'static str, String), System360Error> {
        let op = raw[0];
        let r1 = usize::from(raw[1] >> 4);
        let r2 = usize::from(raw[1] & 15);
        let a = self.state.gprs[r1];
        let b = self.state.gprs[r2];
        let name = match op {
            0x0a => {
                if raw[1] != 0 {
                    return Err(System360Error::UnsupportedSupervisorCall { number: raw[1] });
                }
                self.state.halted = true;
                "SVC 0"
            }
            0x18 => {
                self.state.gprs[r1] = b;
                "LR"
            }
            0x12 => {
                self.state.gprs[r1] = b;
                self.signed_cc(b as i32);
                "LTR"
            }
            0x13 => {
                let v = (b as i32)
                    .checked_neg()
                    .ok_or(System360Error::FixedPointOverflow)?;
                self.state.gprs[r1] = v as u32;
                self.signed_cc(v);
                "LCR"
            }
            0x10 => {
                let v = (b as i32)
                    .checked_abs()
                    .ok_or(System360Error::FixedPointOverflow)?;
                self.state.gprs[r1] = v as u32;
                self.signed_cc(v);
                "LPR"
            }
            0x11 => {
                let v = if b as i32 > 0 { -(b as i32) } else { b as i32 };
                self.state.gprs[r1] = v as u32;
                self.signed_cc(v);
                "LNR"
            }
            0x1a => {
                self.state.gprs[r1] = self.add(a, b)?;
                "AR"
            }
            0x1b => {
                self.state.gprs[r1] = self.sub(a, b)?;
                "SR"
            }
            0x1e => {
                let (v, c) = a.overflowing_add(b);
                self.state.gprs[r1] = v;
                self.state.condition_code = (u8::from(c) << 1) | u8::from(v != 0);
                "ALR"
            }
            0x1f => {
                let (v, borrow) = a.overflowing_sub(b);
                self.state.gprs[r1] = v;
                self.state.condition_code = (u8::from(!borrow) << 1) | u8::from(v != 0);
                "SLR"
            }
            0x19 => {
                self.state.condition_code = ord((a as i32).cmp(&(b as i32)));
                "CR"
            }
            0x15 => {
                self.state.condition_code = ord(a.cmp(&b));
                "CLR"
            }
            0x14 => {
                self.state.gprs[r1] = a & b;
                self.logical_cc(self.state.gprs[r1]);
                "NR"
            }
            0x16 => {
                self.state.gprs[r1] = a | b;
                self.logical_cc(self.state.gprs[r1]);
                "OR"
            }
            0x17 => {
                self.state.gprs[r1] = a ^ b;
                self.logical_cc(self.state.gprs[r1]);
                "XR"
            }
            0x1c => {
                self.pair(r1)?;
                let v = i64::from(self.state.gprs[r1 + 1] as i32) * i64::from(b as i32);
                self.state.gprs[r1] = (v >> 32) as u32;
                self.state.gprs[r1 + 1] = v as u32;
                "MR"
            }
            0x1d => {
                self.divide(r1, b as i32)?;
                "DR"
            }
            0x07 => {
                self.branch(r1 as u8, b);
                "BCR"
            }
            0x05 => {
                let target = b & ADDRESS_MASK;
                self.state.gprs[r1] = self.state.pc;
                if r2 != 0 {
                    self.state.pc = target;
                }
                "BALR"
            }
            0x06 => {
                self.state.gprs[r1] = a.wrapping_sub(1);
                if self.state.gprs[r1] != 0 && r2 != 0 {
                    self.state.pc = b & ADDRESS_MASK;
                }
                "BCTR"
            }
            0x20..=0x3f => return self.exec_float_rr(op, r1, r2),
            _ => {
                return Err(System360Error::UnknownOpcode {
                    opcode: op,
                    pc: self.state.pc.wrapping_sub(2),
                })
            }
        };
        Ok((name, format!("{name} R{r1},R{r2}")))
    }

    fn exec_four(&mut self, raw: &[u8]) -> Result<(&'static str, String), System360Error> {
        match raw[0] {
            0x91..=0x97 => self.exec_si(raw),
            0x86..=0x90 | 0x98 => self.exec_rs(raw),
            _ => self.exec_rx(raw),
        }
    }

    fn exec_rx(&mut self, raw: &[u8]) -> Result<(&'static str, String), System360Error> {
        let op = raw[0];
        let r = usize::from(raw[1] >> 4);
        let ea = self.ea_rx(raw);
        let a = self.state.gprs[r];
        let name = match op {
            0x58 => {
                self.state.gprs[r] = self.read32(ea)?;
                "L"
            }
            0x48 => {
                self.state.gprs[r] = i32::from(self.read16(ea)? as i16) as u32;
                "LH"
            }
            0x50 => {
                self.write32(ea, a)?;
                "ST"
            }
            0x40 => {
                self.write16(ea, a as u16)?;
                "STH"
            }
            0x42 => {
                let range = Self::range(ea, 1)?;
                self.state.memory[range].copy_from_slice(&[a as u8]);
                "STC"
            }
            0x43 => {
                let byte = self.state.memory[Self::range(ea, 1)?][0];
                self.state.gprs[r] = (a & 0xffff_ff00) | u32::from(byte);
                "IC"
            }
            0x41 => {
                self.state.gprs[r] = ea;
                "LA"
            }
            0x5a => {
                let b = self.read32(ea)?;
                self.state.gprs[r] = self.add(a, b)?;
                "A"
            }
            0x4a => {
                let b = i32::from(self.read16(ea)? as i16) as u32;
                self.state.gprs[r] = self.add(a, b)?;
                "AH"
            }
            0x5b => {
                let b = self.read32(ea)?;
                self.state.gprs[r] = self.sub(a, b)?;
                "S"
            }
            0x4b => {
                let b = i32::from(self.read16(ea)? as i16) as u32;
                self.state.gprs[r] = self.sub(a, b)?;
                "SH"
            }
            0x5e => {
                let (v, c) = a.overflowing_add(self.read32(ea)?);
                self.state.gprs[r] = v;
                self.state.condition_code = (u8::from(c) << 1) | u8::from(v != 0);
                "AL"
            }
            0x5f => {
                let (v, b) = a.overflowing_sub(self.read32(ea)?);
                self.state.gprs[r] = v;
                self.state.condition_code = (u8::from(!b) << 1) | u8::from(v != 0);
                "SL"
            }
            0x59 => {
                self.state.condition_code = ord((a as i32).cmp(&(self.read32(ea)? as i32)));
                "C"
            }
            0x49 => {
                self.state.condition_code =
                    ord((a as i32).cmp(&i32::from(self.read16(ea)? as i16)));
                "CH"
            }
            0x55 => {
                self.state.condition_code = ord(a.cmp(&self.read32(ea)?));
                "CL"
            }
            0x54 => {
                self.state.gprs[r] = a & self.read32(ea)?;
                self.logical_cc(self.state.gprs[r]);
                "N"
            }
            0x56 => {
                self.state.gprs[r] = a | self.read32(ea)?;
                self.logical_cc(self.state.gprs[r]);
                "O"
            }
            0x57 => {
                self.state.gprs[r] = a ^ self.read32(ea)?;
                self.logical_cc(self.state.gprs[r]);
                "X"
            }
            0x5c => {
                self.pair(r)?;
                let v =
                    i64::from(self.state.gprs[r + 1] as i32) * i64::from(self.read32(ea)? as i32);
                self.state.gprs[r] = (v >> 32) as u32;
                self.state.gprs[r + 1] = v as u32;
                "M"
            }
            0x4c => {
                self.state.gprs[r] =
                    (i64::from(a as i32) * i64::from(self.read16(ea)? as i16)) as u32;
                "MH"
            }
            0x5d => {
                let d = self.read32(ea)? as i32;
                self.divide(r, d)?;
                "D"
            }
            0x47 => {
                self.branch(r as u8, ea);
                "BC"
            }
            0x45 => {
                self.state.gprs[r] = self.state.pc;
                self.state.pc = ea;
                "BAL"
            }
            0x46 => {
                self.state.gprs[r] = a.wrapping_sub(1);
                if self.state.gprs[r] != 0 {
                    self.state.pc = ea;
                }
                "BCT"
            }
            0x60..=0x7f => return self.exec_float_rx(op, r, ea),
            _ => {
                return Err(System360Error::UnknownOpcode {
                    opcode: op,
                    pc: self.state.pc.wrapping_sub(4),
                })
            }
        };
        Ok((name, format!("{name} R{r},{ea:#08x}")))
    }

    fn exec_rs(&mut self, raw: &[u8]) -> Result<(&'static str, String), System360Error> {
        let op = raw[0];
        let r1 = usize::from(raw[1] >> 4);
        let r3 = usize::from(raw[1] & 15);
        let ea = Self::ea_base(raw, 2, &self.state.gprs);
        let count = ea & 63;
        let name = match op {
            0x90 => {
                let n = span(r1, r3);
                Self::aligned(ea, n * 4, 4)?;
                for i in 0..n {
                    self.write32(ea + (i * 4) as u32, self.state.gprs[(r1 + i) & 15])?;
                }
                "STM"
            }
            0x98 => {
                let n = span(r1, r3);
                Self::aligned(ea, n * 4, 4)?;
                let values = (0..n)
                    .map(|i| self.read32(ea + (i * 4) as u32))
                    .collect::<Result<Vec<_>, _>>()?;
                for (i, v) in values.into_iter().enumerate() {
                    self.state.gprs[(r1 + i) & 15] = v;
                }
                "LM"
            }
            0x89 => {
                self.state.gprs[r1] = self.state.gprs[r1].checked_shl(count).unwrap_or(0);
                "SLL"
            }
            0x88 => {
                self.state.gprs[r1] = self.state.gprs[r1].checked_shr(count).unwrap_or(0);
                "SRL"
            }
            0x8a => {
                let source = self.state.gprs[r1] as i32;
                let v = source
                    .checked_shr(count)
                    .unwrap_or(if source < 0 { -1 } else { 0 });
                self.state.gprs[r1] = v as u32;
                self.signed_cc(v);
                "SRA"
            }
            0x8b => {
                let source = self.state.gprs[r1];
                let wide = i128::from(source as i32) << count;
                let sign = source & 0x8000_0000;
                let shifted = source.checked_shl(count).unwrap_or(0) & 0x7fff_ffff;
                let v = (sign | shifted) as i32;
                if !(i128::from(i32::MIN)..=i128::from(i32::MAX)).contains(&wide) {
                    return Err(System360Error::FixedPointOverflow);
                }
                self.state.gprs[r1] = v as u32;
                self.signed_cc(v);
                "SLA"
            }
            0x8c | 0x8d => {
                self.pair(r1)?;
                let pair =
                    (u64::from(self.state.gprs[r1]) << 32) | u64::from(self.state.gprs[r1 + 1]);
                let v = if op == 0x8c {
                    pair.wrapping_shr(count)
                } else {
                    pair.wrapping_shl(count)
                };
                self.state.gprs[r1] = (v >> 32) as u32;
                self.state.gprs[r1 + 1] = v as u32;
                if op == 0x8c {
                    "SRDL"
                } else {
                    "SLDL"
                }
            }
            0x8e => {
                self.pair(r1)?;
                let pair = ((u64::from(self.state.gprs[r1]) << 32)
                    | u64::from(self.state.gprs[r1 + 1])) as i64;
                let v = pair.wrapping_shr(count);
                self.state.gprs[r1] = (v >> 32) as u32;
                self.state.gprs[r1 + 1] = v as u32;
                self.state.condition_code = ord(v.cmp(&0));
                "SRDA"
            }
            0x8f => {
                self.pair(r1)?;
                let source =
                    (u64::from(self.state.gprs[r1]) << 32) | u64::from(self.state.gprs[r1 + 1]);
                let pair = source as i64;
                let wide = i128::from(pair) << count;
                let sign = source & 0x8000_0000_0000_0000;
                let shifted = source.wrapping_shl(count) & 0x7fff_ffff_ffff_ffff;
                let v = (sign | shifted) as i64;
                if !(i128::from(i64::MIN)..=i128::from(i64::MAX)).contains(&wide) {
                    return Err(System360Error::FixedPointOverflow);
                }
                self.state.gprs[r1] = (v >> 32) as u32;
                self.state.gprs[r1 + 1] = v as u32;
                self.state.condition_code = ord(v.cmp(&0));
                "SLDA"
            }
            0x86 | 0x87 => {
                let compare = if r3.is_multiple_of(2) { r3 + 1 } else { r3 };
                let first = self.state.gprs[r1];
                let increment = self.state.gprs[r3];
                let limit = self.state.gprs[compare];
                let v = first.wrapping_add(increment);
                self.state.gprs[r1] = v;
                let high = (v as i32) > limit as i32;
                if (op == 0x86 && high) || (op == 0x87 && !high) {
                    self.state.pc = ea;
                }
                if op == 0x86 {
                    "BXH"
                } else {
                    "BXLE"
                }
            }
            _ => {
                return Err(System360Error::UnknownOpcode {
                    opcode: op,
                    pc: self.state.pc.wrapping_sub(4),
                })
            }
        };
        Ok((name, format!("{name} R{r1},{ea:#08x}")))
    }

    fn exec_si(&mut self, raw: &[u8]) -> Result<(&'static str, String), System360Error> {
        let op = raw[0];
        let imm = raw[1];
        let ea = Self::ea_base(raw, 2, &self.state.gprs);
        let range = Self::range(ea, 1)?;
        let a = self.state.memory[range.clone()][0];
        let name = match op {
            0x92 => {
                self.state.memory[range].copy_from_slice(&[imm]);
                "MVI"
            }
            0x95 => {
                self.state.condition_code = ord(a.cmp(&imm));
                "CLI"
            }
            0x94 | 0x96 | 0x97 => {
                let v = if op == 0x94 {
                    a & imm
                } else if op == 0x96 {
                    a | imm
                } else {
                    a ^ imm
                };
                self.state.memory[range].copy_from_slice(&[v]);
                self.state.condition_code = u8::from(v != 0);
                if op == 0x94 {
                    "NI"
                } else if op == 0x96 {
                    "OI"
                } else {
                    "XI"
                }
            }
            0x91 => {
                let v = a & imm;
                self.state.condition_code = if v == 0 {
                    0
                } else if v == imm {
                    3
                } else {
                    1
                };
                "TM"
            }
            _ => {
                return Err(System360Error::UnknownOpcode {
                    opcode: op,
                    pc: self.state.pc.wrapping_sub(4),
                })
            }
        };
        Ok((name, format!("{name} {ea:#08x},{imm:#04x}")))
    }

    fn exec_ss(&mut self, raw: &[u8]) -> Result<(&'static str, String), System360Error> {
        let op = raw[0];
        let n = usize::from(raw[1]) + 1;
        let a = Self::ea_base(raw, 2, &self.state.gprs);
        let b = Self::ea_base(raw, 4, &self.state.gprs);
        Self::range(a, n)?;
        Self::range(b, n)?;
        let name = match op {
            0xd2 => {
                for i in 0..n {
                    self.state.memory[a as usize + i] = self.state.memory[b as usize + i];
                }
                "MVC"
            }
            0xd5 => {
                self.state.condition_code = ord(self.state.memory[a as usize..a as usize + n]
                    .cmp(&self.state.memory[b as usize..b as usize + n]));
                "CLC"
            }
            0xd4 | 0xd6 | 0xd7 => {
                let mut nonzero = false;
                for i in 0..n {
                    let x = self.state.memory[a as usize + i];
                    let y = self.state.memory[b as usize + i];
                    let v = if op == 0xd4 {
                        x & y
                    } else if op == 0xd6 {
                        x | y
                    } else {
                        x ^ y
                    };
                    self.state.memory[a as usize + i] = v;
                    nonzero |= v != 0;
                }
                self.state.condition_code = u8::from(nonzero);
                if op == 0xd4 {
                    "NC"
                } else if op == 0xd6 {
                    "OC"
                } else {
                    "XC"
                }
            }
            _ => {
                return Err(System360Error::UnknownOpcode {
                    opcode: op,
                    pc: self.state.pc.wrapping_sub(6),
                })
            }
        };
        Ok((name, format!("{name} {a:#08x},{b:#08x},{n}")))
    }
}

/// A raw hexadecimal floating value expressed as an integer fraction and a
/// radix-16 exponent. Integer intermediates retain all 56 architectural bits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Hfp {
    negative: bool,
    exponent: i16,
    fraction: u128,
}

impl Hfp {
    fn decode(raw: u64, long: bool) -> Self {
        Self {
            negative: raw >> 63 != 0,
            exponent: ((raw >> 56) & 0x7f) as i16 - 64,
            fraction: if long {
                u128::from(raw & 0x00ff_ffff_ffff_ffff)
            } else {
                u128::from((raw >> 32) & 0x00ff_ffff)
            },
        }
    }
    fn encode(self, long: bool) -> Result<u64, System360Error> {
        if self.fraction == 0 {
            return Ok(0);
        }
        if !(-64..=63).contains(&self.exponent) {
            return Err(System360Error::FloatingExponent);
        }
        let prefix = (u64::from(self.negative) << 63) | (((self.exponent + 64) as u64) << 56);
        Ok(if long {
            prefix | self.fraction as u64
        } else {
            prefix | ((self.fraction as u64) << 32)
        })
    }
    fn sign(self, negative: bool) -> Self {
        Self { negative, ..self }
    }
    fn cc(self) -> u8 {
        if self.fraction == 0 {
            0
        } else if self.negative {
            1
        } else {
            2
        }
    }
    fn signed(self) -> i128 {
        if self.negative {
            -(self.fraction as i128)
        } else {
            self.fraction as i128
        }
    }
    fn normalize(mut value: i128, mut exponent: i16, digits: u32) -> Result<Self, System360Error> {
        if value == 0 {
            return Ok(Self {
                negative: false,
                exponent: 0,
                fraction: 0,
            });
        }
        let negative = value < 0;
        if negative {
            value = value
                .checked_neg()
                .ok_or(System360Error::FloatingExponent)?;
        }
        let mut fraction = value as u128;
        let limit = 1_u128 << (digits * 4);
        let floor = 1_u128 << ((digits - 1) * 4);
        while fraction >= limit {
            fraction >>= 4;
            exponent += 1;
        }
        while fraction < floor {
            fraction <<= 4;
            exponent -= 1;
        }
        if !(-64..=63).contains(&exponent) {
            return Err(System360Error::FloatingExponent);
        }
        Ok(Self {
            negative,
            exponent,
            fraction,
        })
    }
    fn add(a: Self, b: Self, digits: u32) -> Result<Self, System360Error> {
        let exponent = a.exponent.max(b.exponent);
        let align = |v: Self| {
            let places = (exponent - v.exponent) as u32;
            if places >= 32 {
                0
            } else {
                v.signed() >> (places * 4)
            }
        };
        Self::normalize(align(a) + align(b), exponent, digits)
    }
    fn halve(v: Self, digits: u32) -> Result<Self, System360Error> {
        Self::normalize(v.signed() >> 1, v.exponent, digits)
    }
    fn multiply(a: Self, b: Self, digits: u32) -> Result<Self, System360Error> {
        if a.fraction == 0 || b.fraction == 0 {
            return Self::normalize(0, 0, digits);
        }
        let fraction = a
            .fraction
            .checked_mul(b.fraction)
            .ok_or(System360Error::FloatingExponent)?
            >> (digits * 4);
        let signed = if a.negative ^ b.negative {
            -(fraction as i128)
        } else {
            fraction as i128
        };
        Self::normalize(signed, a.exponent + b.exponent, digits)
    }
    fn divide(a: Self, b: Self, digits: u32) -> Result<Self, System360Error> {
        if b.fraction == 0 {
            return Err(System360Error::FloatingDivide);
        }
        if a.fraction == 0 {
            return Self::normalize(0, 0, digits);
        }
        let numerator = a
            .fraction
            .checked_shl(digits * 4)
            .ok_or(System360Error::FloatingExponent)?;
        let fraction = numerator / b.fraction;
        let signed = if a.negative ^ b.negative {
            -(fraction as i128)
        } else {
            fraction as i128
        };
        Self::normalize(signed, a.exponent - b.exponent, digits)
    }
    fn compare(self, other: Self) -> u8 {
        if self.fraction == 0 && other.fraction == 0 {
            return 0;
        }
        if self.fraction == 0 {
            return if other.negative { 2 } else { 1 };
        }
        if other.fraction == 0 {
            return if self.negative { 1 } else { 2 };
        }
        if self.negative != other.negative {
            return if self.negative { 1 } else { 2 };
        }
        let normalized = |mut value: Self| {
            while value.fraction < (1_u128 << 124) {
                value.fraction <<= 4;
                value.exponent -= 1;
            }
            (value.exponent, value.fraction)
        };
        let ordering = normalized(self).cmp(&normalized(other));
        ord(if self.negative {
            ordering.reverse()
        } else {
            ordering
        })
    }
}

impl System360Simulator {
    fn store_float(&mut self, slot: usize, value: Hfp, long: bool) -> Result<(), System360Error> {
        let raw = value.encode(long)?;
        if long {
            self.state.fprs[slot] = raw;
        } else {
            self.state.fprs[slot] =
                (raw & 0xffff_ffff_0000_0000) | (self.state.fprs[slot] & 0xffff_ffff);
        }
        Ok(())
    }
    fn exec_float_rr(
        &mut self,
        op: u8,
        r1: usize,
        r2: usize,
    ) -> Result<(&'static str, String), System360Error> {
        let s1 = Self::fpr_slot(r1)?;
        let s2 = Self::fpr_slot(r2)?;
        let long = op < 0x30;
        let digits = if long { 14 } else { 6 };
        let a = Hfp::decode(self.state.fprs[s1], long);
        let b = Hfp::decode(self.state.fprs[s2], long);
        let name = match op {
            0x28 | 0x38 => {
                self.store_float(s1, b, long)?;
                if long {
                    "LDR"
                } else {
                    "LER"
                }
            }
            0x22 | 0x32 => {
                self.store_float(s1, b, long)?;
                self.state.condition_code = b.cc();
                if long {
                    "LTDR"
                } else {
                    "LTER"
                }
            }
            0x23 | 0x33 => {
                let v = b.sign(!b.negative);
                self.store_float(s1, v, long)?;
                self.state.condition_code = v.cc();
                if long {
                    "LCDR"
                } else {
                    "LCER"
                }
            }
            0x20 | 0x30 => {
                let v = b.sign(false);
                self.store_float(s1, v, long)?;
                self.state.condition_code = v.cc();
                if long {
                    "LPDR"
                } else {
                    "LPER"
                }
            }
            0x21 | 0x31 => {
                let v = b.sign(b.fraction != 0);
                self.store_float(s1, v, long)?;
                self.state.condition_code = v.cc();
                if long {
                    "LNDR"
                } else {
                    "LNER"
                }
            }
            0x2a | 0x3a => {
                let v = Hfp::add(a, b, digits)?;
                self.store_float(s1, v, long)?;
                self.state.condition_code = v.cc();
                if long {
                    "ADR"
                } else {
                    "AER"
                }
            }
            0x2b | 0x3b => {
                let v = Hfp::add(a, b.sign(!b.negative), digits)?;
                self.store_float(s1, v, long)?;
                self.state.condition_code = v.cc();
                if long {
                    "SDR"
                } else {
                    "SER"
                }
            }
            0x29 | 0x39 => {
                self.state.condition_code = a.compare(b);
                if long {
                    "CDR"
                } else {
                    "CER"
                }
            }
            0x24 | 0x34 => {
                let v = Hfp::halve(a, digits)?;
                self.store_float(s1, v, long)?;
                if long {
                    "HDR"
                } else {
                    "HER"
                }
            }
            0x2c | 0x3c => {
                let v = Hfp::multiply(a, b, digits)?;
                self.store_float(s1, v, long)?;
                if long {
                    "MDR"
                } else {
                    "MER"
                }
            }
            0x2d | 0x3d => {
                let v = Hfp::divide(a, b, digits)?;
                self.store_float(s1, v, long)?;
                if long {
                    "DDR"
                } else {
                    "DER"
                }
            }
            _ => {
                return Err(System360Error::UnknownOpcode {
                    opcode: op,
                    pc: self.state.pc.wrapping_sub(2),
                })
            }
        };
        Ok((name, format!("{name} F{r1},F{r2}")))
    }
    fn exec_float_rx(
        &mut self,
        op: u8,
        r: usize,
        ea: u32,
    ) -> Result<(&'static str, String), System360Error> {
        let slot = Self::fpr_slot(r)?;
        let long = op < 0x70;
        let digits = if long { 14 } else { 6 };
        let raw = if long {
            self.read64(ea)?
        } else {
            u64::from(self.read32(ea)?) << 32
        };
        let a = Hfp::decode(self.state.fprs[slot], long);
        let b = Hfp::decode(raw, long);
        let name = match op {
            0x68 | 0x78 => {
                self.store_float(slot, b, long)?;
                if long {
                    "LD"
                } else {
                    "LE"
                }
            }
            0x60 => {
                self.write64(ea, self.state.fprs[slot])?;
                "STD"
            }
            0x70 => {
                self.write32(ea, (self.state.fprs[slot] >> 32) as u32)?;
                "STE"
            }
            0x6a | 0x7a => {
                let v = Hfp::add(a, b, digits)?;
                self.store_float(slot, v, long)?;
                self.state.condition_code = v.cc();
                if long {
                    "AD"
                } else {
                    "AE"
                }
            }
            0x6b | 0x7b => {
                let v = Hfp::add(a, b.sign(!b.negative), digits)?;
                self.store_float(slot, v, long)?;
                self.state.condition_code = v.cc();
                if long {
                    "SD"
                } else {
                    "SE"
                }
            }
            0x69 | 0x79 => {
                self.state.condition_code = a.compare(b);
                if long {
                    "CD"
                } else {
                    "CE"
                }
            }
            0x6c | 0x7c => {
                let v = Hfp::multiply(a, b, digits)?;
                self.store_float(slot, v, long)?;
                if long {
                    "MD"
                } else {
                    "ME"
                }
            }
            0x6d | 0x7d => {
                let v = Hfp::divide(a, b, digits)?;
                self.store_float(slot, v, long)?;
                if long {
                    "DD"
                } else {
                    "DE"
                }
            }
            _ => {
                return Err(System360Error::UnknownOpcode {
                    opcode: op,
                    pc: self.state.pc.wrapping_sub(4),
                })
            }
        };
        Ok((name, format!("{name} F{r},{ea:#08x}")))
    }
}
