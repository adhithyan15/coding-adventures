//! A checked, bounded Microwatt/Power ISA scalar fixed-point simulator.
//!
//! The important split is between **transport** and **instruction bit
//! numbering**. Program bytes are little-endian, while fields in the resulting
//! `u32` follow Power ISA's most-significant-bit-first diagrams. The decoder
//! centralizes that translation so execution never guesses at byte order.

pub mod encoding;

use std::cmp::Ordering;
use std::fmt;

pub const MEMORY_SIZE: usize = 65_536;
pub const MAX_RETAINED_TRACES: usize = 64;
pub const TRACE_BUDGET_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub supervisor: bool,
    pub sf: bool,
    pub little_endian: bool,
    pub memory_size: usize,
}

/// Immutable execution profile shared by every simulator instance.
pub const PROFILE: Profile = Profile {
    supervisor: true,
    sf: true,
    little_endian: true,
    memory_size: MEMORY_SIZE,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Xer {
    pub so: bool,
    pub ov: bool,
    pub ca: bool,
    pub ov32: bool,
    pub ca32: bool,
}

impl Xer {
    pub const fn to_u64(self) -> u64 {
        ((self.so as u64) << 31)
            | ((self.ov as u64) << 30)
            | ((self.ca as u64) << 29)
            | ((self.ov32 as u64) << 19)
            | ((self.ca32 as u64) << 18)
    }

    pub const fn from_u64(value: u64) -> Self {
        Self {
            so: value & (1 << 31) != 0,
            ov: value & (1 << 30) != 0,
            ca: value & (1 << 29) != 0,
            ov32: value & (1 << 19) != 0,
            ca32: value & (1 << 18) != 0,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct State {
    pub gpr: [u64; 32],
    pub cia: u64,
    pub lr: u64,
    pub ctr: u64,
    pub cr: u32,
    pub xer: Xer,
    pub memory: [u8; MEMORY_SIZE],
    pub halted: bool,
    pub program_origin: Option<u16>,
    pub program_len: u32,
}

/// Profile-specific public state name used by Spec 07bd.
pub type MicrowattPowerIsaState = State;

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("gpr", &self.gpr)
            .field("cia", &self.cia)
            .field("lr", &self.lr)
            .field("ctr", &self.ctr)
            .field("cr", &format_args!("{:#010x}", self.cr))
            .field("xer", &self.xer)
            .field("memory_checksum", &memory_checksum(&self.memory))
            .field("halted", &self.halted)
            .field("program_origin", &self.program_origin)
            .field("program_len", &self.program_len)
            .finish()
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            gpr: [0; 32],
            cia: 0,
            lr: 0,
            ctr: 0,
            cr: 0,
            xer: Xer::default(),
            memory: [0; MEMORY_SIZE],
            halted: false,
            program_origin: None,
            program_len: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Halted,
    InvalidRegister {
        index: usize,
    },
    InvalidState(String),
    InvalidProgram(String),
    MisalignedFetch {
        cia: u64,
    },
    FetchOutsideProgram {
        cia: u64,
    },
    TruncatedInstruction {
        cia: u64,
        bytes: Vec<u8>,
    },
    IllegalInstruction {
        cia: u64,
        raw: u32,
    },
    UnsupportedInstruction {
        cia: u64,
        raw: u32,
        name: &'static str,
    },
    UnsupportedSpecialRegister {
        cia: u64,
        spr: u16,
    },
    MisalignedAccess {
        address: u64,
        width: usize,
    },
    MemoryOutOfRange {
        address: u64,
        width: usize,
    },
    ProgramOutOfRange {
        origin: u64,
        length: usize,
    },
    BranchOutsideProgram {
        cia: u64,
        target: u64,
    },
    DivideFault {
        cia: u64,
        signed: bool,
    },
    StepLimit {
        max_steps: usize,
    },
    AllocationFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Halted => f.write_str("CPU is halted"),
            Self::InvalidRegister { index } => write!(f, "register {index} is outside r0-r31"),
            Self::InvalidState(message) | Self::InvalidProgram(message) => f.write_str(message),
            Self::MisalignedFetch { cia } => write!(f, "CIA {cia:#018x} is not word-aligned"),
            Self::FetchOutsideProgram { cia } => {
                write!(f, "CIA {cia:#018x} is outside the installed program")
            }
            Self::TruncatedInstruction { cia, bytes } => write!(
                f,
                "instruction at {cia:#018x} has only {} installed bytes",
                bytes.len()
            ),
            Self::IllegalInstruction { cia, raw } => {
                write!(f, "illegal instruction {raw:#010x} at {cia:#018x}")
            }
            Self::UnsupportedInstruction { cia, raw, name } => write!(
                f,
                "unsupported instruction {name} ({raw:#010x}) at {cia:#018x}"
            ),
            Self::UnsupportedSpecialRegister { cia, spr } => {
                write!(f, "unsupported SPR {spr} at {cia:#018x}")
            }
            Self::MisalignedAccess { address, width } => {
                write!(f, "misaligned {width}-byte access at {address:#018x}")
            }
            Self::MemoryOutOfRange { address, width } => {
                write!(f, "{width}-byte access at {address:#018x} exceeds memory")
            }
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes at {origin:#018x} exceeds memory"
            ),
            Self::BranchOutsideProgram { cia, target } => write!(
                f,
                "branch at {cia:#018x} targets {target:#018x} outside the program"
            ),
            Self::DivideFault { cia, signed } => write!(
                f,
                "{} divide at {cia:#018x} has zero divisor or unrepresentable quotient",
                if *signed { "signed" } else { "unsigned" }
            ),
            Self::StepLimit { max_steps } => write!(f, "step limit {max_steps} exhausted"),
            Self::AllocationFailed => f.write_str("allocation preflight failed"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    Supported(&'static str),
    Unsupported(&'static str),
    Illegal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Load,
    Store,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryAccess {
    pub kind: MemoryKind,
    pub address: u64,
    pub width: u8,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialRegister {
    Lr,
    Ctr,
    Cr,
    Xer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecialWrite {
    pub register: SpecialRegister,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub cia_before: u64,
    pub cia_after: u64,
    pub bytes: [u8; 4],
    pub raw: u32,
    pub width: u8,
    pub mnemonic: String,
    pub branch_taken: Option<bool>,
    pub branch_target: Option<u64>,
    pub memory_access: Option<MemoryAccess>,
    pub gpr_writes: Vec<(u8, u64)>,
    pub special_writes: Vec<SpecialWrite>,
    pub state_before: State,
    pub state_after: State,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: State,
}

#[derive(Debug, Clone, Default)]
pub struct MicrowattPowerIsaSimulator {
    state: State,
}

#[derive(Default)]
struct Outcome {
    mnemonic: String,
    branch_taken: Option<bool>,
    branch_target: Option<u64>,
    memory_access: Option<MemoryAccess>,
}

impl MicrowattPowerIsaSimulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = State::default();
    }

    pub fn get_state(&self) -> State {
        self.state.clone()
    }

    pub fn restore(&mut self, state: &State) -> Result<(), Error> {
        validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    pub fn load_program(&mut self, bytes: &[u8], origin: u64) -> Result<(), Error> {
        if bytes.is_empty() {
            return Err(Error::InvalidProgram("program must not be empty".into()));
        }
        if origin & 3 != 0 {
            return Err(Error::InvalidProgram(format!(
                "program origin {origin:#x} is not word-aligned"
            )));
        }
        let start = usize::try_from(origin).map_err(|_| Error::ProgramOutOfRange {
            origin,
            length: bytes.len(),
        })?;
        let end = start
            .checked_add(bytes.len())
            .filter(|end| *end <= MEMORY_SIZE)
            .ok_or(Error::ProgramOutOfRange {
                origin,
                length: bytes.len(),
            })?;
        let mut candidate = State::default();
        candidate.memory[start..end].copy_from_slice(bytes);
        candidate.cia = origin;
        candidate.program_origin = Some(origin as u16);
        candidate.program_len = bytes.len() as u32;
        self.state = candidate;
        Ok(())
    }

    pub fn read_register(&self, index: usize) -> Result<u64, Error> {
        self.state
            .gpr
            .get(index)
            .copied()
            .ok_or(Error::InvalidRegister { index })
    }

    pub fn write_register(&mut self, index: usize, value: u64) -> Result<(), Error> {
        let register = self
            .state
            .gpr
            .get_mut(index)
            .ok_or(Error::InvalidRegister { index })?;
        *register = value;
        Ok(())
    }

    pub fn read_byte(&self, address: u64) -> Result<u8, Error> {
        Ok(self.state.memory[checked_range(address, 1)?])
    }

    pub fn write_byte(&mut self, address: u64, value: u8) -> Result<(), Error> {
        let index = checked_range(address, 1)?;
        self.state.memory[index] = value;
        Ok(())
    }

    pub fn read_half(&self, address: u64) -> Result<u16, Error> {
        let start = checked_aligned_range(address, 2)?;
        Ok(u16::from_le_bytes(
            self.state.memory[start..start + 2].try_into().unwrap(),
        ))
    }

    pub fn write_half(&mut self, address: u64, value: u16) -> Result<(), Error> {
        let start = checked_aligned_range(address, 2)?;
        self.state.memory[start..start + 2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn read_word(&self, address: u64) -> Result<u32, Error> {
        let start = checked_aligned_range(address, 4)?;
        Ok(u32::from_le_bytes(
            self.state.memory[start..start + 4].try_into().unwrap(),
        ))
    }

    pub fn write_word(&mut self, address: u64, value: u32) -> Result<(), Error> {
        let start = checked_aligned_range(address, 4)?;
        self.state.memory[start..start + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn read_double(&self, address: u64) -> Result<u64, Error> {
        let start = checked_aligned_range(address, 8)?;
        Ok(u64::from_le_bytes(
            self.state.memory[start..start + 8].try_into().unwrap(),
        ))
    }

    pub fn write_double(&mut self, address: u64, value: u64) -> Result<(), Error> {
        let start = checked_aligned_range(address, 8)?;
        self.state.memory[start..start + 8].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn step(&mut self) -> Result<StepTrace, Error> {
        if self.state.halted {
            return Err(Error::Halted);
        }
        let bytes = self.fetch()?;
        let raw = u32::from_le_bytes(bytes);
        let cia = self.state.cia;
        match classify(raw) {
            Classification::Illegal => return Err(Error::IllegalInstruction { cia, raw }),
            Classification::Unsupported(name) => {
                return Err(Error::UnsupportedInstruction { cia, raw, name });
            }
            Classification::Supported(_) => {}
        }
        let before = self.state.clone();
        let mut candidate = before.clone();
        let outcome = execute(&mut candidate, raw)?;
        let after = candidate.clone();
        let (gpr_writes, special_writes) = effects(&before, &after)?;
        let trace = StepTrace {
            cia_before: cia,
            cia_after: after.cia,
            bytes,
            raw,
            width: 4,
            mnemonic: outcome.mnemonic,
            branch_taken: outcome.branch_taken,
            branch_target: outcome.branch_target,
            memory_access: outcome.memory_access,
            gpr_writes,
            special_writes,
            state_before: before,
            state_after: after,
        };
        self.state = candidate;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Error> {
        if max_steps == 0 {
            return Err(Error::InvalidProgram("max_steps must be positive".into()));
        }
        let before = self.state.clone();
        let per_trace = std::mem::size_of::<StepTrace>().max(1);
        let retained = max_steps
            .min(MAX_RETAINED_TRACES)
            .min(TRACE_BUDGET_BYTES / per_trace);
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(retained)
            .map_err(|_| Error::AllocationFailed)?;
        for step in 0..max_steps {
            match self.step() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    if step < retained {
                        traces.push(trace);
                    }
                    if halted {
                        return Ok(ExecutionResult {
                            halted,
                            steps: step + 1,
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
        Err(Error::StepLimit { max_steps })
    }

    pub fn execute(
        &mut self,
        bytes: &[u8],
        origin: u64,
        max_steps: usize,
    ) -> Result<ExecutionResult, Error> {
        let before = self.state.clone();
        self.load_program(bytes, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }

    fn fetch(&self) -> Result<[u8; 4], Error> {
        let cia = self.state.cia;
        if cia & 3 != 0 {
            return Err(Error::MisalignedFetch { cia });
        }
        let Some(origin) = self.state.program_origin else {
            return Err(Error::FetchOutsideProgram { cia });
        };
        let start = u64::from(origin);
        let end = start + u64::from(self.state.program_len);
        if cia < start || cia >= end {
            return Err(Error::FetchOutsideProgram { cia });
        }
        let available = (end - cia).min(4) as usize;
        if available < 4 {
            let index = cia as usize;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(available)
                .map_err(|_| Error::AllocationFailed)?;
            bytes.extend_from_slice(&self.state.memory[index..index + available]);
            return Err(Error::TruncatedInstruction { cia, bytes });
        }
        let index = cia as usize;
        Ok(self.state.memory[index..index + 4].try_into().unwrap())
    }
}

/// Classify an encoded instruction without observing operands or mutating state.
pub fn classify(raw: u32) -> Classification {
    if raw == encoding::attn() {
        return Classification::Supported("attn");
    }
    let op = raw >> 26;
    let rt = (raw >> 21) & 31;
    let ra = (raw >> 16) & 31;
    match op {
        7 => Classification::Supported("mulli"),
        8 => Classification::Supported("subfic"),
        10 if raw & (1 << 22) == 0 => Classification::Supported("cmpli"),
        11 if raw & (1 << 22) == 0 => Classification::Supported("cmpi"),
        12 => Classification::Supported("addic"),
        13 => Classification::Supported("addic."),
        14 => Classification::Supported("addi"),
        15 => Classification::Supported("addis"),
        16 => Classification::Supported("bc"),
        18 => Classification::Supported("b"),
        19 => classify_19(raw),
        20 => Classification::Supported("rlwimi"),
        21 => Classification::Supported("rlwinm"),
        23 => Classification::Supported("rlwnm"),
        24 => Classification::Supported("ori"),
        25 => Classification::Supported("oris"),
        26 => Classification::Supported("xori"),
        27 => Classification::Supported("xoris"),
        28 => Classification::Supported("andi."),
        29 => Classification::Supported("andis."),
        30 => classify_30(raw),
        31 => classify_31(raw),
        32 => Classification::Supported("lwz"),
        33 if ra != 0 && ra != rt => Classification::Supported("lwzu"),
        34 => Classification::Supported("lbz"),
        35 if ra != 0 && ra != rt => Classification::Supported("lbzu"),
        36 => Classification::Supported("stw"),
        37 if ra != 0 => Classification::Supported("stwu"),
        38 => Classification::Supported("stb"),
        39 if ra != 0 => Classification::Supported("stbu"),
        40 => Classification::Supported("lhz"),
        41 if ra != 0 && ra != rt => Classification::Supported("lhzu"),
        42 => Classification::Supported("lha"),
        43 if ra != 0 && ra != rt => Classification::Supported("lhau"),
        44 => Classification::Supported("sth"),
        45 if ra != 0 => Classification::Supported("sthu"),
        58 => match raw & 3 {
            0 => Classification::Supported("ld"),
            1 if ra != 0 && ra != rt => Classification::Supported("ldu"),
            2 => Classification::Supported("lwa"),
            _ => Classification::Illegal,
        },
        62 => match raw & 3 {
            0 => Classification::Supported("std"),
            1 if ra != 0 => Classification::Supported("stdu"),
            _ => Classification::Illegal,
        },
        2 | 3 | 4 | 17 | 46..=57 | 59..=61 | 63 => {
            Classification::Unsupported("excluded Power ISA facility")
        }
        _ => Classification::Illegal,
    }
}

fn classify_19(raw: u32) -> Classification {
    let xo = (raw >> 1) & 0x3ff;
    let bt = (raw >> 21) & 31;
    let ba = (raw >> 16) & 31;
    let bb = (raw >> 11) & 31;
    match xo {
        0 if bt & 3 == 0 && ba & 3 == 0 && bb == 0 && raw & 1 == 0 => {
            Classification::Supported("mcrf")
        }
        16 if bb <= 3 => Classification::Supported("bclr"),
        33 if raw & 1 == 0 => Classification::Supported("crnor"),
        129 if raw & 1 == 0 => Classification::Supported("crandc"),
        150 if raw & 0x03ff_f801 == 0 => Classification::Supported("isync"),
        193 if raw & 1 == 0 => Classification::Supported("crxor"),
        225 if raw & 1 == 0 => Classification::Supported("crnand"),
        257 if raw & 1 == 0 => Classification::Supported("crand"),
        289 if raw & 1 == 0 => Classification::Supported("creqv"),
        417 if raw & 1 == 0 => Classification::Supported("crorc"),
        449 if raw & 1 == 0 => Classification::Supported("cror"),
        528 => {
            let bo = (raw >> 21) & 31;
            if bo & 4 == 0 || bb > 3 {
                Classification::Illegal
            } else {
                Classification::Supported("bcctr")
            }
        }
        18 | 50 | 274 | 306 | 370 | 402 | 566 => {
            Classification::Unsupported("excluded control facility")
        }
        _ => Classification::Illegal,
    }
}

fn classify_30(raw: u32) -> Classification {
    let xo3 = (raw >> 2) & 7;
    let xo4 = (raw >> 1) & 15;
    match (xo3, xo4) {
        (0, _) => Classification::Supported("rldicl"),
        (1, _) => Classification::Supported("rldicr"),
        (2, _) => Classification::Supported("rldic"),
        (3, _) => Classification::Supported("rldimi"),
        (_, 8) => Classification::Supported("rldcl"),
        (_, 9) => Classification::Supported("rldcr"),
        _ => Classification::Illegal,
    }
}

fn classify_31(raw: u32) -> Classification {
    let xo10 = (raw >> 1) & 0x3ff;
    let xo9 = xo10 & 0x1ff;
    let rt = (raw >> 21) & 31;
    let ra = (raw >> 16) & 31;
    if matches!(xo10, 53 | 55 | 119 | 311 | 373 | 375) && (ra == 0 || ra == rt) {
        return Classification::Illegal;
    }
    if matches!(xo10, 181 | 183 | 247 | 439) && ra == 0 {
        return Classification::Illegal;
    }
    if raw & 1 != 0
        && matches!(
            xo10,
            0 | 19
                | 21
                | 23
                | 32
                | 53
                | 55
                | 87
                | 119
                | 144
                | 149
                | 151
                | 181
                | 183
                | 215
                | 247
                | 279
                | 311
                | 339
                | 341
                | 343
                | 373
                | 375
                | 407
                | 439
                | 467
                | 532
                | 534
                | 598
                | 660
                | 662
                | 790
                | 854
                | 918
        )
    {
        return Classification::Illegal;
    }
    if matches!(
        xo9,
        8 | 10
            | 40
            | 104
            | 136
            | 138
            | 200
            | 202
            | 232
            | 233
            | 234
            | 235
            | 266
            | 457
            | 459
            | 489
            | 491
    ) {
        return Classification::Supported(match xo9 {
            8 => "subfc",
            10 => "addc",
            40 => "subf",
            104 => "neg",
            136 => "subfe",
            138 => "adde",
            200 => "subfze",
            202 => "addze",
            232 => "subfme",
            233 => "mulld",
            234 => "addme",
            235 => "mullw",
            266 => "add",
            457 => "divdu",
            459 => "divwu",
            489 => "divd",
            _ => "divw",
        });
    }
    match xo10 {
        0 if raw & ((1 << 22) | 1) == 0 => Classification::Supported("cmp"),
        9 => Classification::Supported("mulhdu"),
        11 => Classification::Supported("mulhwu"),
        19 => {
            if raw & (1 << 20) == 0 {
                Classification::Supported("mfcr")
            } else {
                Classification::Unsupported("mfocrf")
            }
        }
        21 => Classification::Supported("ldx"),
        23 => Classification::Supported("lwzx"),
        24 => Classification::Supported("slw"),
        26 => Classification::Supported("cntlzw"),
        27 => Classification::Supported("sld"),
        28 => Classification::Supported("and"),
        32 if raw & ((1 << 22) | 1) == 0 => Classification::Supported("cmpl"),
        53 => Classification::Supported("ldux"),
        55 => Classification::Supported("lwzux"),
        58 => Classification::Supported("cntlzd"),
        60 => Classification::Supported("andc"),
        73 => Classification::Supported("mulhd"),
        75 => Classification::Supported("mulhw"),
        87 => Classification::Supported("lbzx"),
        119 => Classification::Supported("lbzux"),
        122 => Classification::Supported("popcntb"),
        124 => Classification::Supported("nor"),
        144 => {
            if raw & (1 << 20) == 0 {
                Classification::Supported("mtcrf")
            } else {
                Classification::Unsupported("mtocrf")
            }
        }
        149 => Classification::Supported("stdx"),
        151 => Classification::Supported("stwx"),
        154 => Classification::Supported("prtyw"),
        181 => Classification::Supported("stdux"),
        183 => Classification::Supported("stwux"),
        186 => Classification::Supported("prtyd"),
        215 => Classification::Supported("stbx"),
        247 => Classification::Supported("stbux"),
        279 => Classification::Supported("lhzx"),
        284 => Classification::Supported("eqv"),
        311 => Classification::Supported("lhzux"),
        316 => Classification::Supported("xor"),
        339 => Classification::Supported("mfspr"),
        341 => Classification::Supported("lwax"),
        343 => Classification::Supported("lhax"),
        375 => Classification::Supported("lhaux"),
        373 => Classification::Supported("lwaux"),
        378 => Classification::Supported("popcntw"),
        407 => Classification::Supported("sthx"),
        412 => Classification::Supported("orc"),
        413 | 445 => Classification::Supported("sradi"),
        439 => Classification::Supported("sthux"),
        444 => Classification::Supported("or"),
        467 => Classification::Supported("mtspr"),
        476 => Classification::Supported("nand"),
        506 => Classification::Supported("popcntd"),
        532 => Classification::Supported("ldbrx"),
        534 => Classification::Supported("lwbrx"),
        536 => Classification::Supported("srw"),
        538 => Classification::Supported("cnttzw"),
        539 => Classification::Supported("srd"),
        570 => Classification::Supported("cnttzd"),
        598 if raw & 0x039f_f801 == 0 => match (raw >> 21) & 3 {
            0 => Classification::Supported("sync"),
            1 => Classification::Supported("lwsync"),
            2 => Classification::Unsupported("ptesync"),
            _ => Classification::Illegal,
        },
        660 => Classification::Supported("stdbrx"),
        662 => Classification::Supported("stwbrx"),
        790 => Classification::Supported("lhbrx"),
        792 => Classification::Supported("sraw"),
        794 => Classification::Supported("srad"),
        824 => Classification::Supported("srawi"),
        854 if raw & 0x03ff_f801 == 0 => Classification::Supported("eieio"),
        918 => Classification::Supported("sthbrx"),
        922 => Classification::Supported("extsh"),
        954 => Classification::Supported("extsb"),
        986 => Classification::Supported("extsw"),
        4 | 20 | 54 | 83 | 84 | 86 | 146 | 150 | 214 | 306 | 334 | 371 | 498 | 566 | 982 | 1014 => {
            Classification::Unsupported("excluded Power ISA facility")
        }
        _ => Classification::Illegal,
    }
}

fn execute(state: &mut State, raw: u32) -> Result<Outcome, Error> {
    let cia = state.cia;
    if raw == encoding::attn() {
        state.halted = true;
        state.cia = cia + 4;
        return Ok(simple_outcome("attn"));
    }
    let op = raw >> 26;
    let rt = ((raw >> 21) & 31) as usize;
    let ra = ((raw >> 16) & 31) as usize;
    let rb = ((raw >> 11) & 31) as usize;
    let ui = raw as u16;
    let si = ui as i16 as i64 as u64;
    let next = cia + 4;
    let mut outcome = Outcome::default();
    match op {
        7 => {
            state.gpr[rt] = (state.gpr[ra] as i64).wrapping_mul(ui as i16 as i64) as u64;
            outcome.mnemonic = "mulli".into();
        }
        8 => {
            let a = state.gpr[ra];
            let (value, ca, ca32) = add_carries(!a, si, true);
            state.gpr[rt] = value;
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            outcome.mnemonic = "subfic".into();
        }
        10 | 11 => {
            let field = ((raw >> 23) & 7) as usize;
            let is_64 = raw & (1 << 21) != 0;
            let relation = if op == 11 {
                if is_64 {
                    (state.gpr[ra] as i64).cmp(&(ui as i16 as i64))
                } else {
                    (state.gpr[ra] as u32 as i32).cmp(&(ui as i16 as i32))
                }
            } else if is_64 {
                state.gpr[ra].cmp(&u64::from(ui))
            } else {
                (state.gpr[ra] as u32).cmp(&u32::from(ui))
            };
            set_cr_field(state, field, relation);
            outcome.mnemonic = match (op, is_64) {
                (11, false) => "cmpwi",
                (11, true) => "cmpdi",
                (10, false) => "cmplwi",
                _ => "cmpldi",
            }
            .into();
        }
        12 | 13 => {
            let (value, ca, ca32) = add_carries(state.gpr[ra], si, false);
            state.gpr[rt] = value;
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            if op == 13 {
                update_cr0(state, value);
            }
            outcome.mnemonic = if op == 13 { "addic." } else { "addic" }.into();
        }
        14 | 15 => {
            let base = ra_or_zero(state, ra);
            let immediate = if op == 15 { si << 16 } else { si };
            state.gpr[rt] = base.wrapping_add(immediate);
            outcome.mnemonic = if op == 14 { "addi" } else { "addis" }.into();
        }
        16 => return execute_bc(state, raw),
        18 => return execute_b(state, raw),
        19 => return execute_19(state, raw),
        20 | 21 | 23 => {
            let rs = state.gpr[rt] as u32;
            let sh = if op == 23 {
                state.gpr[rb] as u32 & 31
            } else {
                (raw >> 11) & 31
            };
            let mb = (raw >> 6) & 31;
            let me = (raw >> 1) & 31;
            let rotated = rs.rotate_left(sh);
            let mask = mask32(mb, me);
            let value = if op == 20 {
                (state.gpr[ra] as u32 & !mask) | (rotated & mask)
            } else {
                rotated & mask
            };
            state.gpr[ra] = u64::from(value);
            if raw & 1 != 0 {
                update_cr0(state, u64::from(value));
            }
            outcome.mnemonic = record_name(
                if op == 20 {
                    "rlwimi"
                } else if op == 21 {
                    "rlwinm"
                } else {
                    "rlwnm"
                },
                false,
                raw & 1 != 0,
            )?;
        }
        24..=29 => {
            let source = state.gpr[rt];
            let immediate = if matches!(op, 25 | 27 | 29) {
                u64::from(ui) << 16
            } else {
                u64::from(ui)
            };
            let value = match op {
                24 | 25 => source | immediate,
                26 | 27 => source ^ immediate,
                _ => source & immediate,
            };
            state.gpr[ra] = value;
            if matches!(op, 28 | 29) {
                update_cr0(state, value);
            }
            outcome.mnemonic = match op {
                24 if raw == 0x6000_0000 => "nop",
                24 => "ori",
                25 => "oris",
                26 => "xori",
                27 => "xoris",
                28 => "andi.",
                _ => "andis.",
            }
            .into();
        }
        30 => return execute_30(state, raw),
        31 => return execute_31(state, raw),
        32..=45 | 58 | 62 => return execute_displacement_memory(state, raw),
        _ => return Err(Error::IllegalInstruction { cia, raw }),
    }
    state.cia = next;
    Ok(outcome)
}

fn execute_b(state: &mut State, raw: u32) -> Result<Outcome, Error> {
    let cia = state.cia;
    let displacement = sign_extend(raw & 0x03ff_fffc, 26);
    let absolute = raw & 2 != 0;
    let link = raw & 1 != 0;
    let target = if absolute {
        displacement
    } else {
        cia.wrapping_add(displacement)
    };
    validate_branch(state, cia, target)?;
    if link {
        state.lr = cia + 4;
    }
    state.cia = target;
    Ok(Outcome {
        mnemonic: branch_name("b", absolute, link)?,
        branch_taken: Some(true),
        branch_target: Some(target),
        memory_access: None,
    })
}

fn execute_bc(state: &mut State, raw: u32) -> Result<Outcome, Error> {
    let cia = state.cia;
    let bo = (raw >> 21) & 31;
    let bi = (raw >> 16) & 31;
    let displacement = sign_extend(raw & 0xfffc, 16);
    let absolute = raw & 2 != 0;
    let link = raw & 1 != 0;
    let taken = branch_condition(state, bo, bi);
    let target = if taken {
        let target = if absolute {
            displacement
        } else {
            cia.wrapping_add(displacement)
        };
        validate_branch(state, cia, target)?;
        target
    } else {
        cia + 4
    };
    if link {
        state.lr = cia + 4;
    }
    state.cia = target;
    Ok(Outcome {
        mnemonic: branch_name("bc", absolute, link)?,
        branch_taken: Some(taken),
        branch_target: taken.then_some(target),
        memory_access: None,
    })
}

fn execute_19(state: &mut State, raw: u32) -> Result<Outcome, Error> {
    let cia = state.cia;
    let xo = (raw >> 1) & 0x3ff;
    let bt = ((raw >> 21) & 31) as usize;
    let ba = ((raw >> 16) & 31) as usize;
    let bb = ((raw >> 11) & 31) as usize;
    let mut outcome = Outcome::default();
    match xo {
        0 => {
            let bf = bt >> 2;
            let bfa = ba >> 2;
            let source = (state.cr >> (28 - bfa * 4)) & 0xf;
            state.cr = (state.cr & !(0xf << (28 - bf * 4))) | (source << (28 - bf * 4));
            outcome.mnemonic = "mcrf".into();
            state.cia = cia + 4;
        }
        16 | 528 => {
            let bo = bt as u32;
            let bi = ba as u32;
            let link = raw & 1 != 0;
            let source_target = if xo == 16 { state.lr } else { state.ctr } & !3;
            let taken = branch_condition(state, bo, bi);
            if taken {
                validate_branch(state, cia, source_target)?;
            }
            if link {
                state.lr = cia + 4;
            }
            state.cia = if taken { source_target } else { cia + 4 };
            outcome.mnemonic = branch_name(if xo == 16 { "bclr" } else { "bcctr" }, false, link)?;
            outcome.branch_taken = Some(taken);
            outcome.branch_target = taken.then_some(source_target);
        }
        150 => {
            outcome.mnemonic = "isync".into();
            state.cia = cia + 4;
        }
        33 | 129 | 193 | 225 | 257 | 289 | 417 | 449 => {
            let a = cr_bit(state.cr, ba);
            let b = cr_bit(state.cr, bb);
            let value = match xo {
                33 => !(a | b),
                129 => a & !b,
                193 => a ^ b,
                225 => !(a & b),
                257 => a & b,
                289 => !(a ^ b),
                417 => a | !b,
                _ => a | b,
            };
            set_cr_bit(&mut state.cr, bt, value);
            outcome.mnemonic = match xo {
                33 => "crnor",
                129 => "crandc",
                193 => "crxor",
                225 => "crnand",
                257 => "crand",
                289 => "creqv",
                417 => "crorc",
                _ => "cror",
            }
            .into();
            state.cia = cia + 4;
        }
        _ => return Err(Error::IllegalInstruction { cia, raw }),
    }
    Ok(outcome)
}

fn execute_30(state: &mut State, raw: u32) -> Result<Outcome, Error> {
    let cia = state.cia;
    let rs = ((raw >> 21) & 31) as usize;
    let ra = ((raw >> 16) & 31) as usize;
    let rb = ((raw >> 11) & 31) as usize;
    let xo3 = (raw >> 2) & 7;
    let xo4 = (raw >> 1) & 15;
    let sh = ((raw >> 11) & 31) | ((raw & 2) << 4);
    let mb = ((raw >> 6) & 31) | (raw & 0x20);
    let (base, shift, mask, insert) = match (xo3, xo4) {
        (0, _) => ("rldicl", sh, mask64(mb, 63), false),
        (1, _) => ("rldicr", sh, mask64(0, mb), false),
        (2, _) => ("rldic", sh, mask64(mb, 63 - sh), false),
        (3, _) => ("rldimi", sh, mask64(mb, 63 - sh), true),
        (_, 8) => ("rldcl", state.gpr[rb] as u32 & 63, mask64(mb, 63), false),
        (_, 9) => ("rldcr", state.gpr[rb] as u32 & 63, mask64(0, mb), false),
        _ => return Err(Error::IllegalInstruction { cia, raw }),
    };
    let rotated = state.gpr[rs].rotate_left(shift);
    let value = if insert {
        (state.gpr[ra] & !mask) | (rotated & mask)
    } else {
        rotated & mask
    };
    state.gpr[ra] = value;
    if raw & 1 != 0 {
        update_cr0(state, value);
    }
    state.cia = cia + 4;
    Ok(Outcome {
        mnemonic: record_name(base, false, raw & 1 != 0)?,
        ..Outcome::default()
    })
}

fn execute_31(state: &mut State, raw: u32) -> Result<Outcome, Error> {
    let cia = state.cia;
    let rs = ((raw >> 21) & 31) as usize;
    let ra = ((raw >> 16) & 31) as usize;
    let rb = ((raw >> 11) & 31) as usize;
    let xo10 = (raw >> 1) & 0x3ff;
    let xo9 = xo10 & 0x1ff;
    let oe = raw & (1 << 10) != 0;
    let rc = raw & 1 != 0;
    if matches!(
        xo9,
        8 | 10
            | 40
            | 104
            | 136
            | 138
            | 200
            | 202
            | 232
            | 233
            | 234
            | 235
            | 266
            | 457
            | 459
            | 489
            | 491
    ) {
        return execute_arithmetic_31(state, raw, xo9, rs, ra, rb, oe, rc);
    }
    let mut outcome = Outcome::default();
    match xo10 {
        0 | 32 => {
            let field = ((raw >> 23) & 7) as usize;
            let is_64 = raw & (1 << 21) != 0;
            let relation = if xo10 == 0 {
                if is_64 {
                    (state.gpr[ra] as i64).cmp(&(state.gpr[rb] as i64))
                } else {
                    (state.gpr[ra] as u32 as i32).cmp(&(state.gpr[rb] as u32 as i32))
                }
            } else if is_64 {
                state.gpr[ra].cmp(&state.gpr[rb])
            } else {
                (state.gpr[ra] as u32).cmp(&(state.gpr[rb] as u32))
            };
            set_cr_field(state, field, relation);
            outcome.mnemonic = match (xo10, is_64) {
                (0, false) => "cmpw",
                (0, true) => "cmpd",
                (32, false) => "cmplw",
                _ => "cmpld",
            }
            .into();
        }
        9 | 11 | 73 | 75 => {
            let a32 = state.gpr[ra] as u32;
            let b32 = state.gpr[rb] as u32;
            let (value, base) = match xo10 {
                9 => {
                    let product = u128::from(state.gpr[ra]) * u128::from(state.gpr[rb]);
                    ((product >> 64) as u64, "mulhdu")
                }
                11 => {
                    let high = ((u64::from(a32) * u64::from(b32)) >> 32) as u32;
                    ((u64::from(high) << 32) | u64::from(high), "mulhwu")
                }
                75 => {
                    let high = ((i64::from(a32 as i32) * i64::from(b32 as i32)) >> 32) as u32;
                    ((u64::from(high) << 32) | u64::from(high), "mulhw")
                }
                _ => {
                    let product =
                        i128::from(state.gpr[ra] as i64) * i128::from(state.gpr[rb] as i64);
                    ((product >> 64) as u64, "mulhd")
                }
            };
            state.gpr[rs] = value;
            if rc {
                update_cr0(state, value);
            }
            outcome.mnemonic = record_name(base, false, rc)?;
        }
        19 => {
            state.gpr[rs] = u64::from(state.cr);
            outcome.mnemonic = "mfcr".into();
        }
        21 | 23 | 53 | 55 | 87 | 119 | 149 | 151 | 181 | 183 | 215 | 247 | 279 | 311 | 341
        | 343 | 373 | 375 | 407 | 439 | 532 | 534 | 660 | 662 | 790 | 918 => {
            return execute_indexed_memory(state, raw, xo10)
        }
        24 | 27 | 28 | 60 | 124 | 284 | 316 | 412 | 444 | 476 | 536 | 539 | 792 | 794 => {
            let source = state.gpr[rs];
            let other = state.gpr[rb];
            let (value, base) = match xo10 {
                24 => {
                    let n = other & 0x3f;
                    (
                        if n >= 32 {
                            0
                        } else {
                            u64::from((source as u32) << n)
                        },
                        "slw",
                    )
                }
                27 => {
                    let n = other & 0x7f;
                    (if n >= 64 { 0 } else { source << n }, "sld")
                }
                28 => (source & other, "and"),
                60 => (source & !other, "andc"),
                124 => (!(source | other), "nor"),
                284 => (!(source ^ other), "eqv"),
                316 => (source ^ other, "xor"),
                412 => (source | !other, "orc"),
                444 => (source | other, "or"),
                476 => (!(source & other), "nand"),
                536 => {
                    let n = other & 0x3f;
                    (
                        if n >= 32 {
                            0
                        } else {
                            u64::from((source as u32) >> n)
                        },
                        "srw",
                    )
                }
                539 => {
                    let n = other & 0x7f;
                    (if n >= 64 { 0 } else { source >> n }, "srd")
                }
                792 => {
                    let n = (other & 0x3f) as u32;
                    let word = source as u32;
                    let value = if n >= 32 {
                        if (word as i32) < 0 {
                            u32::MAX
                        } else {
                            0
                        }
                    } else {
                        ((word as i32) >> n) as u32
                    };
                    state.xer.ca = (word as i32) < 0
                        && if n >= 32 {
                            word != 0
                        } else {
                            n != 0 && word & ((1_u32 << n) - 1) != 0
                        };
                    state.xer.ca32 = state.xer.ca;
                    (u64::from(value), "sraw")
                }
                _ => {
                    let n = (other & 0x7f) as u32;
                    let signed = source as i64;
                    let value = if n >= 64 {
                        if signed < 0 {
                            u64::MAX
                        } else {
                            0
                        }
                    } else {
                        (signed >> n) as u64
                    };
                    state.xer.ca = signed < 0
                        && if n >= 64 {
                            source != 0
                        } else {
                            n != 0 && source & ((1_u64 << n) - 1) != 0
                        };
                    state.xer.ca32 = state.xer.ca;
                    (value, "srad")
                }
            };
            state.gpr[ra] = value;
            if rc {
                update_cr0(state, value);
            }
            outcome.mnemonic = record_name(base, false, rc)?;
        }
        26 | 58 | 122 | 154 | 186 | 378 | 506 | 538 | 570 | 922 | 954 | 986 => {
            let source = state.gpr[rs];
            let (value, base) = match xo10 {
                26 => (u64::from((source as u32).leading_zeros()), "cntlzw"),
                58 => (u64::from(source.leading_zeros()), "cntlzd"),
                122 => {
                    let mut value = 0_u64;
                    for index in 0..8 {
                        value |=
                            u64::from(((source >> (index * 8)) as u8).count_ones()) << (index * 8);
                    }
                    (value, "popcntb")
                }
                154 => (u64::from((source as u32).count_ones() & 1), "prtyw"),
                186 => (u64::from(source.count_ones() & 1), "prtyd"),
                378 => {
                    let lo = u64::from((source as u32).count_ones());
                    let hi = u64::from(((source >> 32) as u32).count_ones());
                    ((hi << 32) | lo, "popcntw")
                }
                506 => (u64::from(source.count_ones()), "popcntd"),
                538 => (u64::from((source as u32).trailing_zeros()), "cnttzw"),
                570 => (u64::from(source.trailing_zeros()), "cnttzd"),
                922 => ((source as u16 as i16 as i64) as u64, "extsh"),
                954 => ((source as u8 as i8 as i64) as u64, "extsb"),
                _ => ((source as u32 as i32 as i64) as u64, "extsw"),
            };
            state.gpr[ra] = value;
            if rc {
                update_cr0(state, value);
            }
            outcome.mnemonic = record_name(base, false, rc)?;
        }
        144 => {
            let mask = ((raw >> 12) & 0xff) as u8;
            for field in 0..8 {
                if mask & (0x80 >> field) != 0 {
                    let shift = 28 - field * 4;
                    state.cr =
                        (state.cr & !(0xf << shift)) | ((state.gpr[rs] as u32) & (0xf << shift));
                }
            }
            outcome.mnemonic = "mtcrf".into();
        }
        339 => {
            let spr = decode_spr(raw);
            state.gpr[rs] = match spr {
                1 => state.xer.to_u64(),
                8 => state.lr,
                9 => state.ctr,
                _ => return Err(Error::UnsupportedSpecialRegister { cia, spr }),
            };
            outcome.mnemonic = "mfspr".into();
        }
        413 | 445 | 824 => {
            let immediate = if xo10 == 824 {
                (raw >> 11) & 31
            } else {
                ((raw >> 11) & 31) | ((raw & 2) << 4)
            };
            let source = state.gpr[rs];
            let (value, base) = if xo10 == 824 {
                let word = source as u32;
                let value = ((word as i32) >> immediate) as u32;
                state.xer.ca =
                    (word as i32) < 0 && immediate != 0 && word & ((1_u32 << immediate) - 1) != 0;
                state.xer.ca32 = state.xer.ca;
                (u64::from(value), "srawi")
            } else {
                let value = ((source as i64) >> immediate) as u64;
                state.xer.ca = (source as i64) < 0
                    && immediate != 0
                    && source & ((1_u64 << immediate) - 1) != 0;
                state.xer.ca32 = state.xer.ca;
                (value, "sradi")
            };
            state.gpr[ra] = value;
            if rc {
                update_cr0(state, value);
            }
            outcome.mnemonic = record_name(base, false, rc)?;
        }
        467 => {
            let spr = decode_spr(raw);
            match spr {
                1 => state.xer = Xer::from_u64(state.gpr[rs]),
                8 => state.lr = state.gpr[rs],
                9 => state.ctr = state.gpr[rs],
                _ => return Err(Error::UnsupportedSpecialRegister { cia, spr }),
            }
            outcome.mnemonic = "mtspr".into();
        }
        598 => {
            outcome.mnemonic = if (raw >> 21) & 3 == 0 {
                "sync"
            } else {
                "lwsync"
            }
            .into();
        }
        854 => outcome.mnemonic = "eieio".into(),
        _ => return Err(Error::IllegalInstruction { cia, raw }),
    }
    state.cia = cia + 4;
    Ok(outcome)
}

#[allow(clippy::too_many_arguments)]
fn execute_arithmetic_31(
    state: &mut State,
    _raw: u32,
    xo: u32,
    rt: usize,
    ra: usize,
    rb: usize,
    oe: bool,
    rc: bool,
) -> Result<Outcome, Error> {
    let a = state.gpr[ra];
    let b = state.gpr[rb];
    let (value, base, ov, ov32) = match xo {
        8 => {
            let (value, ca, ca32) = add_carries(!a, b, true);
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            (value, "subfc", sub_overflow(b, a), sub_overflow32(b, a))
        }
        10 => {
            let (value, ca, ca32) = add_carries(a, b, false);
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            (
                value,
                "addc",
                add_overflow(a, b, value),
                add_overflow32(a, b, value),
            )
        }
        40 => (
            b.wrapping_sub(a),
            "subf",
            sub_overflow(b, a),
            sub_overflow32(b, a),
        ),
        104 => (
            a.wrapping_neg(),
            "neg",
            a == 0x8000_0000_0000_0000,
            a as u32 == 0x8000_0000,
        ),
        136 => {
            let carry = state.xer.ca;
            let (value, ca, ca32) = add_carries(!a, b, carry);
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            (
                value,
                "subfe",
                signed_difference_overflow(b, a, carry),
                signed_difference_overflow32(b, a, carry),
            )
        }
        138 => {
            let carry = state.xer.ca;
            let (value, ca, ca32) = add_carries(a, b, carry);
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            (
                value,
                "adde",
                signed_sum_overflow(a, b, carry),
                signed_sum_overflow32(a, b, carry),
            )
        }
        200 => {
            let carry = state.xer.ca;
            let (value, ca, ca32) = add_carries(!a, 0, carry);
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            (
                value,
                "subfze",
                signed_difference_overflow(0, a, carry),
                signed_difference_overflow32(0, a, carry),
            )
        }
        202 => {
            let carry = state.xer.ca;
            let (value, ca, ca32) = add_carries(a, 0, carry);
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            (
                value,
                "addze",
                signed_sum_overflow(a, 0, carry),
                signed_sum_overflow32(a, 0, carry),
            )
        }
        232 => {
            let carry = state.xer.ca;
            let (value, ca, ca32) = add_carries(!a, u64::MAX, carry);
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            (
                value,
                "subfme",
                signed_difference_overflow(u64::MAX, a, carry),
                signed_difference_overflow32(u64::MAX, a, carry),
            )
        }
        234 => {
            let carry = state.xer.ca;
            let (value, ca, ca32) = add_carries(a, u64::MAX, carry);
            state.xer.ca = ca;
            state.xer.ca32 = ca32;
            (
                value,
                "addme",
                signed_sum_overflow(a, u64::MAX, carry),
                signed_sum_overflow32(a, u64::MAX, carry),
            )
        }
        233 => {
            let product = i128::from(a as i64) * i128::from(b as i64);
            let value = product as u64;
            (
                value,
                "mulld",
                product != i128::from(value as i64),
                product != i128::from(value as u32 as i32),
            )
        }
        235 => {
            let product = i64::from(a as u32 as i32) * i64::from(b as u32 as i32);
            let value = (product as u32 as i32 as i64) as u64;
            (value, "mullw", false, product != i64::from(product as i32))
        }
        266 => {
            let value = a.wrapping_add(b);
            (
                value,
                "add",
                add_overflow(a, b, value),
                add_overflow32(a, b, value),
            )
        }
        457 | 459 | 489 | 491 => {
            let word = matches!(xo, 459 | 491);
            let signed = matches!(xo, 489 | 491);
            let value = divide(a, b, word, signed, state.cia)?;
            let base = match xo {
                457 => "divdu",
                459 => "divwu",
                489 => "divd",
                _ => "divw",
            };
            (value, base, false, false)
        }
        _ => unreachable!(),
    };
    if oe {
        state.xer.ov = ov;
        state.xer.ov32 = ov32;
        state.xer.so |= ov;
    }
    state.gpr[rt] = value;
    if rc {
        update_cr0(state, value);
    }
    state.cia += 4;
    Ok(Outcome {
        mnemonic: record_name(base, oe, rc)?,
        ..Outcome::default()
    })
}

fn memory_checksum(memory: &[u8; MEMORY_SIZE]) -> u64 {
    memory.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn validate_state(state: &State) -> Result<(), Error> {
    if state.cia & 3 != 0 || state.cia > MEMORY_SIZE as u64 {
        return Err(Error::InvalidState(
            "CIA must be word-aligned and at most the one-past-memory sentinel".into(),
        ));
    }
    match state.program_origin {
        None if state.program_len != 0 || state.halted => Err(Error::InvalidState(
            "program metadata and halt state require a program_origin".into(),
        )),
        Some(_) if state.program_len == 0 => Err(Error::InvalidState(
            "program_origin requires a non-empty program".into(),
        )),
        Some(origin) => {
            if origin & 3 != 0 {
                return Err(Error::InvalidState(
                    "program_origin must be word-aligned".into(),
                ));
            }
            usize::from(origin)
                .checked_add(state.program_len as usize)
                .filter(|end| *end <= MEMORY_SIZE)
                .ok_or_else(|| Error::InvalidState("installed program exceeds memory".into()))?;
            Ok(())
        }
        None => Ok(()),
    }
}

fn checked_range(address: u64, width: usize) -> Result<usize, Error> {
    let start = usize::try_from(address).map_err(|_| Error::MemoryOutOfRange { address, width })?;
    start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .map(|_| start)
        .ok_or(Error::MemoryOutOfRange { address, width })
}

fn checked_aligned_range(address: u64, width: usize) -> Result<usize, Error> {
    if !address.is_multiple_of(width as u64) {
        return Err(Error::MisalignedAccess { address, width });
    }
    checked_range(address, width)
}

type Effects = (Vec<(u8, u64)>, Vec<SpecialWrite>);

fn effects(before: &State, after: &State) -> Result<Effects, Error> {
    let register_count = before
        .gpr
        .iter()
        .zip(after.gpr.iter())
        .filter(|(left, right)| left != right)
        .count();
    let mut gpr = Vec::new();
    gpr.try_reserve_exact(register_count)
        .map_err(|_| Error::AllocationFailed)?;
    for index in 0..32 {
        if before.gpr[index] != after.gpr[index] {
            gpr.push((index as u8, after.gpr[index]));
        }
    }
    let special_count = usize::from(before.lr != after.lr)
        + usize::from(before.ctr != after.ctr)
        + usize::from(before.cr != after.cr)
        + usize::from(before.xer != after.xer);
    let mut special = Vec::new();
    special
        .try_reserve_exact(special_count)
        .map_err(|_| Error::AllocationFailed)?;
    if before.lr != after.lr {
        special.push(SpecialWrite {
            register: SpecialRegister::Lr,
            value: after.lr,
        });
    }
    if before.ctr != after.ctr {
        special.push(SpecialWrite {
            register: SpecialRegister::Ctr,
            value: after.ctr,
        });
    }
    if before.cr != after.cr {
        special.push(SpecialWrite {
            register: SpecialRegister::Cr,
            value: u64::from(after.cr),
        });
    }
    if before.xer != after.xer {
        special.push(SpecialWrite {
            register: SpecialRegister::Xer,
            value: after.xer.to_u64(),
        });
    }
    Ok((gpr, special))
}

fn simple_outcome(mnemonic: &str) -> Outcome {
    Outcome {
        mnemonic: mnemonic.into(),
        ..Outcome::default()
    }
}

fn ra_or_zero(state: &State, ra: usize) -> u64 {
    if ra == 0 {
        0
    } else {
        state.gpr[ra]
    }
}

fn add_carries(a: u64, b: u64, carry: bool) -> (u64, bool, bool) {
    let wide = u128::from(a) + u128::from(b) + u128::from(carry);
    let low = u64::from(a as u32) + u64::from(b as u32) + u64::from(carry);
    (
        wide as u64,
        wide > u128::from(u64::MAX),
        low > u64::from(u32::MAX),
    )
}

fn set_cr_field(state: &mut State, field: usize, relation: Ordering) {
    let relation = match relation {
        Ordering::Less => 0b1000,
        Ordering::Greater => 0b0100,
        Ordering::Equal => 0b0010,
    } | u32::from(state.xer.so);
    let shift = 28 - field * 4;
    state.cr = (state.cr & !(0xf << shift)) | (relation << shift);
}

fn update_cr0(state: &mut State, value: u64) {
    set_cr_field(state, 0, (value as i64).cmp(&0));
}

fn mask32(mb: u32, me: u32) -> u32 {
    let mut mask = 0;
    let mut bit = mb & 31;
    loop {
        mask |= 1_u32 << (31 - bit);
        if bit == (me & 31) {
            break;
        }
        bit = (bit + 1) & 31;
    }
    mask
}

fn mask64(mb: u32, me: u32) -> u64 {
    let mut mask = 0;
    let mut bit = mb & 63;
    loop {
        mask |= 1_u64 << (63 - bit);
        if bit == (me & 63) {
            break;
        }
        bit = (bit + 1) & 63;
    }
    mask
}

fn record_name(base: &str, overflow: bool, record: bool) -> Result<String, Error> {
    let extra = usize::from(overflow) + usize::from(record);
    let mut name = String::new();
    name.try_reserve_exact(base.len() + extra)
        .map_err(|_| Error::AllocationFailed)?;
    name.push_str(base);
    if overflow {
        name.push('o');
    }
    if record {
        name.push('.');
    }
    Ok(name)
}

fn sign_extend(value: u32, bits: u32) -> u64 {
    (((value as i32) << (32 - bits)) >> (32 - bits)) as i64 as u64
}

fn validate_branch(state: &State, cia: u64, target: u64) -> Result<(), Error> {
    if target & 3 != 0 {
        return Err(Error::BranchOutsideProgram { cia, target });
    }
    let Some(origin) = state.program_origin else {
        return Err(Error::BranchOutsideProgram { cia, target });
    };
    let start = u64::from(origin);
    let end = start + u64::from(state.program_len);
    if target < start || target >= end {
        return Err(Error::BranchOutsideProgram { cia, target });
    }
    Ok(())
}

fn branch_name(base: &str, absolute: bool, link: bool) -> Result<String, Error> {
    let mut name = String::new();
    name.try_reserve_exact(base.len() + usize::from(absolute) + usize::from(link))
        .map_err(|_| Error::AllocationFailed)?;
    name.push_str(base);
    if link {
        name.push('l');
    }
    if absolute {
        name.push('a');
    }
    Ok(name)
}

fn branch_condition(state: &mut State, bo: u32, bi: u32) -> bool {
    let condition_ok = bo & 0x10 != 0 || cr_bit(state.cr, bi as usize) == (bo & 0x08 != 0);
    let counter_ok = if bo & 0x04 != 0 {
        true
    } else {
        state.ctr = state.ctr.wrapping_sub(1);
        (state.ctr != 0) ^ (bo & 0x02 != 0)
    };
    condition_ok && counter_ok
}

fn cr_bit(cr: u32, index: usize) -> bool {
    cr & (1_u32 << (31 - index)) != 0
}

fn set_cr_bit(cr: &mut u32, index: usize, value: bool) {
    let mask = 1_u32 << (31 - index);
    *cr = if value { *cr | mask } else { *cr & !mask };
}

fn decode_spr(raw: u32) -> u16 {
    (((raw >> 16) & 0x1f) | (((raw >> 11) & 0x1f) << 5)) as u16
}

fn add_overflow(a: u64, b: u64, result: u64) -> bool {
    (!(a ^ b) & (a ^ result) & (1_u64 << 63)) != 0
}

fn add_overflow32(a: u64, b: u64, result: u64) -> bool {
    (!(a ^ b) & (a ^ result) & (1_u64 << 31)) != 0
}

fn sub_overflow(minuend: u64, subtrahend: u64) -> bool {
    let result = minuend.wrapping_sub(subtrahend);
    ((minuend ^ subtrahend) & (minuend ^ result) & (1_u64 << 63)) != 0
}

fn sub_overflow32(minuend: u64, subtrahend: u64) -> bool {
    let result = minuend.wrapping_sub(subtrahend);
    ((minuend ^ subtrahend) & (minuend ^ result) & (1_u64 << 31)) != 0
}

fn signed_sum_overflow(a: u64, b: u64, carry: bool) -> bool {
    let exact = i128::from(a as i64) + i128::from(b as i64) + i128::from(carry);
    exact < i128::from(i64::MIN) || exact > i128::from(i64::MAX)
}

fn signed_sum_overflow32(a: u64, b: u64, carry: bool) -> bool {
    let exact = i64::from(a as u32 as i32) + i64::from(b as u32 as i32) + i64::from(carry);
    exact < i64::from(i32::MIN) || exact > i64::from(i32::MAX)
}

fn signed_difference_overflow(minuend: u64, subtrahend: u64, carry: bool) -> bool {
    let exact = i128::from(minuend as i64) - i128::from(subtrahend as i64) - i128::from(!carry);
    exact < i128::from(i64::MIN) || exact > i128::from(i64::MAX)
}

fn signed_difference_overflow32(minuend: u64, subtrahend: u64, carry: bool) -> bool {
    let exact =
        i64::from(minuend as u32 as i32) - i64::from(subtrahend as u32 as i32) - i64::from(!carry);
    exact < i64::from(i32::MIN) || exact > i64::from(i32::MAX)
}

fn divide(a: u64, b: u64, word: bool, signed: bool, cia: u64) -> Result<u64, Error> {
    if signed {
        if word {
            let dividend = a as u32 as i32;
            let divisor = b as u32 as i32;
            let quotient = dividend
                .checked_div(divisor)
                .ok_or(Error::DivideFault { cia, signed: true })?;
            // Microwatt's divider zeroes the upper word; Rc therefore observes
            // the complete positive 64-bit result for negative word quotients.
            Ok(u64::from(quotient as u32))
        } else {
            let quotient = (a as i64)
                .checked_div(b as i64)
                .ok_or(Error::DivideFault { cia, signed: true })?;
            Ok(quotient as u64)
        }
    } else if word {
        let quotient = (a as u32)
            .checked_div(b as u32)
            .ok_or(Error::DivideFault { cia, signed: false })?;
        Ok(u64::from(quotient))
    } else {
        a.checked_div(b)
            .ok_or(Error::DivideFault { cia, signed: false })
    }
}

fn execute_displacement_memory(state: &mut State, raw: u32) -> Result<Outcome, Error> {
    let cia = state.cia;
    let op = raw >> 26;
    let rt = ((raw >> 21) & 31) as usize;
    let ra = ((raw >> 16) & 31) as usize;
    let update =
        matches!(op, 33 | 35 | 37 | 39 | 41 | 43 | 45) || (matches!(op, 58 | 62) && raw & 3 == 1);
    let load = matches!(op, 32..=35 | 40..=43 | 58);
    if update && (ra == 0 || (load && ra == rt)) {
        return Err(Error::IllegalInstruction { cia, raw });
    }
    let displacement = if matches!(op, 58 | 62) {
        sign_extend(raw & 0xfffc, 16)
    } else {
        sign_extend(raw & 0xffff, 16)
    };
    let address = ra_or_zero(state, ra).wrapping_add(displacement);
    let store_value = state.gpr[rt];
    let (width, signed, name) = match (op, raw & 3) {
        (32, _) => (4, false, "lwz"),
        (33, _) => (4, false, "lwzu"),
        (34, _) => (1, false, "lbz"),
        (35, _) => (1, false, "lbzu"),
        (36, _) => (4, false, "stw"),
        (37, _) => (4, false, "stwu"),
        (38, _) => (1, false, "stb"),
        (39, _) => (1, false, "stbu"),
        (40, _) => (2, false, "lhz"),
        (41, _) => (2, false, "lhzu"),
        (42, _) => (2, true, "lha"),
        (43, _) => (2, true, "lhau"),
        (44, _) => (2, false, "sth"),
        (45, _) => (2, false, "sthu"),
        (58, 0) => (8, false, "ld"),
        (58, 1) => (8, false, "ldu"),
        (58, 2) => (4, true, "lwa"),
        (62, 0) => (8, false, "std"),
        (62, 1) => (8, false, "stdu"),
        _ => return Err(Error::IllegalInstruction { cia, raw }),
    };
    let start = if width == 1 {
        checked_range(address, width)?
    } else {
        checked_aligned_range(address, width)?
    };
    let transferred = match width {
        1 => u64::from(state.memory[start]),
        2 => u64::from(u16::from_le_bytes(
            state.memory[start..start + 2].try_into().unwrap(),
        )),
        4 => u64::from(u32::from_le_bytes(
            state.memory[start..start + 4].try_into().unwrap(),
        )),
        8 => u64::from_le_bytes(state.memory[start..start + 8].try_into().unwrap()),
        _ => unreachable!(),
    };
    if load {
        state.gpr[rt] = match (width, signed) {
            (2, true) => (transferred as u16 as i16 as i64) as u64,
            (4, true) => (transferred as u32 as i32 as i64) as u64,
            _ => transferred,
        };
    } else {
        let value = store_value;
        match width {
            1 => state.memory[start] = value as u8,
            2 => state.memory[start..start + 2].copy_from_slice(&(value as u16).to_le_bytes()),
            4 => state.memory[start..start + 4].copy_from_slice(&(value as u32).to_le_bytes()),
            8 => state.memory[start..start + 8].copy_from_slice(&value.to_le_bytes()),
            _ => unreachable!(),
        }
    }
    if update {
        state.gpr[ra] = address;
    }
    state.cia = cia + 4;
    Ok(Outcome {
        mnemonic: name.into(),
        memory_access: Some(MemoryAccess {
            kind: if load {
                MemoryKind::Load
            } else {
                MemoryKind::Store
            },
            address,
            width: width as u8,
            value: if load {
                transferred
            } else {
                store_value & width_mask(width)
            },
        }),
        ..Outcome::default()
    })
}

fn execute_indexed_memory(state: &mut State, raw: u32, xo: u32) -> Result<Outcome, Error> {
    let cia = state.cia;
    let rt = ((raw >> 21) & 31) as usize;
    let ra = ((raw >> 16) & 31) as usize;
    let rb = ((raw >> 11) & 31) as usize;
    let update = matches!(xo, 53 | 55 | 119 | 181 | 183 | 247 | 311 | 373 | 375 | 439);
    let load = matches!(
        xo,
        21 | 23 | 53 | 55 | 87 | 119 | 279 | 311 | 341 | 343 | 373 | 375 | 532 | 534 | 790
    );
    if update && (ra == 0 || (load && ra == rt)) {
        return Err(Error::IllegalInstruction { cia, raw });
    }
    let address = ra_or_zero(state, ra).wrapping_add(state.gpr[rb]);
    let store_value = state.gpr[rt];
    let (width, signed, reverse, name) = match xo {
        21 => (8, false, false, "ldx"),
        53 => (8, false, false, "ldux"),
        23 => (4, false, false, "lwzx"),
        55 => (4, false, false, "lwzux"),
        87 => (1, false, false, "lbzx"),
        119 => (1, false, false, "lbzux"),
        149 => (8, false, false, "stdx"),
        181 => (8, false, false, "stdux"),
        151 => (4, false, false, "stwx"),
        183 => (4, false, false, "stwux"),
        215 => (1, false, false, "stbx"),
        247 => (1, false, false, "stbux"),
        279 => (2, false, false, "lhzx"),
        311 => (2, false, false, "lhzux"),
        341 => (4, true, false, "lwax"),
        373 => (4, true, false, "lwaux"),
        343 => (2, true, false, "lhax"),
        375 => (2, true, false, "lhaux"),
        407 => (2, false, false, "sthx"),
        439 => (2, false, false, "sthux"),
        532 => (8, false, true, "ldbrx"),
        534 => (4, false, true, "lwbrx"),
        660 => (8, false, true, "stdbrx"),
        662 => (4, false, true, "stwbrx"),
        790 => (2, false, true, "lhbrx"),
        918 => (2, false, true, "sthbrx"),
        _ => return Err(Error::IllegalInstruction { cia, raw }),
    };
    let start = if width == 1 {
        checked_range(address, width)?
    } else {
        checked_aligned_range(address, width)?
    };
    let transferred = match (width, reverse) {
        (1, _) => u64::from(state.memory[start]),
        (2, false) => u64::from(u16::from_le_bytes(
            state.memory[start..start + 2].try_into().unwrap(),
        )),
        (2, true) => u64::from(u16::from_be_bytes(
            state.memory[start..start + 2].try_into().unwrap(),
        )),
        (4, false) => u64::from(u32::from_le_bytes(
            state.memory[start..start + 4].try_into().unwrap(),
        )),
        (4, true) => u64::from(u32::from_be_bytes(
            state.memory[start..start + 4].try_into().unwrap(),
        )),
        (8, false) => u64::from_le_bytes(state.memory[start..start + 8].try_into().unwrap()),
        (8, true) => u64::from_be_bytes(state.memory[start..start + 8].try_into().unwrap()),
        _ => unreachable!(),
    };
    if load {
        state.gpr[rt] = match (width, signed) {
            (2, true) => (transferred as u16 as i16 as i64) as u64,
            (4, true) => (transferred as u32 as i32 as i64) as u64,
            _ => transferred,
        };
    } else {
        let value = store_value;
        match (width, reverse) {
            (1, _) => state.memory[start] = value as u8,
            (2, false) => {
                state.memory[start..start + 2].copy_from_slice(&(value as u16).to_le_bytes())
            }
            (2, true) => {
                state.memory[start..start + 2].copy_from_slice(&(value as u16).to_be_bytes())
            }
            (4, false) => {
                state.memory[start..start + 4].copy_from_slice(&(value as u32).to_le_bytes())
            }
            (4, true) => {
                state.memory[start..start + 4].copy_from_slice(&(value as u32).to_be_bytes())
            }
            (8, false) => state.memory[start..start + 8].copy_from_slice(&value.to_le_bytes()),
            (8, true) => state.memory[start..start + 8].copy_from_slice(&value.to_be_bytes()),
            _ => unreachable!(),
        }
    }
    if update {
        state.gpr[ra] = address;
    }
    state.cia = cia + 4;
    Ok(Outcome {
        mnemonic: name.into(),
        memory_access: Some(MemoryAccess {
            kind: if load {
                MemoryKind::Load
            } else {
                MemoryKind::Store
            },
            address,
            width: width as u8,
            value: if load {
                transferred
            } else {
                store_value & width_mask(width)
            },
        }),
        ..Outcome::default()
    })
}

fn width_mask(width: usize) -> u64 {
    match width {
        1 => 0xff,
        2 => 0xffff,
        4 => 0xffff_ffff,
        8 => u64::MAX,
        _ => unreachable!(),
    }
}
