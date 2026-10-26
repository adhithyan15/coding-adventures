//! DFF-backed OpenRISC 1000 ORBIS32 architectural gate simulator.
//!
//! The transition engine is a local, independent implementation. Repository
//! gates reconstruct primary decode and arithmetic paths before the settled
//! candidate is clocked into persistent DFF storage.

mod engine;
pub mod gates;
mod state;

pub use engine::{
    primary_opcode_is_implemented, ExecutionResult, OpenRisc1000Error, OpenRisc1000State,
    StepTrace, MAX_RETAINED_STEPS, MEMORY_SIZE, SR_CY, SR_DSX, SR_F, SR_FO, SR_OV, SR_SM,
};

use state::{clock_bit, clock_bits, read_bits, DffMemory};

/// Exact number of persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 32 * 32 + 8 * 32 + 33 + 1 + 32 + 17;
/// Six primitive gates per master/slave DFF plus a stable combinational budget.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 131_072;

/// Complete OpenRISC architectural state stored in simulated D flip-flops.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenRisc1000GateLevel {
    memory: DffMemory,
    gpr: [[u8; 32]; 32],
    pc: [u8; 32],
    ppc: [u8; 32],
    sr: [u8; 32],
    epcr0: [u8; 32],
    eear0: [u8; 32],
    esr0: [u8; 32],
    maclo: [u8; 32],
    machi: [u8; 32],
    reservation_address: [u8; 32],
    reservation_valid: u8,
    halted: u8,
    loaded_origin: [u8; 32],
    loaded_len: [u8; 17],
}

impl Default for OpenRisc1000GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenRisc1000GateLevel {
    /// Construct deterministic supervisor reset state.
    #[must_use]
    pub fn new() -> Self {
        let mut cpu = Self {
            memory: DffMemory::new(MEMORY_SIZE),
            gpr: [[0; 32]; 32],
            pc: [0; 32],
            ppc: [0; 32],
            sr: [0; 32],
            epcr0: [0; 32],
            eear0: [0; 32],
            esr0: [0; 32],
            maclo: [0; 32],
            machi: [0; 32],
            reservation_address: [0; 32],
            reservation_valid: 0,
            halted: 0,
            loaded_origin: [0; 32],
            loaded_len: [0; 17],
        };
        clock_bits(&mut cpu.sr, u64::from(SR_FO | SR_SM));
        cpu
    }

    /// Clear every architectural bit through its DFF input.
    pub fn reset(&mut self) {
        self.memory.clear();
        for register in &mut self.gpr {
            clock_bits(register, 0);
        }
        clock_bits(&mut self.pc, 0);
        clock_bits(&mut self.ppc, 0);
        clock_bits(&mut self.sr, u64::from(SR_FO | SR_SM));
        clock_bits(&mut self.epcr0, 0);
        clock_bits(&mut self.eear0, 0);
        clock_bits(&mut self.esr0, 0);
        clock_bits(&mut self.maclo, 0);
        clock_bits(&mut self.machi, 0);
        clock_bits(&mut self.reservation_address, 0);
        clock_bit(&mut self.reservation_valid, false);
        clock_bit(&mut self.halted, false);
        clock_bits(&mut self.loaded_origin, 0);
        clock_bits(&mut self.loaded_len, 0);
    }

    /// Snapshot every architectural bit into owned storage.
    #[must_use]
    pub fn get_state(&self) -> OpenRisc1000State {
        self.state_with_memory(self.memory.snapshot())
    }

    fn try_get_state(&self) -> Result<OpenRisc1000State, OpenRisc1000Error> {
        let memory = self
            .memory
            .try_snapshot()
            .map_err(|()| OpenRisc1000Error::AllocationFailed)?;
        Ok(self.state_with_memory(memory))
    }

    fn state_in_preallocated(&self, mut memory: Vec<u8>) -> OpenRisc1000State {
        self.memory.snapshot_into(&mut memory);
        self.state_with_memory(memory)
    }

    fn state_with_memory(&self, memory: Vec<u8>) -> OpenRisc1000State {
        OpenRisc1000State {
            gpr: std::array::from_fn(|index| read_bits(&self.gpr[index]) as u32),
            pc: read_bits(&self.pc) as u32,
            ppc: read_bits(&self.ppc) as u32,
            sr: read_bits(&self.sr) as u32,
            epcr0: read_bits(&self.epcr0) as u32,
            eear0: read_bits(&self.eear0) as u32,
            esr0: read_bits(&self.esr0) as u32,
            maclo: read_bits(&self.maclo) as u32,
            machi: read_bits(&self.machi) as u32,
            reservation: (self.reservation_valid == 1)
                .then(|| read_bits(&self.reservation_address) as u32),
            memory,
            halted: self.halted == 1,
            loaded_origin: read_bits(&self.loaded_origin) as u32,
            loaded_len: read_bits(&self.loaded_len) as usize,
        }
    }

    /// Validate and atomically clock a complete state into the fixed topology.
    pub fn restore(&mut self, snapshot: OpenRisc1000State) -> Result<(), OpenRisc1000Error> {
        if snapshot.memory.len() != MEMORY_SIZE {
            return Err(OpenRisc1000Error::InvalidState {
                reason: "memory must contain exactly 65536 bytes",
            });
        }
        engine::validate_state(&snapshot)?;
        self.restore_validated(&snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &OpenRisc1000State) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (target, value) in self.gpr.iter_mut().zip(snapshot.gpr) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.pc, u64::from(snapshot.pc));
        clock_bits(&mut self.ppc, u64::from(snapshot.ppc));
        clock_bits(&mut self.sr, u64::from(snapshot.sr | SR_FO));
        clock_bits(&mut self.epcr0, u64::from(snapshot.epcr0));
        clock_bits(&mut self.eear0, u64::from(snapshot.eear0));
        clock_bits(&mut self.esr0, u64::from(snapshot.esr0));
        clock_bits(&mut self.maclo, u64::from(snapshot.maclo));
        clock_bits(&mut self.machi, u64::from(snapshot.machi));
        clock_bits(
            &mut self.reservation_address,
            u64::from(snapshot.reservation.unwrap_or(0)),
        );
        clock_bit(&mut self.reservation_valid, snapshot.reservation.is_some());
        clock_bit(&mut self.halted, snapshot.halted);
        clock_bits(&mut self.loaded_origin, u64::from(snapshot.loaded_origin));
        clock_bits(&mut self.loaded_len, snapshot.loaded_len as u64);
    }

    fn commit_transition(&mut self, next: &OpenRisc1000State) {
        for (address, &new) in next.memory.iter().enumerate() {
            let old = self.memory.read(address);
            if gates::byte_changed(old, new) == 1 {
                self.memory.write(address, new);
            }
        }
        for (target, value) in self.gpr.iter_mut().zip(next.gpr) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.pc, u64::from(next.pc));
        clock_bits(&mut self.ppc, u64::from(next.ppc));
        clock_bits(&mut self.sr, u64::from(next.sr));
        clock_bits(&mut self.epcr0, u64::from(next.epcr0));
        clock_bits(&mut self.eear0, u64::from(next.eear0));
        clock_bits(&mut self.esr0, u64::from(next.esr0));
        clock_bits(&mut self.maclo, u64::from(next.maclo));
        clock_bits(&mut self.machi, u64::from(next.machi));
        clock_bits(
            &mut self.reservation_address,
            u64::from(next.reservation.unwrap_or(0)),
        );
        clock_bit(&mut self.reservation_valid, next.reservation.is_some());
        clock_bit(&mut self.halted, next.halted);
        clock_bits(&mut self.loaded_origin, u64::from(next.loaded_origin));
        clock_bits(&mut self.loaded_len, next.loaded_len as u64);
    }

    /// Install a checked, nonempty, word-aligned big-endian image.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), OpenRisc1000Error> {
        engine::validate_image(program, origin)?;
        self.reset();
        for (offset, value) in program.iter().copied().enumerate() {
            self.memory.write(origin as usize + offset, value);
        }
        clock_bits(&mut self.pc, u64::from(origin));
        clock_bits(&mut self.ppc, u64::from(origin));
        clock_bits(&mut self.loaded_origin, u64::from(origin));
        clock_bits(&mut self.loaded_len, program.len() as u64);
        Ok(())
    }

    pub fn read_register(&self, register: usize) -> Result<u32, OpenRisc1000Error> {
        self.gpr
            .get(register)
            .map(|bits| read_bits(bits) as u32)
            .ok_or(OpenRisc1000Error::InvalidRegister { register })
    }

    pub fn write_register(&mut self, register: usize, value: u32) -> Result<(), OpenRisc1000Error> {
        let target = self
            .gpr
            .get_mut(register)
            .ok_or(OpenRisc1000Error::InvalidRegister { register })?;
        clock_bits(target, u64::from(value));
        Ok(())
    }

    fn memory_range(address: u32, len: usize) -> Result<std::ops::Range<usize>, OpenRisc1000Error> {
        let start = address as usize;
        let Some(end) = start.checked_add(len) else {
            return Err(OpenRisc1000Error::DataOutOfRange {
                address,
                width: len,
            });
        };
        if end > MEMORY_SIZE {
            return Err(OpenRisc1000Error::DataOutOfRange {
                address,
                width: len,
            });
        }
        Ok(start..end)
    }

    pub fn read_memory(&self, address: u32, len: usize) -> Result<Vec<u8>, OpenRisc1000Error> {
        Ok(Self::memory_range(address, len)?
            .map(|index| self.memory.read(index))
            .collect())
    }

    pub fn write_memory(&mut self, address: u32, bytes: &[u8]) -> Result<(), OpenRisc1000Error> {
        let range = Self::memory_range(address, bytes.len())?;
        for (index, value) in range.zip(bytes.iter().copied()) {
            self.memory.write(index, value);
        }
        if self.reservation_valid == 1 {
            let reserved = read_bits(&self.reservation_address) as u32;
            let write_end = gates::add_32(address, bytes.len() as u32, 0).0;
            let reservation_end = gates::add_32(reserved, 4, 0).0;
            if logic_gates::gates::and_gate(
                gates::unsigned_less_32(address, reservation_end),
                gates::unsigned_less_32(reserved, write_end),
            ) == 1
            {
                clock_bit(&mut self.reservation_valid, false);
            }
        }
        Ok(())
    }

    /// Execute one gate-derived transition and clock its settled result.
    pub fn step(&mut self) -> Result<StepTrace, OpenRisc1000Error> {
        if self.halted == 1 {
            return Err(OpenRisc1000Error::Halted);
        }
        let before = self.try_get_state()?;
        let trace = engine::step_state(before)?;
        self.commit_transition(&trace.state_after);
        Ok(trace)
    }

    fn step_in_preallocated(&mut self, memory: Vec<u8>) -> Result<StepTrace, OpenRisc1000Error> {
        if self.halted == 1 {
            return Err(OpenRisc1000Error::Halted);
        }
        let before = self.state_in_preallocated(memory);
        let trace = engine::step_state(before)?;
        self.commit_transition(&trace.state_after);
        Ok(trace)
    }

    /// Execute until halt or the supplied bound, rolling back on any failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, OpenRisc1000Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(OpenRisc1000Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let allocation_count = max_steps
            .checked_add(2)
            .ok_or(OpenRisc1000Error::AllocationFailed)?;
        let mut state_memories = Vec::new();
        state_memories
            .try_reserve_exact(allocation_count)
            .map_err(|_| OpenRisc1000Error::AllocationFailed)?;
        for _ in 0..allocation_count {
            let mut memory = Vec::new();
            memory
                .try_reserve_exact(MEMORY_SIZE)
                .map_err(|_| OpenRisc1000Error::AllocationFailed)?;
            memory.resize(MEMORY_SIZE, 0);
            state_memories.push(memory);
        }
        let before = self.state_in_preallocated(
            state_memories
                .pop()
                .expect("rollback memory was preallocated"),
        );
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| OpenRisc1000Error::AllocationFailed)?;
        for _ in 0..max_steps {
            let memory = state_memories
                .pop()
                .expect("one trace memory was preallocated per allowed step");
            match self.step_in_preallocated(memory) {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: self.state_in_preallocated(
                                state_memories
                                    .pop()
                                    .expect("final-state memory was preallocated"),
                            ),
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
        Err(OpenRisc1000Error::StepLimitExceeded { limit: max_steps })
    }

    /// Transactionally install and execute one image.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, OpenRisc1000Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(OpenRisc1000Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let before = self.try_get_state()?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore_validated(&before);
                Err(error)
            }
        }
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
