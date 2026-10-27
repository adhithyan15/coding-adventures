//! Checked Armv7E-M functional model of one bounded Cortex-M7 core.
//!
//! # Why this crate composes Cortex-M3
//!
//! Cortex-M7 and Cortex-M3 share the Armv7-M programmer's model. Reusing the
//! checked Cortex-M3 transition system makes that compatibility executable:
//! every inherited instruction and lifecycle rule has one owner. This crate
//! intercepts only the explicitly supported Armv7E-M encodings and retains the
//! Cortex-M7-only GE flags beside the inherited complete state.
//!
//! Caches, TCM, branch prediction, buses, and timing sit outside this scalar
//! contract. They can be composed around it by later SoC and gate models
//! without making architectural instruction results depend on invented timing.

#[cfg(test)]
use core::cell::Cell;
use core::fmt;

pub use cortex_m3_simulator::{CortexM3Error, CortexM3State, MEMORY_SIZE};

/// Sticky saturation flag in the inherited APSR.
const APSR_Q: u32 = 1 << 27;
/// N, Z, C, V, and Q bits writable through APSR_nzcvq.
const APSR_NZCVQ_MASK: u32 = 0xf800_0000;
/// The complete-state retention limit inherited from the 8 MiB teaching budget.
pub const MAX_RETAINED_STEPS: usize = cortex_m3_simulator::MAX_RETAINED_STEPS;

/// Decode families visible at the Cortex-M7 boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionClass {
    /// An instruction delegated to the complete Cortex-M3 compatibility core.
    Inherited(cortex_m3_simulator::InstructionClass),
    /// Signed saturating add.
    Qadd,
    /// Signed saturating subtract.
    Qsub,
    /// Four signed byte additions with per-lane GE results.
    Sadd8,
    /// Four unsigned byte additions with per-lane carry results.
    Uadd8,
}

/// Complete restorable architectural state at an instruction boundary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CortexM7State {
    /// The complete Armv7-M compatibility state, including owned memory.
    pub core: CortexM3State,
    /// `APSR.GE[3:0]`, stored in the low nibble.
    pub ge: u8,
}

/// Typed lifecycle, state, and execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CortexM7Error {
    /// Failure reported by the checked Cortex-M3 compatibility boundary.
    Core(CortexM3Error),
    /// Cortex-M7-only state failed validation.
    InvalidState { reason: &'static str },
    /// A run requested more complete snapshots than the teaching budget holds.
    RetentionLimitExceeded { requested: usize, limit: usize },
    /// A bounded run exhausted its step allowance without stopping.
    StepLimitExceeded { limit: usize },
    /// Trace-vector allocation failed before execution.
    AllocationFailed,
}

impl fmt::Display for CortexM7Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::InvalidState { reason } => write!(formatter, "invalid Cortex-M7 state: {reason}"),
            Self::RetentionLimitExceeded { requested, limit } => write!(
                formatter,
                "requested {requested} traces exceeds retention limit {limit}"
            ),
            Self::StepLimitExceeded { limit } => write!(formatter, "step limit {limit} exhausted"),
            Self::AllocationFailed => formatter.write_str("trace allocation failed"),
        }
    }
}

impl std::error::Error for CortexM7Error {}

impl From<CortexM3Error> for CortexM7Error {
    fn from(error: CortexM3Error) -> Self {
        Self::Core(error)
    }
}

/// One committed architectural transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: [u8; 4],
    pub width: usize,
    pub mnemonic: &'static str,
    pub state_before: CortexM7State,
    pub state_after: CortexM7State,
}

/// Successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub stopped: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: CortexM7State,
}

/// Exact single-core functional machine.
#[derive(Clone, Debug, Default)]
pub struct CortexM7Simulator {
    core: cortex_m3_simulator::CortexM3Simulator,
    ge: u8,
    #[cfg(test)]
    snapshot_failure_after: Cell<Option<usize>>,
}

impl CortexM7Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return to the architectural reset state.
    pub fn reset(&mut self) {
        self.core.reset();
        self.ge = 0;
    }

    /// Snapshot all programmer-visible state and owned memory.
    #[must_use]
    pub fn get_state(&self) -> CortexM7State {
        CortexM7State {
            core: self.core.get_state(),
            ge: self.ge,
        }
    }

    /// Fallibly snapshot all programmer-visible state and owned memory.
    ///
    /// Checked execution uses this path so allocation exhaustion is reported
    /// as a typed failure without committing a partial transition.
    pub fn try_get_state(&self) -> Result<CortexM7State, CortexM7Error> {
        self.try_snapshot()
    }

    /// Atomically validate and install a complete snapshot.
    pub fn restore(&mut self, state: CortexM7State) -> Result<(), CortexM7Error> {
        if state.ge & !0x0f != 0 {
            return Err(CortexM7Error::InvalidState {
                reason: "GE contains reserved bits",
            });
        }
        self.core.restore(state.core)?;
        self.ge = state.ge;
        Ok(())
    }

    /// Atomically replace the machine with a freshly loaded program.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM7Error> {
        self.core.load(program, origin)?;
        self.ge = 0;
        Ok(())
    }

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM7Error> {
        self.core.read_register(index).map_err(Into::into)
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM7Error> {
        self.core.write_register(index, value).map_err(Into::into)
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM7Error> {
        self.core.read_memory(address, width).map_err(Into::into)
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
    ) -> Result<(), CortexM7Error> {
        self.core
            .write_memory(address, width, value)
            .map_err(Into::into)
    }

    /// Register an event using the inherited WFE lifecycle.
    pub fn signal_event(&mut self) {
        self.core.signal_event();
    }

    /// Wake an inherited WFI/WFE wait to represent an accepted interrupt.
    pub fn signal_interrupt(&mut self) {
        self.core.signal_interrupt();
    }

    /// Execute one checked, atomic instruction.
    pub fn step(&mut self) -> Result<StepTrace, CortexM7Error> {
        let before = self.try_snapshot()?;
        if before.core.halted {
            return Err(CortexM3Error::Halted.into());
        }
        if before.core.sleeping {
            return Err(CortexM3Error::Sleeping.into());
        }
        let first = fetch16(&before.core, before.core.pc)?;
        if first & 0xfff0 == 0xf380 {
            let second = fetch_second(&before.core)?;
            if let Some(mask) = cortex_m7_apsr_mask(second) {
                return self.step_apsr_masked(before, first, second, mask);
            }
        }
        if first & 0xfff0 == 0xfa80 {
            let second = fetch_second(&before.core)?;
            if let Some(class) = classify_dsp(first, second) {
                return self.step_dsp(before, first, second, class);
            }
        }
        self.step_inherited(before)
    }

    fn step_inherited(&mut self, before: CortexM7State) -> Result<StepTrace, CortexM7Error> {
        let trace = self.core.step()?;
        if let Err(error) = self.apply_ge_transport(&trace) {
            return Err(self.rollback_error(before, error));
        }
        let after = match self.try_snapshot() {
            Ok(state) => state,
            Err(error) => return Err(self.rollback_error(before, error)),
        };
        Ok(StepTrace {
            pc_before: trace.pc_before,
            pc_after: trace.pc_after,
            raw: trace.raw,
            width: trace.width,
            mnemonic: trace.mnemonic,
            state_before: before,
            state_after: after,
        })
    }

    fn step_dsp(
        &mut self,
        before: CortexM7State,
        first: u16,
        second: u16,
        class: InstructionClass,
    ) -> Result<StepTrace, CortexM7Error> {
        let pc = before.core.pc;
        validate_dsp_operands(first, second, class, pc)?;
        let mut after = self.try_snapshot()?;
        after.core.pc = pc.wrapping_add(4);

        let mnemonic = if before.core.itstate != 0 {
            let condition = before.core.itstate >> 4;
            if condition_holds(before.core.apsr, condition) {
                execute_dsp(&mut after, first, second, class, pc)?
            } else {
                "it-skip"
            }
        } else {
            execute_dsp(&mut after, first, second, class, pc)?
        };
        if before.core.itstate != 0 {
            after.core.itstate = advance_itstate(before.core.itstate);
        }

        let committed = try_clone_state(&after)?;
        self.restore(committed)?;

        Ok(StepTrace {
            pc_before: pc,
            pc_after: after.core.pc,
            raw: [
                first as u8,
                (first >> 8) as u8,
                second as u8,
                (second >> 8) as u8,
            ],
            width: 4,
            mnemonic,
            state_before: before,
            state_after: after,
        })
    }

    fn step_apsr_masked(
        &mut self,
        before: CortexM7State,
        first: u16,
        second: u16,
        mask: u8,
    ) -> Result<StepTrace, CortexM7Error> {
        let pc = before.core.pc;
        let rn = (first & 0xf) as usize;
        if rn > 12 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc }.into());
        }
        let mut after = self.try_snapshot()?;
        after.core.pc = pc.wrapping_add(4);
        let execute =
            before.core.itstate == 0 || condition_holds(before.core.apsr, before.core.itstate >> 4);
        let mnemonic = if execute {
            let value = before.core.registers[rn];
            if mask & 2 != 0 {
                after.core.apsr = value & APSR_NZCVQ_MASK;
            }
            after.ge = ((value >> 16) & 0xf) as u8;
            "msr"
        } else {
            "it-skip"
        };
        if before.core.itstate != 0 {
            after.core.itstate = advance_itstate(before.core.itstate);
        }

        let committed = try_clone_state(&after)?;
        self.restore(committed)?;
        Ok(StepTrace {
            pc_before: pc,
            pc_after: after.core.pc,
            raw: [
                first as u8,
                (first >> 8) as u8,
                second as u8,
                (second >> 8) as u8,
            ],
            width: 4,
            mnemonic,
            state_before: before,
            state_after: after,
        })
    }

    /// Execute until halt/sleep, rolling back the whole run on any failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM7Error> {
        check_retention(max_steps)?;
        let before = self.try_snapshot()?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| CortexM7Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    return Err(self.rollback_error(before, error));
                }
            }
            let stopped = traces.last().is_some_and(|trace| {
                trace.state_after.core.halted || trace.state_after.core.sleeping
            });
            if stopped {
                let final_state = match self.try_snapshot() {
                    Ok(state) => state,
                    Err(error) => return Err(self.rollback_error(before, error)),
                };
                return Ok(ExecutionResult {
                    stopped: true,
                    steps: traces.len(),
                    traces,
                    final_state,
                });
            }
        }
        Err(self.rollback_error(
            before,
            CortexM7Error::StepLimitExceeded { limit: max_steps },
        ))
    }

    /// Atomically load and run a program under the trace-retention bound.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM7Error> {
        check_retention(max_steps)?;
        let before = self.try_snapshot()?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => Err(self.rollback_error(before, error)),
        }
    }

    fn try_snapshot(&self) -> Result<CortexM7State, CortexM7Error> {
        #[cfg(test)]
        if let Some(remaining) = self.snapshot_failure_after.get() {
            if remaining == 0 {
                self.snapshot_failure_after.set(None);
                return Err(CortexM7Error::AllocationFailed);
            }
            self.snapshot_failure_after.set(Some(remaining - 1));
        }
        let core = self.core.try_get_state().map_err(|error| match error {
            CortexM3Error::AllocationFailed => CortexM7Error::AllocationFailed,
            other => CortexM7Error::Core(other),
        })?;
        Ok(CortexM7State { core, ge: self.ge })
    }

    fn rollback_error(&mut self, before: CortexM7State, original: CortexM7Error) -> CortexM7Error {
        match self.restore(before) {
            Ok(()) => original,
            Err(rollback) => rollback,
        }
    }

    fn apply_ge_transport(
        &mut self,
        trace: &cortex_m3_simulator::StepTrace,
    ) -> Result<(), CortexM7Error> {
        if trace.mnemonic == "it-skip" {
            return Ok(());
        }
        let first = u16::from_le_bytes([trace.raw[0], trace.raw[1]]);
        let second = u16::from_le_bytes([trace.raw[2], trace.raw[3]]);
        let sysm = second as u8;
        if !matches!(sysm, 0..=3) {
            return Ok(());
        }
        match trace.mnemonic {
            "mrs" if first == 0xf3ef => {
                let rd = ((second >> 8) & 0xf) as u8;
                let value = self.core.read_register(rd)?;
                self.core
                    .write_register(rd, value | (u32::from(self.ge) << 16))?;
            }
            "msr" if first & 0xfff0 == 0xf380 => {
                let mask = ((second >> 10) & 3) as u8;
                if mask & 1 != 0 {
                    let rn = (first & 0xf) as u8;
                    let value = self.core.read_register(rn)?;
                    self.ge = ((value >> 16) & 0xf) as u8;
                }
            }
            _ => {}
        }
        Ok(())
    }

    #[cfg(test)]
    fn fail_snapshot_after(&self, successful_snapshots: usize) {
        self.snapshot_failure_after.set(Some(successful_snapshots));
    }
}

/// Classify an encoded instruction without executing it.
pub fn classify(first: u16, second: Option<u16>) -> Result<InstructionClass, CortexM7Error> {
    if first & 0xfff0 == 0xf380 {
        let Some(second) = second else {
            return Err(CortexM3Error::FetchOutsideProgram { pc: 0, width: 4 }.into());
        };
        if cortex_m7_apsr_mask(second).is_some() {
            return Ok(InstructionClass::Inherited(
                cortex_m3_simulator::InstructionClass::SpecialRegister,
            ));
        }
    }
    if first & 0xfff0 == 0xfa80 {
        let Some(second) = second else {
            return Err(CortexM3Error::FetchOutsideProgram { pc: 0, width: 4 }.into());
        };
        if let Some(class) = classify_dsp(first, second) {
            return Ok(class);
        }
    }
    cortex_m3_simulator::classify(first, second)
        .map(InstructionClass::Inherited)
        .map_err(Into::into)
}

fn cortex_m7_apsr_mask(second: u16) -> Option<u8> {
    if second & 0xf300 != 0x8000 || !matches!(second as u8, 0..=3) {
        return None;
    }
    let mask = ((second >> 10) & 3) as u8;
    matches!(mask, 1 | 3).then_some(mask)
}

fn classify_dsp(first: u16, second: u16) -> Option<InstructionClass> {
    if first & 0xfff0 != 0xfa80 {
        return None;
    }
    match second & 0xf0f0 {
        0xf080 => Some(InstructionClass::Qadd),
        0xf0a0 => Some(InstructionClass::Qsub),
        0xf000 => Some(InstructionClass::Sadd8),
        0xf040 => Some(InstructionClass::Uadd8),
        _ => None,
    }
}

fn execute_dsp(
    state: &mut CortexM7State,
    first: u16,
    second: u16,
    class: InstructionClass,
    pc: u32,
) -> Result<&'static str, CortexM7Error> {
    let (rd, rn, rm) = validate_dsp_operands(first, second, class, pc)?;

    let left = state.core.registers[rn];
    let right = state.core.registers[rm];
    match class {
        InstructionClass::Qadd => {
            let (value, saturated) =
                saturating_i32(i64::from(left as i32) + i64::from(right as i32));
            state.core.registers[rd] = value as u32;
            if saturated {
                state.core.apsr |= APSR_Q;
            }
            Ok("qadd")
        }
        InstructionClass::Qsub => {
            let (value, saturated) =
                saturating_i32(i64::from(left as i32) - i64::from(right as i32));
            state.core.registers[rd] = value as u32;
            if saturated {
                state.core.apsr |= APSR_Q;
            }
            Ok("qsub")
        }
        InstructionClass::Sadd8 => {
            let mut value = 0_u32;
            let mut ge = 0_u8;
            for lane in 0..4 {
                let shift = lane * 8;
                let a = ((left >> shift) as u8) as i8;
                let b = ((right >> shift) as u8) as i8;
                let sum = i16::from(a) + i16::from(b);
                value |= u32::from(sum as u8) << shift;
                if sum >= 0 {
                    ge |= 1 << lane;
                }
            }
            state.core.registers[rd] = value;
            state.ge = ge;
            Ok("sadd8")
        }
        InstructionClass::Uadd8 => {
            let mut value = 0_u32;
            let mut ge = 0_u8;
            for lane in 0..4 {
                let shift = lane * 8;
                let a = (left >> shift) as u8;
                let b = (right >> shift) as u8;
                let sum = u16::from(a) + u16::from(b);
                value |= u32::from(sum as u8) << shift;
                if sum > u16::from(u8::MAX) {
                    ge |= 1 << lane;
                }
            }
            state.core.registers[rd] = value;
            state.ge = ge;
            Ok("uadd8")
        }
        InstructionClass::Inherited(_) => unreachable!("DSP dispatcher received inherited class"),
    }
}

fn validate_dsp_operands(
    first: u16,
    second: u16,
    class: InstructionClass,
    pc: u32,
) -> Result<(usize, usize, usize), CortexM7Error> {
    let rd = ((second >> 8) & 0xf) as usize;
    let (rn, rm) = match class {
        InstructionClass::Qadd | InstructionClass::Qsub => {
            ((second & 0xf) as usize, (first & 0xf) as usize)
        }
        InstructionClass::Sadd8 | InstructionClass::Uadd8 => {
            ((first & 0xf) as usize, (second & 0xf) as usize)
        }
        InstructionClass::Inherited(_) => unreachable!("DSP dispatcher received inherited class"),
    };
    if rd > 12 || rn > 12 || rm > 12 {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc }.into());
    }
    Ok((rd, rn, rm))
}

fn try_clone_state(state: &CortexM7State) -> Result<CortexM7State, CortexM7Error> {
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(state.core.memory.len())
        .map_err(|_| CortexM7Error::AllocationFailed)?;
    memory.extend_from_slice(&state.core.memory);
    let core = CortexM3State {
        registers: state.core.registers,
        msp: state.core.msp,
        psp: state.core.psp,
        lr: state.core.lr,
        pc: state.core.pc,
        apsr: state.core.apsr,
        ipsr: state.core.ipsr,
        itstate: state.core.itstate,
        primask: state.core.primask,
        basepri: state.core.basepri,
        faultmask: state.core.faultmask,
        control: state.core.control,
        handler_mode: state.core.handler_mode,
        memory,
        halted: state.core.halted,
        sleeping: state.core.sleeping,
        waiting_for_event: state.core.waiting_for_event,
        event_registered: state.core.event_registered,
        exclusive_address: state.core.exclusive_address,
        loaded_origin: state.core.loaded_origin,
        loaded_len: state.core.loaded_len,
    };
    Ok(CortexM7State { core, ge: state.ge })
}

fn saturating_i32(value: i64) -> (i32, bool) {
    if value > i64::from(i32::MAX) {
        (i32::MAX, true)
    } else if value < i64::from(i32::MIN) {
        (i32::MIN, true)
    } else {
        (value as i32, false)
    }
}

fn fetch16(state: &CortexM3State, pc: u32) -> Result<u16, CortexM3Error> {
    if pc & 1 != 0 {
        return Err(CortexM3Error::MisalignedFetch { pc });
    }
    if state.loaded_len < 2 {
        return Err(CortexM3Error::FetchOutsideProgram { pc, width: 2 });
    }
    let relative = pc.checked_sub(state.loaded_origin);
    if relative.is_none_or(|offset| offset > state.loaded_len.saturating_sub(2) as u32) {
        return Err(CortexM3Error::FetchOutsideProgram { pc, width: 2 });
    }
    let start = pc as usize;
    Ok(u16::from_le_bytes([
        state.memory[start],
        state.memory[start + 1],
    ]))
}

fn fetch_second(state: &CortexM3State) -> Result<u16, CortexM3Error> {
    fetch16(state, state.pc.wrapping_add(2)).map_err(|error| {
        if matches!(error, CortexM3Error::FetchOutsideProgram { .. }) {
            CortexM3Error::FetchOutsideProgram {
                pc: state.pc,
                width: 4,
            }
        } else {
            error
        }
    })
}

fn condition_holds(apsr: u32, condition: u8) -> bool {
    let n = apsr & (1 << 31) != 0;
    let z = apsr & (1 << 30) != 0;
    let c = apsr & (1 << 29) != 0;
    let v = apsr & (1 << 28) != 0;
    match condition {
        0 => z,
        1 => !z,
        2 => c,
        3 => !c,
        4 => n,
        5 => !n,
        6 => v,
        7 => !v,
        8 => c && !z,
        9 => !c || z,
        10 => n == v,
        11 => n != v,
        12 => !z && n == v,
        13 => z || n != v,
        14 => true,
        _ => false,
    }
}

fn advance_itstate(itstate: u8) -> u8 {
    if itstate & 7 == 0 {
        0
    } else {
        (itstate & 0xe0) | ((itstate << 1) & 0x1f)
    }
}

fn check_retention(max_steps: usize) -> Result<(), CortexM7Error> {
    if max_steps > MAX_RETAINED_STEPS {
        return Err(CortexM7Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    Ok(())
}

/// Little-endian encoding helpers for the inherited and added instruction set.
pub mod encoding {
    pub use cortex_m3_simulator::encoding::*;

    /// Encode `QADD Rd, Rn, Rm`.
    #[must_use]
    pub const fn qadd(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | ((rm as u16) & 0xf),
            0xf080 | (((rd as u16) & 0xf) << 8) | ((rn as u16) & 0xf),
        ]
    }

    /// Encode `QSUB Rd, Rn, Rm`.
    #[must_use]
    pub const fn qsub(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | ((rm as u16) & 0xf),
            0xf0a0 | (((rd as u16) & 0xf) << 8) | ((rn as u16) & 0xf),
        ]
    }

    /// Encode `SADD8 Rd, Rn, Rm`.
    #[must_use]
    pub const fn sadd8(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | ((rn as u16) & 0xf),
            0xf000 | (((rd as u16) & 0xf) << 8) | ((rm as u16) & 0xf),
        ]
    }

    /// Encode `UADD8 Rd, Rn, Rm`.
    #[must_use]
    pub const fn uadd8(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | ((rn as u16) & 0xf),
            0xf040 | (((rd as u16) & 0xf) << 8) | ((rm as u16) & 0xf),
        ]
    }
}

#[cfg(test)]
mod allocation_regressions {
    use super::{encoding, CortexM7Error, CortexM7Simulator};

    fn load_words(simulator: &mut CortexM7Simulator, words: &[u16]) {
        let mut program = Vec::with_capacity(words.len() * 2);
        for word in words {
            program.extend_from_slice(&word.to_le_bytes());
        }
        simulator.load(&program, 0x100).unwrap();
    }

    #[test]
    fn fallible_snapshot_reports_typed_allocation_failure() {
        let simulator = CortexM7Simulator::new();
        simulator.fail_snapshot_after(0);
        assert_eq!(
            simulator.try_get_state(),
            Err(CortexM7Error::AllocationFailed)
        );
    }

    #[test]
    fn inherited_post_step_snapshot_failure_rolls_back() {
        let mut simulator = CortexM7Simulator::new();
        load_words(&mut simulator, &[encoding::movs(0, 7)]);
        let before = simulator.get_state();
        simulator.fail_snapshot_after(1);

        assert_eq!(simulator.step(), Err(CortexM7Error::AllocationFailed));
        assert_eq!(simulator.get_state(), before);
    }

    #[test]
    fn ge_transport_post_step_snapshot_failure_rolls_back() {
        let mut simulator = CortexM7Simulator::new();
        load_words(&mut simulator, &encoding::mrs(0, 0));
        let mut state = simulator.get_state();
        state.ge = 0b1011;
        simulator.restore(state).unwrap();
        let before = simulator.get_state();
        simulator.fail_snapshot_after(1);

        assert_eq!(simulator.step(), Err(CortexM7Error::AllocationFailed));
        assert_eq!(simulator.get_state(), before);
    }

    #[test]
    fn masked_apsr_candidate_snapshot_failure_is_atomic() {
        let mut simulator = CortexM7Simulator::new();
        load_words(
            &mut simulator,
            &encoding::msr_masked(0, 0, encoding::ApsrMask::NzcvqGe),
        );
        simulator.write_register(0, 0xa80b_0000).unwrap();
        let before = simulator.get_state();
        simulator.fail_snapshot_after(1);

        assert_eq!(simulator.step(), Err(CortexM7Error::AllocationFailed));
        assert_eq!(simulator.get_state(), before);
    }

    #[test]
    fn dsp_candidate_snapshot_failure_is_atomic() {
        let mut simulator = CortexM7Simulator::new();
        load_words(&mut simulator, &encoding::qadd(0, 1, 2));
        simulator.write_register(1, 4).unwrap();
        simulator.write_register(2, 5).unwrap();
        let before = simulator.get_state();
        simulator.fail_snapshot_after(1);

        assert_eq!(simulator.step(), Err(CortexM7Error::AllocationFailed));
        assert_eq!(simulator.get_state(), before);
    }

    #[test]
    fn final_run_snapshot_failure_restores_the_whole_run() {
        let mut simulator = CortexM7Simulator::new();
        load_words(&mut simulator, &[encoding::bkpt(0)]);
        let before = simulator.get_state();
        simulator.fail_snapshot_after(3);

        assert_eq!(simulator.run(1), Err(CortexM7Error::AllocationFailed));
        assert_eq!(simulator.get_state(), before);
    }
}
