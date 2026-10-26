//! Checked classic MSP430 functional simulator.
//!
//! The MSP430's small instruction word hides a wonderfully regular machine:
//! two operand formats share one address generator, R0/R1/R2/R3 become
//! PC/SP/SR/constant-generator, and extension words turn the same six operand
//! bits into indexed, symbolic, absolute, and immediate forms. This crate
//! keeps those rules explicit so it can serve as a readable CPU oracle rather
//! than as a fast emulator.
//!
//! ```
//! use msp430_simulator::{Msp430, HALT_WORD};
//!
//! let mut cpu = Msp430::new();
//! cpu.load(&HALT_WORD.to_le_bytes(), 0x0200)?;
//! let traces = cpu.run(1)?;
//! assert!(traces[0].state.halted);
//! # Ok::<(), msp430_simulator::Msp430Error>(())
//! ```

use std::error::Error;
use std::fmt;

/// The complete classic address space.
pub const MEMORY_SIZE: usize = 65_536;
/// Repository-only deterministic halt instruction.
pub const HALT_WORD: u16 = 0x0000;
/// Maximum number of complete-state traces retained by one bounded run.
pub const MAX_TRACES: usize = 64;

const SR_C: u16 = 1 << 0;
const SR_Z: u16 = 1 << 1;
const SR_N: u16 = 1 << 2;
const SR_V: u16 = 1 << 8;
const SR_MASK: u16 = 0x01ff;

/// Operand width selected by the B/W bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandWidth {
    /// Eight-bit operation.
    Byte,
    /// Sixteen-bit operation.
    Word,
}

impl OperandWidth {
    fn mask(self) -> u16 {
        match self {
            Self::Byte => 0x00ff,
            Self::Word => 0xffff,
        }
    }

    fn sign(self) -> u16 {
        match self {
            Self::Byte => 0x0080,
            Self::Word => 0x8000,
        }
    }

    fn bytes(self) -> u16 {
        match self {
            Self::Byte => 1,
            Self::Word => 2,
        }
    }
}

/// Stable trace-level addressing classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    /// Register-direct operand.
    Register,
    /// Register plus a signed extension word.
    Indexed,
    /// PC-relative indexed form.
    Symbolic,
    /// Absolute extension-word address.
    Absolute,
    /// Indirect register operand.
    Indirect,
    /// Indirect operand followed by register increment.
    IndirectAutoincrement,
    /// Immediate extension word.
    Immediate,
    /// R2/R3 constant-generator value.
    Constant,
}

/// The 27 classic encoded families plus the repository halt sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Mov,
    Add,
    Addc,
    Subc,
    Sub,
    Cmp,
    Dadd,
    Bit,
    Bic,
    Bis,
    Xor,
    And,
    Rrc,
    Swpb,
    Rra,
    Sxt,
    Push,
    Call,
    Reti,
    Jne,
    Jeq,
    Jnc,
    Jc,
    Jn,
    Jge,
    Jl,
    Jmp,
    Halt,
}

impl Instruction {
    /// Stable uppercase core mnemonic. Aliases such as `CLR` and `BR` are not
    /// separate decoder families.
    pub fn mnemonic(self) -> &'static str {
        match self {
            Self::Mov => "MOV",
            Self::Add => "ADD",
            Self::Addc => "ADDC",
            Self::Subc => "SUBC",
            Self::Sub => "SUB",
            Self::Cmp => "CMP",
            Self::Dadd => "DADD",
            Self::Bit => "BIT",
            Self::Bic => "BIC",
            Self::Bis => "BIS",
            Self::Xor => "XOR",
            Self::And => "AND",
            Self::Rrc => "RRC",
            Self::Swpb => "SWPB",
            Self::Rra => "RRA",
            Self::Sxt => "SXT",
            Self::Push => "PUSH",
            Self::Call => "CALL",
            Self::Reti => "RETI",
            Self::Jne => "JNE",
            Self::Jeq => "JEQ",
            Self::Jnc => "JNC",
            Self::Jc => "JC",
            Self::Jn => "JN",
            Self::Jge => "JGE",
            Self::Jl => "JL",
            Self::Jmp => "JMP",
            Self::Halt => "HALT",
        }
    }
}

/// Decode one first instruction word without reading extensions.
///
/// Fixed B/W bits on SWPB, SXT, and CALL are enforced here. Reserved Format
/// II neighbors do not get accepted merely because their high nibble is one.
pub fn decode(word: u16) -> Option<Instruction> {
    if word == HALT_WORD {
        return Some(Instruction::Halt);
    }
    let opcode = word >> 12;
    if (4..=15).contains(&opcode) {
        return Some(
            [
                Instruction::Mov,
                Instruction::Add,
                Instruction::Addc,
                Instruction::Subc,
                Instruction::Sub,
                Instruction::Cmp,
                Instruction::Dadd,
                Instruction::Bit,
                Instruction::Bic,
                Instruction::Bis,
                Instruction::Xor,
                Instruction::And,
            ][usize::from(opcode - 4)],
        );
    }
    if (0x2000..=0x3fff).contains(&word) {
        return Some(
            [
                Instruction::Jne,
                Instruction::Jeq,
                Instruction::Jnc,
                Instruction::Jc,
                Instruction::Jn,
                Instruction::Jge,
                Instruction::Jl,
                Instruction::Jmp,
            ][usize::from((word >> 10) & 7)],
        );
    }
    match word {
        0x1300 => Some(Instruction::Reti),
        _ if word & 0xff80 == 0x1000 => Some(Instruction::Rrc),
        _ if word & 0xffc0 == 0x1080 => Some(Instruction::Swpb),
        _ if word & 0xff80 == 0x1100 => Some(Instruction::Rra),
        _ if word & 0xffc0 == 0x1180 => Some(Instruction::Sxt),
        _ if word & 0xff80 == 0x1200 => Some(Instruction::Push),
        _ if word & 0xffc0 == 0x1280 => Some(Instruction::Call),
        _ => None,
    }
}

/// Complete architectural state. The variable-length memory representation is
/// intentional: restore can reject malformed snapshots rather than relying on
/// construction through private fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Msp430State {
    pub registers: [u16; 16],
    pub memory: Vec<u8>,
    pub halted: bool,
    pub origin: Option<u16>,
    pub length: u32,
}

impl Default for Msp430State {
    fn default() -> Self {
        Self {
            registers: [0; 16],
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            origin: None,
            length: 0,
        }
    }
}

/// One successful instruction and its complete post-state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub fetched_bytes: Vec<u8>,
    pub raw_words: Vec<u16>,
    pub width: u8,
    pub mnemonic: &'static str,
    pub operand_width: Option<OperandWidth>,
    pub source_mode: Option<AddressingMode>,
    pub destination_mode: Option<AddressingMode>,
    pub taken: Option<bool>,
    pub state: Msp430State,
}

/// Stable typed lifecycle and semantic failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msp430Error {
    InvalidState {
        reason: &'static str,
    },
    InvalidRegister {
        register: u8,
    },
    EmptyProgram,
    OddProgramLength {
        length: usize,
    },
    MisalignedProgram {
        origin: u16,
    },
    ProgramOutOfRange {
        origin: u16,
        length: usize,
    },
    NoProgramInstalled,
    FetchOutsideProgram {
        pc: u16,
    },
    TruncatedInstruction {
        pc: u16,
        width: u8,
    },
    MisalignedAccess {
        address: u16,
    },
    UnknownInstruction {
        pc: u16,
        raw: u16,
    },
    UnpredictableOperand {
        pc: u16,
        raw: u16,
        reason: &'static str,
    },
    UndefinedBcdOperand {
        pc: u16,
        raw: u16,
    },
    Halted,
    TraceLimitExceeded {
        requested: usize,
        maximum: usize,
    },
    StepBudgetExhausted {
        maximum: usize,
    },
    AllocationFailure,
    ExecutionFailed {
        step: usize,
        pc: u16,
        source: Box<Msp430Error>,
    },
}

impl fmt::Display for Msp430Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState { reason } => write!(f, "invalid MSP430 state: {reason}"),
            Self::InvalidRegister { register } => write!(f, "invalid MSP430 register R{register}"),
            Self::EmptyProgram => write!(f, "program image is empty"),
            Self::OddProgramLength { length } => write!(f, "program length {length} is odd"),
            Self::MisalignedProgram { origin } => {
                write!(f, "program origin 0x{origin:04x} is not word aligned")
            }
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program at 0x{origin:04x} with {length} bytes exceeds 64 KiB"
            ),
            Self::NoProgramInstalled => write!(f, "no MSP430 program is installed"),
            Self::FetchOutsideProgram { pc } => {
                write!(
                    f,
                    "instruction fetch at 0x{pc:04x} is outside the installed image"
                )
            }
            Self::TruncatedInstruction { pc, width } => {
                write!(f, "instruction at 0x{pc:04x} needs {width} installed bytes")
            }
            Self::MisalignedAccess { address } => {
                write!(f, "word access at 0x{address:04x} is misaligned")
            }
            Self::UnknownInstruction { pc, raw } => {
                write!(f, "unknown instruction 0x{raw:04x} at 0x{pc:04x}")
            }
            Self::UnpredictableOperand { pc, raw, reason } => write!(
                f,
                "unpredictable operand for 0x{raw:04x} at 0x{pc:04x}: {reason}"
            ),
            Self::UndefinedBcdOperand { pc, raw } => write!(
                f,
                "undefined packed-BCD operand for 0x{raw:04x} at 0x{pc:04x}"
            ),
            Self::Halted => write!(f, "MSP430 is halted"),
            Self::TraceLimitExceeded { requested, maximum } => write!(
                f,
                "requested {requested} traces exceeds the limit {maximum}"
            ),
            Self::StepBudgetExhausted { maximum } => {
                write!(f, "MSP430 did not halt within {maximum} steps")
            }
            Self::AllocationFailure => write!(f, "MSP430 allocation failed"),
            Self::ExecutionFailed { step, pc, source } => {
                write!(
                    f,
                    "MSP430 execution failed at step {step}, PC 0x{pc:04x}: {source}"
                )
            }
        }
    }
}

impl Error for Msp430Error {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ExecutionFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Msp430 {
    state: Msp430State,
}

#[derive(Debug, Clone, Copy)]
enum Location {
    Register(u8),
    Memory(u16),
}

#[derive(Debug, Clone, Copy)]
struct Destination {
    location: Location,
    old: u16,
    mode: AddressingMode,
}

#[derive(Debug)]
struct Fetched {
    instruction: Instruction,
    raw: u16,
    bytes: Vec<u8>,
    words: Vec<u16>,
}

#[derive(Debug, Clone, Copy, Default)]
struct FlagUpdate {
    mask: u16,
    value: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllocationSite {
    CloneState,
    LoadMemory,
    FetchBytes,
    FetchWords,
    TraceRetention,
    FinalTraceState,
}

#[cfg(not(test))]
fn allocation_seam(_site: AllocationSite) -> Result<(), Msp430Error> {
    Ok(())
}

#[cfg(test)]
thread_local! {
    static FAIL_NEXT_ALLOCATION: std::cell::Cell<Option<AllocationSite>> = const {
        std::cell::Cell::new(None)
    };
}

#[cfg(test)]
fn allocation_seam(site: AllocationSite) -> Result<(), Msp430Error> {
    FAIL_NEXT_ALLOCATION.with(|failure| {
        if failure.get() == Some(site) {
            failure.set(None);
            Err(Msp430Error::AllocationFailure)
        } else {
            Ok(())
        }
    })
}

#[cfg(test)]
fn fail_next_allocation(site: AllocationSite) {
    FAIL_NEXT_ALLOCATION.with(|failure| {
        assert!(failure.replace(Some(site)).is_none());
    });
}

impl Msp430 {
    /// Construct deterministic zeroed state.
    pub fn new() -> Self {
        Self {
            state: Msp430State::default(),
        }
    }

    /// Reset registers, memory, halt, and image metadata.
    pub fn reset(&mut self) {
        self.state.registers = [0; 16];
        self.state.memory.fill(0);
        self.state.halted = false;
        self.state.origin = None;
        self.state.length = 0;
    }

    /// Return an owned complete snapshot.
    pub fn snapshot(&self) -> Msp430State {
        self.state.clone()
    }

    /// Restore a canonical complete state atomically.
    pub fn restore(&mut self, state: &Msp430State) -> Result<(), Msp430Error> {
        validate_state(state)?;
        self.state = try_clone_state(state, AllocationSite::CloneState)?;
        Ok(())
    }

    /// Install one contiguous little-endian byte image.
    pub fn load(&mut self, program: &[u8], origin: u16) -> Result<(), Msp430Error> {
        if program.is_empty() {
            return Err(Msp430Error::EmptyProgram);
        }
        if program.len() & 1 != 0 {
            return Err(Msp430Error::OddProgramLength {
                length: program.len(),
            });
        }
        if origin & 1 != 0 {
            return Err(Msp430Error::MisalignedProgram { origin });
        }
        if usize::from(origin)
            .checked_add(program.len())
            .is_none_or(|end| end > MEMORY_SIZE)
        {
            return Err(Msp430Error::ProgramOutOfRange {
                origin,
                length: program.len(),
            });
        }
        let mut memory = Vec::new();
        allocation_seam(AllocationSite::LoadMemory)?;
        memory
            .try_reserve_exact(MEMORY_SIZE)
            .map_err(|_| Msp430Error::AllocationFailure)?;
        memory.resize(MEMORY_SIZE, 0);
        let mut candidate = Msp430State {
            registers: [0; 16],
            memory,
            halted: false,
            origin: None,
            length: 0,
        };
        let start = usize::from(origin);
        candidate.memory[start..start + program.len()].copy_from_slice(program);
        candidate.registers[0] = origin;
        candidate.origin = Some(origin);
        candidate.length = program.len() as u32;
        self.state = candidate;
        Ok(())
    }

    pub fn read_register(&self, register: u8) -> Result<u16, Msp430Error> {
        self.state
            .registers
            .get(usize::from(register))
            .copied()
            .ok_or(Msp430Error::InvalidRegister { register })
    }

    pub fn write_register(&mut self, register: u8, value: u16) -> Result<(), Msp430Error> {
        if register >= 16 {
            return Err(Msp430Error::InvalidRegister { register });
        }
        write_register_value(&mut self.state, register, OperandWidth::Word, value);
        Ok(())
    }

    pub fn read_memory_byte(&self, address: u16) -> u8 {
        self.state.memory[usize::from(address)]
    }

    pub fn write_memory_byte(&mut self, address: u16, value: u8) {
        self.state.memory[usize::from(address)] = value;
    }

    pub fn read_memory_word(&self, address: u16) -> Result<u16, Msp430Error> {
        read_word(&self.state, address)
    }

    pub fn write_memory_word(&mut self, address: u16, value: u16) -> Result<(), Msp430Error> {
        write_word(&mut self.state, address, value)
    }

    /// Execute one instruction atomically.
    pub fn step(&mut self) -> Result<StepTrace, Msp430Error> {
        if self.state.halted {
            return Err(Msp430Error::Halted);
        }
        let pc_before = self.state.registers[0];
        let fetched = self.fetch()?;
        let mut candidate = try_clone_state(&self.state, AllocationSite::CloneState)?;
        candidate.registers[0] = pc_before.wrapping_add(2);

        let mut source_mode = None;
        let mut destination_mode = None;
        let mut taken = None;
        let operand_width = match fetched.instruction {
            Instruction::Reti
            | Instruction::Jne
            | Instruction::Jeq
            | Instruction::Jnc
            | Instruction::Jc
            | Instruction::Jn
            | Instruction::Jge
            | Instruction::Jl
            | Instruction::Jmp
            | Instruction::Halt => None,
            Instruction::Swpb | Instruction::Sxt | Instruction::Call => Some(OperandWidth::Word),
            _ => Some(if fetched.raw & 0x0040 != 0 {
                OperandWidth::Byte
            } else {
                OperandWidth::Word
            }),
        };

        match fetched.instruction {
            Instruction::Mov
            | Instruction::Add
            | Instruction::Addc
            | Instruction::Subc
            | Instruction::Sub
            | Instruction::Cmp
            | Instruction::Dadd
            | Instruction::Bit
            | Instruction::Bic
            | Instruction::Bis
            | Instruction::Xor
            | Instruction::And => {
                let (src_mode, dst_mode) = execute_double(
                    &mut candidate,
                    pc_before,
                    fetched.raw,
                    fetched.instruction,
                    &fetched.words[1..],
                )?;
                source_mode = Some(src_mode);
                destination_mode = Some(dst_mode);
            }
            Instruction::Rrc | Instruction::Swpb | Instruction::Rra | Instruction::Sxt => {
                destination_mode = Some(execute_single(
                    &mut candidate,
                    pc_before,
                    fetched.raw,
                    fetched.instruction,
                    &fetched.words[1..],
                )?);
            }
            Instruction::Push | Instruction::Call => {
                source_mode = Some(execute_single(
                    &mut candidate,
                    pc_before,
                    fetched.raw,
                    fetched.instruction,
                    &fetched.words[1..],
                )?);
            }
            Instruction::Reti => execute_reti(&mut candidate)?,
            Instruction::Jne
            | Instruction::Jeq
            | Instruction::Jnc
            | Instruction::Jc
            | Instruction::Jn
            | Instruction::Jge
            | Instruction::Jl
            | Instruction::Jmp => {
                taken = Some(execute_jump(
                    &mut candidate,
                    fetched.raw,
                    fetched.instruction,
                ));
            }
            Instruction::Halt => candidate.halted = true,
        }

        candidate.registers[0] &= !1;
        candidate.registers[1] &= !1;
        candidate.registers[2] &= SR_MASK;
        candidate.registers[3] = 0;
        let trace = StepTrace {
            pc_before,
            pc_after: candidate.registers[0],
            fetched_bytes: fetched.bytes,
            raw_words: fetched.words,
            width: u8::try_from(2 + extension_count(fetched.raw, fetched.instruction) * 2)
                .expect("instruction width is at most six bytes"),
            mnemonic: fetched.instruction.mnemonic(),
            operand_width,
            source_mode,
            destination_mode,
            taken,
            state: try_clone_state(&candidate, AllocationSite::FinalTraceState)?,
        };
        self.state = candidate;
        Ok(trace)
    }

    /// Run until HALT. Any failure rolls the complete machine back.
    pub fn run(&mut self, max_steps: usize) -> Result<Vec<StepTrace>, Msp430Error> {
        if max_steps > MAX_TRACES {
            return Err(Msp430Error::TraceLimitExceeded {
                requested: max_steps,
                maximum: MAX_TRACES,
            });
        }
        let entry = try_clone_state(&self.state, AllocationSite::CloneState)?;
        let mut traces = Vec::new();
        allocation_seam(AllocationSite::TraceRetention)?;
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| Msp430Error::AllocationFailure)?;
        if self.state.halted {
            return Ok(traces);
        }
        for step_index in 0..max_steps {
            let pc = self.state.registers[0];
            match self.step() {
                Ok(trace) => {
                    let halted = trace.state.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(traces);
                    }
                }
                Err(source) => {
                    self.state = entry;
                    return Err(Msp430Error::ExecutionFailed {
                        step: step_index,
                        pc,
                        source: Box::new(source),
                    });
                }
            }
        }
        self.state = entry;
        Err(Msp430Error::StepBudgetExhausted { maximum: max_steps })
    }

    /// Atomically load and run a program.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u16,
        max_steps: usize,
    ) -> Result<Vec<StepTrace>, Msp430Error> {
        let entry = try_clone_state(&self.state, AllocationSite::CloneState)?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(traces) => Ok(traces),
            Err(error) => {
                self.state = entry;
                Err(error)
            }
        }
    }

    fn fetch(&self) -> Result<Fetched, Msp430Error> {
        let pc = self.state.registers[0];
        if pc & 1 != 0 {
            return Err(Msp430Error::InvalidState { reason: "odd PC" });
        }
        let raw = self.fetch_word(pc, pc, true, 2)?;
        let instruction = decode(raw).ok_or(Msp430Error::UnknownInstruction { pc, raw })?;
        let extensions = extension_count(raw, instruction);
        let width = 2 + extensions * 2;
        let end = usize::from(pc) + width;
        if end > MEMORY_SIZE {
            return Err(Msp430Error::TruncatedInstruction {
                pc,
                width: width as u8,
            });
        }
        let mut bytes = Vec::new();
        allocation_seam(AllocationSite::FetchBytes)?;
        bytes
            .try_reserve_exact(width)
            .map_err(|_| Msp430Error::AllocationFailure)?;
        let mut words = Vec::new();
        allocation_seam(AllocationSite::FetchWords)?;
        words
            .try_reserve_exact(1 + extensions)
            .map_err(|_| Msp430Error::AllocationFailure)?;
        for offset in 0..=extensions {
            let address = pc.wrapping_add((offset * 2) as u16);
            let word = self.fetch_word(address, pc, offset == 0, width as u8)?;
            words.push(word);
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        Ok(Fetched {
            instruction,
            raw,
            bytes,
            words,
        })
    }

    fn fetch_word(
        &self,
        address: u16,
        first_pc: u16,
        first: bool,
        width: u8,
    ) -> Result<u16, Msp430Error> {
        let Some(origin) = self.state.origin else {
            return Err(Msp430Error::NoProgramInstalled);
        };
        let start = u32::from(origin);
        let end = start + self.state.length;
        let address_u32 = u32::from(address);
        if address_u32 < start || address_u32 + 2 > end {
            return Err(if first {
                Msp430Error::FetchOutsideProgram { pc: address }
            } else {
                Msp430Error::TruncatedInstruction {
                    pc: first_pc,
                    width,
                }
            });
        }
        read_word(&self.state, address)
    }
}

impl Default for Msp430 {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_state(state: &Msp430State) -> Result<(), Msp430Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(Msp430Error::InvalidState {
            reason: "memory must contain exactly 65,536 bytes",
        });
    }
    if state.registers[0] & 1 != 0 {
        return Err(Msp430Error::InvalidState { reason: "odd PC" });
    }
    if state.registers[1] & 1 != 0 {
        return Err(Msp430Error::InvalidState { reason: "odd SP" });
    }
    if state.registers[2] & !SR_MASK != 0 {
        return Err(Msp430Error::InvalidState {
            reason: "reserved SR bits are set",
        });
    }
    if state.registers[3] != 0 {
        return Err(Msp430Error::InvalidState {
            reason: "stored R3 must be zero",
        });
    }
    match state.origin {
        None if state.length != 0 => Err(Msp430Error::InvalidState {
            reason: "unloaded state has nonzero image length",
        }),
        None => Ok(()),
        Some(origin) => {
            if origin & 1 != 0 || state.length == 0 || state.length & 1 != 0 {
                return Err(Msp430Error::InvalidState {
                    reason: "installed image must be nonempty and word aligned",
                });
            }
            if u32::from(origin) + state.length > MEMORY_SIZE as u32 {
                return Err(Msp430Error::InvalidState {
                    reason: "installed image exceeds memory",
                });
            }
            Ok(())
        }
    }
}

fn try_clone_state(state: &Msp430State, site: AllocationSite) -> Result<Msp430State, Msp430Error> {
    let mut memory = Vec::new();
    allocation_seam(site)?;
    memory
        .try_reserve_exact(state.memory.len())
        .map_err(|_| Msp430Error::AllocationFailure)?;
    memory.extend_from_slice(&state.memory);
    Ok(Msp430State {
        registers: state.registers,
        memory,
        halted: state.halted,
        origin: state.origin,
        length: state.length,
    })
}

fn read_word(state: &Msp430State, address: u16) -> Result<u16, Msp430Error> {
    if address & 1 != 0 || address == u16::MAX {
        return Err(Msp430Error::MisalignedAccess { address });
    }
    let index = usize::from(address);
    Ok(u16::from_le_bytes([
        state.memory[index],
        state.memory[index + 1],
    ]))
}

fn write_word(state: &mut Msp430State, address: u16, value: u16) -> Result<(), Msp430Error> {
    if address & 1 != 0 || address == u16::MAX {
        return Err(Msp430Error::MisalignedAccess { address });
    }
    let index = usize::from(address);
    let bytes = value.to_le_bytes();
    state.memory[index] = bytes[0];
    state.memory[index + 1] = bytes[1];
    Ok(())
}

fn read_memory(state: &Msp430State, address: u16, width: OperandWidth) -> Result<u16, Msp430Error> {
    match width {
        OperandWidth::Byte => Ok(u16::from(state.memory[usize::from(address)])),
        OperandWidth::Word => read_word(state, address),
    }
}

fn write_memory(
    state: &mut Msp430State,
    address: u16,
    width: OperandWidth,
    value: u16,
) -> Result<(), Msp430Error> {
    match width {
        OperandWidth::Byte => {
            state.memory[usize::from(address)] = value as u8;
            Ok(())
        }
        OperandWidth::Word => write_word(state, address, value),
    }
}

fn read_register_value(state: &Msp430State, register: u8, width: OperandWidth) -> u16 {
    let value = if register == 3 {
        0
    } else {
        state.registers[usize::from(register)]
    };
    value & width.mask()
}

fn write_register_value(state: &mut Msp430State, register: u8, width: OperandWidth, value: u16) {
    if register == 3 {
        return;
    }
    let value = match width {
        OperandWidth::Byte => value & 0x00ff,
        OperandWidth::Word => value,
    };
    state.registers[usize::from(register)] = match register {
        0 | 1 => value & !1,
        2 => value & SR_MASK,
        _ => value,
    };
}

fn extension_count(word: u16, instruction: Instruction) -> usize {
    match instruction {
        Instruction::Mov
        | Instruction::Add
        | Instruction::Addc
        | Instruction::Subc
        | Instruction::Sub
        | Instruction::Cmp
        | Instruction::Dadd
        | Instruction::Bit
        | Instruction::Bic
        | Instruction::Bis
        | Instruction::Xor
        | Instruction::And => {
            let source = ((word >> 8) & 0xf) as u8;
            let as_bits = ((word >> 4) & 3) as u8;
            usize::from(source_needs_extension(source, as_bits)) + usize::from(word & 0x0080 != 0)
        }
        Instruction::Rrc
        | Instruction::Swpb
        | Instruction::Rra
        | Instruction::Sxt
        | Instruction::Push
        | Instruction::Call => {
            let register = (word & 0xf) as u8;
            let as_bits = ((word >> 4) & 3) as u8;
            usize::from(source_needs_extension(register, as_bits))
        }
        _ => 0,
    }
}

fn source_needs_extension(register: u8, as_bits: u8) -> bool {
    match (register, as_bits) {
        (3, _) | (2, 2 | 3) => false,
        (0, 3) => true,
        (_, 1) => true,
        _ => false,
    }
}

struct Extensions<'a> {
    words: &'a [u16],
    cursor: usize,
}

impl<'a> Extensions<'a> {
    fn new(words: &'a [u16]) -> Self {
        Self { words, cursor: 0 }
    }

    fn take(&mut self, state: &mut Msp430State) -> (u16, u16) {
        let address = state.registers[0];
        let word = self.words[self.cursor];
        self.cursor += 1;
        state.registers[0] = state.registers[0].wrapping_add(2);
        (address, word)
    }
}

fn signed_add(base: u16, extension: u16) -> u16 {
    base.wrapping_add(extension)
}

fn check_sr_byte(
    register: u8,
    mode: u8,
    width: OperandWidth,
    pc: u16,
    raw: u16,
) -> Result<(), Msp430Error> {
    if register == 2 && mode == 0 && width == OperandWidth::Byte {
        return Err(Msp430Error::UnpredictableOperand {
            pc,
            raw,
            reason: "byte register access to SR",
        });
    }
    Ok(())
}

fn resolve_source(
    state: &mut Msp430State,
    width: OperandWidth,
    register: u8,
    as_bits: u8,
    extensions: &mut Extensions<'_>,
    pc: u16,
    raw: u16,
) -> Result<(u16, AddressingMode), Msp430Error> {
    check_sr_byte(register, as_bits, width, pc, raw)?;
    let mask = width.mask();
    match (register, as_bits) {
        (3, 0) => Ok((0, AddressingMode::Constant)),
        (3, 1) => Ok((1, AddressingMode::Constant)),
        (3, 2) => Ok((2, AddressingMode::Constant)),
        (3, 3) => Ok((mask, AddressingMode::Constant)),
        (2, 2) => Ok((4, AddressingMode::Constant)),
        (2, 3) => Ok((8, AddressingMode::Constant)),
        (0, 3) => {
            let (_, value) = extensions.take(state);
            Ok((value & mask, AddressingMode::Immediate))
        }
        (0, 1) => {
            let (extension_address, extension) = extensions.take(state);
            let address = signed_add(extension_address, extension);
            Ok((
                read_memory(state, address, width)?,
                AddressingMode::Symbolic,
            ))
        }
        (2, 1) => {
            let (_, address) = extensions.take(state);
            Ok((
                read_memory(state, address, width)?,
                AddressingMode::Absolute,
            ))
        }
        (_, 0) => Ok((
            read_register_value(state, register, width),
            AddressingMode::Register,
        )),
        (_, 1) => {
            let (_, extension) = extensions.take(state);
            let address = signed_add(state.registers[usize::from(register)], extension);
            Ok((read_memory(state, address, width)?, AddressingMode::Indexed))
        }
        (_, 2) => {
            let address = state.registers[usize::from(register)];
            Ok((
                read_memory(state, address, width)?,
                AddressingMode::Indirect,
            ))
        }
        (_, 3) => {
            let address = state.registers[usize::from(register)];
            let value = read_memory(state, address, width)?;
            let increment = if register == 1 { 2 } else { width.bytes() };
            write_register_value(
                state,
                register,
                OperandWidth::Word,
                address.wrapping_add(increment),
            );
            Ok((value, AddressingMode::IndirectAutoincrement))
        }
        _ => unreachable!("two-bit As field"),
    }
}

fn resolve_destination(
    state: &mut Msp430State,
    width: OperandWidth,
    register: u8,
    ad: u8,
    extensions: &mut Extensions<'_>,
    pc: u16,
    raw: u16,
) -> Result<Destination, Msp430Error> {
    check_sr_byte(register, ad, width, pc, raw)?;
    if ad == 0 {
        return Ok(Destination {
            location: Location::Register(register),
            old: read_register_value(state, register, width),
            mode: AddressingMode::Register,
        });
    }
    let (extension_address, extension) = extensions.take(state);
    let (address, mode) = match register {
        0 => (
            signed_add(extension_address, extension),
            AddressingMode::Symbolic,
        ),
        2 => (extension, AddressingMode::Absolute),
        3 => (extension, AddressingMode::Indexed),
        _ => (
            signed_add(state.registers[usize::from(register)], extension),
            AddressingMode::Indexed,
        ),
    };
    Ok(Destination {
        location: Location::Memory(address),
        old: read_memory(state, address, width)?,
        mode,
    })
}

fn apply_flags(state: &mut Msp430State, update: FlagUpdate) {
    let sr = state.registers[2];
    state.registers[2] = ((sr & !update.mask) | (update.value & update.mask)) & SR_MASK;
}

fn write_destination(
    state: &mut Msp430State,
    destination: Destination,
    width: OperandWidth,
    value: u16,
    update: FlagUpdate,
) -> Result<(), Msp430Error> {
    match destination.location {
        Location::Register(2) if update.mask != 0 => {
            let base = value & SR_MASK;
            state.registers[2] = ((base & !update.mask) | (update.value & update.mask)) & SR_MASK;
        }
        Location::Register(register) => {
            write_register_value(state, register, width, value);
            apply_flags(state, update);
        }
        Location::Memory(address) => {
            write_memory(state, address, width, value)?;
            apply_flags(state, update);
        }
    }
    Ok(())
}

fn nz(width: OperandWidth, result: u16) -> FlagUpdate {
    let result = result & width.mask();
    FlagUpdate {
        mask: SR_N | SR_Z,
        value: if result == 0 { SR_Z } else { 0 }
            | if result & width.sign() != 0 { SR_N } else { 0 },
    }
}

fn nzcv(width: OperandWidth, result: u16, carry: bool, overflow: bool) -> FlagUpdate {
    let mut update = nz(width, result);
    update.mask |= SR_C | SR_V;
    if carry {
        update.value |= SR_C;
    }
    if overflow {
        update.value |= SR_V;
    }
    update
}

fn add_values(width: OperandWidth, left: u16, right: u16, carry_in: bool) -> (u16, bool, bool) {
    let mask = u32::from(width.mask());
    let sign = u32::from(width.sign());
    let left = u32::from(left & width.mask());
    let right = u32::from(right & width.mask());
    let sum = left + right + u32::from(carry_in);
    let result = sum & mask;
    let carry = sum > mask;
    let overflow = ((!(left ^ right) & (left ^ result)) & sign) != 0;
    (result as u16, carry, overflow)
}

fn sub_values(width: OperandWidth, left: u16, right: u16, borrow: bool) -> (u16, bool, bool) {
    let mask = u32::from(width.mask());
    let sign = u32::from(width.sign());
    let left_u = u32::from(left & width.mask());
    let right_u = u32::from(right & width.mask());
    let subtrahend = right_u + u32::from(borrow);
    let result = left_u.wrapping_sub(subtrahend) & mask;
    let no_borrow = left_u >= subtrahend;
    // MSP430 subtraction is A + !B + C.  V compares the original A/B sign
    // bits; folding the borrow into B would misclassify e.g. 0-127-1.
    let overflow = (((left_u ^ right_u) & (left_u ^ result)) & sign) != 0;
    (result as u16, no_borrow, overflow)
}

fn valid_bcd(width: OperandWidth, value: u16) -> bool {
    let digits = if width == OperandWidth::Byte { 2 } else { 4 };
    (0..digits).all(|digit| ((value >> (digit * 4)) & 0xf) <= 9)
}

fn dadd(width: OperandWidth, left: u16, right: u16, carry_in: bool) -> (u16, bool) {
    let digits = if width == OperandWidth::Byte { 2 } else { 4 };
    let mut result = 0_u16;
    let mut carry = u16::from(carry_in);
    for digit in 0..digits {
        let shift = digit * 4;
        let sum = ((left >> shift) & 0xf) + ((right >> shift) & 0xf) + carry;
        if sum >= 10 {
            result |= (sum - 10) << shift;
            carry = 1;
        } else {
            result |= sum << shift;
            carry = 0;
        }
    }
    (result, carry != 0)
}

fn execute_double(
    state: &mut Msp430State,
    pc: u16,
    raw: u16,
    instruction: Instruction,
    extension_words: &[u16],
) -> Result<(AddressingMode, AddressingMode), Msp430Error> {
    let width = if raw & 0x0040 != 0 {
        OperandWidth::Byte
    } else {
        OperandWidth::Word
    };
    let source_register = ((raw >> 8) & 0xf) as u8;
    let destination_register = (raw & 0xf) as u8;
    let as_bits = ((raw >> 4) & 3) as u8;
    let ad = ((raw >> 7) & 1) as u8;
    let mut extensions = Extensions::new(extension_words);
    let (source, source_mode) = resolve_source(
        state,
        width,
        source_register,
        as_bits,
        &mut extensions,
        pc,
        raw,
    )?;
    let destination = resolve_destination(
        state,
        width,
        destination_register,
        ad,
        &mut extensions,
        pc,
        raw,
    )?;
    let old = destination.old & width.mask();
    let carry_in = state.registers[2] & SR_C != 0;
    let (result, update, write) = match instruction {
        Instruction::Mov => (source, FlagUpdate::default(), true),
        Instruction::Add => {
            let (result, carry, overflow) = add_values(width, old, source, false);
            (result, nzcv(width, result, carry, overflow), true)
        }
        Instruction::Addc => {
            let (result, carry, overflow) = add_values(width, old, source, carry_in);
            (result, nzcv(width, result, carry, overflow), true)
        }
        Instruction::Subc => {
            let (result, no_borrow, overflow) = sub_values(width, old, source, !carry_in);
            (result, nzcv(width, result, no_borrow, overflow), true)
        }
        Instruction::Sub | Instruction::Cmp => {
            let (result, no_borrow, overflow) = sub_values(width, old, source, false);
            (
                result,
                nzcv(width, result, no_borrow, overflow),
                instruction == Instruction::Sub,
            )
        }
        Instruction::Dadd => {
            if !valid_bcd(width, source) || !valid_bcd(width, old) {
                return Err(Msp430Error::UndefinedBcdOperand { pc, raw });
            }
            let (result, carry) = dadd(width, old, source, carry_in);
            let mut update = nz(width, result);
            update.mask |= SR_C;
            if carry {
                update.value |= SR_C;
            }
            (result, update, true)
        }
        Instruction::Bit | Instruction::And => {
            let result = old & source;
            let mut update = nz(width, result);
            update.mask |= SR_C | SR_V;
            if result != 0 {
                update.value |= SR_C;
            }
            (result, update, instruction == Instruction::And)
        }
        Instruction::Bic => (old & !source, FlagUpdate::default(), true),
        Instruction::Bis => (old | source, FlagUpdate::default(), true),
        Instruction::Xor => {
            let result = old ^ source;
            let mut update = nz(width, result);
            update.mask |= SR_C | SR_V;
            if result != 0 {
                update.value |= SR_C;
            }
            if old & width.sign() != 0 && source & width.sign() != 0 {
                update.value |= SR_V;
            }
            (result, update, true)
        }
        _ => unreachable!("double-operand decoder family"),
    };
    if write {
        write_destination(state, destination, width, result, update)?;
    } else {
        apply_flags(state, update);
    }
    Ok((source_mode, destination.mode))
}

fn execute_single(
    state: &mut Msp430State,
    pc: u16,
    raw: u16,
    instruction: Instruction,
    extension_words: &[u16],
) -> Result<AddressingMode, Msp430Error> {
    let width = match instruction {
        Instruction::Swpb | Instruction::Sxt | Instruction::Call => OperandWidth::Word,
        _ if raw & 0x0040 != 0 => OperandWidth::Byte,
        _ => OperandWidth::Word,
    };
    let register = (raw & 0xf) as u8;
    let as_bits = ((raw >> 4) & 3) as u8;
    let modifies_operand = matches!(
        instruction,
        Instruction::Rrc | Instruction::Rra | Instruction::Swpb | Instruction::Sxt
    );
    if modifies_operand && register == 0 && as_bits == 3 {
        return Err(Msp430Error::UnpredictableOperand {
            pc,
            raw,
            reason: "immediate operand for modifying single-operand instruction",
        });
    }
    if modifies_operand && (register == 3 || (register == 2 && as_bits >= 2)) {
        return Err(Msp430Error::UnpredictableOperand {
            pc,
            raw,
            reason: "constant-generator operand is source-only",
        });
    }
    let mut extensions = Extensions::new(extension_words);
    let (source, mode, modifying_destination) = if modifies_operand {
        let destination =
            resolve_modifying_operand(state, width, register, as_bits, &mut extensions, pc, raw)?;
        (destination.old, destination.mode, Some(destination))
    } else {
        let (source, mode) =
            resolve_source(state, width, register, as_bits, &mut extensions, pc, raw)?;
        (source, mode, None)
    };
    match instruction {
        Instruction::Rrc | Instruction::Rra | Instruction::Swpb | Instruction::Sxt => {
            let destination = modifying_destination.expect("modifying instruction has destination");
            let old_carry = state.registers[2] & SR_C != 0;
            let (result, update) = match instruction {
                Instruction::Rrc => {
                    let result = (source >> 1) | if old_carry { width.sign() } else { 0 };
                    let mut update = nz(width, result);
                    update.mask |= SR_C | SR_V;
                    if source & 1 != 0 {
                        update.value |= SR_C;
                    }
                    (result, update)
                }
                Instruction::Rra => {
                    let result = (source >> 1) | (source & width.sign());
                    let mut update = nz(width, result);
                    update.mask |= SR_C | SR_V;
                    if source & 1 != 0 {
                        update.value |= SR_C;
                    }
                    (result, update)
                }
                Instruction::Swpb => (source.rotate_left(8), FlagUpdate::default()),
                Instruction::Sxt => {
                    let result = (source as u8 as i8 as i16) as u16;
                    let mut update = nz(OperandWidth::Word, result);
                    update.mask |= SR_C | SR_V;
                    if result != 0 {
                        update.value |= SR_C;
                    }
                    (result, update)
                }
                _ => unreachable!(),
            };
            write_destination(state, destination, width, result, update)?;
        }
        Instruction::Push => {
            let new_sp = state.registers[1].wrapping_sub(2) & !1;
            let value = if register == 1 && as_bits == 0 {
                new_sp & width.mask()
            } else {
                source
            };
            if width == OperandWidth::Word {
                write_word(state, new_sp, value)?;
            } else {
                state.memory[usize::from(new_sp)] = value as u8;
            }
            state.registers[1] = new_sp;
        }
        Instruction::Call => {
            let new_sp = state.registers[1].wrapping_sub(2) & !1;
            write_word(state, new_sp, state.registers[0])?;
            state.registers[1] = new_sp;
            state.registers[0] = source & !1;
        }
        _ => unreachable!("single-operand decoder family"),
    }
    Ok(mode)
}

fn resolve_modifying_operand(
    state: &mut Msp430State,
    width: OperandWidth,
    register: u8,
    as_bits: u8,
    extensions: &mut Extensions<'_>,
    pc: u16,
    raw: u16,
) -> Result<Destination, Msp430Error> {
    check_sr_byte(register, as_bits, width, pc, raw)?;
    let (location, mode, old) = match (register, as_bits) {
        (3, _) | (2, 2 | 3) => {
            unreachable!("constant-generator modifying operand was rejected")
        }
        (0, 3) => unreachable!("immediate modifying operand was rejected"),
        (0, 1) => {
            let (extension_address, extension) = extensions.take(state);
            let address = signed_add(extension_address, extension);
            (
                Location::Memory(address),
                AddressingMode::Symbolic,
                read_memory(state, address, width)?,
            )
        }
        (2, 1) => {
            let (_, address) = extensions.take(state);
            (
                Location::Memory(address),
                AddressingMode::Absolute,
                read_memory(state, address, width)?,
            )
        }
        (_, 0) => (
            Location::Register(register),
            AddressingMode::Register,
            read_register_value(state, register, width),
        ),
        (_, 1) => {
            let (_, extension) = extensions.take(state);
            let address = signed_add(state.registers[usize::from(register)], extension);
            (
                Location::Memory(address),
                AddressingMode::Indexed,
                read_memory(state, address, width)?,
            )
        }
        (_, 2) => {
            let address = state.registers[usize::from(register)];
            (
                Location::Memory(address),
                AddressingMode::Indirect,
                read_memory(state, address, width)?,
            )
        }
        (_, 3) => {
            let address = state.registers[usize::from(register)];
            let old = read_memory(state, address, width)?;
            let increment = if register == 1 { 2 } else { width.bytes() };
            write_register_value(
                state,
                register,
                OperandWidth::Word,
                address.wrapping_add(increment),
            );
            (
                Location::Memory(address),
                AddressingMode::IndirectAutoincrement,
                old,
            )
        }
        _ => unreachable!("two-bit As field"),
    };
    Ok(Destination {
        location,
        old,
        mode,
    })
}

fn execute_reti(state: &mut Msp430State) -> Result<(), Msp430Error> {
    let sp = state.registers[1];
    let restored_sr = read_word(state, sp)? & SR_MASK;
    let pc_address = sp.wrapping_add(2);
    let restored_pc = read_word(state, pc_address)? & !1;
    state.registers[2] = restored_sr;
    state.registers[0] = restored_pc;
    state.registers[1] = pc_address.wrapping_add(2) & !1;
    Ok(())
}

fn execute_jump(state: &mut Msp430State, raw: u16, instruction: Instruction) -> bool {
    let sr = state.registers[2];
    let c = sr & SR_C != 0;
    let z = sr & SR_Z != 0;
    let n = sr & SR_N != 0;
    let v = sr & SR_V != 0;
    let taken = match instruction {
        Instruction::Jne => !z,
        Instruction::Jeq => z,
        Instruction::Jnc => !c,
        Instruction::Jc => c,
        Instruction::Jn => n,
        Instruction::Jge => n == v,
        Instruction::Jl => n != v,
        Instruction::Jmp => true,
        _ => unreachable!("jump decoder family"),
    };
    if taken {
        let encoded = (raw & 0x03ff) as i16;
        let words = (encoded << 6) >> 6;
        let bytes = words.wrapping_mul(2) as u16;
        state.registers[0] = state.registers[0].wrapping_add(bytes) & !1;
    }
    taken
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    fn double(opcode: u16, source: u8, ad: u8, byte: bool, as_bits: u8, dest: u8) -> u16 {
        (opcode << 12)
            | (u16::from(source) << 8)
            | (u16::from(ad) << 7)
            | (u16::from(byte) << 6)
            | (u16::from(as_bits) << 4)
            | u16::from(dest)
    }

    fn single(base: u16, byte: bool, as_bits: u8, register: u8) -> u16 {
        base | (u16::from(byte) << 6) | (u16::from(as_bits) << 4) | u16::from(register)
    }

    fn jump(condition: u8, offset: i16) -> u16 {
        0x2000 | (u16::from(condition) << 10) | ((offset as u16) & 0x03ff)
    }

    fn loaded(program: &[u16]) -> Msp430 {
        let mut cpu = Msp430::new();
        cpu.load(&words(program), 0x0200).unwrap();
        cpu
    }

    fn expected_source_mode(register: u8, as_bits: u8) -> AddressingMode {
        match (register, as_bits) {
            (3, _) | (2, 2 | 3) => AddressingMode::Constant,
            (0, 1) => AddressingMode::Symbolic,
            (2, 1) => AddressingMode::Absolute,
            (0, 3) => AddressingMode::Immediate,
            (_, 0) => AddressingMode::Register,
            (_, 1) => AddressingMode::Indexed,
            (_, 2) => AddressingMode::Indirect,
            (_, 3) => AddressingMode::IndirectAutoincrement,
            _ => unreachable!(),
        }
    }

    fn expected_destination_mode(register: u8, ad: u8) -> AddressingMode {
        match (register, ad) {
            (_, 0) => AddressingMode::Register,
            (0, 1) => AddressingMode::Symbolic,
            (2, 1) => AddressingMode::Absolute,
            (_, 1) => AddressingMode::Indexed,
            _ => unreachable!(),
        }
    }

    fn source_extension(register: u8, as_bits: u8) -> Option<u16> {
        match (register, as_bits) {
            (3, _) | (2, 2 | 3) => None,
            (0, 1) => Some(0x1000),
            (0, 3) => Some(0x2468),
            (2, 1) => Some(0x4000),
            (_, 1) => Some(0),
            _ => None,
        }
    }

    fn seed_mode_matrix_cpu(program: &[u16]) -> Msp430 {
        let mut cpu = loaded(program);
        cpu.write_register(1, 0x4000).unwrap();
        for register in 4..16 {
            cpu.write_register(register, 0x4000).unwrap();
        }
        for address in (0x4000..=0x4020).step_by(2) {
            cpu.write_memory_word(address, 0x2222).unwrap();
        }
        for address in (0x1200..=0x1220).step_by(2) {
            cpu.write_memory_word(address, 0x2222).unwrap();
        }
        cpu
    }

    #[test]
    fn decoder_exhaustively_matches_independent_ranges() {
        let mut counts = BTreeMap::<&'static str, usize>::new();
        let mut unknown = 0;
        for word in 0_u16..=u16::MAX {
            let expected = if word == 0 {
                Some("HALT")
            } else if (4..=15).contains(&(word >> 12)) {
                Some(
                    [
                        "MOV", "ADD", "ADDC", "SUBC", "SUB", "CMP", "DADD", "BIT", "BIC", "BIS",
                        "XOR", "AND",
                    ][usize::from((word >> 12) - 4)],
                )
            } else if (0x2000..=0x3fff).contains(&word) {
                Some(
                    ["JNE", "JEQ", "JNC", "JC", "JN", "JGE", "JL", "JMP"]
                        [usize::from((word >> 10) & 7)],
                )
            } else if word == 0x1300 {
                Some("RETI")
            } else if word & 0xff80 == 0x1000 {
                Some("RRC")
            } else if word & 0xffc0 == 0x1080 {
                Some("SWPB")
            } else if word & 0xff80 == 0x1100 {
                Some("RRA")
            } else if word & 0xffc0 == 0x1180 {
                Some("SXT")
            } else if word & 0xff80 == 0x1200 {
                Some("PUSH")
            } else if word & 0xffc0 == 0x1280 {
                Some("CALL")
            } else {
                None
            };
            assert_eq!(
                decode(word).map(Instruction::mnemonic),
                expected,
                "{word:04x}"
            );
            if let Some(name) = expected {
                *counts.entry(name).or_default() += 1;
            } else {
                unknown += 1;
            }
        }
        assert_eq!(unknown, 7_614);
        assert_eq!(counts["MOV"], 4_096);
        assert_eq!(counts["JMP"], 1_024);
        assert_eq!(counts["RRC"], 128);
        assert_eq!(counts["SWPB"], 64);
        assert_eq!(counts["RETI"], 1);
        assert_eq!(counts["HALT"], 1);
    }

    #[test]
    fn format_i_mode_width_and_extension_matrix_is_exhaustive() {
        for source in 0..16_u8 {
            for as_bits in 0..4_u8 {
                for destination in 0..16_u8 {
                    for ad in 0..2_u8 {
                        for byte in [false, true] {
                            let raw = double(4, source, ad, byte, as_bits, destination);
                            let mut program = vec![raw];
                            if let Some(extension) = source_extension(source, as_bits) {
                                program.push(extension);
                            }
                            if ad == 1 {
                                program.push(match destination {
                                    0 => 0x1000,
                                    2 | 3 => 0x4000,
                                    _ => 0,
                                });
                            }
                            program.push(HALT_WORD);
                            let mut cpu = seed_mode_matrix_cpu(&program);
                            let entry = cpu.snapshot();
                            let result = cpu.step();
                            let sr_byte = byte
                                && ((source == 2 && as_bits == 0) || (destination == 2 && ad == 0));
                            if sr_byte {
                                assert!(matches!(
                                    result,
                                    Err(Msp430Error::UnpredictableOperand { .. })
                                ));
                                assert_eq!(cpu.snapshot(), entry);
                                continue;
                            }
                            let trace = result.unwrap_or_else(|error| {
                                panic!(
                                    "MOV matrix failed: src={source} As={as_bits} dst={destination} Ad={ad} byte={byte}: {error}"
                                )
                            });
                            assert_eq!(
                                trace.source_mode,
                                Some(expected_source_mode(source, as_bits))
                            );
                            assert_eq!(
                                trace.destination_mode,
                                Some(expected_destination_mode(destination, ad))
                            );
                            let expected_extensions =
                                usize::from(source_extension(source, as_bits).is_some())
                                    + usize::from(ad == 1);
                            assert_eq!(trace.width, (2 + expected_extensions * 2) as u8);
                            assert_eq!(trace.raw_words, program[..=expected_extensions]);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn format_ii_addressing_matrix_covers_all_included_families() {
        let families = [
            (0x1000, Instruction::Rrc, true),
            (0x1080, Instruction::Swpb, false),
            (0x1100, Instruction::Rra, true),
            (0x1180, Instruction::Sxt, false),
            (0x1200, Instruction::Push, true),
            (0x1280, Instruction::Call, false),
        ];
        for (base, instruction, byte_capable) in families {
            for register in 0..16_u8 {
                for as_bits in 0..4_u8 {
                    for byte in [false, true] {
                        if byte && !byte_capable {
                            continue;
                        }
                        let raw = single(base, byte, as_bits, register);
                        assert_eq!(decode(raw), Some(instruction));
                        let mut program = vec![raw];
                        if let Some(extension) = source_extension(register, as_bits) {
                            program.push(extension);
                        }
                        program.push(HALT_WORD);
                        let mut cpu = seed_mode_matrix_cpu(&program);
                        cpu.write_register(2, SR_C).unwrap();
                        let entry = cpu.snapshot();
                        let result = cpu.step();
                        let modifying = matches!(
                            instruction,
                            Instruction::Rrc
                                | Instruction::Rra
                                | Instruction::Swpb
                                | Instruction::Sxt
                        );
                        let modifying_immediate = modifying && register == 0 && as_bits == 3;
                        let modifying_constant =
                            modifying && (register == 3 || (register == 2 && as_bits >= 2));
                        let sr_byte = byte && register == 2 && as_bits == 0;
                        if modifying_immediate || modifying_constant || sr_byte {
                            assert!(matches!(
                                result,
                                Err(Msp430Error::UnpredictableOperand { .. })
                            ));
                            assert_eq!(cpu.snapshot(), entry);
                        } else {
                            let trace = result.unwrap_or_else(|error| {
                                panic!(
                                    "Format II matrix failed: {instruction:?} r={register} As={as_bits} byte={byte}: {error}"
                                )
                            });
                            let expected_mode = Some(expected_source_mode(register, as_bits));
                            if modifying {
                                assert_eq!(trace.source_mode, None);
                                assert_eq!(trace.destination_mode, expected_mode);
                            } else {
                                assert_eq!(trace.source_mode, expected_mode);
                                assert_eq!(trace.destination_mode, None);
                            }
                            assert_eq!(
                                trace.width,
                                if source_extension(register, as_bits).is_some() {
                                    4
                                } else {
                                    2
                                }
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn format_ii_constant_generator_is_source_only_and_atomic() {
        let modifying_families = [0x1000, 0x1080, 0x1100, 0x1180];
        let constant_forms = [(2_u8, 2_u8), (2, 3), (3, 0), (3, 1), (3, 2), (3, 3)];
        for base in modifying_families {
            for (register, as_bits) in constant_forms {
                let raw = single(base, false, as_bits, register);
                let mut cpu = loaded(&[raw, HALT_WORD]);
                cpu.write_register(2, SR_C | SR_N | SR_V).unwrap();
                cpu.write_memory_word(0x4000, 0xa55a).unwrap();
                let entry = cpu.snapshot();
                assert!(matches!(
                    cpu.step(),
                    Err(Msp430Error::UnpredictableOperand {
                        reason: "constant-generator operand is source-only",
                        ..
                    })
                ));
                assert_eq!(cpu.snapshot(), entry, "raw={raw:04x}");
            }
        }

        // The adjacent writable forms remain destinations: R2/As=01 is
        // absolute memory and an ordinary R4 supports all four modes.
        for (register, as_bits, extension, expected_mode) in [
            (2_u8, 1_u8, Some(0x4000), AddressingMode::Absolute),
            (4, 0, None, AddressingMode::Register),
            (4, 1, Some(0), AddressingMode::Indexed),
            (4, 2, None, AddressingMode::Indirect),
            (4, 3, None, AddressingMode::IndirectAutoincrement),
        ] {
            let raw = single(0x1000, false, as_bits, register);
            let mut program = vec![raw];
            program.extend(extension);
            program.push(HALT_WORD);
            let mut cpu = seed_mode_matrix_cpu(&program);
            let trace = cpu.step().unwrap();
            assert_eq!(trace.source_mode, None);
            assert_eq!(trace.destination_mode, Some(expected_mode));
        }

        // All six generator encodings remain legal sources for PUSH and CALL.
        for base in [0x1200, 0x1280] {
            for (register, as_bits) in constant_forms {
                let raw = single(base, false, as_bits, register);
                let mut cpu = loaded(&[raw, HALT_WORD]);
                cpu.write_register(1, 0x4000).unwrap();
                let trace = cpu.step().unwrap();
                assert_eq!(trace.source_mode, Some(AddressingMode::Constant));
                assert_eq!(trace.destination_mode, None);
            }
        }
    }

    #[test]
    fn lifecycle_is_canonical_and_atomic() {
        let mut cpu = Msp430::new();
        let pristine = cpu.snapshot();
        assert_eq!(cpu.load(&[], 0), Err(Msp430Error::EmptyProgram));
        assert_eq!(cpu.snapshot(), pristine);
        assert_eq!(
            cpu.load(&[0], 0),
            Err(Msp430Error::OddProgramLength { length: 1 })
        );
        assert_eq!(
            cpu.load(&[0, 0], 1),
            Err(Msp430Error::MisalignedProgram { origin: 1 })
        );
        assert!(matches!(
            cpu.load(&[0, 0, 0, 0], 0xfffe),
            Err(Msp430Error::ProgramOutOfRange { .. })
        ));
        assert_eq!(cpu.step(), Err(Msp430Error::NoProgramInstalled));

        cpu.write_register(4, 0xaaaa).unwrap();
        cpu.write_memory_byte(0x5000, 0xbb);
        cpu.load(&words(&[HALT_WORD]), 0x0200).unwrap();
        assert_eq!(cpu.read_register(4).unwrap(), 0);
        assert_eq!(cpu.read_memory_byte(0x5000), 0);
        let traces = cpu.run(1).unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].fetched_bytes, vec![0, 0]);
        assert_eq!(traces[0].pc_after, 0x0202);
        assert!(cpu.snapshot().halted);
        assert_eq!(cpu.step(), Err(Msp430Error::Halted));
        assert!(cpu.run(0).unwrap().is_empty());
        cpu.reset();
        assert_eq!(cpu.snapshot(), Msp430State::default());
    }

    #[test]
    fn restore_rejects_every_noncanonical_shape_without_mutation() {
        let mut cpu = loaded(&[HALT_WORD]);
        let entry = cpu.snapshot();
        let mut cases = Vec::new();
        let mut bad = entry.clone();
        bad.registers[0] |= 1;
        cases.push(bad);
        let mut bad = entry.clone();
        bad.memory.pop();
        cases.push(bad);
        let mut bad = entry.clone();
        bad.registers[1] = 1;
        cases.push(bad);
        let mut bad = entry.clone();
        bad.registers[2] = 0x8000;
        cases.push(bad);
        let mut bad = entry.clone();
        bad.registers[3] = 1;
        cases.push(bad);
        let mut bad = entry.clone();
        bad.origin = None;
        cases.push(bad);
        let mut bad = entry.clone();
        bad.length = 3;
        cases.push(bad);
        let mut bad = entry.clone();
        bad.origin = Some(0x0201);
        cases.push(bad);
        let mut bad = entry.clone();
        bad.length = 0;
        cases.push(bad);
        let mut bad = entry.clone();
        bad.origin = Some(0xfffe);
        bad.length = 4;
        cases.push(bad);
        for bad in cases {
            assert!(cpu.restore(&bad).is_err());
            assert_eq!(cpu.snapshot(), entry);
        }
        cpu.restore(&entry).unwrap();
    }

    #[test]
    fn register_and_memory_helpers_enforce_architecture() {
        let mut cpu = Msp430::new();
        cpu.write_register(4, 0xabcd).unwrap();
        assert_eq!(cpu.read_register(4).unwrap(), 0xabcd);
        cpu.write_register(0, 0x1235).unwrap();
        cpu.write_register(1, 0x4567).unwrap();
        cpu.write_register(2, 0xffff).unwrap();
        cpu.write_register(3, 0xffff).unwrap();
        assert_eq!(cpu.read_register(0).unwrap(), 0x1234);
        assert_eq!(cpu.read_register(1).unwrap(), 0x4566);
        assert_eq!(cpu.read_register(2).unwrap(), SR_MASK);
        assert_eq!(cpu.read_register(3).unwrap(), 0);
        assert!(matches!(
            cpu.read_register(16),
            Err(Msp430Error::InvalidRegister { .. })
        ));
        assert!(cpu.write_register(16, 0).is_err());
        cpu.write_memory_byte(0xffff, 0xaa);
        assert_eq!(cpu.read_memory_byte(0xffff), 0xaa);
        cpu.write_memory_word(0x1000, 0x1234).unwrap();
        assert_eq!(cpu.read_memory_word(0x1000).unwrap(), 0x1234);
        assert!(cpu.read_memory_word(1).is_err());
        assert!(cpu.write_memory_word(0xffff, 0).is_err());
    }

    #[test]
    fn mov_covers_register_constants_immediate_and_byte_canonicalization() {
        let program = [
            double(4, 0, 0, false, 3, 4),
            0xbeef,
            double(4, 4, 0, true, 0, 5),
            double(4, 3, 0, false, 1, 6),
            double(4, 2, 0, false, 2, 7),
            HALT_WORD,
        ];
        let mut cpu = loaded(&program);
        cpu.run(6).unwrap();
        let state = cpu.snapshot();
        assert_eq!(state.registers[4], 0xbeef);
        assert_eq!(state.registers[5], 0x00ef);
        assert_eq!(state.registers[6], 1);
        assert_eq!(state.registers[7], 4);

        let constants = [
            (3, 0, 0x0000),
            (3, 1, 0x0001),
            (3, 2, 0x0002),
            (3, 3, 0xffff),
            (2, 2, 0x0004),
            (2, 3, 0x0008),
        ];
        for (register, as_bits, expected) in constants {
            for byte in [false, true] {
                let mut cpu = loaded(&[double(4, register, 0, byte, as_bits, 4), HALT_WORD]);
                cpu.step().unwrap();
                assert_eq!(
                    cpu.read_register(4).unwrap(),
                    if byte { expected & 0xff } else { expected }
                );
            }
        }
    }

    #[test]
    fn indexed_symbolic_absolute_indirect_and_autoincrement_are_ordered() {
        let program = [
            double(4, 5, 0, false, 1, 6),
            4,
            double(4, 0, 0, false, 1, 7),
            0x0100,
            double(4, 2, 0, false, 1, 8),
            0x3000,
            double(4, 9, 0, false, 2, 10),
            double(4, 9, 0, false, 3, 11),
            HALT_WORD,
        ];
        let mut cpu = loaded(&program);
        cpu.write_register(5, 0x2ffc).unwrap();
        cpu.write_register(9, 0x3000).unwrap();
        cpu.write_memory_word(0x3000, 0x1111).unwrap();
        cpu.write_memory_word(0x0306, 0x2222).unwrap();
        cpu.run(6).unwrap();
        let state = cpu.snapshot();
        assert_eq!(state.registers[6], 0x1111);
        assert_eq!(state.registers[7], 0x2222);
        assert_eq!(state.registers[8], 0x1111);
        assert_eq!(state.registers[10], 0x1111);
        assert_eq!(state.registers[11], 0x1111);
        assert_eq!(state.registers[9], 0x3002);
    }

    #[test]
    fn destination_extension_order_and_overlap_use_candidate_state() {
        // MOV @R5+, -2(R5): source increments R5 before destination address.
        let instruction = double(4, 5, 1, false, 3, 5);
        let mut cpu = loaded(&[instruction, 0xfffe, HALT_WORD]);
        cpu.write_register(5, 0x3000).unwrap();
        cpu.write_memory_word(0x3000, 0xcafe).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.width, 4);
        assert_eq!(cpu.read_register(5).unwrap(), 0x3002);
        assert_eq!(cpu.read_memory_word(0x3000).unwrap(), 0xcafe);

        // Source and destination symbolic forms use their own extension address.
        let instruction = double(4, 0, 1, false, 1, 0);
        let mut cpu = loaded(&[instruction, 0x0100, 0x0200, HALT_WORD]);
        cpu.write_memory_word(0x0302, 0x4567).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.read_memory_word(0x0404).unwrap(), 0x4567);
    }

    #[test]
    fn arithmetic_helpers_are_exhaustive_for_byte_add_and_sub() {
        for left in 0_u16..=255 {
            for right in 0_u16..=255 {
                for carry_in in [false, true] {
                    let (result, carry, overflow) =
                        add_values(OperandWidth::Byte, left, right, carry_in);
                    let expected = u32::from(left) + u32::from(right) + u32::from(carry_in);
                    assert_eq!(result, expected as u8 as u16);
                    assert_eq!(carry, expected > 255);
                    let signed = i16::from(left as u8 as i8)
                        + i16::from(right as u8 as i8)
                        + i16::from(carry_in);
                    assert_eq!(overflow, !(-128..=127).contains(&signed));

                    let (result, no_borrow, overflow) =
                        sub_values(OperandWidth::Byte, left, right, carry_in);
                    let expected = i32::from(left) - i32::from(right) - i32::from(carry_in);
                    assert_eq!(result, (expected as u32 & 0xff) as u16);
                    assert_eq!(no_borrow, expected >= 0);
                    let signed = i16::from(left as u8 as i8)
                        - i16::from(right as u8 as i8)
                        - i16::from(carry_in);
                    assert_eq!(overflow, !(-128..=127).contains(&signed));
                }
            }
        }

        let boundaries = [0, 1, 0x7ffe, 0x7fff, 0x8000, 0x8001, 0xfffe, 0xffff];
        for left in boundaries {
            for right in boundaries {
                for carry_in in [false, true] {
                    let (result, carry, overflow) =
                        add_values(OperandWidth::Word, left, right, carry_in);
                    let unsigned = u32::from(left) + u32::from(right) + u32::from(carry_in);
                    assert_eq!(result, unsigned as u16);
                    assert_eq!(carry, unsigned > 0xffff);
                    let signed =
                        i32::from(left as i16) + i32::from(right as i16) + i32::from(carry_in);
                    assert_eq!(overflow, !(-32_768..=32_767).contains(&signed));

                    let (result, no_borrow, overflow) =
                        sub_values(OperandWidth::Word, left, right, carry_in);
                    let unsigned = i64::from(left) - i64::from(right) - i64::from(carry_in);
                    assert_eq!(result, unsigned as u16);
                    assert_eq!(no_borrow, unsigned >= 0);
                    let signed =
                        i32::from(left as i16) - i32::from(right as i16) - i32::from(carry_in);
                    assert_eq!(overflow, !(-32_768..=32_767).contains(&signed));
                }
            }
        }
    }

    #[test]
    fn packed_bcd_byte_domain_and_named_v_policy_are_exact() {
        for left_decimal in 0..100_u16 {
            let left = (left_decimal / 10) * 16 + left_decimal % 10;
            for right_decimal in 0..100_u16 {
                let right = (right_decimal / 10) * 16 + right_decimal % 10;
                for carry_in in [false, true] {
                    let (result, carry) = dadd(OperandWidth::Byte, left, right, carry_in);
                    let decimal = left_decimal + right_decimal + u16::from(carry_in);
                    let reduced = decimal % 100;
                    let expected = (reduced / 10) * 16 + reduced % 10;
                    assert_eq!(result, expected);
                    assert_eq!(carry, decimal >= 100);
                }
            }
        }

        for value in 0_u16..=u8::MAX.into() {
            assert_eq!(
                valid_bcd(OperandWidth::Byte, value),
                value & 0x0f <= 9 && value >> 4 <= 9
            );
        }
        let mut cpu = loaded(&[double(10, 0, 0, false, 3, 4), 0, HALT_WORD]);
        cpu.write_register(2, SR_V).unwrap();
        cpu.step().unwrap();
        assert_ne!(cpu.read_register(2).unwrap() & SR_V, 0);
    }

    #[test]
    fn all_double_families_update_results_and_flags() {
        let families = [
            (5, 1),
            (6, 1),
            (7, 0),
            (8, 0),
            (9, 0),
            (11, 0),
            (12, 0),
            (13, 0),
            (14, 0),
            (15, 0),
        ];
        for (opcode, carry) in families {
            let mut cpu = loaded(&[double(opcode, 0, 0, false, 3, 4), 1, HALT_WORD]);
            cpu.write_register(4, 2).unwrap();
            cpu.write_register(2, carry).unwrap();
            cpu.step().unwrap();
        }

        let mut cpu = loaded(&[double(10, 0, 0, false, 3, 4), 0x0001, HALT_WORD]);
        cpu.write_register(4, 0x9999).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.read_register(4).unwrap(), 0x0000);
        assert_ne!(cpu.read_register(2).unwrap() & SR_C, 0);

        let mut invalid = loaded(&[double(10, 0, 0, false, 3, 4), 0x000a]);
        invalid.write_register(4, 0).unwrap();
        let entry = invalid.snapshot();
        assert!(matches!(
            invalid.step(),
            Err(Msp430Error::UndefinedBcdOperand { .. })
        ));
        assert_eq!(invalid.snapshot(), entry);
    }

    #[test]
    fn status_destination_merges_alu_flags_and_result_bits() {
        let mut cpu = loaded(&[double(5, 0, 0, false, 3, 2), 1, HALT_WORD]);
        cpu.write_register(2, 0x0101).unwrap();
        cpu.step().unwrap();
        let sr = cpu.read_register(2).unwrap();
        assert_eq!(sr & SR_V, 0);
        assert_eq!(sr & SR_C, 0);

        let mut byte = loaded(&[double(4, 4, 0, true, 0, 2)]);
        let entry = byte.snapshot();
        assert!(matches!(
            byte.step(),
            Err(Msp430Error::UnpredictableOperand { .. })
        ));
        assert_eq!(byte.snapshot(), entry);
    }

    #[test]
    fn single_operand_register_memory_and_immediate_boundaries() {
        let program = [
            single(0x1000, false, 0, 4),
            single(0x1100, true, 0, 5),
            single(0x1080, false, 0, 6),
            single(0x1180, false, 0, 7),
            HALT_WORD,
        ];
        let mut cpu = loaded(&program);
        cpu.write_register(4, 1).unwrap();
        cpu.write_register(5, 0x0081).unwrap();
        cpu.write_register(6, 0x1234).unwrap();
        cpu.write_register(7, 0x0080).unwrap();
        cpu.write_register(2, SR_C).unwrap();
        cpu.run(5).unwrap();
        let state = cpu.snapshot();
        assert_eq!(state.registers[4], 0x8000);
        assert_eq!(state.registers[5], 0x00c0);
        assert_eq!(state.registers[6], 0x3412);
        assert_eq!(state.registers[7], 0xff80);

        for base in [0x1000, 0x1080, 0x1100, 0x1180] {
            let mut cpu = loaded(&[single(base, false, 3, 0), 1]);
            let entry = cpu.snapshot();
            assert!(matches!(
                cpu.step(),
                Err(Msp430Error::UnpredictableOperand { .. })
            ));
            assert_eq!(cpu.snapshot(), entry);
        }
    }

    #[test]
    fn shift_flags_match_independent_byte_and_word_oracles() {
        let byte_values = 0_u16..=u16::from(u8::MAX);
        for value in byte_values {
            for carry_in in [false, true] {
                for (base, rotate) in [(0x1000, true), (0x1100, false)] {
                    let raw = single(base, true, 0, 4);
                    let mut state = Msp430State::default();
                    state.registers[4] = value;
                    state.registers[2] = 0x00f0 | SR_V | if carry_in { SR_C } else { 0 };
                    execute_single(&mut state, 0x0200, raw, decode(raw).unwrap(), &[]).unwrap();
                    let expected = if rotate {
                        (value >> 1) | if carry_in { 0x80 } else { 0 }
                    } else {
                        (value >> 1) | (value & 0x80)
                    };
                    assert_eq!(state.registers[4], expected);
                    let expected_flags = if expected == 0 { SR_Z } else { 0 }
                        | if expected & 0x80 != 0 { SR_N } else { 0 }
                        | if value & 1 != 0 { SR_C } else { 0 };
                    assert_eq!(
                        state.registers[2] & (SR_C | SR_Z | SR_N | SR_V),
                        expected_flags
                    );
                    assert_eq!(state.registers[2] & 0x00f0, 0x00f0);
                }
            }
        }

        for value in [0, 1, 2, 0x7fff, 0x8000, 0x8001, 0xfffe, 0xffff] {
            for carry_in in [false, true] {
                for (base, rotate) in [(0x1000, true), (0x1100, false)] {
                    let raw = single(base, false, 0, 4);
                    let mut state = Msp430State::default();
                    state.registers[4] = value;
                    state.registers[2] = if carry_in { SR_C } else { 0 };
                    execute_single(&mut state, 0x0200, raw, decode(raw).unwrap(), &[]).unwrap();
                    let expected = if rotate {
                        (value >> 1) | if carry_in { 0x8000 } else { 0 }
                    } else {
                        (value >> 1) | (value & 0x8000)
                    };
                    assert_eq!(state.registers[4], expected);
                    let expected_flags = if expected == 0 { SR_Z } else { 0 }
                        | if expected & 0x8000 != 0 { SR_N } else { 0 }
                        | if value & 1 != 0 { SR_C } else { 0 };
                    assert_eq!(
                        state.registers[2] & (SR_C | SR_Z | SR_N | SR_V),
                        expected_flags
                    );
                }
            }
        }
    }

    #[test]
    fn modifying_single_operand_modes_write_back_to_original_location() {
        let mut cpu = loaded(&[single(0x1000, false, 1, 5), 4, HALT_WORD]);
        cpu.write_register(5, 0x3000).unwrap();
        cpu.write_memory_word(0x3004, 2).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.read_memory_word(0x3004).unwrap(), 1);

        let mut cpu = loaded(&[single(0x1100, false, 3, 5), HALT_WORD]);
        cpu.write_register(5, 0x3000).unwrap();
        cpu.write_memory_word(0x3000, 4).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.read_memory_word(0x3000).unwrap(), 2);
        assert_eq!(cpu.read_register(5).unwrap(), 0x3002);
    }

    #[test]
    fn push_call_and_reti_preserve_special_ordering() {
        let mut cpu = loaded(&[
            single(0x1200, false, 0, 1),
            single(0x1280, false, 3, 0),
            0x0300,
            HALT_WORD,
        ]);
        cpu.write_register(1, 0x4000).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.read_register(1).unwrap(), 0x3ffe);
        assert_eq!(cpu.read_memory_word(0x3ffe).unwrap(), 0x3ffe);
        cpu.step().unwrap();
        assert_eq!(cpu.read_register(0).unwrap(), 0x0300);
        assert_eq!(cpu.read_memory_word(0x3ffc).unwrap(), 0x0206);

        let mut cpu = loaded(&[0x1300, HALT_WORD]);
        cpu.write_register(1, 0xfffe).unwrap();
        cpu.write_memory_word(0xfffe, 0x0101).unwrap();
        cpu.write_memory_word(0x0000, 0x0223).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.read_register(2).unwrap(), 0x0101);
        assert_eq!(cpu.read_register(0).unwrap(), 0x0222);
        assert_eq!(cpu.read_register(1).unwrap(), 0x0002);
    }

    #[test]
    fn push_byte_leaves_adjacent_byte_unchanged() {
        let mut cpu = loaded(&[single(0x1200, true, 0, 4), HALT_WORD]);
        cpu.write_register(1, 0x3002).unwrap();
        cpu.write_register(4, 0x12ab).unwrap();
        cpu.write_memory_byte(0x3001, 0x77);
        cpu.step().unwrap();
        assert_eq!(cpu.read_register(1).unwrap(), 0x3000);
        assert_eq!(cpu.read_memory_byte(0x3000), 0xab);
        assert_eq!(cpu.read_memory_byte(0x3001), 0x77);
    }

    #[test]
    fn every_jump_condition_and_displacement_boundary_is_exact() {
        for condition in 0..8_u8 {
            for bits in 0..16_u16 {
                let flags = ((bits & 1) * SR_C)
                    | (((bits >> 1) & 1) * SR_Z)
                    | (((bits >> 2) & 1) * SR_N)
                    | (((bits >> 3) & 1) * SR_V);
                let c = flags & SR_C != 0;
                let z = flags & SR_Z != 0;
                let n = flags & SR_N != 0;
                let v = flags & SR_V != 0;
                let expected = match condition {
                    0 => !z,
                    1 => z,
                    2 => !c,
                    3 => c,
                    4 => n,
                    5 => n == v,
                    6 => n != v,
                    7 => true,
                    _ => unreachable!(),
                };
                let mut cpu = loaded(&[jump(condition, 1), HALT_WORD, HALT_WORD]);
                cpu.write_register(2, flags).unwrap();
                let trace = cpu.step().unwrap();
                assert_eq!(trace.taken, Some(expected));
                assert_eq!(trace.pc_after, if expected { 0x0204 } else { 0x0202 });
            }
        }
        for offset in [-512, -511, -1, 0, 1, 511] {
            let mut cpu = loaded(&[jump(7, offset), HALT_WORD]);
            let trace = cpu.step().unwrap();
            assert_eq!(trace.pc_after, 0x0202_u16.wrapping_add((offset * 2) as u16));
        }
    }

    #[test]
    fn fetch_width_truncation_unknown_and_terminal_pc_are_atomic() {
        let mut truncated = loaded(&[double(4, 0, 0, false, 3, 4)]);
        let entry = truncated.snapshot();
        assert!(matches!(
            truncated.step(),
            Err(Msp430Error::TruncatedInstruction { width: 4, .. })
        ));
        assert_eq!(truncated.snapshot(), entry);

        let mut unknown = loaded(&[0x0001]);
        assert!(matches!(
            unknown.step(),
            Err(Msp430Error::UnknownInstruction { .. })
        ));

        let mut terminal = Msp430::new();
        terminal
            .load(&words(&[double(4, 4, 0, false, 0, 4)]), 0xfffe)
            .unwrap();
        terminal.step().unwrap();
        assert_eq!(terminal.read_register(0).unwrap(), 0);
        assert!(matches!(
            terminal.step(),
            Err(Msp430Error::FetchOutsideProgram { pc: 0 })
        ));
    }

    #[test]
    fn self_modifying_trace_preserves_prefetched_bytes() {
        // MOV #0, &0x0200 overwrites its own first word after all six bytes
        // have already been copied to the trace.
        let instruction = double(4, 0, 1, false, 3, 2);
        let image = words(&[instruction, 0, 0x0200, HALT_WORD]);
        let mut cpu = Msp430::new();
        cpu.load(&image, 0x0200).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.fetched_bytes, image[..6]);
        assert_eq!(cpu.read_memory_word(0x0200).unwrap(), 0);
    }

    #[test]
    fn run_and_execute_roll_back_all_failures() {
        let mut cpu = loaded(&[double(4, 0, 0, false, 3, 4), 1, 0x0001]);
        let entry = cpu.snapshot();
        let error = cpu.run(2).unwrap_err();
        assert!(matches!(
            error,
            Msp430Error::ExecutionFailed { step: 1, .. }
        ));
        assert_eq!(cpu.snapshot(), entry);
        assert!(matches!(
            cpu.run(MAX_TRACES + 1),
            Err(Msp430Error::TraceLimitExceeded { .. })
        ));
        assert!(matches!(
            cpu.run(1),
            Err(Msp430Error::StepBudgetExhausted { maximum: 1 })
        ));
        assert_eq!(cpu.snapshot(), entry);

        let mut other = Msp430::new();
        other.write_register(4, 0xaaaa).unwrap();
        let before = other.snapshot();
        assert!(other.execute(&words(&[0x0001]), 0x0200, 1).is_err());
        assert_eq!(other.snapshot(), before);
        let traces = other
            .execute(
                &words(&[double(4, 3, 0, false, 1, 4), HALT_WORD]),
                0x0200,
                2,
            )
            .unwrap();
        assert_eq!(traces.len(), 2);
        assert_eq!(other.read_register(4).unwrap(), 1);
        assert!(other.snapshot().halted);
    }

    #[test]
    fn allocation_failure_seams_are_deterministic_and_atomic() {
        let halt = words(&[HALT_WORD]);

        let mut load_cpu = Msp430::new();
        load_cpu.write_register(4, 0xa55a).unwrap();
        load_cpu.write_memory_byte(0x4000, 0x5a);
        let load_entry = load_cpu.snapshot();
        fail_next_allocation(AllocationSite::LoadMemory);
        assert_eq!(
            load_cpu.load(&halt, 0x0200),
            Err(Msp430Error::AllocationFailure)
        );
        assert_eq!(load_cpu.snapshot(), load_entry);

        let mut restore_cpu = loaded(&[HALT_WORD]);
        restore_cpu.write_register(4, 0x1111).unwrap();
        let restore_entry = restore_cpu.snapshot();
        let mut replacement = Msp430State::default();
        replacement.registers[4] = 0x2222;
        fail_next_allocation(AllocationSite::CloneState);
        assert_eq!(
            restore_cpu.restore(&replacement),
            Err(Msp430Error::AllocationFailure)
        );
        assert_eq!(restore_cpu.snapshot(), restore_entry);

        for site in [
            AllocationSite::CloneState,
            AllocationSite::FetchBytes,
            AllocationSite::FetchWords,
            AllocationSite::FinalTraceState,
        ] {
            let mut cpu = loaded(&[single(0x1000, false, 0, 4), HALT_WORD]);
            cpu.write_register(4, 0x8001).unwrap();
            let entry = cpu.snapshot();
            fail_next_allocation(site);
            assert_eq!(cpu.step(), Err(Msp430Error::AllocationFailure));
            assert_eq!(cpu.snapshot(), entry, "site={site:?}");
        }

        let mut run_cpu = loaded(&[HALT_WORD]);
        let run_entry = run_cpu.snapshot();
        fail_next_allocation(AllocationSite::TraceRetention);
        assert_eq!(run_cpu.run(1), Err(Msp430Error::AllocationFailure));
        assert_eq!(run_cpu.snapshot(), run_entry);

        let mut execute_cpu = Msp430::new();
        execute_cpu.write_register(4, 0xfeed).unwrap();
        execute_cpu.write_memory_byte(0x4000, 0xaa);
        let execute_entry = execute_cpu.snapshot();
        fail_next_allocation(AllocationSite::FinalTraceState);
        let error = execute_cpu.execute(&halt, 0x0200, 1).unwrap_err();
        assert!(matches!(
            error,
            Msp430Error::ExecutionFailed {
                source,
                ..
            } if *source == Msp430Error::AllocationFailure
        ));
        assert_eq!(execute_cpu.snapshot(), execute_entry);
    }

    #[test]
    fn error_displays_are_stable_and_nonempty() {
        let errors = [
            Msp430Error::InvalidState { reason: "x" },
            Msp430Error::InvalidRegister { register: 16 },
            Msp430Error::EmptyProgram,
            Msp430Error::OddProgramLength { length: 1 },
            Msp430Error::MisalignedProgram { origin: 1 },
            Msp430Error::ProgramOutOfRange {
                origin: 2,
                length: MEMORY_SIZE,
            },
            Msp430Error::NoProgramInstalled,
            Msp430Error::FetchOutsideProgram { pc: 0 },
            Msp430Error::TruncatedInstruction { pc: 0, width: 6 },
            Msp430Error::MisalignedAccess { address: 1 },
            Msp430Error::UnknownInstruction { pc: 0, raw: 1 },
            Msp430Error::UnpredictableOperand {
                pc: 0,
                raw: 1,
                reason: "x",
            },
            Msp430Error::UndefinedBcdOperand { pc: 0, raw: 1 },
            Msp430Error::Halted,
            Msp430Error::TraceLimitExceeded {
                requested: 65,
                maximum: 64,
            },
            Msp430Error::StepBudgetExhausted { maximum: 1 },
            Msp430Error::AllocationFailure,
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
            assert!(Error::source(&error).is_none());
        }
        let wrapped = Msp430Error::ExecutionFailed {
            step: 0,
            pc: 0,
            source: Box::new(Msp430Error::Halted),
        };
        assert!(Error::source(&wrapped).is_some());
        assert!(wrapped.to_string().contains("step 0"));
    }
}
