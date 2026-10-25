//! Gate-backed Motorola MC6809 architectural simulator.
//!
//! Every persistent programmer-visible bit is stored in repository D
//! flip-flops. Successful transitions are produced by the independent gate
//! engine; the checked functional simulator is only a fail-closed oracle.

pub use motorola6809_simulator::{
    indexed_postbyte_is_valid, opcode_is_documented, ExecutionResult, Motorola6809Error,
    Motorola6809Simulator, Motorola6809State, StepTrace, CC_C, CC_E, CC_F, CC_H, CC_I, CC_N, CC_V,
    CC_Z, MAX_RETAINED_STEPS, MEMORY_SIZE,
};

mod executor;
pub mod gates;
mod state;

use state::{clock_bit, clock_bits, read_bits, DffMemory};

/// Exact number of persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 16 + 16 * 5 + 16 + 1 + 16 + 17;
/// Stable educational storage-plus-combinational gate estimate.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 65_536;

/// MC6809 state stored in simulated D flip-flops.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Motorola6809GateLevel {
    memory: DffMemory,
    a: [u8; 8],
    b: [u8; 8],
    x: [u8; 16],
    y: [u8; 16],
    u: [u8; 16],
    s: [u8; 16],
    pc: [u8; 16],
    dp: [u8; 8],
    cc: [u8; 8],
    halted: u8,
    loaded_origin: [u8; 16],
    loaded_len: [u8; 17],
}

impl Default for Motorola6809GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Motorola6809GateLevel {
    /// Construct deterministic reset state.
    #[must_use]
    pub fn new() -> Self {
        let mut cpu = Self {
            memory: DffMemory::new(MEMORY_SIZE),
            a: [0; 8],
            b: [0; 8],
            x: [0; 16],
            y: [0; 16],
            u: [0; 16],
            s: [0; 16],
            pc: [0; 16],
            dp: [0; 8],
            cc: [0; 8],
            halted: 0,
            loaded_origin: [0; 16],
            loaded_len: [0; 17],
        };
        clock_bits(&mut cpu.cc, u64::from(CC_I | CC_F));
        cpu
    }

    /// Clear memory and restore deterministic architectural reset state.
    pub fn reset(&mut self) {
        self.memory.clear();
        clock_bits(&mut self.a, 0);
        clock_bits(&mut self.b, 0);
        clock_bits(&mut self.x, 0);
        clock_bits(&mut self.y, 0);
        clock_bits(&mut self.u, 0);
        clock_bits(&mut self.s, 0);
        clock_bits(&mut self.pc, 0);
        clock_bits(&mut self.dp, 0);
        clock_bits(&mut self.cc, u64::from(CC_I | CC_F));
        clock_bit(&mut self.halted, false);
        clock_bits(&mut self.loaded_origin, 0);
        clock_bits(&mut self.loaded_len, 0);
    }

    /// Snapshot the complete architectural state.
    #[must_use]
    pub fn get_state(&self) -> Motorola6809State {
        Motorola6809State {
            a: self.read_a(),
            b: self.read_b(),
            x: self.read_x(),
            y: self.read_y(),
            u: self.read_u(),
            s: self.read_s(),
            pc: read_bits(&self.pc) as u16,
            dp: self.read_dp(),
            cc: self.read_cc(),
            memory: self.memory.snapshot(),
            halted: self.halted != 0,
            loaded_origin: read_bits(&self.loaded_origin) as u16,
            loaded_len: read_bits(&self.loaded_len) as usize,
        }
    }

    /// Validate and clock a complete state into the DFF topology.
    pub fn restore(&mut self, snapshot: Motorola6809State) -> Result<(), Motorola6809Error> {
        let mut validator = Motorola6809Simulator::new();
        validator.restore(snapshot.clone())?;
        self.restore_validated(&snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &Motorola6809State) {
        self.memory.restore_snapshot(&snapshot.memory);
        clock_bits(&mut self.a, u64::from(snapshot.a));
        clock_bits(&mut self.b, u64::from(snapshot.b));
        clock_bits(&mut self.x, u64::from(snapshot.x));
        clock_bits(&mut self.y, u64::from(snapshot.y));
        clock_bits(&mut self.u, u64::from(snapshot.u));
        clock_bits(&mut self.s, u64::from(snapshot.s));
        clock_bits(&mut self.pc, u64::from(snapshot.pc));
        clock_bits(&mut self.dp, u64::from(snapshot.dp));
        clock_bits(&mut self.cc, u64::from(snapshot.cc));
        clock_bit(&mut self.halted, snapshot.halted);
        clock_bits(&mut self.loaded_origin, u64::from(snapshot.loaded_origin));
        clock_bits(&mut self.loaded_len, snapshot.loaded_len as u64);
    }

    fn commit_transition(&mut self, next: &Motorola6809State) {
        for (address, value) in next.memory.iter().copied().enumerate() {
            if self.memory.read(address) != value {
                self.memory.write(address, value);
            }
        }
        clock_bits(&mut self.a, u64::from(next.a));
        clock_bits(&mut self.b, u64::from(next.b));
        clock_bits(&mut self.x, u64::from(next.x));
        clock_bits(&mut self.y, u64::from(next.y));
        clock_bits(&mut self.u, u64::from(next.u));
        clock_bits(&mut self.s, u64::from(next.s));
        clock_bits(&mut self.pc, u64::from(next.pc));
        clock_bits(&mut self.dp, u64::from(next.dp));
        clock_bits(&mut self.cc, u64::from(next.cc));
        clock_bit(&mut self.halted, next.halted);
        clock_bits(&mut self.loaded_origin, u64::from(next.loaded_origin));
        clock_bits(&mut self.loaded_len, next.loaded_len as u64);
    }

    /// Install raw bytes after checked range validation.
    pub fn load(&mut self, program: &[u8], origin: u16) -> Result<(), Motorola6809Error> {
        let mut validator = Motorola6809Simulator::new();
        validator.load(program, origin)?;
        self.reset();
        for (offset, value) in program.iter().copied().enumerate() {
            self.memory.write(usize::from(origin) + offset, value);
        }
        clock_bits(&mut self.pc, u64::from(origin));
        clock_bits(&mut self.loaded_origin, u64::from(origin));
        clock_bits(&mut self.loaded_len, program.len() as u64);
        Ok(())
    }

    #[must_use]
    pub fn read_a(&self) -> u8 {
        read_bits(&self.a) as u8
    }
    #[must_use]
    pub fn read_b(&self) -> u8 {
        read_bits(&self.b) as u8
    }
    #[must_use]
    pub fn read_d(&self) -> u16 {
        u16::from_be_bytes([self.read_a(), self.read_b()])
    }
    #[must_use]
    pub fn read_x(&self) -> u16 {
        read_bits(&self.x) as u16
    }
    #[must_use]
    pub fn read_y(&self) -> u16 {
        read_bits(&self.y) as u16
    }
    #[must_use]
    pub fn read_u(&self) -> u16 {
        read_bits(&self.u) as u16
    }
    #[must_use]
    pub fn read_s(&self) -> u16 {
        read_bits(&self.s) as u16
    }
    #[must_use]
    pub fn read_dp(&self) -> u8 {
        read_bits(&self.dp) as u8
    }
    #[must_use]
    pub fn read_cc(&self) -> u8 {
        read_bits(&self.cc) as u8
    }

    pub fn write_a(&mut self, value: u8) {
        clock_bits(&mut self.a, u64::from(value));
    }
    pub fn write_b(&mut self, value: u8) {
        clock_bits(&mut self.b, u64::from(value));
    }
    pub fn write_d(&mut self, value: u16) {
        let [a, b] = value.to_be_bytes();
        self.write_a(a);
        self.write_b(b);
    }
    pub fn write_x(&mut self, value: u16) {
        clock_bits(&mut self.x, u64::from(value));
    }
    pub fn write_y(&mut self, value: u16) {
        clock_bits(&mut self.y, u64::from(value));
    }
    pub fn write_u(&mut self, value: u16) {
        clock_bits(&mut self.u, u64::from(value));
    }
    pub fn write_s(&mut self, value: u16) {
        clock_bits(&mut self.s, u64::from(value));
    }
    pub fn write_dp(&mut self, value: u8) {
        clock_bits(&mut self.dp, u64::from(value));
    }
    pub fn write_cc(&mut self, value: u8) {
        clock_bits(&mut self.cc, u64::from(value));
    }

    fn memory_range(address: u16, len: usize) -> Result<std::ops::Range<usize>, Motorola6809Error> {
        let start = usize::from(address);
        let Some(end) = start.checked_add(len) else {
            return Err(Motorola6809Error::MemoryOutOfRange { address, len });
        };
        if end > MEMORY_SIZE {
            return Err(Motorola6809Error::MemoryOutOfRange { address, len });
        }
        Ok(start..end)
    }

    pub fn read_memory(&self, address: u16, len: usize) -> Result<Vec<u8>, Motorola6809Error> {
        Ok(Self::memory_range(address, len)?
            .map(|index| self.memory.read(index))
            .collect())
    }

    pub fn write_memory(&mut self, address: u16, bytes: &[u8]) -> Result<(), Motorola6809Error> {
        for (index, value) in Self::memory_range(address, bytes.len())?.zip(bytes.iter().copied()) {
            self.memory.write(index, value);
        }
        Ok(())
    }

    /// Execute one checked gate transition and clock its settled state.
    pub fn step(&mut self) -> Result<StepTrace, Motorola6809Error> {
        let before = self.get_state();
        let gate_result = executor::execute(&before);
        let mut oracle = Motorola6809Simulator::new();
        oracle.restore(before.clone())?;
        let oracle_result = oracle.step();
        match (gate_result, oracle_result) {
            (Ok(gate_trace), Ok(oracle_trace)) if gate_trace == oracle_trace => {
                self.commit_transition(&gate_trace.state_after);
                Ok(gate_trace)
            }
            (Err(gate_error), Err(oracle_error)) if gate_error == oracle_error => Err(gate_error),
            _ => Err(Motorola6809Error::InvalidState {
                reason: "independent gate transition diverged from the functional oracle",
            }),
        }
    }

    /// Execute until wait/halt or the supplied bound, rolling back on failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Motorola6809Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(Motorola6809Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let before = self.get_state();
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| Motorola6809Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: self.get_state(),
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
        Err(Motorola6809Error::StepLimitExceeded { limit: max_steps })
    }

    /// Transactionally load and execute a raw image.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u16,
        max_steps: usize,
    ) -> Result<ExecutionResult, Motorola6809Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(Motorola6809Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let before = self.get_state();
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
