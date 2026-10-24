//! # Intel 8085A functional simulator
//!
//! The 8085 is source- and machine-code compatible with the documented Intel
//! 8080 instruction set, but it is not merely a renamed 8080. Its `RIM` and
//! `SIM` instructions expose three vectored interrupt masks, three pending
//! signals, and one-bit serial input/output latches. This crate composes the
//! completed [`intel8080_simulator`] core with that 8085-only control plane.
//!
//! The boundary is intentionally checked: installed program ranges are exact,
//! failures are typed and atomic, interrupts are accepted between
//! instructions, EI observes the 8085's one-instruction delay, and snapshots
//! own every memory and port byte.

use intel8080_simulator::{Intel8080Error, Intel8080Simulator, Intel8080State};

const RIM: u8 = 0x20;
const SIM: u8 = 0x30;
const EI: u8 = 0xFB;
const DI: u8 = 0xF3;

const UNDOCUMENTED: [u8; 10] = [0x08, 0x10, 0x18, 0x28, 0x38, 0xCB, 0xD9, 0xDD, 0xED, 0xFD];

/// Per-source mask latches. A set bit suppresses that restart input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartMasks {
    pub rst75: bool,
    pub rst65: bool,
    pub rst55: bool,
}

impl RestartMasks {
    /// RESET IN state: all three maskable restart inputs are disabled.
    #[must_use]
    pub const fn masked() -> Self {
        Self {
            rst75: true,
            rst65: true,
            rst55: true,
        }
    }

    /// Convenience state used by interrupt-controller tests and applications.
    #[must_use]
    pub const fn unmasked() -> Self {
        Self {
            rst75: false,
            rst65: false,
            rst55: false,
        }
    }
}

impl Default for RestartMasks {
    fn default() -> Self {
        Self::masked()
    }
}

/// Host-visible interrupt request lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptRequest {
    Trap,
    Rst75,
    Rst65,
    Rst55,
    /// General INTR plus the instruction byte returned during acknowledge.
    /// The checked model accepts the conventional eight `RST n` bytes.
    Intr(u8),
}

/// Latched interrupt requests reported by RIM and complete snapshots.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PendingInterrupts {
    pub trap: bool,
    pub rst75: bool,
    pub rst65: bool,
    pub rst55: bool,
    pub intr_opcode: Option<u8>,
}

impl PendingInterrupts {
    /// Pack the three restart-pending flags as RST7.5:RST6.5:RST5.5.
    #[must_use]
    pub const fn restart_bits(self) -> u8 {
        ((self.rst75 as u8) << 2) | ((self.rst65 as u8) << 1) | self.rst55 as u8
    }
}

/// A checked Intel 8085 lifecycle or execution failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intel8085Error {
    ProgramOutOfRange {
        origin: u16,
        length: usize,
        memory_size: usize,
    },
    FetchOutsideProgram {
        address: u16,
        origin: u16,
        length: usize,
    },
    TruncatedInstruction {
        address: u16,
        expected: usize,
        available: usize,
    },
    UnknownOpcode {
        address: u16,
        opcode: u8,
    },
    Halted,
    MemoryOutOfRange {
        address: u16,
        memory_size: usize,
    },
    StackOutOfRange {
        address: u16,
        memory_size: usize,
    },
    UnsupportedInterruptOpcode {
        opcode: u8,
    },
}

impl std::fmt::Display for Intel8085Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProgramOutOfRange {
                origin,
                length,
                memory_size,
            } => write!(
                formatter,
                "program of {length} bytes at {origin:#06X} exceeds configured memory of {memory_size} bytes"
            ),
            Self::FetchOutsideProgram {
                address,
                origin,
                length,
            } => write!(
                formatter,
                "fetch at {address:#06X} is outside installed program at {origin:#06X} with length {length}"
            ),
            Self::TruncatedInstruction {
                address,
                expected,
                available,
            } => write!(
                formatter,
                "instruction at {address:#06X} needs {expected} bytes but only {available} remain"
            ),
            Self::UnknownOpcode { address, opcode } => {
                write!(formatter, "unknown opcode {opcode:#04X} at {address:#06X}")
            }
            Self::Halted => formatter.write_str("CPU is halted"),
            Self::MemoryOutOfRange {
                address,
                memory_size,
            } => write!(
                formatter,
                "address {address:#06X} is outside configured memory of {memory_size} bytes"
            ),
            Self::StackOutOfRange {
                address,
                memory_size,
            } => write!(
                formatter,
                "stack address {address:#06X} is outside configured memory of {memory_size} bytes"
            ),
            Self::UnsupportedInterruptOpcode { opcode } => write!(
                formatter,
                "INTR acknowledge byte {opcode:#04X} is not a RST instruction"
            ),
        }
    }
}

impl std::error::Error for Intel8085Error {}

/// Complete, owned Intel 8085 architectural state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intel8085State {
    pub core: Intel8080State,
    pub loaded_origin: u16,
    pub loaded_len: usize,
    pub restart_masks: RestartMasks,
    pub pending: PendingInterrupts,
    pub serial_input: bool,
    pub serial_output: bool,
    pub ei_pending: bool,
}

/// Complete record of one instruction or interrupt acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub address: u16,
    pub raw: Vec<u8>,
    pub mnemonic: String,
    pub state_before: Intel8085State,
    pub state_after: Intel8085State,
}

/// Observable result of a bounded execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: u16,
    pub traces: Vec<StepTrace>,
    pub final_state: Intel8085State,
}

/// Intel 8085 machine with an 8080-compatible execution core.
pub struct Intel8085Simulator {
    /// Shared documented instruction engine. Its registers, flags, memory,
    /// ports, PC, halt, and IE fields remain directly inspectable.
    pub core: Intel8080Simulator,
    pub restart_masks: RestartMasks,
    pub serial_input: bool,
    pub serial_output: bool,
    loaded_origin: u16,
    loaded_len: usize,
    pending: PendingInterrupts,
    ei_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedInterrupt {
    Trap,
    Rst75,
    Rst65,
    Rst55,
    Intr { number: u8 },
}

impl Intel8085Simulator {
    /// Create a reset 8085 with the requested memory arena.
    #[must_use]
    pub fn new(memory_size: usize) -> Self {
        Self {
            core: Intel8080Simulator::new(memory_size),
            restart_masks: RestartMasks::masked(),
            serial_input: false,
            serial_output: false,
            loaded_origin: 0,
            loaded_len: 0,
            pending: PendingInterrupts::default(),
            ei_pending: false,
        }
    }

    /// Reset CPU, interrupt controller, serial latches, ports, and memory.
    pub fn reset(&mut self) {
        *self = Self::new(self.core.mem.size());
    }

    /// Return the current latched requests without granting them.
    #[must_use]
    pub const fn pending(&self) -> PendingInterrupts {
        self.pending
    }

    /// Record an external interrupt request.
    pub fn request_interrupt(&mut self, request: InterruptRequest) {
        match request {
            InterruptRequest::Trap => self.pending.trap = true,
            InterruptRequest::Rst75 => self.pending.rst75 = true,
            InterruptRequest::Rst65 => self.pending.rst65 = true,
            InterruptRequest::Rst55 => self.pending.rst55 = true,
            InterruptRequest::Intr(opcode) => self.pending.intr_opcode = Some(opcode),
        }
    }

    /// Install a program at address zero.
    pub fn load_program(&mut self, program: &[u8]) -> Result<(), Intel8085Error> {
        self.load_program_at(program, 0)
    }

    /// Install an exact executable range without otherwise resetting state.
    pub fn load_program_at(&mut self, program: &[u8], origin: u16) -> Result<(), Intel8085Error> {
        let start = origin as usize;
        let end = start.checked_add(program.len());
        if end.is_none_or(|end| end > self.core.mem.size() || end > 65_536) {
            return Err(Intel8085Error::ProgramOutOfRange {
                origin,
                length: program.len(),
                memory_size: self.core.mem.size(),
            });
        }
        self.core.mem.load_bytes(start, program);
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        self.core.pc = origin;
        Ok(())
    }

    /// Capture every architecturally visible bit in owned storage.
    #[must_use]
    pub fn snapshot(&self) -> Intel8085State {
        Intel8085State {
            core: self.core.snapshot(),
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
            restart_masks: self.restart_masks,
            pending: self.pending,
            serial_input: self.serial_input,
            serial_output: self.serial_output,
            ei_pending: self.ei_pending,
        }
    }

    /// Execute a program transactionally on a fresh CPU.
    pub fn run(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, Intel8085Error> {
        if program.len() > self.core.mem.size() || program.len() > 65_536 {
            return Err(Intel8085Error::ProgramOutOfRange {
                origin: 0,
                length: program.len(),
                memory_size: self.core.mem.size(),
            });
        }
        let mut candidate = Self::new(self.core.mem.size());
        candidate.core.input_ports = self.core.input_ports;
        candidate.serial_input = self.serial_input;
        candidate.load_program(program)?;
        let result = candidate.run_loaded_with_limit(max_steps)?;
        *self = candidate;
        Ok(result)
    }

    /// Execute an installed program with a generous deterministic limit.
    pub fn run_loaded(&mut self) -> Result<ExecutionResult, Intel8085Error> {
        self.run_loaded_with_limit(1_000_000)
    }

    /// Execute at most `max_steps` instructions or interrupt acknowledges.
    pub fn run_loaded_with_limit(
        &mut self,
        max_steps: usize,
    ) -> Result<ExecutionResult, Intel8085Error> {
        let mut traces = Vec::new();
        while traces.len() < max_steps {
            if self.core.halted && !self.has_serviceable_interrupt() {
                break;
            }
            traces.push(self.step()?);
        }
        Ok(ExecutionResult {
            halted: self.core.halted,
            steps: traces.len(),
            pc: self.core.pc,
            traces,
            final_state: self.snapshot(),
        })
    }

    /// Execute one instruction or accept one pending interrupt.
    pub fn step(&mut self) -> Result<StepTrace, Intel8085Error> {
        if let Some(selected) = self.select_interrupt()? {
            return self.accept_interrupt(selected);
        }
        if self.core.halted {
            return Err(Intel8085Error::Halted);
        }

        let address = self.core.pc;
        let start = self.loaded_origin as usize;
        let end = start + self.loaded_len;
        let pc = address as usize;
        if pc < start || pc >= end {
            return Err(Intel8085Error::FetchOutsideProgram {
                address,
                origin: self.loaded_origin,
                length: self.loaded_len,
            });
        }
        let opcode = self.core.mem.read_byte(pc);
        let expected = Self::instruction_length(opcode)
            .ok_or(Intel8085Error::UnknownOpcode { address, opcode })?;
        let available = end - pc;
        if available < expected {
            return Err(Intel8085Error::TruncatedInstruction {
                address,
                expected,
                available,
            });
        }

        let before = self.snapshot();
        let enable_after = self.ei_pending;
        let (raw, mnemonic) = match opcode {
            RIM => {
                self.core.regs.a = self.rim_value();
                self.core.pc = self.core.pc.wrapping_add(1);
                (vec![RIM], "RIM".to_string())
            }
            SIM => {
                self.apply_sim();
                self.core.pc = self.core.pc.wrapping_add(1);
                (vec![SIM], "SIM".to_string())
            }
            _ => {
                let trace = self.core.step().map_err(Intel8085Error::from)?;
                (trace.raw, trace.mnemonic)
            }
        };
        self.finish_interrupt_enable(opcode, enable_after);
        Ok(StepTrace {
            address,
            raw,
            mnemonic,
            state_before: before,
            state_after: self.snapshot(),
        })
    }

    fn instruction_length(opcode: u8) -> Option<usize> {
        if UNDOCUMENTED.contains(&opcode) {
            None
        } else if opcode & 0xC7 == 0x06 || opcode & 0xC7 == 0xC6 || matches!(opcode, 0xD3 | 0xDB) {
            Some(2)
        } else if opcode & 0xCF == 0x01
            || opcode & 0xC7 == 0xC2
            || opcode & 0xC7 == 0xC4
            || matches!(opcode, 0x22 | 0x2A | 0x32 | 0x3A | 0xC3 | 0xCD)
        {
            Some(3)
        } else {
            Some(1)
        }
    }

    fn rim_value(&self) -> u8 {
        ((self.serial_input as u8) << 7)
            | ((self.pending.rst75 as u8) << 6)
            | ((self.pending.rst65 as u8) << 5)
            | ((self.pending.rst55 as u8) << 4)
            | ((self.core.interrupts_enabled as u8) << 3)
            | ((self.restart_masks.rst75 as u8) << 2)
            | ((self.restart_masks.rst65 as u8) << 1)
            | self.restart_masks.rst55 as u8
    }

    fn apply_sim(&mut self) {
        let accumulator = self.core.regs.a;
        if accumulator & 0x08 != 0 {
            self.restart_masks = RestartMasks {
                rst75: accumulator & 0x04 != 0,
                rst65: accumulator & 0x02 != 0,
                rst55: accumulator & 0x01 != 0,
            };
        }
        if accumulator & 0x10 != 0 {
            self.pending.rst75 = false;
        }
        if accumulator & 0x40 != 0 {
            self.serial_output = accumulator & 0x80 != 0;
        }
    }

    fn finish_interrupt_enable(&mut self, opcode: u8, enable_after: bool) {
        if opcode == DI {
            self.core.interrupts_enabled = false;
            self.ei_pending = false;
            return;
        }
        if enable_after {
            self.core.interrupts_enabled = true;
        }
        if opcode == EI {
            if !enable_after {
                self.core.interrupts_enabled = false;
            }
            self.ei_pending = true;
        } else {
            self.ei_pending = false;
        }
    }

    fn has_serviceable_interrupt(&self) -> bool {
        if self.pending.trap {
            return true;
        }
        self.core.interrupts_enabled
            && ((self.pending.rst75 && !self.restart_masks.rst75)
                || (self.pending.rst65 && !self.restart_masks.rst65)
                || (self.pending.rst55 && !self.restart_masks.rst55)
                || self.pending.intr_opcode.is_some())
    }

    fn select_interrupt(&self) -> Result<Option<SelectedInterrupt>, Intel8085Error> {
        if self.pending.trap {
            return Ok(Some(SelectedInterrupt::Trap));
        }
        if !self.core.interrupts_enabled {
            return Ok(None);
        }
        if self.pending.rst75 && !self.restart_masks.rst75 {
            return Ok(Some(SelectedInterrupt::Rst75));
        }
        if self.pending.rst65 && !self.restart_masks.rst65 {
            return Ok(Some(SelectedInterrupt::Rst65));
        }
        if self.pending.rst55 && !self.restart_masks.rst55 {
            return Ok(Some(SelectedInterrupt::Rst55));
        }
        if let Some(opcode) = self.pending.intr_opcode {
            if opcode & 0xC7 != 0xC7 {
                return Err(Intel8085Error::UnsupportedInterruptOpcode { opcode });
            }
            return Ok(Some(SelectedInterrupt::Intr {
                number: (opcode >> 3) & 0x07,
            }));
        }
        Ok(None)
    }

    fn accept_interrupt(
        &mut self,
        selected: SelectedInterrupt,
    ) -> Result<StepTrace, Intel8085Error> {
        let new_sp = self.core.regs.sp.wrapping_sub(2);
        for address in [new_sp, new_sp.wrapping_add(1)] {
            if address as usize >= self.core.mem.size() {
                return Err(Intel8085Error::StackOutOfRange {
                    address,
                    memory_size: self.core.mem.size(),
                });
            }
        }

        let before = self.snapshot();
        let return_pc = self.core.pc;
        self.core.mem.write_byte(new_sp as usize, return_pc as u8);
        self.core
            .mem
            .write_byte(new_sp.wrapping_add(1) as usize, (return_pc >> 8) as u8);
        self.core.regs.sp = new_sp;
        self.core.halted = false;
        self.core.interrupts_enabled = false;
        self.ei_pending = false;

        let (vector, mnemonic) = match selected {
            SelectedInterrupt::Trap => {
                self.pending.trap = false;
                (0x0024, "INT TRAP".to_string())
            }
            SelectedInterrupt::Rst75 => {
                self.pending.rst75 = false;
                (0x003C, "INT RST7.5".to_string())
            }
            SelectedInterrupt::Rst65 => {
                self.pending.rst65 = false;
                (0x0034, "INT RST6.5".to_string())
            }
            SelectedInterrupt::Rst55 => {
                self.pending.rst55 = false;
                (0x002C, "INT RST5.5".to_string())
            }
            SelectedInterrupt::Intr { number } => {
                self.pending.intr_opcode = None;
                (u16::from(number) * 8, format!("INT INTR/RST{number}"))
            }
        };
        self.core.pc = vector;
        Ok(StepTrace {
            address: return_pc,
            raw: Vec::new(),
            mnemonic,
            state_before: before,
            state_after: self.snapshot(),
        })
    }
}

impl From<Intel8080Error> for Intel8085Error {
    fn from(error: Intel8080Error) -> Self {
        match error {
            Intel8080Error::ProgramOutOfRange {
                length,
                memory_size,
            } => Intel8085Error::ProgramOutOfRange {
                origin: 0,
                length,
                memory_size,
            },
            Intel8080Error::TruncatedInstruction {
                address,
                expected,
                available,
            } => Intel8085Error::TruncatedInstruction {
                address,
                expected,
                available,
            },
            Intel8080Error::UnknownOpcode { address, opcode } => {
                Intel8085Error::UnknownOpcode { address, opcode }
            }
            Intel8080Error::Halted => Intel8085Error::Halted,
            Intel8080Error::MemoryOutOfRange {
                address,
                memory_size,
            } => Intel8085Error::MemoryOutOfRange {
                address,
                memory_size,
            },
        }
    }
}
