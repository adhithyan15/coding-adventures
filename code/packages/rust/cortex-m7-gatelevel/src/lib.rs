//! DFF-backed Armv7E-M Cortex-M7 architectural gate simulator.

use core::fmt;
use cortex_m3_gatelevel::{
    integration::{self, AllocationController},
    CortexM3Error, CortexM3GateLevel, CortexM3State,
};
use logic_gates::sequential::{register, FlipFlopState};

pub mod gates;
use gates::{DspClass, RepositoryGates};

pub use cortex_m3_gatelevel::{MEMORY_SIZE, PC};
pub const MAX_RETAINED_STEPS: usize = cortex_m3_gatelevel::MAX_RETAINED_STEPS;
pub const FLIP_FLOP_COUNT: usize = cortex_m3_gatelevel::FLIP_FLOP_COUNT + 4;
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 135_168;
const APSR_Q: u32 = 1 << 27;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstructionClass {
    Inherited(cortex_m3_gatelevel::InstructionClass),
    Qadd,
    Qsub,
    Sadd8,
    Uadd8,
}

impl From<DspClass> for InstructionClass {
    fn from(value: DspClass) -> Self {
        match value {
            DspClass::Qadd => Self::Qadd,
            DspClass::Qsub => Self::Qsub,
            DspClass::Sadd8 => Self::Sadd8,
            DspClass::Uadd8 => Self::Uadd8,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CortexM7State {
    pub core: CortexM3State,
    pub ge: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CortexM7Error {
    Core(CortexM3Error),
    InvalidState { reason: &'static str },
    RetentionLimitExceeded { requested: usize, limit: usize },
    StepLimitExceeded { limit: usize },
    AllocationFailed,
}

impl From<CortexM3Error> for CortexM7Error {
    fn from(value: CortexM3Error) -> Self {
        match value {
            CortexM3Error::AllocationFailed => Self::AllocationFailed,
            other => Self::Core(other),
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: [u8; 4],
    pub width: usize,
    pub mnemonic: &'static str,
    pub state_before: CortexM7State,
    pub state_after: CortexM7State,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub stopped: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: CortexM7State,
}

fn stable_state(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

fn clock_ge(target: &mut [u8; 4], value: u8) {
    let input: [u8; 4] = std::array::from_fn(|bit| (value >> bit) & 1);
    let mut state: [FlipFlopState; 4] = std::array::from_fn(|bit| stable_state(target[bit]));
    register(&input, 0, &mut state);
    target.copy_from_slice(&register(&input, 1, &mut state));
}

fn read_ge(bits: &[u8; 4]) -> u8 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (*q << bit))
}

#[derive(Debug)]
pub struct CortexM7GateLevel {
    core: CortexM3GateLevel,
    ge: [u8; 4],
    allocations: AllocationController,
}

impl Default for CortexM7GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl CortexM7GateLevel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: CortexM3GateLevel::new(),
            ge: [0; 4],
            allocations: AllocationController::new(),
        }
    }

    pub fn reset(&mut self) {
        self.core.reset();
        clock_ge(&mut self.ge, 0);
    }

    #[must_use]
    pub fn get_state(&self) -> CortexM7State {
        CortexM7State {
            core: self.core.get_state(),
            ge: read_ge(&self.ge),
        }
    }

    pub fn try_get_state(&self) -> Result<CortexM7State, CortexM7Error> {
        let memory = self.allocations.memory_buffer()?;
        let core = integration::snapshot_into(&self.core, memory, &self.allocations)?;
        Ok(CortexM7State {
            core,
            ge: read_ge(&self.ge),
        })
    }

    pub fn restore(&mut self, state: CortexM7State) -> Result<(), CortexM7Error> {
        if state.ge & !0x0f != 0 {
            return Err(CortexM7Error::InvalidState {
                reason: "GE contains reserved bits",
            });
        }
        let ge = state.ge;
        let candidate = integration::settle(state.core)?;
        integration::commit(&mut self.core, &candidate);
        clock_ge(&mut self.ge, ge);
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM7Error> {
        self.core.load(program, origin)?;
        clock_ge(&mut self.ge, 0);
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

    pub fn signal_event(&mut self) {
        self.core.signal_event();
    }

    pub fn signal_interrupt(&mut self) {
        self.core.signal_interrupt();
    }

    pub fn step(&mut self) -> Result<StepTrace, CortexM7Error> {
        let state = self.allocations.memory_buffer()?;
        let before = self.allocations.memory_buffer()?;
        let mut validation = self.allocations.memory_buffer()?;
        self.step_preallocated(state, before, &mut validation)
    }

    fn step_preallocated(
        &mut self,
        state_memory: Vec<u8>,
        before_memory: Vec<u8>,
        validation_memory: &mut Vec<u8>,
    ) -> Result<StepTrace, CortexM7Error> {
        let before_core = integration::snapshot_into(&self.core, state_memory, &self.allocations)?;
        if before_core.halted {
            return Err(CortexM3Error::Halted.into());
        }
        if before_core.sleeping {
            return Err(CortexM3Error::Sleeping.into());
        }
        let first = fetch16(&before_core, before_core.pc)?;
        let dsp = if gates::dsp_prefix(&mut RepositoryGates, first) == 1 {
            let second = fetch_second(&before_core, before_core.pc)?;
            gates::classify(&mut RepositoryGates, first, second).map(|class| (second, class))
        } else {
            None
        };
        match dsp {
            Some((second, class)) => {
                self.step_dsp(before_core, before_memory, first, second, class)
            }
            None => self.step_inherited(before_core, before_memory, validation_memory),
        }
    }

    fn step_inherited(
        &mut self,
        before: CortexM3State,
        before_memory: Vec<u8>,
        validation_memory: &mut Vec<u8>,
    ) -> Result<StepTrace, CortexM7Error> {
        let ge_before = read_ge(&self.ge);
        let mut trace = integration::delegated_step_from(
            before,
            before_memory,
            validation_memory,
            &self.allocations,
        )?;
        let ge_after = apply_ge_transport(
            &mut trace.state_after,
            ge_before,
            trace.raw,
            trace.mnemonic,
            trace.state_before.registers,
        );
        let candidate = integration::settle(trace.state_after)?;
        integration::commit(&mut self.core, &candidate);
        trace.state_after = candidate.into_state();
        clock_ge(&mut self.ge, ge_after);
        Ok(StepTrace {
            pc_before: trace.pc_before,
            pc_after: trace.pc_after,
            raw: trace.raw,
            width: trace.width,
            mnemonic: trace.mnemonic,
            state_before: CortexM7State {
                core: trace.state_before,
                ge: ge_before,
            },
            state_after: CortexM7State {
                core: trace.state_after,
                ge: ge_after,
            },
        })
    }

    fn step_dsp(
        &mut self,
        before: CortexM3State,
        candidate_memory: Vec<u8>,
        first: u16,
        second: u16,
        class: DspClass,
    ) -> Result<StepTrace, CortexM7Error> {
        let mut backend = RepositoryGates;
        let ([rd, rn, rm], operands_ok) = gates::fields(&mut backend, first, second, class);
        if operands_ok == 0 {
            return Err(CortexM3Error::UnknownInstruction32 {
                first,
                second,
                pc: before.pc,
            }
            .into());
        }
        let mut after = integration::clone_into(&before, candidate_memory, &self.allocations)?;
        let pc = before.pc;
        after.pc = cortex_m3_gatelevel::gates::add_32(pc, 4, 0).0;
        let ge_before = read_ge(&self.ge);
        let in_it = gates::it_active(&mut backend, before.itstate);
        let condition = gates::condition_holds(&mut backend, before.apsr, before.itstate >> 4);
        let write = gates::write_enable(&mut backend, in_it, condition);
        let mut ge_after = ge_before;
        let mnemonic = if write == 0 {
            "it-skip"
        } else {
            let left = before.registers[rn];
            let right = before.registers[rm];
            let (result, mnemonic) = match class {
                DspClass::Qadd | DspClass::Qsub => {
                    let old_q = ((before.apsr >> 27) & 1) as u8;
                    let (result, q, _) = gates::saturating(
                        &mut backend,
                        left,
                        right,
                        u8::from(class == DspClass::Qsub),
                        old_q,
                    );
                    after.apsr = (after.apsr & !APSR_Q) | (u32::from(q) << 27);
                    (
                        result,
                        if class == DspClass::Qadd {
                            "qadd"
                        } else {
                            "qsub"
                        },
                    )
                }
                DspClass::Sadd8 | DspClass::Uadd8 => {
                    let signed = u8::from(class == DspClass::Sadd8);
                    let (result, ge) = gates::parallel_add(&mut backend, left, right, signed);
                    ge_after = ge;
                    (result, if signed == 1 { "sadd8" } else { "uadd8" })
                }
            };
            after.registers[rd] = result;
            mnemonic
        };
        if in_it == 1 {
            after.itstate = gates::advance_itstate(&mut backend, before.itstate);
        }
        let state_before = CortexM7State {
            core: before,
            ge: ge_before,
        };
        let candidate = integration::settle(after)?;
        integration::commit(&mut self.core, &candidate);
        let after = candidate.into_state();
        clock_ge(&mut self.ge, ge_after);
        Ok(StepTrace {
            pc_before: pc,
            pc_after: after.pc,
            raw: [
                first as u8,
                (first >> 8) as u8,
                second as u8,
                (second >> 8) as u8,
            ],
            width: 4,
            mnemonic,
            state_before,
            state_after: CortexM7State {
                core: after,
                ge: ge_after,
            },
        })
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM7Error> {
        check_retention(max_steps)?;
        let traces = self.allocations.trace_buffer(max_steps)?;
        let memories =
            self.preallocate_memories(max_steps.checked_mul(2).and_then(|n| n.checked_add(3)))?;
        self.run_preallocated(max_steps, traces, memories)
    }

    fn preallocate_memories(&self, count: Option<usize>) -> Result<Vec<Vec<u8>>, CortexM7Error> {
        let count = count.ok_or(CortexM7Error::AllocationFailed)?;
        let mut memories = self.allocations.trace_buffer(count)?;
        for _ in 0..count {
            memories.push(self.allocations.memory_buffer()?);
        }
        Ok(memories)
    }

    fn run_preallocated(
        &mut self,
        max_steps: usize,
        mut traces: Vec<StepTrace>,
        mut memories: Vec<Vec<u8>>,
    ) -> Result<ExecutionResult, CortexM7Error> {
        let rollback_core = integration::snapshot_into(
            &self.core,
            memories.pop().expect("rollback buffer preallocated"),
            &self.allocations,
        )?;
        let rollback_ge = read_ge(&self.ge);
        let mut validation = memories.pop().expect("validation buffer preallocated");
        for _ in 0..max_steps {
            let state = memories.pop().expect("state buffer preallocated");
            let before = memories.pop().expect("before buffer preallocated");
            match self.step_preallocated(state, before, &mut validation) {
                Ok(trace) => {
                    let stopped = trace.state_after.core.halted || trace.state_after.core.sleeping;
                    traces.push(trace);
                    if stopped {
                        let core = match integration::snapshot_into(
                            &self.core,
                            memories.pop().expect("final buffer preallocated"),
                            &self.allocations,
                        ) {
                            Ok(core) => core,
                            Err(error) => {
                                self.rollback(rollback_core, rollback_ge);
                                return Err(error.into());
                            }
                        };
                        return Ok(ExecutionResult {
                            stopped: true,
                            steps: traces.len(),
                            traces,
                            final_state: CortexM7State {
                                core,
                                ge: read_ge(&self.ge),
                            },
                        });
                    }
                }
                Err(error) => {
                    self.rollback(rollback_core, rollback_ge);
                    return Err(error);
                }
            }
        }
        self.rollback(rollback_core, rollback_ge);
        Err(CortexM7Error::StepLimitExceeded { limit: max_steps })
    }

    fn rollback(&mut self, state: CortexM3State, ge: u8) {
        let candidate = integration::settle(state).expect("captured Q snapshot is valid");
        integration::commit(&mut self.core, &candidate);
        clock_ge(&mut self.ge, ge);
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM7Error> {
        check_retention(max_steps)?;
        let traces = self.allocations.trace_buffer(max_steps)?;
        let mut memories =
            self.preallocate_memories(max_steps.checked_mul(2).and_then(|n| n.checked_add(4)))?;
        let original = integration::snapshot_into(
            &self.core,
            memories.pop().expect("outer rollback buffer preallocated"),
            &self.allocations,
        )?;
        let original_ge = read_ge(&self.ge);
        self.load(program, origin)?;
        match self.run_preallocated(max_steps, traces, memories) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.rollback(original, original_ge);
                Err(error)
            }
        }
    }

    #[doc(hidden)]
    pub fn fail_allocation_after(&self, successful: usize) {
        self.allocations.fail_after(successful);
    }

    #[must_use]
    pub fn flip_flop_count(&self) -> usize {
        integration::storage_inventory(&self.core).total() + self.ge.len()
    }

    #[must_use]
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }
}

fn apply_ge_transport(
    after: &mut CortexM3State,
    old_ge: u8,
    raw: [u8; 4],
    mnemonic: &'static str,
    before_registers: [u32; 13],
) -> u8 {
    if mnemonic == "it-skip" {
        return old_ge;
    }
    let first = u16::from_le_bytes([raw[0], raw[1]]);
    let second = u16::from_le_bytes([raw[2], raw[3]]);
    let mut backend = RepositoryGates;
    let (mrs, msr) = gates::apsr_transport_lines(&mut backend, first, second);
    if mrs == 1 {
        let rd = ((second >> 8) & 0xf) as usize;
        after.registers[rd] |= u32::from(old_ge) << 16;
        old_ge
    } else if msr == 1 {
        let rn = (first & 0xf) as usize;
        gates::transport_ge(&mut backend, old_ge, before_registers[rn], 1)
    } else {
        old_ge
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

fn fetch_second(state: &CortexM3State, pc: u32) -> Result<u16, CortexM3Error> {
    fetch16(state, pc.wrapping_add(2)).map_err(|error| match error {
        CortexM3Error::FetchOutsideProgram { .. } => {
            CortexM3Error::FetchOutsideProgram { pc, width: 4 }
        }
        other => other,
    })
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

pub fn classify(first: u16, second: Option<u16>) -> Result<InstructionClass, CortexM7Error> {
    if gates::dsp_prefix(&mut RepositoryGates, first) == 1 {
        let Some(second) = second else {
            return Err(CortexM3Error::FetchOutsideProgram { pc: 0, width: 4 }.into());
        };
        if let Some(class) = gates::classify(&mut RepositoryGates, first, second) {
            return Ok(class.into());
        }
    }
    cortex_m3_gatelevel::classify(first, second)
        .map(InstructionClass::Inherited)
        .map_err(Into::into)
}

pub mod encoding {
    pub use cortex_m3_gatelevel::encoding::*;

    #[must_use]
    pub const fn qadd(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | ((rm as u16) & 0xf),
            0xf080 | (((rd as u16) & 0xf) << 8) | ((rn as u16) & 0xf),
        ]
    }
    #[must_use]
    pub const fn qsub(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | ((rm as u16) & 0xf),
            0xf0a0 | (((rd as u16) & 0xf) << 8) | ((rn as u16) & 0xf),
        ]
    }
    #[must_use]
    pub const fn sadd8(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | ((rn as u16) & 0xf),
            0xf000 | (((rd as u16) & 0xf) << 8) | ((rm as u16) & 0xf),
        ]
    }
    #[must_use]
    pub const fn uadd8(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | ((rn as u16) & 0xf),
            0xf040 | (((rd as u16) & 0xf) << 8) | ((rm as u16) & 0xf),
        ]
    }
}
