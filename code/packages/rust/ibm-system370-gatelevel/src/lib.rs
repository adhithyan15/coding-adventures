//! DFF-backed educational IBM System/370 architectural simulator.
//!
//! One embedded System/360 gate engine owns every persistent architectural
//! bit. The five bounded System/370 orders settle independent gate-derived
//! next values and clock that same stable-Q bank only after oracle agreement.

mod allocation;
mod engine;
pub mod gates;

pub use allocation::{clear_allocation_failure, fail_allocation, AllocationSite};
pub use ibm_system370_simulator::{
    ExecutionResult, StepTrace, System370Error, System370State, MEMORY_BYTES,
};

use ibm_system360_gatelevel::{System360GateLevel, System360State};
use ibm_system370_simulator::System370Simulator;

/// Exact persistent architectural D-flip-flop count.
pub const FLIP_FLOP_COUNT: usize = 8_389_403;
/// Stable storage-plus-extension-network educational estimate.
pub const ESTIMATED_GATE_COUNT: usize = ibm_system360_gatelevel::ESTIMATED_GATE_COUNT + 65_536;

/// System/370 gate model with exactly one inherited stable-Q state bank.
#[derive(Debug, Eq, PartialEq)]
pub struct System370GateLevel {
    core: System360GateLevel,
}

impl Default for System370GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl System370GateLevel {
    /// Construct deterministic reset state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: System360GateLevel::new(),
        }
    }

    /// Construct reset state through the checked initial-storage allocation.
    pub fn try_new() -> Result<Self, System370Error> {
        allocation::checkpoint(AllocationSite::InitialStorage)
            .map_err(|()| System370Error::AllocationFailed)?;
        Ok(Self {
            core: System360GateLevel::try_new_storage()
                .map_err(|_| System370Error::AllocationFailed)?,
        })
    }

    /// Clock every architectural bit and lifecycle scalar to reset state.
    pub fn reset(&mut self) {
        self.core.reset();
    }

    /// Reconstruct the complete public state from stable Q.
    #[must_use]
    pub fn get_state(&self) -> System370State {
        from_system360(self.core.get_state())
    }

    fn try_state(&self, site: AllocationSite) -> Result<System370State, System370Error> {
        allocation::checkpoint(site).map_err(|()| System370Error::AllocationFailed)?;
        Ok(from_system360(
            self.core
                .try_get_state_storage()
                .map_err(|_| System370Error::AllocationFailed)?,
        ))
    }

    /// Validate and atomically clock a complete snapshot.
    pub fn restore(&mut self, snapshot: System370State) -> Result<(), System370Error> {
        validate_state(&snapshot)?;
        allocation::checkpoint(AllocationSite::RestoreMemory)
            .map_err(|()| System370Error::AllocationFailed)?;
        let mut memory = Vec::new();
        memory
            .try_reserve_exact(snapshot.memory.len())
            .map_err(|_| System370Error::AllocationFailed)?;
        memory.extend_from_slice(&snapshot.memory);
        let mut staged = snapshot;
        staged.memory = memory;
        self.core
            .restore(to_system360(staged))
            .map_err(map_system360_error)
    }

    /// Reset and install a raw big-endian program image atomically.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<usize, System370Error> {
        allocation::checkpoint(AllocationSite::LoadImage)
            .map_err(|()| System370Error::AllocationFailed)?;
        let mut staged = Vec::new();
        staged
            .try_reserve_exact(program.len())
            .map_err(|_| System370Error::AllocationFailed)?;
        staged.extend_from_slice(program);
        self.core.load(&staged, origin).map_err(map_system360_error)
    }

    pub fn read_gpr(&self, index: usize) -> Result<u32, System370Error> {
        self.core.read_gpr(index).map_err(map_system360_error)
    }

    pub fn write_gpr(&mut self, index: usize, value: u32) -> Result<(), System370Error> {
        self.core
            .write_gpr(index, value)
            .map_err(map_system360_error)
    }

    pub fn read_fpr(&self, index: usize) -> Result<u64, System370Error> {
        self.core.read_fpr(index).map_err(map_system360_error)
    }

    pub fn write_fpr(&mut self, index: usize, value: u64) -> Result<(), System370Error> {
        self.core
            .write_fpr(index, value)
            .map_err(map_system360_error)
    }

    pub fn set_program_counter(&mut self, pc: u32) -> Result<(), System370Error> {
        self.core
            .set_program_counter(pc)
            .map_err(map_system360_error)
    }

    pub fn read_memory(&self, address: u32, bytes: usize) -> Result<Vec<u8>, System370Error> {
        self.core
            .read_memory(address, bytes)
            .map_err(map_system360_error)
    }

    pub fn write_memory(&mut self, address: u32, data: &[u8]) -> Result<(), System370Error> {
        self.core
            .write_memory(address, data)
            .map_err(map_system360_error)
    }

    /// Execute one gate-authoritative checked transition.
    pub fn step(&mut self) -> Result<StepTrace, System370Error> {
        if self.core.halted_q() {
            return Err(System370Error::Halted);
        }
        let _rollback_only = self.try_state(AllocationSite::StateSnapshot)?;
        let pc = self.core.program_counter_q();
        let opcode = self.fetch_opcode_q(pc)?;
        let Some(extension) = engine::classify_extension(opcode) else {
            return self
                .core
                .step()
                .map(convert_trace)
                .map_err(map_system360_error);
        };
        let width = engine::instruction_width(opcode).expect("extension has a width") as usize;
        let raw = self.fetch_exact_q(pc, width)?;
        let settled_result = engine::settle(&CoreQ(&self.core), pc, &raw[..width], extension);
        if matches!(&settled_result, Err(System370Error::AllocationFailed)) {
            return Err(System370Error::AllocationFailed);
        }

        let oracle_before = self.try_state(AllocationSite::OracleState)?;
        let mut oracle = System370Simulator::new();
        oracle.restore(oracle_before)?;
        let oracle_result = oracle.step();

        let settled = match (settled_result, oracle_result) {
            (Err(System370Error::AllocationFailed), _) => {
                return Err(System370Error::AllocationFailed)
            }
            (Err(gate_error), Err(oracle_error)) if gate_error == oracle_error => {
                return Err(gate_error)
            }
            (Err(_), Err(_)) => {
                return gate_disagrees("gate and oracle fault classifications differ")
            }
            (Err(_), Ok(_)) => return gate_disagrees("gate rejected an oracle-valid extension"),
            (Ok(_), Err(_)) => return gate_disagrees("gate accepted an oracle-invalid extension"),
            (Ok(settled), Ok(oracle_trace)) => {
                if settled.raw != oracle_trace.raw
                    || settled.width != oracle_trace.width
                    || settled.mnemonic != oracle_trace.mnemonic
                    || settled.description != oracle_trace.description
                    || pc != oracle_trace.pc_before
                    || settled.pc != oracle_trace.pc_after
                {
                    return gate_disagrees("gate trace and functional trace differ");
                }
                settled
            }
        };

        let oracle_next = oracle.get_state();
        if !self.settled_matches_oracle(&settled, &oracle_next) {
            return gate_disagrees("settled gate transition diverged from functional oracle");
        }
        self.commit_extension(&settled);
        Ok(StepTrace {
            pc_before: pc,
            pc_after: settled.pc,
            raw: settled.raw,
            width: settled.width,
            mnemonic: settled.mnemonic,
            description: settled.description,
        })
    }

    fn commit_extension(&mut self, settled: &engine::SettledExtension) {
        for &(address, value) in &settled.memory_writes {
            self.core.clock_memory_q(address, value);
        }
        for &(register, value) in &settled.gpr_updates[..settled.gpr_count] {
            self.core.clock_gpr_q(register, value);
        }
        if let Some(condition_code) = settled.condition_code {
            self.core.clock_condition_code_q(condition_code);
        }
        self.core.clock_program_counter_q(settled.pc);
    }

    fn fetch_opcode_q(&self, pc: u32) -> Result<u8, System370Error> {
        if !pc.is_multiple_of(2) {
            return Err(System370Error::InvalidState {
                reason: "instruction address is not halfword aligned",
            });
        }
        let (origin, len) = self.core.installed_range();
        let start = pc as usize;
        let low = origin as usize;
        let high = low + len;
        if start < low || start >= high {
            return Err(System370Error::FetchOutsideProgram { pc });
        }
        Ok(self.core.memory_q(start))
    }

    fn fetch_exact_q(&self, pc: u32, width: usize) -> Result<[u8; 6], System370Error> {
        let start = pc as usize;
        let (origin, len) = self.core.installed_range();
        let high = origin as usize + len;
        if start + width > high {
            return Err(System370Error::TruncatedInstruction { pc, needed: width });
        }
        let mut raw = [0; 6];
        for (offset, byte) in raw[..width].iter_mut().enumerate() {
            *byte = self.core.memory_q(start + offset);
        }
        Ok(raw)
    }

    fn settled_matches_oracle(
        &self,
        settled: &engine::SettledExtension,
        oracle: &System370State,
    ) -> bool {
        for register in 0..16 {
            let mut expected = self.core.gpr_q(register);
            for &(updated, value) in &settled.gpr_updates[..settled.gpr_count] {
                if updated == register {
                    expected = value;
                }
            }
            if oracle.gprs[register] != expected {
                return false;
            }
        }
        for slot in 0..4 {
            if oracle.fprs[slot] != self.core.fpr_q(slot) {
                return false;
            }
        }
        let mut writes = settled.memory_writes.iter().peekable();
        for address in 0..MEMORY_BYTES {
            let expected = if writes.peek().is_some_and(|(next, _)| *next == address) {
                writes.next().expect("peeked write exists").1
            } else {
                self.core.memory_q(address)
            };
            if oracle.memory[address] != expected {
                return false;
            }
        }
        let (origin, len) = self.core.installed_range();
        oracle.pc == settled.pc
            && oracle.condition_code
                == settled
                    .condition_code
                    .unwrap_or_else(|| self.core.condition_code_q())
            && oracle.halted == self.core.halted_q()
            && oracle.loaded_origin == origin
            && oracle.loaded_len == len
    }

    /// Execute until halt or the supplied bound, rolling back on every error.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, System370Error> {
        let before = self.try_state(AllocationSite::StateSnapshot)?;
        allocation::checkpoint(AllocationSite::TraceReservation)
            .map_err(|()| System370Error::AllocationFailed)?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| System370Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => {
                    if allocation::checkpoint(AllocationSite::TraceRetention).is_err()
                        || traces.try_reserve(1).is_err()
                    {
                        self.restore_rollback(before);
                        return Err(System370Error::AllocationFailed);
                    }
                    traces.push(trace);
                    if self.core.halted_q() {
                        let final_state = match self.try_state(AllocationSite::FinalState) {
                            Ok(state) => state,
                            Err(error) => {
                                self.restore_rollback(before);
                                return Err(error);
                            }
                        };
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            final_state,
                            traces,
                        });
                    }
                }
                Err(error) => {
                    self.restore_rollback(before);
                    return Err(error);
                }
            }
        }
        self.restore_rollback(before);
        Err(System370Error::MaxStepsExceeded { max_steps })
    }

    /// Atomically load and run one image on this CPU.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, System370Error> {
        let before = self.try_state(AllocationSite::StateSnapshot)?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore_rollback(before);
                Err(error)
            }
        }
    }

    fn restore_rollback(&mut self, state: System370State) {
        let state = to_system360(state);
        self.core.restore_validated_storage(&state);
    }

    #[must_use]
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    #[must_use]
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }
}

struct CoreQ<'a>(&'a System360GateLevel);

impl engine::StableQ for CoreQ<'_> {
    fn gpr_q(&self, index: usize) -> u32 {
        self.0.gpr_q(index)
    }

    fn memory_q(&self, index: usize) -> u8 {
        self.0.memory_q(index)
    }
}

fn validate_state(state: &System370State) -> Result<(), System370Error> {
    if state.memory.len() != MEMORY_BYTES {
        return Err(System370Error::InvalidState {
            reason: "memory has the wrong installed size",
        });
    }
    if state.condition_code > 3 {
        return Err(System370Error::InvalidState {
            reason: "condition code is outside 0..3",
        });
    }
    if state.pc > 0x00ff_ffff || !state.pc.is_multiple_of(2) {
        return Err(System370Error::InvalidState {
            reason: "instruction address is invalid",
        });
    }
    let end = (state.loaded_origin as usize).checked_add(state.loaded_len);
    if state.loaded_origin > 0x00ff_ffff
        || state.loaded_len != 0 && !state.loaded_origin.is_multiple_of(2)
        || end.is_none_or(|end| end > MEMORY_BYTES)
    {
        return Err(System370Error::InvalidState {
            reason: "installed program range is invalid",
        });
    }
    Ok(())
}

fn to_system360(state: System370State) -> System360State {
    System360State {
        gprs: state.gprs,
        fprs: state.fprs,
        pc: state.pc,
        condition_code: state.condition_code,
        memory: state.memory,
        halted: state.halted,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    }
}

fn from_system360(state: System360State) -> System370State {
    System370State {
        gprs: state.gprs,
        fprs: state.fprs,
        pc: state.pc,
        condition_code: state.condition_code,
        memory: state.memory,
        halted: state.halted,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    }
}

fn convert_trace(trace: ibm_system360_gatelevel::StepTrace) -> StepTrace {
    StepTrace {
        pc_before: trace.pc_before,
        pc_after: trace.pc_after,
        raw: trace.raw,
        width: trace.width,
        mnemonic: trace.mnemonic,
        description: trace.description,
    }
}

fn gate_disagrees<T>(reason: &'static str) -> Result<T, System370Error> {
    Err(System370Error::Specification { reason })
}

fn map_system360_error(error: ibm_system360_gatelevel::System360Error) -> System370Error {
    use ibm_system360_gatelevel::System360Error as Source;
    match error {
        Source::EmptyProgram => System370Error::EmptyProgram,
        Source::MisalignedProgram { origin } => System370Error::MisalignedProgram { origin },
        Source::ProgramOutOfRange { origin, bytes } => {
            System370Error::ProgramOutOfRange { origin, bytes }
        }
        Source::FetchOutsideProgram { pc } => System370Error::FetchOutsideProgram { pc },
        Source::TruncatedInstruction { pc, needed } => {
            System370Error::TruncatedInstruction { pc, needed }
        }
        Source::UnknownOpcode { opcode, pc } => System370Error::UnknownOpcode { opcode, pc },
        Source::UnsupportedSupervisorCall { number } => {
            System370Error::UnsupportedSupervisorCall { number }
        }
        Source::MisalignedAccess { address, alignment } => {
            System370Error::MisalignedAccess { address, alignment }
        }
        Source::MemoryOutOfRange { address, bytes } => {
            System370Error::MemoryOutOfRange { address, bytes }
        }
        Source::InvalidRegister { bank, index } => System370Error::InvalidRegister { bank, index },
        Source::InvalidState { reason } => System370Error::InvalidState { reason },
        Source::Specification { reason } => System370Error::Specification { reason },
        Source::FixedPointOverflow => System370Error::FixedPointOverflow,
        Source::FixedPointDivide => System370Error::FixedPointDivide,
        Source::FloatingExponent => System370Error::FloatingExponent,
        Source::FloatingDivide => System370Error::FloatingDivide,
        Source::Halted => System370Error::Halted,
        Source::MaxStepsExceeded { max_steps } => System370Error::MaxStepsExceeded { max_steps },
    }
}
