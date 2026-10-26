//! DFF-backed Armv7-M Cortex-M3 architectural gate simulator.
//!
//! A local transition engine reconstructs decode, arithmetic, address, flag,
//! and control paths with repository gates. Only settled candidate values are
//! clocked into persistent repository D flip-flops.

mod engine;
pub mod gates;
mod state;

pub use engine::{
    classify, encoding, CortexM3Error, CortexM3State, ExecutionResult, InstructionClass, StepTrace,
    LR, MAX_RETAINED_BYTES, MAX_RETAINED_STEPS, MEMORY_SIZE, PC, SP,
};

use state::{clock_bit, clock_bits, read_bits, DffMemory};

/// Exact number of mutable persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize =
    MEMORY_SIZE * 8 + 13 * 32 + 5 * 32 + 2 * 8 + 1 + 8 + 1 + 8 + 1 + 4 + 33 + 32 + 17;
/// Six primitive gates per master/slave DFF plus a finite combinational budget.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 131_072;

/// Complete Cortex-M3 state held in stable-Q simulated storage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CortexM3GateLevel {
    memory: DffMemory,
    registers: [[u8; 32]; 13],
    msp: [u8; 32],
    psp: [u8; 32],
    lr: [u8; 32],
    pc: [u8; 32],
    apsr: [u8; 5],
    ipsr: [u8; 8],
    itstate: [u8; 8],
    primask: u8,
    basepri: [u8; 8],
    faultmask: u8,
    control: [u8; 2],
    handler_mode: u8,
    halted: u8,
    sleeping: u8,
    waiting_for_event: u8,
    event_registered: u8,
    exclusive_address: [u8; 32],
    exclusive_valid: u8,
    loaded_origin: [u8; 32],
    loaded_len: [u8; 17],
}

impl Default for CortexM3GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl CortexM3GateLevel {
    /// Construct deterministic privileged Thread-mode reset state.
    #[must_use]
    pub fn new() -> Self {
        let mut cpu = Self {
            memory: DffMemory::new(MEMORY_SIZE),
            registers: [[0; 32]; 13],
            msp: [0; 32],
            psp: [0; 32],
            lr: [0; 32],
            pc: [0; 32],
            apsr: [0; 5],
            ipsr: [0; 8],
            itstate: [0; 8],
            primask: 0,
            basepri: [0; 8],
            faultmask: 0,
            control: [0; 2],
            handler_mode: 0,
            halted: 0,
            sleeping: 0,
            waiting_for_event: 0,
            event_registered: 0,
            exclusive_address: [0; 32],
            exclusive_valid: 0,
            loaded_origin: [0; 32],
            loaded_len: [0; 17],
        };
        clock_bits(&mut cpu.msp, MEMORY_SIZE as u64);
        clock_bits(&mut cpu.psp, MEMORY_SIZE as u64);
        cpu
    }

    /// Clock every architectural bit to reset state.
    pub fn reset(&mut self) {
        self.memory.clear();
        for register in &mut self.registers {
            clock_bits(register, 0);
        }
        clock_bits(&mut self.msp, MEMORY_SIZE as u64);
        clock_bits(&mut self.psp, MEMORY_SIZE as u64);
        clock_bits(&mut self.lr, 0);
        clock_bits(&mut self.pc, 0);
        clock_bits(&mut self.apsr, 0);
        clock_bits(&mut self.ipsr, 0);
        clock_bits(&mut self.itstate, 0);
        clock_bit(&mut self.primask, false);
        clock_bits(&mut self.basepri, 0);
        clock_bit(&mut self.faultmask, false);
        clock_bits(&mut self.control, 0);
        clock_bit(&mut self.handler_mode, false);
        clock_bit(&mut self.halted, false);
        clock_bit(&mut self.sleeping, false);
        clock_bit(&mut self.waiting_for_event, false);
        clock_bit(&mut self.event_registered, false);
        clock_bits(&mut self.exclusive_address, 0);
        clock_bit(&mut self.exclusive_valid, false);
        clock_bits(&mut self.loaded_origin, 0);
        clock_bits(&mut self.loaded_len, 0);
    }

    /// Snapshot every architectural bit into owned value storage.
    #[must_use]
    pub fn get_state(&self) -> CortexM3State {
        self.state_with_memory(self.memory.snapshot())
    }

    fn try_get_state(&self) -> Result<CortexM3State, CortexM3Error> {
        let memory = self
            .memory
            .try_snapshot()
            .map_err(|()| CortexM3Error::AllocationFailed)?;
        Ok(self.state_with_memory(memory))
    }

    fn state_in_preallocated(&self, mut memory: Vec<u8>) -> CortexM3State {
        self.memory.snapshot_into(&mut memory);
        self.state_with_memory(memory)
    }

    fn state_with_memory(&self, memory: Vec<u8>) -> CortexM3State {
        let apsr = (read_bits(&self.apsr) as u32) << 27;
        CortexM3State {
            registers: std::array::from_fn(|index| read_bits(&self.registers[index]) as u32),
            msp: read_bits(&self.msp) as u32,
            psp: read_bits(&self.psp) as u32,
            lr: read_bits(&self.lr) as u32,
            pc: read_bits(&self.pc) as u32,
            apsr,
            ipsr: read_bits(&self.ipsr) as u8,
            itstate: read_bits(&self.itstate) as u8,
            primask: self.primask == 1,
            basepri: read_bits(&self.basepri) as u8,
            faultmask: self.faultmask == 1,
            control: read_bits(&self.control) as u8,
            handler_mode: self.handler_mode == 1,
            memory,
            halted: self.halted == 1,
            sleeping: self.sleeping == 1,
            waiting_for_event: self.waiting_for_event == 1,
            event_registered: self.event_registered == 1,
            exclusive_address: (self.exclusive_valid == 1)
                .then(|| read_bits(&self.exclusive_address) as u32),
            loaded_origin: read_bits(&self.loaded_origin) as u32,
            loaded_len: read_bits(&self.loaded_len) as usize,
        }
    }

    /// Validate and atomically clock a complete snapshot.
    pub fn restore(&mut self, snapshot: CortexM3State) -> Result<(), CortexM3Error> {
        engine::validate_state(&snapshot)?;
        self.restore_validated(&snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &CortexM3State) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (target, value) in self.registers.iter_mut().zip(snapshot.registers) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.msp, u64::from(snapshot.msp));
        clock_bits(&mut self.psp, u64::from(snapshot.psp));
        clock_bits(&mut self.lr, u64::from(snapshot.lr));
        clock_bits(&mut self.pc, u64::from(snapshot.pc));
        clock_bits(&mut self.apsr, u64::from(snapshot.apsr >> 27));
        clock_bits(&mut self.ipsr, u64::from(snapshot.ipsr));
        clock_bits(&mut self.itstate, u64::from(snapshot.itstate));
        clock_bit(&mut self.primask, snapshot.primask);
        clock_bits(&mut self.basepri, u64::from(snapshot.basepri));
        clock_bit(&mut self.faultmask, snapshot.faultmask);
        clock_bits(&mut self.control, u64::from(snapshot.control));
        clock_bit(&mut self.handler_mode, snapshot.handler_mode);
        clock_bit(&mut self.halted, snapshot.halted);
        clock_bit(&mut self.sleeping, snapshot.sleeping);
        clock_bit(&mut self.waiting_for_event, snapshot.waiting_for_event);
        clock_bit(&mut self.event_registered, snapshot.event_registered);
        clock_bits(
            &mut self.exclusive_address,
            u64::from(snapshot.exclusive_address.unwrap_or(0)),
        );
        clock_bit(
            &mut self.exclusive_valid,
            snapshot.exclusive_address.is_some(),
        );
        clock_bits(&mut self.loaded_origin, u64::from(snapshot.loaded_origin));
        clock_bits(&mut self.loaded_len, snapshot.loaded_len as u64);
    }

    fn commit_transition(&mut self, next: &CortexM3State) {
        for (address, &value) in next.memory.iter().enumerate() {
            let current = self.memory.read(address);
            if current != value && gates::byte_changed(current, value) == 1 {
                self.memory.write(address, value);
            }
        }
        self.restore_registers_and_control(next);
    }

    fn restore_registers_and_control(&mut self, next: &CortexM3State) {
        for (target, value) in self.registers.iter_mut().zip(next.registers) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.msp, u64::from(next.msp));
        clock_bits(&mut self.psp, u64::from(next.psp));
        clock_bits(&mut self.lr, u64::from(next.lr));
        clock_bits(&mut self.pc, u64::from(next.pc));
        clock_bits(&mut self.apsr, u64::from(next.apsr >> 27));
        clock_bits(&mut self.ipsr, u64::from(next.ipsr));
        clock_bits(&mut self.itstate, u64::from(next.itstate));
        clock_bit(&mut self.primask, next.primask);
        clock_bits(&mut self.basepri, u64::from(next.basepri));
        clock_bit(&mut self.faultmask, next.faultmask);
        clock_bits(&mut self.control, u64::from(next.control));
        clock_bit(&mut self.handler_mode, next.handler_mode);
        clock_bit(&mut self.halted, next.halted);
        clock_bit(&mut self.sleeping, next.sleeping);
        clock_bit(&mut self.waiting_for_event, next.waiting_for_event);
        clock_bit(&mut self.event_registered, next.event_registered);
        clock_bits(
            &mut self.exclusive_address,
            u64::from(next.exclusive_address.unwrap_or(0)),
        );
        clock_bit(&mut self.exclusive_valid, next.exclusive_address.is_some());
        clock_bits(&mut self.loaded_origin, u64::from(next.loaded_origin));
        clock_bits(&mut self.loaded_len, next.loaded_len as u64);
    }

    /// Install a checked nonempty halfword-aligned little-endian image.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM3Error> {
        engine::validate_program(program, origin)?;
        self.reset();
        for (offset, value) in program.iter().copied().enumerate() {
            self.memory.write(origin as usize + offset, value);
        }
        clock_bits(&mut self.pc, u64::from(origin));
        clock_bits(&mut self.loaded_origin, u64::from(origin));
        clock_bits(&mut self.loaded_len, program.len() as u64);
        Ok(())
    }

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM3Error> {
        engine::read_register(&self.get_state(), index)
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM3Error> {
        let mut state = self.get_state();
        engine::write_register(&mut state, index, value)?;
        self.restore_registers_and_control(&state);
        Ok(())
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM3Error> {
        let range = engine::checked_range(address, width)?;
        let mut bytes = [0; 4];
        for (target, index) in bytes.iter_mut().zip(range) {
            *target = self.memory.read(index);
        }
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
    ) -> Result<(), CortexM3Error> {
        let range = engine::checked_range(address, width)?;
        for (index, byte) in range.zip(value.to_le_bytes()) {
            self.memory.write(index, byte);
        }
        clock_bit(&mut self.exclusive_valid, false);
        Ok(())
    }

    /// Register an event and wake a sleeping core through DFF inputs.
    pub fn signal_event(&mut self) {
        if self.sleeping == 1 && self.waiting_for_event == 1 {
            clock_bit(&mut self.sleeping, false);
            clock_bit(&mut self.waiting_for_event, false);
            clock_bit(&mut self.event_registered, false);
        } else {
            clock_bit(&mut self.event_registered, true);
        }
    }

    /// Wake either architectural sleep form after interrupt acceptance.
    pub fn signal_interrupt(&mut self) {
        clock_bit(&mut self.sleeping, false);
        clock_bit(&mut self.waiting_for_event, false);
    }

    /// Execute one independent gate-derived transition.
    pub fn step(&mut self) -> Result<StepTrace, CortexM3Error> {
        let trace = engine::step_state(self.try_get_state()?)?;
        self.commit_transition(&trace.state_after);
        Ok(trace)
    }

    fn step_in_preallocated(&mut self, memory: Vec<u8>) -> Result<StepTrace, CortexM3Error> {
        let trace = engine::step_state(self.state_in_preallocated(memory))?;
        self.commit_transition(&trace.state_after);
        Ok(trace)
    }

    /// Execute until halt/sleep or the supplied bound, rolling back on failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM3Error> {
        check_retention(max_steps)?;
        let allocation_count = max_steps
            .checked_add(2)
            .ok_or(CortexM3Error::AllocationFailed)?;
        let mut state_memories = Vec::new();
        state_memories
            .try_reserve_exact(allocation_count)
            .map_err(|_| CortexM3Error::AllocationFailed)?;
        for _ in 0..allocation_count {
            let mut memory = Vec::new();
            memory
                .try_reserve_exact(MEMORY_SIZE)
                .map_err(|_| CortexM3Error::AllocationFailed)?;
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
            .map_err(|_| CortexM3Error::AllocationFailed)?;
        for _ in 0..max_steps {
            let memory = state_memories
                .pop()
                .expect("one trace memory was preallocated per allowed step");
            match self.step_in_preallocated(memory) {
                Ok(trace) => {
                    let stopped = trace.state_after.halted || trace.state_after.sleeping;
                    traces.push(trace);
                    if stopped {
                        return Ok(ExecutionResult {
                            stopped: true,
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
        Err(CortexM3Error::StepLimitExceeded { limit: max_steps })
    }

    /// Transactionally load and execute one image.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM3Error> {
        check_retention(max_steps)?;
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

fn check_retention(max_steps: usize) -> Result<(), CortexM3Error> {
    if max_steps > MAX_RETAINED_STEPS {
        return Err(CortexM3Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    Ok(())
}
