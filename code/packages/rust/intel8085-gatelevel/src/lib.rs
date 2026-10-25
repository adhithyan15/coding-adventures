//! # Intel 8085A gate-level simulator
//!
//! The documented 8080-compatible instruction surface executes in the
//! completed [`coding_adventures_intel8080_gatelevel`] datapath. This crate
//! adds the 8085 control plane as real DFF state and gate-derived control
//! signals: `RIM`, `SIM`, SID/SOD, mask and pending latches, interrupt
//! arbitration, interrupt entry, exact installed-program bounds, and delayed
//! `EI`.
//!
//! The functional 8085 package supplies shared public value types. It is not
//! called by the transition engine.

use coding_adventures_intel8080_gatelevel::{
    GateLevelCpu as Gate8080, FLIP_FLOP_COUNT as CORE_DFFS,
};
use intel8080_simulator::{Intel8080Error, Intel8080State};
use intel8085_simulator::{
    ExecutionResult, Intel8085Error, Intel8085State, InterruptRequest, PendingInterrupts,
    RestartMasks, StepTrace,
};
use logic_gates::gates::{and_gate, not_gate, or_gate};
use logic_gates::sequential::{register, FlipFlopState};

const RIM: u8 = 0x20;
const SIM: u8 = 0x30;
const EI: u8 = 0xFB;
const DI: u8 = 0xF3;
const MEMORY_SIZE: usize = 65_536;
const RETAINED_TRACE_BUDGET_BYTES: usize = 16 * 1024 * 1024;
const MAX_RETAINED_TRACE_STEPS: usize = 4_096;
const TRACE_STATE_OVERHEAD_BYTES: usize = 1_024;
const CONTROL_DFFS: usize = 52;

const UNDOCUMENTED: [u8; 10] = [0x08, 0x10, 0x18, 0x28, 0x38, 0xCB, 0xD9, 0xDD, 0xED, 0xFD];

/// Exact persistent D flip-flop count for the complete architectural state.
pub const FLIP_FLOP_COUNT: usize = CORE_DFFS + CONTROL_DFFS;

/// Six primitive gates per master/slave DFF, excluding transient datapaths.
pub const PERSISTENT_GATE_EQUIVALENT: usize = FLIP_FLOP_COUNT * 6;

#[derive(Clone)]
struct DffWord {
    state: Vec<FlipFlopState>,
}

impl DffWord {
    fn new(width: usize) -> Self {
        Self {
            state: vec![FlipFlopState::default(); width],
        }
    }

    fn read(&self) -> u64 {
        self.state.iter().enumerate().fold(0, |word, (bit, state)| {
            word | (u64::from(state.slave_q) << bit)
        })
    }

    fn write(&mut self, value: u64) {
        let bits: Vec<u8> = (0..self.state.len())
            .map(|bit| ((value >> bit) & 1) as u8)
            .collect();
        register(&bits, 0, &mut self.state);
        register(&bits, 1, &mut self.state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ControlPlane {
    loaded_origin: u16,
    loaded_len: usize,
    restart_masks: RestartMasks,
    pending: PendingInterrupts,
    serial_input: bool,
    serial_output: bool,
    ei_pending: bool,
}

impl Default for ControlPlane {
    fn default() -> Self {
        Self {
            loaded_origin: 0,
            loaded_len: 0,
            restart_masks: RestartMasks::masked(),
            pending: PendingInterrupts::default(),
            serial_input: false,
            serial_output: false,
            ei_pending: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedInterrupt {
    Trap,
    Rst75,
    Rst65,
    Rst55,
    Intr(u8),
}

/// DFF-backed Intel 8085A architectural machine.
#[derive(Clone)]
pub struct GateLevelCpu {
    core: Gate8080,
    control_state: DffWord,
}

impl Default for GateLevelCpu {
    fn default() -> Self {
        Self::new()
    }
}

impl GateLevelCpu {
    /// Create a reset machine with its architectural 64 KiB address space.
    #[must_use]
    pub fn new() -> Self {
        let mut machine = Self {
            core: Gate8080::new(),
            control_state: DffWord::new(CONTROL_DFFS),
        };
        machine.clock_control(ControlPlane::default());
        machine
    }

    /// Reset registers, memory, ports, interrupt state, serial state, and image bounds.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Drive one input-port latch.
    pub fn set_input_port(&mut self, port: u8, value: u8) {
        self.core.set_input_port(port, value);
    }

    /// Observe one output-port latch.
    #[must_use]
    pub fn output_port(&self, port: u8) -> u8 {
        self.core.get_output_port(port)
    }

    /// Drive the serial-input pin latch.
    pub fn set_serial_input(&mut self, value: bool) {
        let mut control = self.control();
        control.serial_input = value;
        self.clock_control(control);
    }

    /// Return pending interrupt latches without granting them.
    #[must_use]
    pub fn pending(&self) -> PendingInterrupts {
        self.control().pending
    }

    /// Record an external interrupt request in DFF-backed state.
    pub fn request_interrupt(&mut self, request: InterruptRequest) {
        let mut control = self.control();
        match request {
            InterruptRequest::Trap => control.pending.trap = true,
            InterruptRequest::Rst75 => control.pending.rst75 = true,
            InterruptRequest::Rst65 => control.pending.rst65 = true,
            InterruptRequest::Rst55 => control.pending.rst55 = true,
            InterruptRequest::Intr(opcode) => control.pending.intr_opcode = Some(opcode),
        }
        self.clock_control(control);
    }

    /// Install an exact executable image at address zero.
    pub fn load(&mut self, program: &[u8]) -> Result<(), Intel8085Error> {
        self.load_program_at(program, 0)
    }

    /// Alias matching the functional simulator.
    pub fn load_program(&mut self, program: &[u8]) -> Result<(), Intel8085Error> {
        self.load(program)
    }

    /// Install an exact executable image without otherwise resetting the machine.
    pub fn load_program_at(&mut self, program: &[u8], origin: u16) -> Result<(), Intel8085Error> {
        let start = usize::from(origin);
        if start
            .checked_add(program.len())
            .is_none_or(|end| end > MEMORY_SIZE)
        {
            return Err(Intel8085Error::ProgramOutOfRange {
                origin,
                length: program.len(),
                memory_size: MEMORY_SIZE,
            });
        }
        let mut state = self.snapshot();
        state.core.memory[start..start + program.len()].copy_from_slice(program);
        state.core.pc = origin;
        state.loaded_origin = origin;
        state.loaded_len = program.len();
        self.restore(&state)
    }

    /// Capture every architectural bit in owned storage.
    #[must_use]
    pub fn snapshot(&self) -> Intel8085State {
        let control = self.control();
        Intel8085State {
            core: self.core.snapshot(),
            loaded_origin: control.loaded_origin,
            loaded_len: control.loaded_len,
            restart_masks: control.restart_masks,
            pending: control.pending,
            serial_input: control.serial_input,
            serial_output: control.serial_output,
            ei_pending: control.ei_pending,
        }
    }

    /// Restore complete state atomically after validating its fixed topology.
    pub fn restore(&mut self, state: &Intel8085State) -> Result<(), Intel8085Error> {
        let start = usize::from(state.loaded_origin);
        if start
            .checked_add(state.loaded_len)
            .is_none_or(|end| end > MEMORY_SIZE)
        {
            return Err(Intel8085Error::ProgramOutOfRange {
                origin: state.loaded_origin,
                length: state.loaded_len,
                memory_size: MEMORY_SIZE,
            });
        }
        let mut candidate = Self::new();
        candidate
            .core
            .restore(&state.core)
            .map_err(map_core_error)?;
        candidate.clock_control(ControlPlane {
            loaded_origin: state.loaded_origin,
            loaded_len: state.loaded_len,
            restart_masks: state.restart_masks,
            pending: state.pending,
            serial_input: state.serial_input,
            serial_output: state.serial_output,
            ei_pending: state.ei_pending,
        });
        *self = candidate;
        Ok(())
    }

    /// Safe retained-trace bound for full before/after snapshots.
    #[must_use]
    pub fn max_retained_trace_steps(&self) -> usize {
        let bytes_per_state = MEMORY_SIZE.saturating_add(TRACE_STATE_OVERHEAD_BYTES);
        let bytes_per_trace = bytes_per_state.saturating_mul(2).max(1);
        (RETAINED_TRACE_BUDGET_BYTES / bytes_per_trace).min(MAX_RETAINED_TRACE_STEPS)
    }

    /// Run a fresh candidate and commit the machine only on success.
    pub fn run(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, Intel8085Error> {
        self.validate_trace_limit(max_steps)?;
        if program.len() > MEMORY_SIZE {
            return Err(Intel8085Error::ProgramOutOfRange {
                origin: 0,
                length: program.len(),
                memory_size: MEMORY_SIZE,
            });
        }
        let existing = self.snapshot();
        let mut candidate = Self::new();
        let mut initial = candidate.snapshot();
        initial.core.input_ports = existing.core.input_ports;
        initial.serial_input = existing.serial_input;
        candidate.restore(&initial)?;
        candidate.load(program)?;
        let result = candidate.run_loaded_with_limit(max_steps)?;
        *self = candidate;
        Ok(result)
    }

    /// Run the installed image with the safe retained-trace limit.
    pub fn run_loaded(&mut self) -> Result<ExecutionResult, Intel8085Error> {
        self.run_loaded_with_limit(self.max_retained_trace_steps())
    }

    /// Execute at most `max_steps` instructions or interrupt grants.
    pub fn run_loaded_with_limit(
        &mut self,
        max_steps: usize,
    ) -> Result<ExecutionResult, Intel8085Error> {
        self.validate_trace_limit(max_steps)?;
        // A bounded run is one architectural transaction.  Execute against a
        // private DFF image so a later typed fault cannot expose the successful
        // prefix of the run through registers, memory, ports, or control latches.
        let mut candidate = self.clone();
        let mut traces = Vec::new();
        while traces.len() < max_steps {
            if candidate.core.halted() && !candidate.has_serviceable_interrupt() {
                break;
            }
            traces.push(candidate.step()?);
        }
        let final_state = candidate.snapshot();
        let result = ExecutionResult {
            halted: final_state.core.halted,
            steps: traces.len(),
            pc: final_state.core.pc,
            traces,
            final_state,
        };
        *self = candidate;
        Ok(result)
    }

    /// Execute one atomic gate transition.
    pub fn step(&mut self) -> Result<StepTrace, Intel8085Error> {
        let mut candidate = self.clone();
        let trace = candidate.step_inner()?;
        *self = candidate;
        Ok(trace)
    }

    fn step_inner(&mut self) -> Result<StepTrace, Intel8085Error> {
        if let Some(interrupt) = self.select_interrupt()? {
            return self.accept_interrupt(interrupt);
        }
        if self.core.halted() {
            return Err(Intel8085Error::Halted);
        }

        let control = self.control();
        let core_before = self.core.snapshot();
        let address = core_before.pc;
        let start = usize::from(control.loaded_origin);
        let end = start + control.loaded_len;
        let pc = usize::from(address);
        if pc < start || pc >= end {
            return Err(Intel8085Error::FetchOutsideProgram {
                address,
                origin: control.loaded_origin,
                length: control.loaded_len,
            });
        }
        let opcode = core_before.memory[pc];
        let expected =
            instruction_length(opcode).ok_or(Intel8085Error::UnknownOpcode { address, opcode })?;
        let available = end - pc;
        if available < expected {
            return Err(Intel8085Error::TruncatedInstruction {
                address,
                expected,
                available,
            });
        }

        let state_before = self.snapshot();
        let enable_after = control.ei_pending;
        let (raw, mnemonic) = match opcode {
            RIM => {
                self.execute_rim()?;
                (vec![RIM], "RIM".to_string())
            }
            SIM => {
                self.execute_sim()?;
                (vec![SIM], "SIM".to_string())
            }
            _ => {
                let trace = self.core.step().map_err(map_core_error)?;
                (trace.raw, trace_mnemonic(opcode).to_string())
            }
        };
        self.finish_interrupt_enable(opcode, enable_after)?;
        Ok(StepTrace {
            address,
            raw,
            mnemonic,
            state_before,
            state_after: self.snapshot(),
        })
    }

    fn execute_rim(&mut self) -> Result<(), Intel8085Error> {
        let control = self.control();
        let core = self.core.snapshot();
        let signals = [
            control.restart_masks.rst55,
            control.restart_masks.rst65,
            control.restart_masks.rst75,
            core.interrupts_enabled,
            control.pending.rst55,
            control.pending.rst65,
            control.pending.rst75,
            control.serial_input,
        ];
        let mut value = 0u8;
        for (bit, signal) in signals.into_iter().enumerate() {
            value |= (or_gate(signal as u8, 0) & 1) << bit;
        }
        let mut next = core;
        next.regs.a = value;
        next.pc = next.pc.wrapping_add(1);
        self.core.restore(&next).map_err(map_core_error)
    }

    fn execute_sim(&mut self) -> Result<(), Intel8085Error> {
        let core = self.core.snapshot();
        let a = core.regs.a;
        let bit = |index: u32| ((a >> index) & 1) != 0;
        let mut control = self.control();
        if gate(bit(3)) {
            control.restart_masks = RestartMasks {
                rst75: gated_and(bit(3), bit(2)),
                rst65: gated_and(bit(3), bit(1)),
                rst55: gated_and(bit(3), bit(0)),
            };
        }
        if gate(bit(4)) {
            control.pending.rst75 = false;
        }
        if gate(bit(6)) {
            control.serial_output = gated_and(bit(6), bit(7));
        }
        self.clock_control(control);
        let mut next = core;
        next.pc = next.pc.wrapping_add(1);
        self.core.restore(&next).map_err(map_core_error)
    }

    fn finish_interrupt_enable(
        &mut self,
        opcode: u8,
        enable_after: bool,
    ) -> Result<(), Intel8085Error> {
        if opcode != DI && opcode != EI && !enable_after {
            return Ok(());
        }
        let mut control = self.control();
        let mut core = self.core.snapshot();
        if opcode == DI {
            core.interrupts_enabled = false;
            control.ei_pending = false;
        } else {
            if enable_after {
                core.interrupts_enabled = true;
            }
            if opcode == EI {
                if !enable_after {
                    core.interrupts_enabled = false;
                }
                control.ei_pending = true;
            } else {
                control.ei_pending = false;
            }
        }
        self.core.restore(&core).map_err(map_core_error)?;
        self.clock_control(control);
        Ok(())
    }

    fn has_serviceable_interrupt(&self) -> bool {
        let control = self.control();
        let enabled = self.core.snapshot().interrupts_enabled;
        let restart = gated_or(
            gated_and(
                control.pending.rst75,
                gated_not(control.restart_masks.rst75),
            ),
            gated_or(
                gated_and(
                    control.pending.rst65,
                    gated_not(control.restart_masks.rst65),
                ),
                gated_and(
                    control.pending.rst55,
                    gated_not(control.restart_masks.rst55),
                ),
            ),
        );
        gated_or(
            control.pending.trap,
            gated_and(
                enabled,
                gated_or(restart, control.pending.intr_opcode.is_some()),
            ),
        )
    }

    fn select_interrupt(&self) -> Result<Option<SelectedInterrupt>, Intel8085Error> {
        let control = self.control();
        if gate(control.pending.trap) {
            return Ok(Some(SelectedInterrupt::Trap));
        }
        if !gate(self.core.snapshot().interrupts_enabled) {
            return Ok(None);
        }
        if gated_and(
            control.pending.rst75,
            gated_not(control.restart_masks.rst75),
        ) {
            return Ok(Some(SelectedInterrupt::Rst75));
        }
        if gated_and(
            control.pending.rst65,
            gated_not(control.restart_masks.rst65),
        ) {
            return Ok(Some(SelectedInterrupt::Rst65));
        }
        if gated_and(
            control.pending.rst55,
            gated_not(control.restart_masks.rst55),
        ) {
            return Ok(Some(SelectedInterrupt::Rst55));
        }
        if let Some(opcode) = control.pending.intr_opcode {
            if opcode & 0xC7 != 0xC7 {
                return Err(Intel8085Error::UnsupportedInterruptOpcode { opcode });
            }
            return Ok(Some(SelectedInterrupt::Intr((opcode >> 3) & 7)));
        }
        Ok(None)
    }

    fn accept_interrupt(
        &mut self,
        selected: SelectedInterrupt,
    ) -> Result<StepTrace, Intel8085Error> {
        let state_before = self.snapshot();
        let mut core = state_before.core.clone();
        let new_sp = core.regs.sp.wrapping_sub(2);
        for address in [new_sp, new_sp.wrapping_add(1)] {
            if usize::from(address) >= core.memory.len() {
                return Err(Intel8085Error::StackOutOfRange {
                    address,
                    memory_size: core.memory.len(),
                });
            }
        }
        let return_pc = core.pc;
        core.memory[usize::from(new_sp)] = return_pc as u8;
        core.memory[usize::from(new_sp.wrapping_add(1))] = (return_pc >> 8) as u8;
        core.regs.sp = new_sp;
        core.halted = false;
        core.interrupts_enabled = false;

        let mut control = self.control();
        control.ei_pending = false;
        let (vector, mnemonic) = match selected {
            SelectedInterrupt::Trap => {
                control.pending.trap = false;
                (0x0024, "INT TRAP".to_string())
            }
            SelectedInterrupt::Rst75 => {
                control.pending.rst75 = false;
                (0x003C, "INT RST7.5".to_string())
            }
            SelectedInterrupt::Rst65 => {
                control.pending.rst65 = false;
                (0x0034, "INT RST6.5".to_string())
            }
            SelectedInterrupt::Rst55 => {
                control.pending.rst55 = false;
                (0x002C, "INT RST5.5".to_string())
            }
            SelectedInterrupt::Intr(number) => {
                control.pending.intr_opcode = None;
                (u16::from(number) * 8, format!("INT INTR/RST{number}"))
            }
        };
        core.pc = vector;
        self.core.restore(&core).map_err(map_core_error)?;
        self.clock_control(control);
        Ok(StepTrace {
            address: return_pc,
            raw: Vec::new(),
            mnemonic,
            state_before,
            state_after: self.snapshot(),
        })
    }

    fn validate_trace_limit(&self, requested: usize) -> Result<(), Intel8085Error> {
        let maximum = self.max_retained_trace_steps();
        if requested > maximum {
            return Err(Intel8085Error::TraceLimitTooLarge { requested, maximum });
        }
        Ok(())
    }

    fn control(&self) -> ControlPlane {
        let word = self.control_state.read();
        let flag = |bit: usize| ((word >> bit) & 1) != 0;
        let intr_opcode = flag(40).then_some(((word >> 41) & 0xFF) as u8);
        ControlPlane {
            loaded_origin: (word & 0xFFFF) as u16,
            loaded_len: ((word >> 16) & 0x1_FFFF) as usize,
            restart_masks: RestartMasks {
                rst55: flag(33),
                rst65: flag(34),
                rst75: flag(35),
            },
            pending: PendingInterrupts {
                trap: flag(36),
                rst75: flag(37),
                rst65: flag(38),
                rst55: flag(39),
                intr_opcode,
            },
            serial_input: flag(49),
            serial_output: flag(50),
            ei_pending: flag(51),
        }
    }

    fn clock_control(&mut self, control: ControlPlane) {
        let mut word = u64::from(control.loaded_origin) | ((control.loaded_len as u64) << 16);
        set_bit(&mut word, 33, control.restart_masks.rst55);
        set_bit(&mut word, 34, control.restart_masks.rst65);
        set_bit(&mut word, 35, control.restart_masks.rst75);
        set_bit(&mut word, 36, control.pending.trap);
        set_bit(&mut word, 37, control.pending.rst75);
        set_bit(&mut word, 38, control.pending.rst65);
        set_bit(&mut word, 39, control.pending.rst55);
        if let Some(opcode) = control.pending.intr_opcode {
            set_bit(&mut word, 40, true);
            word |= u64::from(opcode) << 41;
        }
        set_bit(&mut word, 49, control.serial_input);
        set_bit(&mut word, 50, control.serial_output);
        set_bit(&mut word, 51, control.ei_pending);
        self.control_state.write(word);
    }
}

fn set_bit(word: &mut u64, bit: usize, value: bool) {
    *word |= u64::from(value) << bit;
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

fn trace_mnemonic(opcode: u8) -> &'static str {
    let group = opcode >> 6;
    let dst = (opcode >> 3) & 7;
    let src = opcode & 7;
    if group == 1 {
        return if opcode == 0x76 { "hlt" } else { "mov" };
    }
    if group == 2 {
        return "alu_reg";
    }
    if group == 0 {
        if opcode == 0 {
            return "nop";
        }
        if src == 1 && dst & 1 == 0 {
            return "lxi";
        }
        if src == 3 && dst & 1 == 0 {
            return "inx";
        }
        if src == 3 && dst & 1 == 1 {
            return "dcx";
        }
        if src == 1 && dst & 1 == 1 {
            return "dad";
        }
        if src == 6 {
            return "mvi";
        }
        if src == 4 {
            return "inr";
        }
        if src == 5 {
            return "dcr";
        }
        return match opcode {
            0x02 | 0x12 => "stax",
            0x0A | 0x1A => "ldax",
            0x22 => "shld",
            0x2A => "lhld",
            0x32 => "sta",
            0x3A => "lda",
            0x07 => "rlc",
            0x0F => "rrc",
            0x17 => "ral",
            0x1F => "rar",
            0x27 => "daa",
            0x2F => "cma",
            0x37 => "stc",
            0x3F => "cmc",
            _ => "undefined",
        };
    }
    if src == 0 {
        return "rcond";
    }
    if src == 1 && dst & 1 == 0 {
        return "pop";
    }
    if src == 2 {
        return "jcond";
    }
    if src == 4 {
        return "ccond";
    }
    if src == 5 && dst & 1 == 0 {
        return "push";
    }
    match opcode {
        0xC9 => "ret",
        0xC3 => "jmp",
        0xCD => "call",
        _ if src == 6 => "alu_imm",
        _ if src == 7 => "rst",
        0xE3 => "xthl",
        0xF9 => "sphl",
        0xEB => "xchg",
        0xE9 => "pchl",
        0xDB => "in",
        0xD3 => "out",
        EI => "ei",
        DI => "di",
        _ => "undefined",
    }
}

fn gate(value: bool) -> bool {
    or_gate(value as u8, 0) != 0
}

fn gated_and(left: bool, right: bool) -> bool {
    and_gate(left as u8, right as u8) != 0
}

fn gated_or(left: bool, right: bool) -> bool {
    or_gate(left as u8, right as u8) != 0
}

fn gated_not(value: bool) -> bool {
    not_gate(value as u8) != 0
}

fn map_core_error(error: Intel8080Error) -> Intel8085Error {
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

/// Shared complete core-state type used by restore/snapshot clients.
pub type CoreState = Intel8080State;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_map_without_losing_fields() {
        let cases = [
            Intel8080Error::ProgramOutOfRange {
                length: 9,
                memory_size: 8,
            },
            Intel8080Error::TruncatedInstruction {
                address: 7,
                expected: 3,
                available: 1,
            },
            Intel8080Error::UnknownOpcode {
                address: 2,
                opcode: 0x08,
            },
            Intel8080Error::Halted,
            Intel8080Error::MemoryOutOfRange {
                address: 9,
                memory_size: 8,
            },
        ];
        let mapped = cases.map(map_core_error);
        assert_eq!(
            mapped[0],
            Intel8085Error::ProgramOutOfRange {
                origin: 0,
                length: 9,
                memory_size: 8,
            }
        );
        assert_eq!(
            mapped[1],
            Intel8085Error::TruncatedInstruction {
                address: 7,
                expected: 3,
                available: 1,
            }
        );
        assert_eq!(
            mapped[2],
            Intel8085Error::UnknownOpcode {
                address: 2,
                opcode: 0x08,
            }
        );
        assert_eq!(mapped[3], Intel8085Error::Halted);
        assert_eq!(
            mapped[4],
            Intel8085Error::MemoryOutOfRange {
                address: 9,
                memory_size: 8,
            }
        );
    }

    #[test]
    fn internal_trace_fallbacks_are_stable() {
        assert_eq!(trace_mnemonic(0x08), "undefined");
        assert_eq!(trace_mnemonic(0xCB), "undefined");
        assert_eq!(
            GateLevelCpu::default().snapshot(),
            GateLevelCpu::new().snapshot()
        );
    }
}
