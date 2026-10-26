//! Checked functional simulator for the bounded Renesas SuperH-2 surface in
//! Spec 07av.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Size of the owned flat teaching memory.
pub const MEMORY_BYTES: usize = 64 * 1024;
/// Conventional load address used by examples and tests.
pub const DEFAULT_ORIGIN: u32 = 0x1000;
/// Maximum number of complete-state traces retained by one run.
pub const MAX_TRACES: usize = 64;

/// Architecturally visible bounded subset of the SH-2 status register.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StatusRegister {
    pub t: bool,
    pub s: bool,
    pub q: bool,
    pub m: bool,
    pub interrupt_mask: u8,
}

/// A delayed target captured before its slot executes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingTransfer {
    pub target: u32,
    pub slot_pc: u32,
}

/// Complete, owned architectural and lifecycle state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuperH2State {
    pub registers: [u32; 16],
    pub pc: u32,
    pub pr: u32,
    pub gbr: u32,
    pub vbr: u32,
    pub mach: u32,
    pub macl: u32,
    pub sr: StatusRegister,
    pub pending_transfer: Option<PendingTransfer>,
    pub halted: bool,
    pub installed_origin: u32,
    pub installed_length: u32,
    pub memory: Vec<u8>,
}

impl Default for SuperH2State {
    fn default() -> Self {
        Self {
            registers: [0; 16],
            pc: 0,
            pr: 0,
            gbr: 0,
            vbr: 0,
            mach: 0,
            macl: 0,
            sr: StatusRegister::default(),
            pending_transfer: None,
            halted: false,
            installed_origin: 0,
            installed_length: 0,
            memory: vec![0; MEMORY_BYTES],
        }
    }
}

/// One deterministic successful transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub bytes: [u8; 2],
    pub word: u16,
    pub mnemonic: String,
    pub delay_slot: bool,
    pub post_state: SuperH2State,
}

/// Successful bounded execution result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: SuperH2State,
    pub traces: Vec<StepTrace>,
}

/// Width of an admitted memory transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Width {
    Byte,
    Word,
    Long,
}

impl Width {
    fn bytes(self) -> u32 {
        match self {
            Self::Byte => 1,
            Self::Word => 2,
            Self::Long => 4,
        }
    }
}

/// Stable classification of every admitted encoding family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstructionClass {
    MoveImmediate,
    MoveRegister,
    MoveLoadIndirect(Width),
    MoveStoreIndirect(Width),
    MoveLoadPostIncrement(Width),
    MoveStorePreDecrement(Width),
    MoveLoadIndexed(Width),
    MoveStoreIndexed(Width),
    MoveLoadDisplacement(Width),
    MoveStoreDisplacement(Width),
    MoveLoadGbr(Width),
    MoveStoreGbr(Width),
    MovePcRelative(Width),
    Movt,
    Mova,
    SwapByte,
    SwapWord,
    Xtrct,
    AddImmediate,
    Add,
    AddCarry,
    AddOverflow,
    Subtract,
    SubtractCarry,
    SubtractOverflow,
    Negate,
    NegateCarry,
    DecrementTest,
    ExtendSignedByte,
    ExtendSignedWord,
    ExtendUnsignedByte,
    ExtendUnsignedWord,
    And,
    Or,
    Xor,
    Not,
    Test,
    Compare,
    ShiftRotate,
    MultiplyWord,
    MultiplyLong,
    DoubleMultiply,
    MacWord,
    MacLong,
    DivideInitializeUnsigned,
    DivideInitializeSigned,
    DivideStep,
    ClearT,
    SetT,
    ClearMac,
    SpecialRegisterTransfer,
    BranchImmediate,
    BranchRegister,
    JumpRegister,
    ReturnSubroutine,
    Nop,
    Sleep,
}

/// Typed checked-boundary failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SuperH2Error {
    EmptyProgram,
    OddProgramLength { bytes: usize },
    MisalignedProgram { origin: u32 },
    ProgramOutOfRange { origin: u32, bytes: usize },
    InvalidMemoryLength { bytes: usize },
    InvalidInterruptMask { mask: u8 },
    InvalidInstalledRange { origin: u32, length: u32 },
    InvalidPendingTransfer,
    MisalignedFetch { pc: u32 },
    FetchOutsideProgram { pc: u32 },
    MisalignedData { address: u32, width: Width },
    DataOutOfRange { address: u32, width: Width },
    UnknownInstruction { word: u16, pc: u32 },
    UnsupportedInstruction { word: u16, pc: u32 },
    IllegalDelaySlot { word: u16, pc: u32 },
    UnsupportedDelaySlot { word: u16, pc: u32 },
    Halted { pc: u32 },
    MaxStepsZero,
    MaxStepsExceeded { max_steps: usize },
    TraceLimitExceeded { requested: usize },
    AllocationFailure,
}

impl Display for SuperH2Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => write!(f, "SH-2 program must not be empty"),
            Self::OddProgramLength { bytes } => {
                write!(f, "SH-2 program length {bytes} is not even")
            }
            Self::MisalignedProgram { origin } => {
                write!(
                    f,
                    "SH-2 program origin {origin:#010x} is not half-word aligned"
                )
            }
            Self::ProgramOutOfRange { origin, bytes } => write!(
                f,
                "SH-2 program of {bytes} bytes at {origin:#010x} exceeds memory"
            ),
            Self::InvalidMemoryLength { bytes } => write!(
                f,
                "SH-2 snapshot has {bytes} memory bytes, expected {MEMORY_BYTES}"
            ),
            Self::InvalidInterruptMask { mask } => {
                write!(f, "SH-2 interrupt mask {mask} exceeds four bits")
            }
            Self::InvalidInstalledRange { origin, length } => write!(
                f,
                "SH-2 installed range {origin:#010x}+{length:#x} is invalid"
            ),
            Self::InvalidPendingTransfer => write!(f, "SH-2 pending transfer is inconsistent"),
            Self::MisalignedFetch { pc } => {
                write!(f, "SH-2 fetch address {pc:#010x} is not half-word aligned")
            }
            Self::FetchOutsideProgram { pc } => write!(
                f,
                "SH-2 fetch address {pc:#010x} is outside the installed image"
            ),
            Self::MisalignedData { address, width } => {
                write!(f, "SH-2 {width:?} access at {address:#010x} is misaligned")
            }
            Self::DataOutOfRange { address, width } => {
                write!(f, "SH-2 {width:?} access at {address:#010x} exceeds memory")
            }
            Self::UnknownInstruction { word, pc } => {
                write!(f, "unknown SH-2 instruction {word:#06x} at {pc:#010x}")
            }
            Self::UnsupportedInstruction { word, pc } => {
                write!(f, "unsupported SH-2 instruction {word:#06x} at {pc:#010x}")
            }
            Self::IllegalDelaySlot { word, pc } => write!(
                f,
                "SH-2 instruction {word:#06x} is illegal in delay slot {pc:#010x}"
            ),
            Self::UnsupportedDelaySlot { word, pc } => write!(
                f,
                "SH-2 instruction {word:#06x} is unsupported in delay slot {pc:#010x}"
            ),
            Self::Halted { pc } => write!(f, "SH-2 processor is halted at {pc:#010x}"),
            Self::MaxStepsZero => write!(f, "SH-2 maximum step count must be nonzero"),
            Self::MaxStepsExceeded { max_steps } => {
                write!(f, "SH-2 did not halt within {max_steps} steps")
            }
            Self::TraceLimitExceeded { requested } => write!(
                f,
                "SH-2 requested {requested} traces, limit is {MAX_TRACES}"
            ),
            Self::AllocationFailure => write!(f, "SH-2 trace allocation failed"),
        }
    }
}

impl Error for SuperH2Error {}

fn n(word: u16) -> usize {
    ((word >> 8) & 0xf) as usize
}

fn m(word: u16) -> usize {
    ((word >> 4) & 0xf) as usize
}

fn sign8(value: u16) -> i32 {
    i32::from(value as u8 as i8)
}

fn sign12(value: u16) -> i32 {
    let raw = i32::from(value & 0x0fff);
    if raw & 0x800 != 0 {
        raw | !0xfff
    } else {
        raw
    }
}

fn is_unsupported_system(word: u16) -> bool {
    word == 0x002b
        || word & 0xff00 == 0xc300
        || (word & 0xf0ff) == 0x0002
        || (word & 0xf0ff) == 0x0022
        || (word & 0xf0ff) == 0x400e
        || (word & 0xf0ff) == 0x4007
        || (word & 0xf0ff) == 0x4003
        || (word & 0xf0ff) == 0x402e
        || (word & 0xf0ff) == 0x4027
        || (word & 0xf0ff) == 0x4023
        || (word & 0xf0ff) == 0x401b
}

/// Classify one instruction word without executing it.
pub fn classify(word: u16) -> Result<InstructionClass, SuperH2Error> {
    use InstructionClass as C;
    let c = match word {
        0x0008 => C::ClearT,
        0x0018 => C::SetT,
        0x0028 => C::ClearMac,
        0x0019 => C::DivideInitializeUnsigned,
        0x0009 => C::Nop,
        0x001b => C::Sleep,
        0x000b => C::ReturnSubroutine,
        _ if word & 0xf000 == 0xe000 => C::MoveImmediate,
        _ if word & 0xf000 == 0x7000 => C::AddImmediate,
        _ if word & 0xf000 == 0x9000 => C::MovePcRelative(Width::Word),
        _ if word & 0xf000 == 0xd000 => C::MovePcRelative(Width::Long),
        _ if word & 0xf000 == 0xa000 || word & 0xf000 == 0xb000 => C::BranchImmediate,
        _ if word & 0xff00 == 0x8900
            || word & 0xff00 == 0x8b00
            || word & 0xff00 == 0x8d00
            || word & 0xff00 == 0x8f00 =>
        {
            C::BranchImmediate
        }
        _ if word & 0xff00 == 0x8800 => C::Compare,
        _ if word & 0xf0f0 == 0x8000 || word & 0xf0f0 == 0x8010 => {
            C::MoveStoreDisplacement(if word & 0x10 == 0 {
                Width::Byte
            } else {
                Width::Word
            })
        }
        _ if word & 0xf0f0 == 0x8040 || word & 0xf0f0 == 0x8050 => {
            C::MoveLoadDisplacement(if word & 0x10 == 0 {
                Width::Byte
            } else {
                Width::Word
            })
        }
        _ if word & 0xf000 == 0x1000 => C::MoveStoreDisplacement(Width::Long),
        _ if word & 0xf000 == 0x5000 => C::MoveLoadDisplacement(Width::Long),
        _ if word & 0xff00 == 0xc400 => C::MoveLoadGbr(Width::Byte),
        _ if word & 0xff00 == 0xc500 => C::MoveLoadGbr(Width::Word),
        _ if word & 0xff00 == 0xc600 => C::MoveLoadGbr(Width::Long),
        _ if word & 0xff00 == 0xc000 => C::MoveStoreGbr(Width::Byte),
        _ if word & 0xff00 == 0xc100 => C::MoveStoreGbr(Width::Word),
        _ if word & 0xff00 == 0xc200 => C::MoveStoreGbr(Width::Long),
        _ if word & 0xff00 == 0xc700 => C::Mova,
        _ if matches!(word & 0xff00, 0xc800 | 0xcc00) => C::Test,
        _ if matches!(word & 0xff00, 0xc900 | 0xcd00) => C::And,
        _ if matches!(word & 0xff00, 0xca00 | 0xce00) => C::Xor,
        _ if matches!(word & 0xff00, 0xcb00 | 0xcf00) => C::Or,
        _ if word & 0xf00f == 0x6003 => C::MoveRegister,
        _ if matches!(word & 0xf00f, 0x6000..=0x6002) => {
            C::MoveLoadIndirect(width_from_low(word & 3))
        }
        _ if matches!(word & 0xf00f, 0x6004..=0x6006) => {
            C::MoveLoadPostIncrement(width_from_low(word & 3))
        }
        _ if word & 0xf00f == 0x6007 => C::Not,
        _ if matches!(word & 0xf00f, 0x6008) => C::SwapByte,
        _ if matches!(word & 0xf00f, 0x6009) => C::SwapWord,
        _ if word & 0xf00f == 0x600a => C::NegateCarry,
        _ if word & 0xf00f == 0x600b => C::Negate,
        _ if word & 0xf00f == 0x600c => C::ExtendUnsignedByte,
        _ if word & 0xf00f == 0x600d => C::ExtendUnsignedWord,
        _ if word & 0xf00f == 0x600e => C::ExtendSignedByte,
        _ if word & 0xf00f == 0x600f => C::ExtendSignedWord,
        _ if matches!(word & 0xf00f, 0x2000..=0x2002) => {
            C::MoveStoreIndirect(width_from_low(word & 3))
        }
        _ if matches!(word & 0xf00f, 0x2004..=0x2006) => {
            C::MoveStorePreDecrement(width_from_low(word & 3))
        }
        _ if word & 0xf00f == 0x2007 => C::DivideInitializeSigned,
        _ if word & 0xf00f == 0x2008 => C::Test,
        _ if word & 0xf00f == 0x2009 => C::And,
        _ if word & 0xf00f == 0x200a => C::Xor,
        _ if word & 0xf00f == 0x200b => C::Or,
        _ if word & 0xf00f == 0x200c => C::Compare,
        _ if word & 0xf00f == 0x200d => C::Xtrct,
        _ if matches!(word & 0xf00f, 0x200e | 0x200f) => C::MultiplyWord,
        _ if word & 0xf00f == 0x3000
            || matches!(word & 0xf00f, 0x3002 | 0x3003 | 0x3006 | 0x3007) =>
        {
            C::Compare
        }
        _ if word & 0xf00f == 0x3004 => C::DivideStep,
        _ if matches!(word & 0xf00f, 0x3005 | 0x300d) => C::DoubleMultiply,
        _ if word & 0xf00f == 0x3008 => C::Subtract,
        _ if word & 0xf00f == 0x300a => C::SubtractCarry,
        _ if word & 0xf00f == 0x300b => C::SubtractOverflow,
        _ if word & 0xf00f == 0x300c => C::Add,
        _ if word & 0xf00f == 0x300e => C::AddCarry,
        _ if word & 0xf00f == 0x300f => C::AddOverflow,
        _ if matches!(word & 0xf00f, 0x0004..=0x0006) => {
            C::MoveStoreIndexed(width_from_low(word & 3))
        }
        _ if word & 0xf00f == 0x0007 => C::MultiplyLong,
        _ if matches!(word & 0xf00f, 0x000c..=0x000e) => {
            C::MoveLoadIndexed(width_from_low(word & 3))
        }
        _ if word & 0xf00f == 0x000f => C::MacLong,
        _ if matches!(word & 0xf0ff, 0x0003 | 0x0023) => C::BranchRegister,
        _ if word & 0xf0ff == 0x0029 => C::Movt,
        _ if word & 0xf0ff == 0x0012 => C::SpecialRegisterTransfer,
        _ if matches!(word & 0xf0ff, 0x000a | 0x001a | 0x002a) => C::SpecialRegisterTransfer,
        _ if word & 0xf0ff == 0x402b || word & 0xf0ff == 0x400b => C::JumpRegister,
        _ if word & 0xf0ff == 0x4010 => C::DecrementTest,
        _ if matches!(word & 0xf0ff, 0x4011 | 0x4015) => C::Compare,
        _ if matches!(
            word & 0xf0ff,
            0x4000
                | 0x4001
                | 0x4004
                | 0x4005
                | 0x4020
                | 0x4021
                | 0x4024
                | 0x4025
                | 0x4008
                | 0x4018
                | 0x4028
                | 0x4009
                | 0x4019
                | 0x4029
        ) =>
        {
            C::ShiftRotate
        }
        _ if word & 0xf00f == 0x400f => C::MacWord,
        _ if matches!(word & 0xf0ff, 0x400a | 0x401a | 0x402a | 0x401e) => {
            C::SpecialRegisterTransfer
        }
        _ if matches!(word & 0xf0ff, 0x4002 | 0x4012 | 0x4022 | 0x4013) => {
            C::SpecialRegisterTransfer
        }
        _ if matches!(word & 0xf0ff, 0x4006 | 0x4016 | 0x4026 | 0x4017) => {
            C::SpecialRegisterTransfer
        }
        _ if is_unsupported_system(word) => {
            return Err(SuperH2Error::UnsupportedInstruction { word, pc: 0 });
        }
        _ => return Err(SuperH2Error::UnknownInstruction { word, pc: 0 }),
    };
    Ok(c)
}

fn width_from_low(low: u16) -> Width {
    match low {
        0 => Width::Byte,
        1 => Width::Word,
        _ => Width::Long,
    }
}

/// Checked bounded SH-2 processor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuperH2Simulator {
    state: SuperH2State,
}

impl Default for SuperH2Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl SuperH2Simulator {
    /// Construct a reset processor with empty installed-program metadata.
    pub fn new() -> Self {
        Self {
            state: SuperH2State::default(),
        }
    }

    /// Return a complete owned snapshot.
    pub fn snapshot(&self) -> SuperH2State {
        self.state.clone()
    }

    /// Atomically restore a validated complete snapshot.
    pub fn restore(&mut self, state: SuperH2State) -> Result<(), SuperH2Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    /// Replace complete state with one installed big-endian program.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), SuperH2Error> {
        validate_program(program, origin)?;
        let mut next = SuperH2State::default();
        let start = origin as usize;
        next.memory[start..start + program.len()].copy_from_slice(program);
        next.pc = origin;
        next.installed_origin = origin;
        next.installed_length = program.len() as u32;
        self.state = next;
        Ok(())
    }

    /// Checked public data read using architectural big-endian ordering.
    pub fn read_data(&self, address: u32, width: Width) -> Result<u32, SuperH2Error> {
        read_data(&self.state, address, width)
    }

    /// Checked, atomic public data write using architectural big-endian ordering.
    pub fn write_data(
        &mut self,
        address: u32,
        width: Width,
        value: u32,
    ) -> Result<(), SuperH2Error> {
        check_data(address, width)?;
        write_data_unchecked(&mut self.state, address, width, value);
        Ok(())
    }

    /// Execute one checked, atomic transition.
    pub fn step(&mut self) -> Result<StepTrace, SuperH2Error> {
        let trace = step_candidate(&self.state)?;
        self.state = trace.post_state.clone();
        Ok(trace)
    }

    /// Run transactionally until SLEEP or the bounded step budget is exhausted.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, SuperH2Error> {
        self.run_inner(max_steps, false)
    }

    fn run_inner(
        &mut self,
        max_steps: usize,
        force_allocation_failure: bool,
    ) -> Result<ExecutionResult, SuperH2Error> {
        if max_steps == 0 {
            return Err(SuperH2Error::MaxStepsZero);
        }
        if max_steps > MAX_TRACES {
            return Err(SuperH2Error::TraceLimitExceeded {
                requested: max_steps,
            });
        }
        if force_allocation_failure {
            return Err(SuperH2Error::AllocationFailure);
        }
        let before = self.state.clone();
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| SuperH2Error::AllocationFailure)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => {
                    let halted = trace.post_state.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            final_state: self.state.clone(),
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
        Err(SuperH2Error::MaxStepsExceeded { max_steps })
    }

    /// Atomically load and run a program.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, SuperH2Error> {
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
}

fn validate_program(program: &[u8], origin: u32) -> Result<(), SuperH2Error> {
    if program.is_empty() {
        return Err(SuperH2Error::EmptyProgram);
    }
    if program.len() & 1 != 0 {
        return Err(SuperH2Error::OddProgramLength {
            bytes: program.len(),
        });
    }
    if origin & 1 != 0 {
        return Err(SuperH2Error::MisalignedProgram { origin });
    }
    if usize::try_from(origin)
        .ok()
        .and_then(|start| start.checked_add(program.len()))
        .is_none_or(|end| end > MEMORY_BYTES)
    {
        return Err(SuperH2Error::ProgramOutOfRange {
            origin,
            bytes: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &SuperH2State) -> Result<(), SuperH2Error> {
    if state.memory.len() != MEMORY_BYTES {
        return Err(SuperH2Error::InvalidMemoryLength {
            bytes: state.memory.len(),
        });
    }
    if state.sr.interrupt_mask > 15 {
        return Err(SuperH2Error::InvalidInterruptMask {
            mask: state.sr.interrupt_mask,
        });
    }
    if state.installed_length == 0
        || state.installed_length & 1 != 0
        || state.installed_origin & 1 != 0
        || usize::try_from(state.installed_origin)
            .ok()
            .and_then(|start| start.checked_add(state.installed_length as usize))
            .is_none_or(|end| end > MEMORY_BYTES)
    {
        return Err(SuperH2Error::InvalidInstalledRange {
            origin: state.installed_origin,
            length: state.installed_length,
        });
    }
    if state.pc & 1 != 0 {
        return Err(SuperH2Error::MisalignedFetch { pc: state.pc });
    }
    if let Some(pending) = state.pending_transfer {
        if state.halted || pending.slot_pc != state.pc || pending.slot_pc & 1 != 0 {
            return Err(SuperH2Error::InvalidPendingTransfer);
        }
        check_fetch(state, pending.slot_pc)?;
    }
    Ok(())
}

fn check_fetch(state: &SuperH2State, pc: u32) -> Result<(), SuperH2Error> {
    if pc & 1 != 0 {
        return Err(SuperH2Error::MisalignedFetch { pc });
    }
    let end = state
        .installed_origin
        .saturating_add(state.installed_length);
    if pc < state.installed_origin || pc.checked_add(2).is_none_or(|next| next > end) {
        return Err(SuperH2Error::FetchOutsideProgram { pc });
    }
    Ok(())
}

fn check_data(address: u32, width: Width) -> Result<usize, SuperH2Error> {
    if width != Width::Byte && address & (width.bytes() - 1) != 0 {
        return Err(SuperH2Error::MisalignedData { address, width });
    }
    let start =
        usize::try_from(address).map_err(|_| SuperH2Error::DataOutOfRange { address, width })?;
    if start
        .checked_add(width.bytes() as usize)
        .is_none_or(|end| end > MEMORY_BYTES)
    {
        return Err(SuperH2Error::DataOutOfRange { address, width });
    }
    Ok(start)
}

fn read_data(state: &SuperH2State, address: u32, width: Width) -> Result<u32, SuperH2Error> {
    let start = check_data(address, width)?;
    Ok(match width {
        Width::Byte => u32::from(state.memory[start]),
        Width::Word => u32::from(u16::from_be_bytes([
            state.memory[start],
            state.memory[start + 1],
        ])),
        Width::Long => u32::from_be_bytes([
            state.memory[start],
            state.memory[start + 1],
            state.memory[start + 2],
            state.memory[start + 3],
        ]),
    })
}

fn write_data_unchecked(state: &mut SuperH2State, address: u32, width: Width, value: u32) {
    let start = address as usize;
    match width {
        Width::Byte => state.memory[start] = value as u8,
        Width::Word => {
            state.memory[start..start + 2].copy_from_slice(&(value as u16).to_be_bytes())
        }
        Width::Long => state.memory[start..start + 4].copy_from_slice(&value.to_be_bytes()),
    }
}

fn load_value(state: &SuperH2State, address: u32, width: Width) -> Result<u32, SuperH2Error> {
    let value = read_data(state, address, width)?;
    Ok(match width {
        Width::Byte => (value as u8 as i8 as i32) as u32,
        Width::Word => (value as u16 as i16 as i32) as u32,
        Width::Long => value,
    })
}

fn step_candidate(state: &SuperH2State) -> Result<StepTrace, SuperH2Error> {
    if state.halted {
        return Err(SuperH2Error::Halted { pc: state.pc });
    }
    check_fetch(state, state.pc)?;
    let pc_before = state.pc;
    let start = pc_before as usize;
    let bytes = [state.memory[start], state.memory[start + 1]];
    let word = u16::from_be_bytes(bytes);
    let class = classify(word).map_err(|error| match error {
        SuperH2Error::UnknownInstruction { .. } => SuperH2Error::UnknownInstruction {
            word,
            pc: pc_before,
        },
        SuperH2Error::UnsupportedInstruction { .. } => SuperH2Error::UnsupportedInstruction {
            word,
            pc: pc_before,
        },
        other => other,
    })?;
    let old_pending = state.pending_transfer;
    if old_pending.is_some() {
        if class == InstructionClass::Sleep {
            return Err(SuperH2Error::UnsupportedDelaySlot {
                word,
                pc: pc_before,
            });
        }
        if is_control(class) {
            return Err(SuperH2Error::IllegalDelaySlot {
                word,
                pc: pc_before,
            });
        }
    }
    let mut next = state.clone();
    next.pc = pc_before.wrapping_add(2);
    next.pending_transfer = None;
    let mnemonic = execute_word(&mut next, word, pc_before, old_pending)?;
    if let Some(pending) = old_pending {
        next.pc = pending.target;
        next.pending_transfer = None;
    }
    Ok(StepTrace {
        pc_before,
        pc_after: next.pc,
        bytes,
        word,
        mnemonic: mnemonic.to_owned(),
        delay_slot: old_pending.is_some(),
        post_state: next,
    })
}

fn is_control(class: InstructionClass) -> bool {
    matches!(
        class,
        InstructionClass::BranchImmediate
            | InstructionClass::BranchRegister
            | InstructionClass::JumpRegister
            | InstructionClass::ReturnSubroutine
    )
}

fn execute_word(
    state: &mut SuperH2State,
    word: u16,
    pc: u32,
    pending: Option<PendingTransfer>,
) -> Result<&'static str, SuperH2Error> {
    match word {
        0x0008 => {
            state.sr.t = false;
            return Ok("CLRT");
        }
        0x0018 => {
            state.sr.t = true;
            return Ok("SETT");
        }
        0x0028 => {
            state.mach = 0;
            state.macl = 0;
            return Ok("CLRMAC");
        }
        0x0019 => {
            state.sr.m = false;
            state.sr.q = false;
            state.sr.t = false;
            return Ok("DIV0U");
        }
        0x0009 => return Ok("NOP"),
        0x001b => {
            state.halted = true;
            return Ok("SLEEP");
        }
        0x000b => {
            schedule(state, state.pr, pc);
            return Ok("RTS");
        }
        _ => {}
    }

    let rn = n(word);
    let rm = m(word);
    match word & 0xf000 {
        0xe000 => {
            state.registers[rn] = sign8(word) as u32;
            return Ok("MOV");
        }
        0x7000 => {
            state.registers[rn] = state.registers[rn].wrapping_add(sign8(word) as u32);
            return Ok("ADD");
        }
        0x9000 => {
            let base = pc_relative_base(pc, pending, false);
            let address = base.wrapping_add(u32::from(word & 0xff) * 2);
            state.registers[rn] = load_value(state, address, Width::Word)?;
            return Ok("MOV.W");
        }
        0xd000 => {
            let base = pc_relative_base(pc, pending, true);
            let address = base.wrapping_add(u32::from(word & 0xff) * 4);
            state.registers[rn] = load_value(state, address, Width::Long)?;
            return Ok("MOV.L");
        }
        0xa000 => {
            let target = pc.wrapping_add(4).wrapping_add((sign12(word) << 1) as u32);
            schedule(state, target, pc);
            return Ok("BRA");
        }
        0xb000 => {
            let target = pc.wrapping_add(4).wrapping_add((sign12(word) << 1) as u32);
            state.pr = pc.wrapping_add(4);
            schedule(state, target, pc);
            return Ok("BSR");
        }
        0x1000 => {
            let address = state.registers[rn].wrapping_add(u32::from(word & 0xf) * 4);
            store_checked(state, address, Width::Long, state.registers[rm])?;
            return Ok("MOV.L");
        }
        0x5000 => {
            let address = state.registers[rm].wrapping_add(u32::from(word & 0xf) * 4);
            state.registers[rn] = load_value(state, address, Width::Long)?;
            return Ok("MOV.L");
        }
        _ => {}
    }

    if word & 0xff00 == 0x8900 || word & 0xff00 == 0x8b00 {
        let take = if word & 0x0200 == 0 {
            state.sr.t
        } else {
            !state.sr.t
        };
        if take {
            state.pc = pc.wrapping_add(4).wrapping_add((sign8(word) << 1) as u32);
        }
        return Ok(if word & 0x0200 == 0 { "BT" } else { "BF" });
    }
    if word & 0xff00 == 0x8d00 || word & 0xff00 == 0x8f00 {
        let take = if word & 0x0200 == 0 {
            state.sr.t
        } else {
            !state.sr.t
        };
        if take {
            let target = pc.wrapping_add(4).wrapping_add((sign8(word) << 1) as u32);
            schedule(state, target, pc);
        }
        return Ok(if word & 0x0200 == 0 { "BT/S" } else { "BF/S" });
    }
    if word & 0xff00 == 0x8800 {
        state.sr.t = state.registers[0] == sign8(word) as u32;
        return Ok("CMP/EQ");
    }
    if word & 0xf0f0 == 0x8000 || word & 0xf0f0 == 0x8010 {
        let width = if word & 0x10 == 0 {
            Width::Byte
        } else {
            Width::Word
        };
        let base = n(word);
        let address = state.registers[base].wrapping_add(u32::from(word & 0xf) * width.bytes());
        store_checked(state, address, width, state.registers[0])?;
        return Ok(width_mnemonic("MOV", width));
    }
    if word & 0xf0f0 == 0x8040 || word & 0xf0f0 == 0x8050 {
        let width = if word & 0x10 == 0 {
            Width::Byte
        } else {
            Width::Word
        };
        let base = n(word);
        let address = state.registers[base].wrapping_add(u32::from(word & 0xf) * width.bytes());
        state.registers[0] = load_value(state, address, width)?;
        return Ok(width_mnemonic("MOV", width));
    }

    match word & 0xff00 {
        0xc000 | 0xc100 | 0xc200 => {
            let width = match word & 0xff00 {
                0xc000 => Width::Byte,
                0xc100 => Width::Word,
                _ => Width::Long,
            };
            let address = state
                .gbr
                .wrapping_add(u32::from(word & 0xff) * width.bytes());
            store_checked(state, address, width, state.registers[0])?;
            return Ok(width_mnemonic("MOV", width));
        }
        0xc400 | 0xc500 | 0xc600 => {
            let width = match word & 0xff00 {
                0xc400 => Width::Byte,
                0xc500 => Width::Word,
                _ => Width::Long,
            };
            let address = state
                .gbr
                .wrapping_add(u32::from(word & 0xff) * width.bytes());
            state.registers[0] = load_value(state, address, width)?;
            return Ok(width_mnemonic("MOV", width));
        }
        0xc700 => {
            let base = pc_relative_base(pc, pending, true);
            state.registers[0] = base.wrapping_add(u32::from(word & 0xff) * 4);
            return Ok("MOVA");
        }
        0xc800 => {
            state.sr.t = state.registers[0] & u32::from(word & 0xff) == 0;
            return Ok("TST");
        }
        0xc900 => {
            state.registers[0] &= u32::from(word & 0xff);
            return Ok("AND");
        }
        0xca00 => {
            state.registers[0] ^= u32::from(word & 0xff);
            return Ok("XOR");
        }
        0xcb00 => {
            state.registers[0] |= u32::from(word & 0xff);
            return Ok("OR");
        }
        0xcc00 | 0xcd00 | 0xce00 | 0xcf00 => {
            let address = state.gbr.wrapping_add(state.registers[0]);
            let old = read_data(state, address, Width::Byte)? as u8;
            let imm = (word & 0xff) as u8;
            match word & 0xff00 {
                0xcc00 => state.sr.t = old & imm == 0,
                0xcd00 => store_checked(state, address, Width::Byte, u32::from(old & imm))?,
                0xce00 => store_checked(state, address, Width::Byte, u32::from(old ^ imm))?,
                _ => store_checked(state, address, Width::Byte, u32::from(old | imm))?,
            }
            return Ok(match word & 0xff00 {
                0xcc00 => "TST.B",
                0xcd00 => "AND.B",
                0xce00 => "XOR.B",
                _ => "OR.B",
            });
        }
        _ => {}
    }

    match word & 0xf00f {
        0x6003 => {
            state.registers[rn] = state.registers[rm];
            Ok("MOV")
        }
        0x6000..=0x6002 => {
            let width = width_from_low(word & 3);
            state.registers[rn] = load_value(state, state.registers[rm], width)?;
            Ok(width_mnemonic("MOV", width))
        }
        0x6004..=0x6006 => {
            let width = width_from_low(word & 3);
            let address = state.registers[rm];
            let value = load_value(state, address, width)?;
            state.registers[rn] = value;
            if rn != rm {
                state.registers[rm] = address.wrapping_add(width.bytes());
            }
            Ok(width_mnemonic("MOV", width))
        }
        0x6007 => {
            state.registers[rn] = !state.registers[rm];
            Ok("NOT")
        }
        0x6008 => {
            let value = state.registers[rm];
            state.registers[rn] =
                (value & 0xffff_0000) | ((value & 0xff) << 8) | ((value >> 8) & 0xff);
            Ok("SWAP.B")
        }
        0x6009 => {
            state.registers[rn] = state.registers[rm].rotate_left(16);
            Ok("SWAP.W")
        }
        0x600a => {
            let source = state.registers[rm];
            let carry = u32::from(state.sr.t);
            let (first, b1) = 0u32.overflowing_sub(source);
            let (result, b2) = first.overflowing_sub(carry);
            state.registers[rn] = result;
            state.sr.t = b1 || b2;
            Ok("NEGC")
        }
        0x600b => {
            state.registers[rn] = 0u32.wrapping_sub(state.registers[rm]);
            Ok("NEG")
        }
        0x600c => {
            state.registers[rn] = state.registers[rm] & 0xff;
            Ok("EXTU.B")
        }
        0x600d => {
            state.registers[rn] = state.registers[rm] & 0xffff;
            Ok("EXTU.W")
        }
        0x600e => {
            state.registers[rn] = (state.registers[rm] as u8 as i8 as i32) as u32;
            Ok("EXTS.B")
        }
        0x600f => {
            state.registers[rn] = (state.registers[rm] as u16 as i16 as i32) as u32;
            Ok("EXTS.W")
        }
        0x2000..=0x2002 => {
            let width = width_from_low(word & 3);
            store_checked(state, state.registers[rn], width, state.registers[rm])?;
            Ok(width_mnemonic("MOV", width))
        }
        0x2004..=0x2006 => {
            let width = width_from_low(word & 3);
            let address = state.registers[rn].wrapping_sub(width.bytes());
            check_data(address, width)?;
            write_data_unchecked(state, address, width, state.registers[rm]);
            state.registers[rn] = address;
            Ok(width_mnemonic("MOV", width))
        }
        0x2007 => {
            state.sr.q = state.registers[rn] >> 31 != 0;
            state.sr.m = state.registers[rm] >> 31 != 0;
            state.sr.t = state.sr.q != state.sr.m;
            Ok("DIV0S")
        }
        0x2008 => {
            state.sr.t = state.registers[rn] & state.registers[rm] == 0;
            Ok("TST")
        }
        0x2009 => {
            state.registers[rn] &= state.registers[rm];
            Ok("AND")
        }
        0x200a => {
            state.registers[rn] ^= state.registers[rm];
            Ok("XOR")
        }
        0x200b => {
            state.registers[rn] |= state.registers[rm];
            Ok("OR")
        }
        0x200c => {
            let a = state.registers[rn].to_be_bytes();
            let b = state.registers[rm].to_be_bytes();
            state.sr.t = a.iter().zip(b).any(|(left, right)| *left == right);
            Ok("CMP/STR")
        }
        0x200d => {
            state.registers[rn] = (state.registers[rn] >> 16) | (state.registers[rm] << 16);
            Ok("XTRCT")
        }
        0x200e => {
            state.macl = u32::from(state.registers[rn] as u16)
                .wrapping_mul(u32::from(state.registers[rm] as u16));
            Ok("MULU.W")
        }
        0x200f => {
            state.macl = ((state.registers[rn] as i16 as i32)
                .wrapping_mul(state.registers[rm] as i16 as i32)) as u32;
            Ok("MULS.W")
        }
        0x3000 => {
            state.sr.t = state.registers[rn] == state.registers[rm];
            Ok("CMP/EQ")
        }
        0x3002 => {
            state.sr.t = state.registers[rn] >= state.registers[rm];
            Ok("CMP/HS")
        }
        0x3003 => {
            state.sr.t = (state.registers[rn] as i32) >= (state.registers[rm] as i32);
            Ok("CMP/GE")
        }
        0x3004 => {
            divide_step(state, rm, rn);
            Ok("DIV1")
        }
        0x3005 => {
            let product = u64::from(state.registers[rn]) * u64::from(state.registers[rm]);
            state.mach = (product >> 32) as u32;
            state.macl = product as u32;
            Ok("DMULU.L")
        }
        0x3006 => {
            state.sr.t = state.registers[rn] > state.registers[rm];
            Ok("CMP/HI")
        }
        0x3007 => {
            state.sr.t = (state.registers[rn] as i32) > (state.registers[rm] as i32);
            Ok("CMP/GT")
        }
        0x3008 => {
            state.registers[rn] = state.registers[rn].wrapping_sub(state.registers[rm]);
            Ok("SUB")
        }
        0x300a => {
            let old = state.registers[rn];
            let carry = u32::from(state.sr.t);
            let (first, b1) = old.overflowing_sub(state.registers[rm]);
            let (result, b2) = first.overflowing_sub(carry);
            state.registers[rn] = result;
            state.sr.t = b1 || b2;
            Ok("SUBC")
        }
        0x300b => {
            let left = state.registers[rn];
            let right = state.registers[rm];
            let result = left.wrapping_sub(right);
            state.registers[rn] = result;
            state.sr.t = ((left ^ right) & (left ^ result) & 0x8000_0000) != 0;
            Ok("SUBV")
        }
        0x300c => {
            state.registers[rn] = state.registers[rn].wrapping_add(state.registers[rm]);
            Ok("ADD")
        }
        0x300d => {
            let product = i64::from(state.registers[rn] as i32)
                .wrapping_mul(i64::from(state.registers[rm] as i32));
            state.mach = (product >> 32) as u32;
            state.macl = product as u32;
            Ok("DMULS.L")
        }
        0x300e => {
            let (first, c1) = state.registers[rn].overflowing_add(state.registers[rm]);
            let (result, c2) = first.overflowing_add(u32::from(state.sr.t));
            state.registers[rn] = result;
            state.sr.t = c1 || c2;
            Ok("ADDC")
        }
        0x300f => {
            let left = state.registers[rn];
            let right = state.registers[rm];
            let result = left.wrapping_add(right);
            state.registers[rn] = result;
            state.sr.t = (!(left ^ right) & (left ^ result) & 0x8000_0000) != 0;
            Ok("ADDV")
        }
        0x0004..=0x0006 => {
            let width = width_from_low(word & 3);
            let address = state.registers[0].wrapping_add(state.registers[rn]);
            store_checked(state, address, width, state.registers[rm])?;
            Ok(width_mnemonic("MOV", width))
        }
        0x0007 => {
            state.macl = state.registers[rn].wrapping_mul(state.registers[rm]);
            Ok("MUL.L")
        }
        0x000c..=0x000e => {
            let width = width_from_low(word & 3);
            let address = state.registers[0].wrapping_add(state.registers[rm]);
            state.registers[rn] = load_value(state, address, width)?;
            Ok(width_mnemonic("MOV", width))
        }
        0x000f => {
            mac(state, rn, rm, Width::Long)?;
            Ok("MAC.L")
        }
        0x400f => {
            mac(state, rn, rm, Width::Word)?;
            Ok("MAC.W")
        }
        _ => execute_unary_or_special(state, word, pc),
    }
}

fn schedule(state: &mut SuperH2State, target: u32, pc: u32) {
    let slot_pc = pc.wrapping_add(2);
    state.pc = slot_pc;
    state.pending_transfer = Some(PendingTransfer { target, slot_pc });
}

fn pc_relative_base(pc: u32, pending: Option<PendingTransfer>, align_long: bool) -> u32 {
    let base = pending.map_or_else(|| pc.wrapping_add(4), |item| item.target.wrapping_add(2));
    if align_long {
        base & !3
    } else {
        base
    }
}

fn width_mnemonic(_: &str, width: Width) -> &'static str {
    match width {
        Width::Byte => "MOV.B",
        Width::Word => "MOV.W",
        Width::Long => "MOV.L",
    }
}

fn store_checked(
    state: &mut SuperH2State,
    address: u32,
    width: Width,
    value: u32,
) -> Result<(), SuperH2Error> {
    check_data(address, width)?;
    write_data_unchecked(state, address, width, value);
    Ok(())
}

fn execute_unary_or_special(
    state: &mut SuperH2State,
    word: u16,
    pc: u32,
) -> Result<&'static str, SuperH2Error> {
    let rn = n(word);
    match word & 0xf0ff {
        0x0003 => {
            state.pr = pc.wrapping_add(4);
            schedule(
                state,
                pc.wrapping_add(4).wrapping_add(state.registers[rn]),
                pc,
            );
            Ok("BSRF")
        }
        0x0023 => {
            schedule(
                state,
                pc.wrapping_add(4).wrapping_add(state.registers[rn]),
                pc,
            );
            Ok("BRAF")
        }
        0x0029 => {
            state.registers[rn] = u32::from(state.sr.t);
            Ok("MOVT")
        }
        0x400b => {
            state.pr = pc.wrapping_add(4);
            schedule(state, state.registers[rn], pc);
            Ok("JSR")
        }
        0x402b => {
            schedule(state, state.registers[rn], pc);
            Ok("JMP")
        }
        0x4010 => {
            state.registers[rn] = state.registers[rn].wrapping_sub(1);
            state.sr.t = state.registers[rn] == 0;
            Ok("DT")
        }
        0x4011 => {
            state.sr.t = state.registers[rn] as i32 >= 0;
            Ok("CMP/PZ")
        }
        0x4015 => {
            state.sr.t = state.registers[rn] as i32 > 0;
            Ok("CMP/PL")
        }
        0x4000 | 0x4020 => {
            state.sr.t = state.registers[rn] >> 31 != 0;
            state.registers[rn] <<= 1;
            Ok(if word & 0x20 == 0 { "SHLL" } else { "SHAL" })
        }
        0x4001 => {
            state.sr.t = state.registers[rn] & 1 != 0;
            state.registers[rn] >>= 1;
            Ok("SHLR")
        }
        0x4021 => {
            state.sr.t = state.registers[rn] & 1 != 0;
            state.registers[rn] = (state.registers[rn] as i32 >> 1) as u32;
            Ok("SHAR")
        }
        0x4004 => {
            let bit = state.registers[rn] >> 31 != 0;
            state.registers[rn] = state.registers[rn].rotate_left(1);
            state.sr.t = bit;
            Ok("ROTL")
        }
        0x4005 => {
            let bit = state.registers[rn] & 1 != 0;
            state.registers[rn] = state.registers[rn].rotate_right(1);
            state.sr.t = bit;
            Ok("ROTR")
        }
        0x4024 => {
            let old_t = state.sr.t;
            state.sr.t = state.registers[rn] >> 31 != 0;
            state.registers[rn] = (state.registers[rn] << 1) | u32::from(old_t);
            Ok("ROTCL")
        }
        0x4025 => {
            let old_t = state.sr.t;
            state.sr.t = state.registers[rn] & 1 != 0;
            state.registers[rn] = (state.registers[rn] >> 1) | (u32::from(old_t) << 31);
            Ok("ROTCR")
        }
        0x4008 | 0x4018 | 0x4028 => {
            let amount = match word & 0x30 {
                0x00 => 2,
                0x10 => 8,
                _ => 16,
            };
            state.registers[rn] <<= amount;
            Ok(match amount {
                2 => "SHLL2",
                8 => "SHLL8",
                _ => "SHLL16",
            })
        }
        0x4009 | 0x4019 | 0x4029 => {
            let amount = match word & 0x30 {
                0x00 => 2,
                0x10 => 8,
                _ => 16,
            };
            state.registers[rn] >>= amount;
            Ok(match amount {
                2 => "SHLR2",
                8 => "SHLR8",
                _ => "SHLR16",
            })
        }
        0x000a => {
            state.registers[rn] = state.mach;
            Ok("STS MACH")
        }
        0x001a => {
            state.registers[rn] = state.macl;
            Ok("STS MACL")
        }
        0x002a => {
            state.registers[rn] = state.pr;
            Ok("STS PR")
        }
        0x0012 => {
            state.registers[rn] = state.gbr;
            Ok("STC GBR")
        }
        0x400a => {
            state.mach = state.registers[rn];
            Ok("LDS MACH")
        }
        0x401a => {
            state.macl = state.registers[rn];
            Ok("LDS MACL")
        }
        0x402a => {
            state.pr = state.registers[rn];
            Ok("LDS PR")
        }
        0x401e => {
            state.gbr = state.registers[rn];
            Ok("LDC GBR")
        }
        0x4002 | 0x4012 | 0x4022 | 0x4013 => {
            let value = match word & 0x30 {
                0x00 => state.mach,
                0x10 => {
                    if word & 3 == 3 {
                        state.gbr
                    } else {
                        state.macl
                    }
                }
                _ => state.pr,
            };
            let address = state.registers[rn].wrapping_sub(4);
            store_checked(state, address, Width::Long, value)?;
            state.registers[rn] = address;
            Ok(if word & 3 == 3 { "STC.L" } else { "STS.L" })
        }
        0x4006 | 0x4016 | 0x4026 | 0x4017 => {
            let address = state.registers[rn];
            let value = read_data(state, address, Width::Long)?;
            match word & 0x30 {
                0x00 => state.mach = value,
                0x10 => {
                    if word & 1 == 1 {
                        state.gbr = value
                    } else {
                        state.macl = value
                    }
                }
                _ => state.pr = value,
            }
            state.registers[rn] = address.wrapping_add(4);
            Ok(if word & 1 == 1 { "LDC.L" } else { "LDS.L" })
        }
        _ if is_unsupported_system(word) => Err(SuperH2Error::UnsupportedInstruction { word, pc }),
        _ => Err(SuperH2Error::UnknownInstruction { word, pc }),
    }
}

fn divide_step(state: &mut SuperH2State, rm: usize, rn: usize) {
    let old_q = state.sr.q;
    state.sr.q = state.registers[rn] >> 31 != 0;
    state.registers[rn] = (state.registers[rn] << 1) | u32::from(state.sr.t);
    let before = state.registers[rn];
    let add = old_q == state.sr.m;
    if add {
        state.registers[rn] = state.registers[rn].wrapping_sub(state.registers[rm]);
        let borrow = state.registers[rn] > before;
        state.sr.q = if state.sr.q { !borrow } else { borrow };
    } else {
        state.registers[rn] = state.registers[rn].wrapping_add(state.registers[rm]);
        let carry = state.registers[rn] < before;
        state.sr.q = if state.sr.q { carry } else { !carry };
    }
    state.sr.t = state.sr.q == state.sr.m;
}

fn mac(state: &mut SuperH2State, rn: usize, rm: usize, width: Width) -> Result<(), SuperH2Error> {
    let first_address = state.registers[rn];
    let second_address = if rn == rm {
        first_address.wrapping_add(width.bytes())
    } else {
        state.registers[rm]
    };
    let first = load_value(state, first_address, width)?;
    let second = load_value(state, second_address, width)?;
    let product = i64::from(first as i32).wrapping_mul(i64::from(second as i32));
    let accumulator = ((u64::from(state.mach) << 32) | u64::from(state.macl)) as i64;
    if state.sr.s && width == Width::Word {
        let value = i128::from(state.macl as i32) + i128::from(product);
        state.macl = value.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32 as u32;
    } else if state.sr.s {
        let current = ((i64::from(state.mach as i16)) << 32) | i64::from(state.macl);
        let value = i128::from(current) + i128::from(product);
        let clamped = value.clamp(-(1_i128 << 47), (1_i128 << 47) - 1) as i64;
        state.mach = (clamped >> 32) as u32;
        state.macl = clamped as u32;
    } else {
        let result = accumulator.wrapping_add(product);
        state.mach = (result >> 32) as u32;
        state.macl = result as u32;
    }
    state.registers[rn] = first_address.wrapping_add(width.bytes());
    state.registers[rm] = second_address.wrapping_add(width.bytes());
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn classification_covers_every_claimed_family() {
        use InstructionClass as C;
        let cases = [
            (0xe12a, C::MoveImmediate),
            (0x6123, C::MoveRegister),
            (0x6120, C::MoveLoadIndirect(Width::Byte)),
            (0x2122, C::MoveStoreIndirect(Width::Long)),
            (0x6125, C::MoveLoadPostIncrement(Width::Word)),
            (0x2126, C::MoveStorePreDecrement(Width::Long)),
            (0x012e, C::MoveLoadIndexed(Width::Long)),
            (0x0124, C::MoveStoreIndexed(Width::Byte)),
            (0x8153, C::MoveLoadDisplacement(Width::Word)),
            (0x1123, C::MoveStoreDisplacement(Width::Long)),
            (0xc501, C::MoveLoadGbr(Width::Word)),
            (0xc201, C::MoveStoreGbr(Width::Long)),
            (0x9101, C::MovePcRelative(Width::Word)),
            (0x0129, C::Movt),
            (0xc701, C::Mova),
            (0x6128, C::SwapByte),
            (0x6129, C::SwapWord),
            (0x212d, C::Xtrct),
            (0x7123, C::AddImmediate),
            (0x312c, C::Add),
            (0x312e, C::AddCarry),
            (0x312f, C::AddOverflow),
            (0x3128, C::Subtract),
            (0x312a, C::SubtractCarry),
            (0x312b, C::SubtractOverflow),
            (0x612b, C::Negate),
            (0x612a, C::NegateCarry),
            (0x4110, C::DecrementTest),
            (0x612e, C::ExtendSignedByte),
            (0x612f, C::ExtendSignedWord),
            (0x612c, C::ExtendUnsignedByte),
            (0x612d, C::ExtendUnsignedWord),
            (0x2129, C::And),
            (0x212b, C::Or),
            (0x212a, C::Xor),
            (0x6127, C::Not),
            (0x2128, C::Test),
            (0x3120, C::Compare),
            (0x4124, C::ShiftRotate),
            (0x212e, C::MultiplyWord),
            (0x0127, C::MultiplyLong),
            (0x312d, C::DoubleMultiply),
            (0x412f, C::MacWord),
            (0x012f, C::MacLong),
            (0x0019, C::DivideInitializeUnsigned),
            (0x2127, C::DivideInitializeSigned),
            (0x3124, C::DivideStep),
            (0x0008, C::ClearT),
            (0x0018, C::SetT),
            (0x0028, C::ClearMac),
            (0x411e, C::SpecialRegisterTransfer),
            (0xa001, C::BranchImmediate),
            (0x0123, C::BranchRegister),
            (0x412b, C::JumpRegister),
            (0x000b, C::ReturnSubroutine),
            (0x0009, C::Nop),
            (0x001b, C::Sleep),
        ];
        for (word, expected) in cases {
            assert_eq!(classify(word), Ok(expected), "word {word:#06x}");
        }
        assert!(matches!(
            classify(0x002b),
            Err(SuperH2Error::UnsupportedInstruction { .. })
        ));
        assert!(matches!(
            classify(0xffff),
            Err(SuperH2Error::UnknownInstruction { .. })
        ));
    }

    #[test]
    fn forced_allocation_failure_is_atomic() {
        let mut cpu = SuperH2Simulator::new();
        cpu.load(&[0x00, 0x1b], DEFAULT_ORIGIN).unwrap();
        let before = cpu.snapshot();
        assert_eq!(cpu.run_inner(1, true), Err(SuperH2Error::AllocationFailure));
        assert_eq!(cpu.snapshot(), before);
    }
}
