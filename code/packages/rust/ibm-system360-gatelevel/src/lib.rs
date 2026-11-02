//! Gate-backed IBM System/360 architectural simulator.
//!
//! All persistent programmer-visible bits are stored in repository D
//! flip-flops. Successful architectural transitions are computed by this
//! package's independent gate networks. The checked functional model supplies
//! trace and fault oracles, and every settled gate state must match it exactly
//! before a clock edge is committed.

pub use ibm_system360_simulator::{
    ExecutionResult, StepTrace, System360Error, System360Simulator, System360State, MEMORY_BYTES,
};

mod executor;
pub mod gates;
mod hfp;
mod state;

use hfp::GateHfp;
use state::{clock_bit, clock_bits, read_bits, DffMemory};

/// Exact number of persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize = MEMORY_BYTES * 8 + 16 * 32 + 4 * 64 + 24 + 2 + 1;
/// Stable educational storage-plus-combinational gate estimate.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 196_608;

/// Failure to allocate checked stable-Q storage for an embedding gate model.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageAllocationError;

/// Checked allocation boundaries exposed to an embedding successor gate.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddedAllocationSite {
    EntryState,
    OracleState,
    GateCandidate,
    RawInstruction,
    Mnemonic,
    Description,
}

/// IBM System/360 state stored in simulated D flip-flops.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct System360GateLevel {
    memory: DffMemory,
    gprs: [[u8; 32]; 16],
    fprs: [[u8; 64]; 4],
    pc: [u8; 24],
    condition_code: [u8; 2],
    halted: u8,
    loaded_origin: u32,
    loaded_len: usize,
}

impl Default for System360GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl System360GateLevel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: DffMemory::new(MEMORY_BYTES),
            gprs: [[0; 32]; 16],
            fprs: [[0; 64]; 4],
            pc: [0; 24],
            condition_code: [0; 2],
            halted: 0,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    /// Construct the exact stable-Q bank through a checked allocation.
    ///
    /// This is exposed for source-backed successor gate packages that embed
    /// this engine as their sole architectural storage bank.
    #[doc(hidden)]
    pub fn try_new_storage() -> Result<Self, StorageAllocationError> {
        Ok(Self {
            memory: DffMemory::try_new(MEMORY_BYTES).map_err(|()| StorageAllocationError)?,
            gprs: [[0; 32]; 16],
            fprs: [[0; 64]; 4],
            pc: [0; 24],
            condition_code: [0; 2],
            halted: 0,
            loaded_origin: 0,
            loaded_len: 0,
        })
    }

    pub fn reset(&mut self) {
        self.memory.clear();
        for register in &mut self.gprs {
            clock_bits(register, 0);
        }
        for register in &mut self.fprs {
            clock_bits(register, 0);
        }
        clock_bits(&mut self.pc, 0);
        clock_bits(&mut self.condition_code, 0);
        clock_bit(&mut self.halted, false);
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    #[must_use]
    pub fn get_state(&self) -> System360State {
        self.state_with_memory(self.memory.snapshot())
    }

    fn state_with_memory(&self, memory: Vec<u8>) -> System360State {
        System360State {
            gprs: std::array::from_fn(|index| read_bits(&self.gprs[index]) as u32),
            fprs: std::array::from_fn(|index| read_bits(&self.fprs[index])),
            pc: read_bits(&self.pc) as u32,
            condition_code: read_bits(&self.condition_code) as u8,
            memory,
            halted: self.halted != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    /// Reconstruct stable-Q state through checked owned-memory allocation.
    #[doc(hidden)]
    pub fn try_get_state_storage(&self) -> Result<System360State, StorageAllocationError> {
        Ok(self.state_with_memory(
            self.memory
                .try_snapshot()
                .map_err(|()| StorageAllocationError)?,
        ))
    }

    pub fn restore(&mut self, snapshot: System360State) -> Result<(), System360Error> {
        let mut validator = System360Simulator::new();
        validator.restore(snapshot.clone())?;
        self.restore_validated(&snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &System360State) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (register, value) in self.gprs.iter_mut().zip(snapshot.gprs) {
            clock_bits(register, u64::from(value));
        }
        for (register, value) in self.fprs.iter_mut().zip(snapshot.fprs) {
            clock_bits(register, value);
        }
        clock_bits(&mut self.pc, u64::from(snapshot.pc));
        clock_bits(&mut self.condition_code, u64::from(snapshot.condition_code));
        clock_bit(&mut self.halted, snapshot.halted);
        self.loaded_origin = snapshot.loaded_origin;
        self.loaded_len = snapshot.loaded_len;
    }

    /// Restore a state already validated by an enclosing transactional API.
    #[doc(hidden)]
    pub fn restore_validated_storage(&mut self, snapshot: &System360State) {
        self.restore_validated(snapshot);
    }

    /// Read the stable-Q halt latch without allocating a full snapshot.
    #[doc(hidden)]
    #[must_use]
    pub const fn halted_q(&self) -> bool {
        self.halted != 0
    }

    /// Read one general register directly from the stable-Q bank.
    #[doc(hidden)]
    #[must_use]
    pub fn gpr_q(&self, index: usize) -> u32 {
        read_bits(&self.gprs[index]) as u32
    }

    /// Read one floating-point register directly from the stable-Q bank.
    #[doc(hidden)]
    #[must_use]
    pub fn fpr_q(&self, slot: usize) -> u64 {
        read_bits(&self.fprs[slot])
    }

    /// Read the instruction address directly from the stable-Q bank.
    #[doc(hidden)]
    #[must_use]
    pub fn program_counter_q(&self) -> u32 {
        read_bits(&self.pc) as u32
    }

    /// Read the condition code directly from the stable-Q bank.
    #[doc(hidden)]
    #[must_use]
    pub fn condition_code_q(&self) -> u8 {
        read_bits(&self.condition_code) as u8
    }

    /// Read one storage byte directly from the stable-Q bank.
    #[doc(hidden)]
    #[must_use]
    pub fn memory_q(&self, index: usize) -> u8 {
        self.memory.read(index)
    }

    /// Read lifecycle metadata paired with the stable-Q bank.
    #[doc(hidden)]
    #[must_use]
    pub const fn installed_range(&self) -> (u32, usize) {
        (self.loaded_origin, self.loaded_len)
    }

    /// Clock one prevalidated general-register value.
    #[doc(hidden)]
    pub fn clock_gpr_q(&mut self, index: usize, value: u32) {
        clock_bits(&mut self.gprs[index], u64::from(value));
    }

    /// Clock one prevalidated storage byte.
    #[doc(hidden)]
    pub fn clock_memory_q(&mut self, index: usize, value: u8) {
        self.memory.write(index, value);
    }

    /// Clock a prevalidated condition code.
    #[doc(hidden)]
    pub fn clock_condition_code_q(&mut self, value: u8) {
        clock_bits(&mut self.condition_code, u64::from(value));
    }

    /// Clock a prevalidated instruction address.
    #[doc(hidden)]
    pub fn clock_program_counter_q(&mut self, value: u32) {
        clock_bits(&mut self.pc, u64::from(value));
    }

    fn commit_transition(&mut self, next: &System360State) {
        for (address, value) in next.memory.iter().copied().enumerate() {
            if self.memory.read(address) != value {
                self.memory.write(address, value);
            }
        }
        for (register, value) in self.gprs.iter_mut().zip(next.gprs) {
            clock_bits(register, u64::from(value));
        }
        for (register, value) in self.fprs.iter_mut().zip(next.fprs) {
            clock_bits(register, value);
        }
        clock_bits(&mut self.pc, u64::from(next.pc));
        clock_bits(&mut self.condition_code, u64::from(next.condition_code));
        clock_bit(&mut self.halted, next.halted);
        self.loaded_origin = next.loaded_origin;
        self.loaded_len = next.loaded_len;
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<usize, System360Error> {
        validate_program(program, origin)?;
        self.reset();
        for (offset, value) in program.iter().copied().enumerate() {
            self.memory.write(origin as usize + offset, value);
        }
        clock_bits(&mut self.pc, u64::from(origin));
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        Ok(program.len())
    }

    pub fn read_gpr(&self, index: usize) -> Result<u32, System360Error> {
        self.gprs
            .get(index)
            .map(|register| read_bits(register) as u32)
            .ok_or(System360Error::InvalidRegister {
                bank: "general",
                index,
            })
    }

    pub fn write_gpr(&mut self, index: usize, value: u32) -> Result<(), System360Error> {
        let register = self
            .gprs
            .get_mut(index)
            .ok_or(System360Error::InvalidRegister {
                bank: "general",
                index,
            })?;
        clock_bits(register, u64::from(value));
        Ok(())
    }

    /// Clock a validated successor condition code into the shared DFF bank.
    #[doc(hidden)]
    pub fn write_condition_code(&mut self, value: u8) -> Result<(), System360Error> {
        if value > 3 {
            return Err(System360Error::InvalidState {
                reason: "condition code is outside 0..3",
            });
        }
        clock_bits(&mut self.condition_code, u64::from(value));
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
        Ok(read_bits(&self.fprs[Self::fpr_slot(index)?]))
    }

    pub fn write_fpr(&mut self, index: usize, value: u64) -> Result<(), System360Error> {
        let slot = Self::fpr_slot(index)?;
        clock_bits(&mut self.fprs[slot], value);
        Ok(())
    }

    pub fn set_program_counter(&mut self, pc: u32) -> Result<(), System360Error> {
        if pc > 0x00ff_ffff || !pc.is_multiple_of(2) {
            return Err(System360Error::InvalidState {
                reason: "instruction address is invalid",
            });
        }
        clock_bits(&mut self.pc, u64::from(pc));
        Ok(())
    }

    fn memory_range(address: u32, bytes: usize) -> Result<std::ops::Range<usize>, System360Error> {
        let end = (address as usize)
            .checked_add(bytes)
            .ok_or(System360Error::MemoryOutOfRange { address, bytes })?;
        if address > 0x00ff_ffff || end > MEMORY_BYTES {
            return Err(System360Error::MemoryOutOfRange { address, bytes });
        }
        Ok(address as usize..end)
    }

    pub fn read_memory(&self, address: u32, bytes: usize) -> Result<Vec<u8>, System360Error> {
        Ok(Self::memory_range(address, bytes)?
            .map(|index| self.memory.read(index))
            .collect())
    }

    pub fn write_memory(&mut self, address: u32, data: &[u8]) -> Result<(), System360Error> {
        for (index, value) in Self::memory_range(address, data.len())?.zip(data.iter().copied()) {
            self.memory.write(index, value);
        }
        Ok(())
    }

    /// Execute one checked transition and clock its settled state into DFFs.
    pub fn step(&mut self) -> Result<StepTrace, System360Error> {
        self.try_step_embedded(&mut |_| Ok(()))
    }

    /// Execute one transition with fallible owned buffers and caller-visible seams.
    #[doc(hidden)]
    pub fn try_step_embedded(
        &mut self,
        checkpoint: &mut dyn FnMut(EmbeddedAllocationSite) -> Result<(), ()>,
    ) -> Result<StepTrace, System360Error> {
        let before = self.try_state_embedded(EmbeddedAllocationSite::EntryState, checkpoint)?;
        let (authority, width) = decode_transition(&before)?;
        let oracle_before =
            self.try_state_embedded(EmbeddedAllocationSite::OracleState, checkpoint)?;
        let oracle = System360Simulator::from_state_for_gate(oracle_before)?;
        let mut oracle_checkpoint = |site| {
            let site = match site {
                ibm_system360_simulator::GateAllocationSite::RawInstruction => {
                    EmbeddedAllocationSite::RawInstruction
                }
                ibm_system360_simulator::GateAllocationSite::Mnemonic => {
                    EmbeddedAllocationSite::Mnemonic
                }
                ibm_system360_simulator::GateAllocationSite::Description => {
                    EmbeddedAllocationSite::Description
                }
            };
            checkpoint(site)
        };
        let (trace, oracle_next) = oracle.try_step_for_gate(&mut oracle_checkpoint)?;
        if trace.raw != before.memory[before.pc as usize..before.pc as usize + width]
            || usize::from(trace.width) != width
        {
            return Err(System360Error::Specification {
                reason: "functional trace changed the gate-authorized transport",
            });
        }
        let candidate = clone_state_embedded(&before, checkpoint)?;
        let gate_next = executor::execute(&before, candidate, &trace, &authority)?;
        validate_gate_transition(&before, &gate_next, &trace, &authority)?;
        if gate_next != oracle_next {
            return Err(System360Error::Specification {
                reason: "independent gate transition diverged from the functional oracle",
            });
        }
        self.commit_transition(&gate_next);
        Ok(trace)
    }

    fn try_state_embedded(
        &self,
        site: EmbeddedAllocationSite,
        checkpoint: &mut dyn FnMut(EmbeddedAllocationSite) -> Result<(), ()>,
    ) -> Result<System360State, System360Error> {
        checkpoint(site).map_err(|()| System360Error::AllocationFailed)?;
        self.try_get_state_storage()
            .map_err(|_| System360Error::AllocationFailed)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, System360Error> {
        let before = self.get_state();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => {
                    traces.push(trace);
                    if self.halted != 0 {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            final_state: self.get_state(),
                            traces,
                        });
                    }
                }
                Err(error) => {
                    self.restore_validated(&before);
                    return Err(error);
                }
            }
        }
        self.restore_validated(&before);
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

    /// Exact persistent D-flip-flop count.
    #[must_use]
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    /// Stable educational storage-plus-combinational gate estimate.
    #[must_use]
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }
}

/// Perform lifecycle fetch checks and settle the single opcode decoder before
/// either the behavioral oracle or an operation candidate may run.
fn decode_transition(
    before: &System360State,
) -> Result<(executor::DecodeAuthority, usize), System360Error> {
    if before.halted {
        return Err(System360Error::Halted);
    }
    if !before.pc.is_multiple_of(2) {
        return Err(System360Error::InvalidState {
            reason: "instruction address is not halfword aligned",
        });
    }
    let start = before.pc as usize;
    let lo = before.loaded_origin as usize;
    let hi = lo + before.loaded_len;
    if start < lo || start >= hi {
        return Err(System360Error::FetchOutsideProgram { pc: before.pc });
    }
    let opcode = before.memory[start];
    let authority = executor::DecodeAuthority::new(opcode)?;
    let width = authority
        .transport_width()
        .ok_or(System360Error::UnknownOpcode {
            opcode,
            pc: before.pc,
        })?;
    if start + width > hi {
        return Err(System360Error::TruncatedInstruction {
            pc: before.pc,
            needed: width,
        });
    }
    if authority.operation().is_none() {
        return Err(System360Error::UnknownOpcode {
            opcode,
            pc: before.pc,
        });
    }
    Ok((authority, width))
}

fn clone_state_embedded(
    state: &System360State,
    checkpoint: &mut dyn FnMut(EmbeddedAllocationSite) -> Result<(), ()>,
) -> Result<System360State, System360Error> {
    checkpoint(EmbeddedAllocationSite::GateCandidate)
        .map_err(|()| System360Error::AllocationFailed)?;
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(state.memory.len())
        .map_err(|_| System360Error::AllocationFailed)?;
    memory.extend_from_slice(&state.memory);
    Ok(System360State {
        gprs: state.gprs,
        fprs: state.fprs,
        pc: state.pc,
        condition_code: state.condition_code,
        memory,
        halted: state.halted,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    })
}

fn validate_program(program: &[u8], origin: u32) -> Result<(), System360Error> {
    if program.is_empty() {
        return Err(System360Error::EmptyProgram);
    }
    if !origin.is_multiple_of(2) {
        return Err(System360Error::MisalignedProgram { origin });
    }
    let end =
        (origin as usize)
            .checked_add(program.len())
            .ok_or(System360Error::ProgramOutOfRange {
                origin,
                bytes: program.len(),
            })?;
    if origin > 0x00ff_ffff || end > MEMORY_BYTES {
        return Err(System360Error::ProgramOutOfRange {
            origin,
            bytes: program.len(),
        });
    }
    Ok(())
}

fn validate_gate_transition(
    before: &System360State,
    next: &System360State,
    trace: &StepTrace,
    authority: &executor::DecodeAuthority,
) -> Result<(), System360Error> {
    let sequential_pc = gates::add_24(before.pc, u32::from(trace.width));
    let operation = authority.operation().ok_or(System360Error::Specification {
        reason: "validator received no gate-authorized operation",
    })?;
    match operation {
        executor::Operation::Lpr
        | executor::Operation::Lnr
        | executor::Operation::Ltr
        | executor::Operation::Lcr
        | executor::Operation::Nr
        | executor::Operation::Clr
        | executor::Operation::Or
        | executor::Operation::Xr
        | executor::Operation::Lr
        | executor::Operation::Cr
        | executor::Operation::Ar
        | executor::Operation::Sr
        | executor::Operation::Mr
        | executor::Operation::Dr
        | executor::Operation::Alr
        | executor::Operation::Slr => {
            validate_integer_rr_transition(before, next, trace, operation)?;
        }
        executor::Operation::FloatRr(float_operation) => {
            validate_float_rr_transition(before, next, trace, float_operation)?;
        }
        executor::Operation::Sth
        | executor::Operation::La
        | executor::Operation::Stc
        | executor::Operation::Ic
        | executor::Operation::Bal
        | executor::Operation::Bct
        | executor::Operation::Bc
        | executor::Operation::Lh
        | executor::Operation::Ch
        | executor::Operation::Ah
        | executor::Operation::Sh
        | executor::Operation::Mh
        | executor::Operation::St
        | executor::Operation::N
        | executor::Operation::Cl
        | executor::Operation::O
        | executor::Operation::X
        | executor::Operation::L
        | executor::Operation::C
        | executor::Operation::A
        | executor::Operation::S
        | executor::Operation::M
        | executor::Operation::D
        | executor::Operation::Al
        | executor::Operation::Sl
        | executor::Operation::FloatRx(_) => {
            validate_rx_transition(before, next, trace, operation)?;
        }
        executor::Operation::Bxh
        | executor::Operation::Bxle
        | executor::Operation::Srl
        | executor::Operation::Sll
        | executor::Operation::Sra
        | executor::Operation::Sla
        | executor::Operation::Srdl
        | executor::Operation::Sldl
        | executor::Operation::Srda
        | executor::Operation::Slda
        | executor::Operation::Stm
        | executor::Operation::Lm => {
            validate_rs_transition(before, next, trace, operation)?;
        }
        executor::Operation::Tm
        | executor::Operation::Mvi
        | executor::Operation::Ni
        | executor::Operation::Cli
        | executor::Operation::Oi
        | executor::Operation::Xi => {
            validate_si_transition(before, next, trace, operation)?;
        }
        executor::Operation::Mvc
        | executor::Operation::Nc
        | executor::Operation::Clc
        | executor::Operation::Oc
        | executor::Operation::Xc => {
            validate_ss_transition(before, next, trace, operation)?;
        }
        executor::Operation::Balr
        | executor::Operation::Bctr
        | executor::Operation::Bcr
        | executor::Operation::Svc => {}
    }
    validate_control_transition(before, next, trace, sequential_pc, operation, authority)?;
    if !matches!(
        operation,
        executor::Operation::Balr
            | executor::Operation::Bctr
            | executor::Operation::Bcr
            | executor::Operation::Bal
            | executor::Operation::Bct
            | executor::Operation::Bc
            | executor::Operation::Bxh
            | executor::Operation::Bxle
    ) && next.pc != sequential_pc
    {
        return Err(System360Error::Specification {
            reason: "gate instruction-address adder and transition disagree",
        });
    }
    Ok(())
}

fn validate_integer_rr_transition(
    before: &System360State,
    next: &System360State,
    trace: &StepTrace,
    operation: executor::Operation,
) -> Result<(), System360Error> {
    let r1 = usize::from(trace.raw[1] >> 4);
    let r2 = usize::from(trace.raw[1] & 15);
    let a = before.gprs[r1];
    let b = before.gprs[r2];
    let expected = match operation {
        executor::Operation::Lpr => Some(if b & 0x8000_0000 == 0 {
            b
        } else {
            gates::negate_32(b).0
        }),
        executor::Operation::Lnr => Some(if b & 0x8000_0000 == 0 && b != 0 {
            gates::negate_32(b).0
        } else {
            b
        }),
        executor::Operation::Ltr | executor::Operation::Lr => Some(b),
        executor::Operation::Lcr => Some(gates::negate_32(b).0),
        executor::Operation::Nr => Some(gates::bit_and_32(a, b)),
        executor::Operation::Or => Some(gates::bit_or_32(a, b)),
        executor::Operation::Xr => Some(gates::bit_xor_32(a, b)),
        executor::Operation::Ar | executor::Operation::Alr => Some(gates::add_32(a, b, 0).0),
        executor::Operation::Sr | executor::Operation::Slr => {
            Some(gates::add_32(a, gates::bit_not_32(b), 1).0)
        }
        _ => None,
    };
    if expected.is_some_and(|value| next.gprs[r1] != value) {
        return gate_disagrees("gate ALU and architectural transition disagree");
    }
    let expected_cc = match operation {
        executor::Operation::Lpr
        | executor::Operation::Lnr
        | executor::Operation::Ltr
        | executor::Operation::Lcr
        | executor::Operation::Ar
        | executor::Operation::Sr => expected.map(gates::signed_condition_code),
        executor::Operation::Nr | executor::Operation::Or | executor::Operation::Xr => {
            expected.map(gates::logical_condition_code)
        }
        executor::Operation::Clr => Some(gates::unsigned_compare_condition_code(a, b)),
        executor::Operation::Cr => Some(gates::signed_compare_condition_code(a, b)),
        executor::Operation::Alr => expected.map(|value| {
            let carry = gates::add_32(a, b, 0).1;
            (carry << 1) | gates::logical_condition_code(value)
        }),
        executor::Operation::Slr => expected.map(|value| {
            let carry = gates::add_32(a, gates::bit_not_32(b), 1).1;
            (carry << 1) | gates::logical_condition_code(value)
        }),
        _ => None,
    };
    if expected_cc.is_some_and(|condition_code| next.condition_code != condition_code) {
        return gate_disagrees("gate condition-code network and transition disagree");
    }
    if operation == executor::Operation::Mr {
        let product = gates::multiply_signed_32(before.gprs[r1 + 1], b);
        if next.gprs[r1] != (product >> 32) as u32 || next.gprs[r1 + 1] != product as u32 {
            return gate_disagrees("gate RR multiplier and transition disagree");
        }
    }
    if operation == executor::Operation::Dr {
        let dividend = (u64::from(before.gprs[r1]) << 32) | u64::from(before.gprs[r1 + 1]);
        let (remainder, quotient) =
            gates::divide_signed_64_by_32(dividend, b).ok_or(System360Error::Specification {
                reason: "gate divider rejected an oracle-valid transition",
            })?;
        if next.gprs[r1] != remainder || next.gprs[r1 + 1] != quotient {
            return gate_disagrees("gate RR divider and transition disagree");
        }
    }
    Ok(())
}

fn validate_control_transition(
    before: &System360State,
    next: &System360State,
    trace: &StepTrace,
    sequential_pc: u32,
    operation: executor::Operation,
    authority: &executor::DecodeAuthority,
) -> Result<(), System360Error> {
    let expected = match operation {
        executor::Operation::Balr => {
            let r1 = usize::from(trace.raw[1] >> 4);
            let r2 = usize::from(trace.raw[1] & 15);
            if next.gprs[r1] != sequential_pc {
                return gate_disagrees("gate BALR link path and transition disagree");
            }
            Some(gates::select_u32(
                sequential_pc,
                before.gprs[r2] & 0x00ff_ffff,
                gates::register_field_nonzero(r2 as u8),
            ))
        }
        executor::Operation::Bctr => {
            let r1 = usize::from(trace.raw[1] >> 4);
            let r2 = usize::from(trace.raw[1] & 15);
            let decremented = gates::add_32(before.gprs[r1], u32::MAX, 0).0;
            if next.gprs[r1] != decremented {
                return gate_disagrees("gate BCTR decrement path and transition disagree");
            }
            Some(gates::select_u32(
                sequential_pc,
                before.gprs[r2] & 0x00ff_ffff,
                gates::count_branch_selected(decremented, r2 as u8),
            ))
        }
        executor::Operation::Bcr => {
            let mask = trace.raw[1] >> 4;
            let r2 = usize::from(trace.raw[1] & 15);
            Some(gates::select_u32(
                sequential_pc,
                before.gprs[r2] & 0x00ff_ffff,
                gates::branch_selected(mask, before.condition_code),
            ))
        }
        executor::Operation::Bal | executor::Operation::Bct | executor::Operation::Bc => {
            let r1 = usize::from(trace.raw[1] >> 4);
            let target = gate_ea_rx(&trace.raw, before);
            let select = match operation {
                executor::Operation::Bal => {
                    if next.gprs[r1] != sequential_pc {
                        return gate_disagrees("gate BAL link path and transition disagree");
                    }
                    1
                }
                executor::Operation::Bct => {
                    let decremented = gates::add_32(before.gprs[r1], u32::MAX, 0).0;
                    if next.gprs[r1] != decremented {
                        return gate_disagrees("gate BCT decrement path and transition disagree");
                    }
                    gates::nonzero_32(decremented)
                }
                executor::Operation::Bc => gates::branch_selected(r1 as u8, before.condition_code),
                _ => unreachable!(),
            };
            Some(gates::select_u32(sequential_pc, target, select))
        }
        executor::Operation::Bxh | executor::Operation::Bxle => {
            let r1 = usize::from(trace.raw[1] >> 4);
            let r3 = usize::from(trace.raw[1] & 15);
            let compare = if r3.is_multiple_of(2) { r3 + 1 } else { r3 };
            let incremented = gates::add_32(before.gprs[r1], before.gprs[r3], 0).0;
            let comparison =
                gates::signed_compare_condition_code(incremented, before.gprs[compare]);
            Some(gates::select_u32(
                sequential_pc,
                gate_ea_base(&trace.raw, 2, before),
                gates::index_branch_selected(
                    authority.line(0x86),
                    authority.line(0x87),
                    comparison,
                ),
            ))
        }
        _ => None,
    };
    if expected.is_some_and(|pc| next.pc != (pc & 0x00ff_ffff)) {
        return gate_disagrees("gate branch selector and transition disagree");
    }
    Ok(())
}

fn gate_ea_base(raw: &[u8], offset: usize, state: &System360State) -> u32 {
    let base = usize::from(raw[offset] >> 4);
    let displacement = (u32::from(raw[offset] & 15) << 8) | u32::from(raw[offset + 1]);
    gates::add_24(if base == 0 { 0 } else { state.gprs[base] }, displacement)
}

fn gate_ea_rx(raw: &[u8], state: &System360State) -> u32 {
    let index = usize::from(raw[1] & 15);
    let indexed = gates::add_24(
        if index == 0 { 0 } else { state.gprs[index] },
        gate_ea_base(raw, 2, state),
    );
    indexed & 0x00ff_ffff
}

fn read_be16(memory: &[u8], address: usize) -> u16 {
    u16::from_be_bytes([memory[address], memory[address + 1]])
}

fn read_be32(memory: &[u8], address: usize) -> u32 {
    u32::from_be_bytes([
        memory[address],
        memory[address + 1],
        memory[address + 2],
        memory[address + 3],
    ])
}

fn read_be64(memory: &[u8], address: usize) -> u64 {
    u64::from_be_bytes(std::array::from_fn(|offset| memory[address + offset]))
}

fn gate_disagrees(reason: &'static str) -> Result<(), System360Error> {
    Err(System360Error::Specification { reason })
}

fn validate_rx_transition(
    before: &System360State,
    next: &System360State,
    trace: &StepTrace,
    operation: executor::Operation,
) -> Result<(), System360Error> {
    let register = usize::from(trace.raw[1] >> 4);
    let address = gate_ea_rx(&trace.raw, before) as usize;
    if let executor::Operation::FloatRx(float_operation) = operation {
        return validate_float_rx_transition(before, next, float_operation, register, address);
    }
    let a = before.gprs[register];
    let word = || read_be32(&before.memory, address);
    let half = || i32::from(read_be16(&before.memory, address) as i16) as u32;
    let expected_register = match operation {
        executor::Operation::La => Some(address as u32),
        executor::Operation::Ic => Some((a & 0xffff_ff00) | u32::from(before.memory[address])),
        executor::Operation::Lh => Some(half()),
        executor::Operation::L => Some(word()),
        executor::Operation::Ah | executor::Operation::A | executor::Operation::Al => Some(
            gates::add_32(
                a,
                if operation == executor::Operation::Ah {
                    half()
                } else {
                    word()
                },
                0,
            )
            .0,
        ),
        executor::Operation::Sh | executor::Operation::S | executor::Operation::Sl => {
            let b = if operation == executor::Operation::Sh {
                half()
            } else {
                word()
            };
            Some(gates::add_32(a, gates::bit_not_32(b), 1).0)
        }
        executor::Operation::N => Some(gates::bit_and_32(a, word())),
        executor::Operation::O => Some(gates::bit_or_32(a, word())),
        executor::Operation::X => Some(gates::bit_xor_32(a, word())),
        _ => None,
    };
    if expected_register.is_some_and(|value| next.gprs[register] != value) {
        return gate_disagrees("gate RX datapath and register transition disagree");
    }
    let expected_cc = match operation {
        executor::Operation::Ah
        | executor::Operation::Sh
        | executor::Operation::A
        | executor::Operation::S => expected_register.map(gates::signed_condition_code),
        executor::Operation::Ch => Some(gates::signed_compare_condition_code(a, half())),
        executor::Operation::C => Some(gates::signed_compare_condition_code(a, word())),
        executor::Operation::Cl => Some(gates::unsigned_compare_condition_code(a, word())),
        executor::Operation::N | executor::Operation::O | executor::Operation::X => {
            expected_register.map(gates::logical_condition_code)
        }
        executor::Operation::Al => expected_register.map(|value| {
            (gates::add_32(a, word(), 0).1 << 1) | gates::logical_condition_code(value)
        }),
        executor::Operation::Sl => expected_register.map(|value| {
            (gates::add_32(a, gates::bit_not_32(word()), 1).1 << 1)
                | gates::logical_condition_code(value)
        }),
        _ => None,
    };
    if expected_cc.is_some_and(|value| next.condition_code != value) {
        return gate_disagrees("gate RX condition-code network and transition disagree");
    }
    let store_matches = match operation {
        executor::Operation::Sth => next.memory[address..address + 2] == (a as u16).to_be_bytes(),
        executor::Operation::Stc => next.memory[address] == a as u8,
        executor::Operation::St => next.memory[address..address + 4] == a.to_be_bytes(),
        _ => true,
    };
    if !store_matches {
        return gate_disagrees("gate RX storage path and transition disagree");
    }
    if operation == executor::Operation::M {
        let product = gates::multiply_signed_32(before.gprs[register + 1], word());
        if next.gprs[register] != (product >> 32) as u32
            || next.gprs[register + 1] != product as u32
        {
            return gate_disagrees("gate RX multiplier and transition disagree");
        }
    }
    if operation == executor::Operation::Mh {
        let product = gates::multiply_signed_32(a, half());
        if next.gprs[register] != product as u32 {
            return gate_disagrees("gate halfword multiplier and transition disagree");
        }
    }
    if operation == executor::Operation::D {
        let dividend =
            (u64::from(before.gprs[register]) << 32) | u64::from(before.gprs[register + 1]);
        let (remainder, quotient) = gates::divide_signed_64_by_32(dividend, word()).ok_or(
            System360Error::Specification {
                reason: "gate divider rejected an oracle-valid transition",
            },
        )?;
        if next.gprs[register] != remainder || next.gprs[register + 1] != quotient {
            return gate_disagrees("gate RX divider and transition disagree");
        }
    }
    Ok(())
}

fn validate_float_rr_transition(
    before: &System360State,
    next: &System360State,
    trace: &StepTrace,
    operation: executor::FloatRrOperation,
) -> Result<(), System360Error> {
    let r1 = usize::from(trace.raw[1] >> 4);
    let r2 = usize::from(trace.raw[1] & 15);
    let slot1 = r1 / 2;
    let slot2 = r2 / 2;
    let long = operation.is_long();
    let a = before.fprs[slot1];
    let b = before.fprs[slot2];
    let digits = if long { 14 } else { 6 };
    let gate_a = GateHfp::decode(a, long);
    let gate_b = GateHfp::decode(b, long);
    let signed = match operation {
        executor::FloatRrOperation::Op28
        | executor::FloatRrOperation::Op38
        | executor::FloatRrOperation::Op22
        | executor::FloatRrOperation::Op32 => Some(gates::hfp_with_sign(b, long, b >> 63 != 0)),
        executor::FloatRrOperation::Op23 | executor::FloatRrOperation::Op33 => {
            Some(gates::hfp_with_sign(b, long, b >> 63 == 0))
        }
        executor::FloatRrOperation::Op20 | executor::FloatRrOperation::Op30 => {
            Some(gates::hfp_with_sign(b, long, false))
        }
        executor::FloatRrOperation::Op21 | executor::FloatRrOperation::Op31 => Some(
            gates::hfp_with_sign(b, long, gates::hfp_fraction_nonzero(b, long) == 1),
        ),
        _ => None,
    };
    if let Some(high) = signed {
        let value = if long {
            high
        } else {
            (high & 0xffff_ffff_0000_0000) | (a & 0xffff_ffff)
        };
        if next.fprs[slot1] != value {
            return gate_disagrees("gate floating sign/load network and transition disagree");
        }
        if matches!(
            operation,
            executor::FloatRrOperation::Op22
                | executor::FloatRrOperation::Op32
                | executor::FloatRrOperation::Op23
                | executor::FloatRrOperation::Op33
                | executor::FloatRrOperation::Op20
                | executor::FloatRrOperation::Op30
                | executor::FloatRrOperation::Op21
                | executor::FloatRrOperation::Op31
        ) && next.condition_code != gates::hfp_condition_code(value, long)
        {
            return gate_disagrees("gate floating condition-code network and transition disagree");
        }
    }
    let arithmetic = match operation {
        executor::FloatRrOperation::Op2a | executor::FloatRrOperation::Op3a => {
            gate_a.add(gate_b, digits)
        }
        executor::FloatRrOperation::Op2b | executor::FloatRrOperation::Op3b => {
            gate_a.add(gate_b.with_complemented_sign(), digits)
        }
        executor::FloatRrOperation::Op24 | executor::FloatRrOperation::Op34 => gate_a.halve(digits),
        executor::FloatRrOperation::Op2c | executor::FloatRrOperation::Op3c => {
            gate_a.multiply(gate_b, digits)
        }
        executor::FloatRrOperation::Op2d | executor::FloatRrOperation::Op3d => {
            gate_a.divide(gate_b, digits)
        }
        _ => None,
    };
    if let Some(value) = arithmetic {
        let encoded = value.encode(long).ok_or(System360Error::Specification {
            reason: "gate floating encoder rejected an oracle-valid transition",
        })?;
        let expected = if long {
            encoded
        } else {
            encoded | (a & 0xffff_ffff)
        };
        if next.fprs[slot1] != expected {
            return gate_disagrees("gate floating arithmetic network and transition disagree");
        }
        if matches!(
            operation,
            executor::FloatRrOperation::Op2a
                | executor::FloatRrOperation::Op3a
                | executor::FloatRrOperation::Op2b
                | executor::FloatRrOperation::Op3b
        ) && next.condition_code != value.condition_code()
        {
            return gate_disagrees(
                "gate floating arithmetic condition code and transition disagree",
            );
        }
    }
    if matches!(
        operation,
        executor::FloatRrOperation::Op29 | executor::FloatRrOperation::Op39
    ) && next.condition_code != gate_a.compare(gate_b)
    {
        return gate_disagrees("gate floating comparator and transition disagree");
    }
    Ok(())
}

fn validate_float_rx_transition(
    before: &System360State,
    next: &System360State,
    operation: executor::FloatRxOperation,
    register: usize,
    address: usize,
) -> Result<(), System360Error> {
    let slot = register / 2;
    let long = operation.is_long();
    let digits = if long { 14 } else { 6 };
    let memory_raw = if long {
        read_be64(&before.memory, address)
    } else {
        u64::from(read_be32(&before.memory, address)) << 32
    };
    let gate_a = GateHfp::decode(before.fprs[slot], long);
    let gate_b = GateHfp::decode(memory_raw, long);
    match operation {
        executor::FloatRxOperation::Op68 | executor::FloatRxOperation::Op78 => {
            let canonical = gates::hfp_with_sign(memory_raw, long, memory_raw >> 63 != 0);
            let expected = if long {
                canonical
            } else {
                canonical | (before.fprs[slot] & 0xffff_ffff)
            };
            if next.fprs[slot] != expected {
                return gate_disagrees("gate floating load network and transition disagree");
            }
        }
        executor::FloatRxOperation::Op60
            if next.memory[address..address + 8] != before.fprs[slot].to_be_bytes() =>
        {
            return gate_disagrees("gate floating long-store network and transition disagree");
        }
        executor::FloatRxOperation::Op70
            if next.memory[address..address + 4]
                != ((before.fprs[slot] >> 32) as u32).to_be_bytes() =>
        {
            return gate_disagrees("gate floating short-store network and transition disagree");
        }
        _ => {}
    }
    let arithmetic = match operation {
        executor::FloatRxOperation::Op6a | executor::FloatRxOperation::Op7a => {
            gate_a.add(gate_b, digits)
        }
        executor::FloatRxOperation::Op6b | executor::FloatRxOperation::Op7b => {
            gate_a.add(gate_b.with_complemented_sign(), digits)
        }
        executor::FloatRxOperation::Op6c | executor::FloatRxOperation::Op7c => {
            gate_a.multiply(gate_b, digits)
        }
        executor::FloatRxOperation::Op6d | executor::FloatRxOperation::Op7d => {
            gate_a.divide(gate_b, digits)
        }
        _ => None,
    };
    if let Some(value) = arithmetic {
        let encoded = value.encode(long).ok_or(System360Error::Specification {
            reason: "gate floating encoder rejected an oracle-valid transition",
        })?;
        let expected = if long {
            encoded
        } else {
            encoded | (before.fprs[slot] & 0xffff_ffff)
        };
        if next.fprs[slot] != expected {
            return gate_disagrees("gate floating storage arithmetic and transition disagree");
        }
        if matches!(
            operation,
            executor::FloatRxOperation::Op6a
                | executor::FloatRxOperation::Op7a
                | executor::FloatRxOperation::Op6b
                | executor::FloatRxOperation::Op7b
        ) && next.condition_code != value.condition_code()
        {
            return gate_disagrees("gate floating storage condition code and transition disagree");
        }
    }
    if matches!(
        operation,
        executor::FloatRxOperation::Op69 | executor::FloatRxOperation::Op79
    ) && next.condition_code != gate_a.compare(gate_b)
    {
        return gate_disagrees("gate floating storage comparator and transition disagree");
    }
    Ok(())
}

fn validate_rs_transition(
    before: &System360State,
    next: &System360State,
    trace: &StepTrace,
    operation: executor::Operation,
) -> Result<(), System360Error> {
    let r1 = usize::from(trace.raw[1] >> 4);
    let r3 = usize::from(trace.raw[1] & 15);
    let address = gate_ea_base(&trace.raw, 2, before);
    let count = address & 63;
    let source = before.gprs[r1];
    let expected = match operation {
        executor::Operation::Srl => Some(gates::shift_right_32(source, count)),
        executor::Operation::Sll => Some(gates::shift_left_32(source, count)),
        executor::Operation::Sra => Some(gates::arithmetic_right_32(source, count)),
        executor::Operation::Sla => {
            Some((source & 0x8000_0000) | (gates::shift_left_32(source, count) & 0x7fff_ffff))
        }
        executor::Operation::Bxh | executor::Operation::Bxle => {
            Some(gates::add_32(source, before.gprs[r3], 0).0)
        }
        _ => None,
    };
    if expected.is_some_and(|value| next.gprs[r1] != value) {
        return gate_disagrees("gate RS shift/index network and transition disagree");
    }
    if matches!(
        operation,
        executor::Operation::Sra | executor::Operation::Sla
    ) && next.condition_code != gates::signed_condition_code(next.gprs[r1])
    {
        return gate_disagrees("gate RS condition-code network and transition disagree");
    }
    if matches!(
        operation,
        executor::Operation::Srdl
            | executor::Operation::Sldl
            | executor::Operation::Srda
            | executor::Operation::Slda
    ) {
        let pair = (u64::from(before.gprs[r1]) << 32) | u64::from(before.gprs[r1 + 1]);
        let value = match operation {
            executor::Operation::Srdl => gates::shift_right_64(pair, count),
            executor::Operation::Sldl => gates::shift_left_64(pair, count),
            executor::Operation::Srda => gates::arithmetic_right_64(pair, count),
            executor::Operation::Slda => {
                (pair & 0x8000_0000_0000_0000)
                    | (gates::shift_left_64(pair, count) & 0x7fff_ffff_ffff_ffff)
            }
            _ => unreachable!(),
        };
        if next.gprs[r1] != (value >> 32) as u32 || next.gprs[r1 + 1] != value as u32 {
            return gate_disagrees("gate doubleword shifter and transition disagree");
        }
    }
    if operation == executor::Operation::Stm {
        let count = if r3 >= r1 { r3 - r1 + 1 } else { 17 - r1 + r3 };
        for offset in 0..count {
            let address = address as usize + offset * 4;
            if next.memory[address..address + 4] != before.gprs[(r1 + offset) & 15].to_be_bytes() {
                return gate_disagrees("gate STM selector and transition disagree");
            }
        }
    }
    if operation == executor::Operation::Lm {
        let count = if r3 >= r1 { r3 - r1 + 1 } else { 17 - r1 + r3 };
        for offset in 0..count {
            let address = address as usize + offset * 4;
            if next.gprs[(r1 + offset) & 15] != read_be32(&before.memory, address) {
                return gate_disagrees("gate LM selector and transition disagree");
            }
        }
    }
    Ok(())
}

fn validate_si_transition(
    before: &System360State,
    next: &System360State,
    trace: &StepTrace,
    operation: executor::Operation,
) -> Result<(), System360Error> {
    let immediate = trace.raw[1];
    let address = gate_ea_base(&trace.raw, 2, before) as usize;
    let old = u32::from(before.memory[address]);
    let imm = u32::from(immediate);
    let result = match operation {
        executor::Operation::Ni | executor::Operation::Tm => gates::bit_and_32(old, imm) as u8,
        executor::Operation::Oi => gates::bit_or_32(old, imm) as u8,
        executor::Operation::Xi => gates::bit_xor_32(old, imm) as u8,
        executor::Operation::Mvi => immediate,
        _ => before.memory[address],
    };
    if matches!(
        operation,
        executor::Operation::Mvi
            | executor::Operation::Ni
            | executor::Operation::Oi
            | executor::Operation::Xi
    ) && next.memory[address] != result
    {
        return gate_disagrees("gate SI byte network and transition disagree");
    }
    let expected_cc = match operation {
        executor::Operation::Ni | executor::Operation::Oi | executor::Operation::Xi => {
            Some(u8::from(result != 0))
        }
        executor::Operation::Cli => Some(gates::unsigned_compare_condition_code(old, imm)),
        executor::Operation::Tm => Some(if result == 0 {
            0
        } else if result == immediate {
            3
        } else {
            1
        }),
        _ => None,
    };
    if expected_cc.is_some_and(|value| next.condition_code != value) {
        return gate_disagrees("gate SI condition-code network and transition disagree");
    }
    Ok(())
}

fn validate_ss_transition(
    before: &System360State,
    next: &System360State,
    trace: &StepTrace,
    operation: executor::Operation,
) -> Result<(), System360Error> {
    let length = usize::from(trace.raw[1]) + 1;
    let destination = gate_ea_base(&trace.raw, 2, before) as usize;
    let source = gate_ea_base(&trace.raw, 4, before) as usize;
    let mut nonzero = false;
    for offset in 0..length {
        let a = u32::from(before.memory[destination + offset]);
        let source_index = source + offset;
        let source_was_written = source_index >= destination && source_index < destination + offset;
        let b = u32::from(if source_was_written {
            next.memory[source_index]
        } else {
            before.memory[source_index]
        });
        let value = match operation {
            executor::Operation::Mvc => b as u8,
            executor::Operation::Nc => gates::bit_and_32(a, b) as u8,
            executor::Operation::Oc => gates::bit_or_32(a, b) as u8,
            executor::Operation::Xc => gates::bit_xor_32(a, b) as u8,
            _ => before.memory[destination + offset],
        };
        nonzero |= value != 0;
        if matches!(
            operation,
            executor::Operation::Mvc
                | executor::Operation::Nc
                | executor::Operation::Oc
                | executor::Operation::Xc
        ) && next.memory[destination + offset] != value
        {
            return gate_disagrees("gate SS byte network and transition disagree");
        }
    }
    if operation == executor::Operation::Clc {
        let mut condition_code = 0;
        for offset in 0..length {
            condition_code = gates::unsigned_compare_condition_code(
                u32::from(before.memory[destination + offset]),
                u32::from(before.memory[source + offset]),
            );
            if condition_code != 0 {
                break;
            }
        }
        if next.condition_code != condition_code {
            return gate_disagrees("gate CLC comparator and transition disagree");
        }
    }
    if matches!(
        operation,
        executor::Operation::Nc | executor::Operation::Oc | executor::Operation::Xc
    ) && next.condition_code != u8::from(nonzero)
    {
        return gate_disagrees("gate SS condition-code network and transition disagree");
    }
    Ok(())
}
