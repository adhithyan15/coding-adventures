//! DFF-backed Armv6-M Cortex-M0 architectural gate simulator.
//!
//! A local transition engine reconstructs decode, arithmetic, address, flag,
//! and control paths with repository gates. Only settled candidate values are
//! clocked into persistent repository D flip-flops.

mod engine;
pub mod gates;
mod state;

pub use engine::{
    classify, encoding, CortexM0Error, CortexM0State, ExecutionResult, InstructionClass, StepTrace,
    LR, MAX_RETAINED_BYTES, MAX_RETAINED_STEPS, MEMORY_SIZE, PC, SP,
};

use state::{clock_bit, clock_bits, read_bits, DffMemory};

/// Exact number of mutable persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize =
    MEMORY_SIZE * 8 + 13 * 32 + 4 * 32 + 4 + 6 + 1 + 2 + 1 + 3 + 32 + 17;
/// Six primitive gates per master/slave DFF plus a finite combinational budget.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 98_304;

/// Complete Cortex-M0 state held in stable-Q simulated storage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CortexM0GateLevel {
    memory: DffMemory,
    registers: [[u8; 32]; 13],
    msp: [u8; 32],
    psp: [u8; 32],
    lr: [u8; 32],
    pc: [u8; 32],
    apsr: [u8; 4],
    ipsr: [u8; 6],
    primask: u8,
    control: [u8; 2],
    handler_mode: u8,
    halted: u8,
    sleeping: u8,
    event_registered: u8,
    loaded_origin: [u8; 32],
    loaded_len: [u8; 17],
}

impl Default for CortexM0GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl CortexM0GateLevel {
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
            apsr: [0; 4],
            ipsr: [0; 6],
            primask: 0,
            control: [0; 2],
            handler_mode: 0,
            halted: 0,
            sleeping: 0,
            event_registered: 0,
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
        clock_bit(&mut self.primask, false);
        clock_bits(&mut self.control, 0);
        clock_bit(&mut self.handler_mode, false);
        clock_bit(&mut self.halted, false);
        clock_bit(&mut self.sleeping, false);
        clock_bit(&mut self.event_registered, false);
        clock_bits(&mut self.loaded_origin, 0);
        clock_bits(&mut self.loaded_len, 0);
    }

    /// Snapshot every architectural bit into owned value storage.
    #[must_use]
    pub fn get_state(&self) -> CortexM0State {
        self.state_with_memory(self.memory.snapshot())
    }

    fn try_get_state(&self) -> Result<CortexM0State, CortexM0Error> {
        let memory = self
            .memory
            .try_snapshot()
            .map_err(|()| CortexM0Error::AllocationFailed)?;
        Ok(self.state_with_memory(memory))
    }

    fn state_with_memory(&self, memory: Vec<u8>) -> CortexM0State {
        let apsr = (read_bits(&self.apsr) as u32) << 28;
        CortexM0State {
            registers: std::array::from_fn(|index| read_bits(&self.registers[index]) as u32),
            msp: read_bits(&self.msp) as u32,
            psp: read_bits(&self.psp) as u32,
            lr: read_bits(&self.lr) as u32,
            pc: read_bits(&self.pc) as u32,
            apsr,
            ipsr: read_bits(&self.ipsr) as u8,
            primask: self.primask == 1,
            control: read_bits(&self.control) as u8,
            handler_mode: self.handler_mode == 1,
            memory,
            halted: self.halted == 1,
            sleeping: self.sleeping == 1,
            event_registered: self.event_registered == 1,
            loaded_origin: read_bits(&self.loaded_origin) as u32,
            loaded_len: read_bits(&self.loaded_len) as usize,
        }
    }

    /// Validate and atomically clock a complete snapshot.
    pub fn restore(&mut self, snapshot: CortexM0State) -> Result<(), CortexM0Error> {
        engine::validate_state(&snapshot)?;
        self.restore_validated(&snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &CortexM0State) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (target, value) in self.registers.iter_mut().zip(snapshot.registers) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.msp, u64::from(snapshot.msp));
        clock_bits(&mut self.psp, u64::from(snapshot.psp));
        clock_bits(&mut self.lr, u64::from(snapshot.lr));
        clock_bits(&mut self.pc, u64::from(snapshot.pc));
        clock_bits(&mut self.apsr, u64::from(snapshot.apsr >> 28));
        clock_bits(&mut self.ipsr, u64::from(snapshot.ipsr));
        clock_bit(&mut self.primask, snapshot.primask);
        clock_bits(&mut self.control, u64::from(snapshot.control));
        clock_bit(&mut self.handler_mode, snapshot.handler_mode);
        clock_bit(&mut self.halted, snapshot.halted);
        clock_bit(&mut self.sleeping, snapshot.sleeping);
        clock_bit(&mut self.event_registered, snapshot.event_registered);
        clock_bits(&mut self.loaded_origin, u64::from(snapshot.loaded_origin));
        clock_bits(&mut self.loaded_len, snapshot.loaded_len as u64);
    }

    fn commit_transition(&mut self, next: &CortexM0State) {
        for (address, &value) in next.memory.iter().enumerate() {
            let current = self.memory.read(address);
            if current != value && gates::byte_changed(current, value) == 1 {
                self.memory.write(address, value);
            }
        }
        self.restore_registers_and_control(next);
    }

    fn restore_registers_and_control(&mut self, next: &CortexM0State) {
        for (target, value) in self.registers.iter_mut().zip(next.registers) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.msp, u64::from(next.msp));
        clock_bits(&mut self.psp, u64::from(next.psp));
        clock_bits(&mut self.lr, u64::from(next.lr));
        clock_bits(&mut self.pc, u64::from(next.pc));
        clock_bits(&mut self.apsr, u64::from(next.apsr >> 28));
        clock_bits(&mut self.ipsr, u64::from(next.ipsr));
        clock_bit(&mut self.primask, next.primask);
        clock_bits(&mut self.control, u64::from(next.control));
        clock_bit(&mut self.handler_mode, next.handler_mode);
        clock_bit(&mut self.halted, next.halted);
        clock_bit(&mut self.sleeping, next.sleeping);
        clock_bit(&mut self.event_registered, next.event_registered);
        clock_bits(&mut self.loaded_origin, u64::from(next.loaded_origin));
        clock_bits(&mut self.loaded_len, next.loaded_len as u64);
    }

    /// Install a checked nonempty halfword-aligned little-endian image.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM0Error> {
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

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM0Error> {
        match index {
            0..=12 => Ok(read_bits(&self.registers[index as usize]) as u32),
            SP => {
                let select = logic_gates::gates::and_gate(
                    logic_gates::gates::not_gate(self.handler_mode),
                    self.control[1],
                );
                Ok(gates::select_32(
                    read_bits(&self.msp) as u32,
                    read_bits(&self.psp) as u32,
                    select,
                ))
            }
            LR => Ok(read_bits(&self.lr) as u32),
            PC => Ok(read_bits(&self.pc) as u32),
            _ => Err(CortexM0Error::InvalidRegister { index }),
        }
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM0Error> {
        match index {
            0..=12 => clock_bits(&mut self.registers[index as usize], u64::from(value)),
            SP => {
                let value = gates::bit_and_32(value, gates::bit_not_32(3));
                let select = logic_gates::gates::and_gate(
                    logic_gates::gates::not_gate(self.handler_mode),
                    self.control[1],
                );
                if select == 1 {
                    clock_bits(&mut self.psp, u64::from(value));
                } else {
                    clock_bits(&mut self.msp, u64::from(value));
                }
            }
            LR => clock_bits(&mut self.lr, u64::from(value)),
            PC => clock_bits(
                &mut self.pc,
                u64::from(gates::bit_and_32(value, gates::bit_not_32(1))),
            ),
            _ => return Err(CortexM0Error::InvalidRegister { index }),
        }
        Ok(())
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM0Error> {
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
    ) -> Result<(), CortexM0Error> {
        let range = engine::checked_range(address, width)?;
        for (index, byte) in range.zip(value.to_le_bytes()) {
            self.memory.write(index, byte);
        }
        Ok(())
    }

    /// Register an event and wake a sleeping core through DFF inputs.
    pub fn signal_event(&mut self) {
        clock_bit(&mut self.event_registered, true);
        clock_bit(&mut self.sleeping, false);
    }

    /// Execute one independent gate-derived transition.
    pub fn step(&mut self) -> Result<StepTrace, CortexM0Error> {
        let trace = engine::step_state(self.try_get_state()?)?;
        self.commit_transition(&trace.state_after);
        Ok(trace)
    }

    /// Execute until halt/sleep or the supplied bound, rolling back on failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM0Error> {
        check_retention(max_steps)?;
        let before = self.try_get_state()?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| CortexM0Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.restore_validated(&before);
                    return Err(error);
                }
            }
            if self.halted == 1 || self.sleeping == 1 {
                return Ok(ExecutionResult {
                    stopped: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.try_get_state().inspect_err(|_| {
                        self.restore_validated(&before);
                    })?,
                });
            }
        }
        self.restore_validated(&before);
        Err(CortexM0Error::StepLimitExceeded { limit: max_steps })
    }

    /// Transactionally load and execute one image.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM0Error> {
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

fn check_retention(max_steps: usize) -> Result<(), CortexM0Error> {
    if max_steps > MAX_RETAINED_STEPS {
        return Err(CortexM0Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    Ok(())
}
