//! Gate-backed Motorola MC6800 architectural simulator.
//!
//! Every persistent programmer-visible bit is stored in repository D
//! flip-flops. Successful architectural transitions are computed by this
//! package's independent gate networks. The checked functional simulator is a
//! fail-closed trace/fault oracle, and a gate transition is committed only
//! after its complete settled state agrees with that oracle.

pub use motorola6800_simulator::{
    instruction_width, ExecutionResult, Motorola6800Error, Motorola6800Simulator,
    Motorola6800State, StepTrace, CC_C, CC_H, CC_I, CC_N, CC_V, CC_Z, MEMORY_SIZE,
};

mod executor;
pub mod gates;
mod state;

use state::{clock_bit, clock_bits, read_bits, DffMemory};

/// Exact number of persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 8 + 8 + 16 + 16 + 16 + 8 + 1;
/// Stable educational storage-plus-combinational gate estimate.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 32_768;

/// MC6800 state stored in simulated D flip-flops.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Motorola6800GateLevel {
    memory: DffMemory,
    a: [u8; 8],
    b: [u8; 8],
    x: [u8; 16],
    sp: [u8; 16],
    pc: [u8; 16],
    cc: [u8; 8],
    halted: u8,
    loaded_origin: u16,
    loaded_len: usize,
}

impl Default for Motorola6800GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Motorola6800GateLevel {
    #[must_use]
    pub fn new() -> Self {
        let mut cpu = Self {
            memory: DffMemory::new(MEMORY_SIZE),
            a: [0; 8],
            b: [0; 8],
            x: [0; 16],
            sp: [0; 16],
            pc: [0; 16],
            cc: [0; 8],
            halted: 0,
            loaded_origin: 0,
            loaded_len: 0,
        };
        clock_bits(&mut cpu.cc, 0xc0);
        cpu
    }

    pub fn reset(&mut self) {
        self.memory.clear();
        clock_bits(&mut self.a, 0);
        clock_bits(&mut self.b, 0);
        clock_bits(&mut self.x, 0);
        clock_bits(&mut self.sp, 0);
        clock_bits(&mut self.pc, 0);
        clock_bits(&mut self.cc, 0xc0);
        clock_bit(&mut self.halted, false);
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    #[must_use]
    pub fn get_state(&self) -> Motorola6800State {
        Motorola6800State {
            a: read_bits(&self.a) as u8,
            b: read_bits(&self.b) as u8,
            x: read_bits(&self.x) as u16,
            sp: read_bits(&self.sp) as u16,
            pc: read_bits(&self.pc) as u16,
            cc: read_bits(&self.cc) as u8,
            memory: self.memory.snapshot(),
            halted: self.halted != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    pub fn restore(&mut self, snapshot: Motorola6800State) -> Result<(), Motorola6800Error> {
        let mut validator = Motorola6800Simulator::new();
        validator.restore(snapshot.clone())?;
        self.restore_validated(&snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &Motorola6800State) {
        self.memory.restore_snapshot(&snapshot.memory);
        clock_bits(&mut self.a, u64::from(snapshot.a));
        clock_bits(&mut self.b, u64::from(snapshot.b));
        clock_bits(&mut self.x, u64::from(snapshot.x));
        clock_bits(&mut self.sp, u64::from(snapshot.sp));
        clock_bits(&mut self.pc, u64::from(snapshot.pc));
        clock_bits(&mut self.cc, u64::from(snapshot.cc | 0xc0));
        clock_bit(&mut self.halted, snapshot.halted);
        self.loaded_origin = snapshot.loaded_origin;
        self.loaded_len = snapshot.loaded_len;
    }

    fn commit_transition(&mut self, next: &Motorola6800State) {
        for (address, value) in next.memory.iter().copied().enumerate() {
            if self.memory.read(address) != value {
                self.memory.write(address, value);
            }
        }
        clock_bits(&mut self.a, u64::from(next.a));
        clock_bits(&mut self.b, u64::from(next.b));
        clock_bits(&mut self.x, u64::from(next.x));
        clock_bits(&mut self.sp, u64::from(next.sp));
        clock_bits(&mut self.pc, u64::from(next.pc));
        clock_bits(&mut self.cc, u64::from(next.cc | 0xc0));
        clock_bit(&mut self.halted, next.halted);
        self.loaded_origin = next.loaded_origin;
        self.loaded_len = next.loaded_len;
    }

    pub fn load(&mut self, program: &[u8], origin: u16) -> Result<(), Motorola6800Error> {
        let mut validator = Motorola6800Simulator::new();
        validator.load(program, origin)?;
        self.reset();
        for (offset, value) in program.iter().copied().enumerate() {
            self.memory.write(usize::from(origin) + offset, value);
        }
        clock_bits(&mut self.pc, u64::from(origin));
        self.loaded_origin = origin;
        self.loaded_len = program.len();
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
    pub fn read_x(&self) -> u16 {
        read_bits(&self.x) as u16
    }

    #[must_use]
    pub fn read_sp(&self) -> u16 {
        read_bits(&self.sp) as u16
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

    pub fn write_x(&mut self, value: u16) {
        clock_bits(&mut self.x, u64::from(value));
    }

    pub fn write_sp(&mut self, value: u16) {
        clock_bits(&mut self.sp, u64::from(value));
    }

    pub fn write_cc(&mut self, value: u8) {
        clock_bits(&mut self.cc, u64::from(value | 0xc0));
    }

    fn memory_range(address: u16, len: usize) -> Result<std::ops::Range<usize>, Motorola6800Error> {
        let start = usize::from(address);
        let end = start
            .checked_add(len)
            .ok_or(Motorola6800Error::InvalidState {
                reason: "memory range wraps",
            })?;
        if end > MEMORY_SIZE {
            return Err(Motorola6800Error::InvalidState {
                reason: "memory range wraps",
            });
        }
        Ok(start..end)
    }

    pub fn read_memory(&self, address: u16, len: usize) -> Result<Vec<u8>, Motorola6800Error> {
        Ok(Self::memory_range(address, len)?
            .map(|index| self.memory.read(index))
            .collect())
    }

    pub fn write_memory(&mut self, address: u16, bytes: &[u8]) -> Result<(), Motorola6800Error> {
        for (index, value) in Self::memory_range(address, bytes.len())?.zip(bytes.iter().copied()) {
            self.memory.write(index, value);
        }
        Ok(())
    }

    /// Execute one checked transition and clock its settled state into DFFs.
    pub fn step(&mut self) -> Result<StepTrace, Motorola6800Error> {
        let before = self.get_state();
        let mut oracle = Motorola6800Simulator::new();
        oracle.restore(before.clone())?;
        let trace = oracle.step()?;
        let oracle_next = oracle.get_state();
        let decode = gates::decode_byte(trace.raw[0]);
        if decode[usize::from(trace.raw[0])] != 1
            || decode.into_iter().map(u16::from).sum::<u16>() != 1
        {
            return Err(Motorola6800Error::InvalidState {
                reason: "gate opcode decoder is not one-hot",
            });
        }
        let gate_next = executor::execute(&before, &trace)?;
        if gate_next != oracle_next {
            return Err(Motorola6800Error::InvalidState {
                reason: "independent gate transition diverged from the functional oracle",
            });
        }
        self.commit_transition(&gate_next);
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Motorola6800Error> {
        let before = self.get_state();
        let mut traces = Vec::new();
        while traces.len() < max_steps {
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
        Err(Motorola6800Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u16,
        max_steps: usize,
    ) -> Result<ExecutionResult, Motorola6800Error> {
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
