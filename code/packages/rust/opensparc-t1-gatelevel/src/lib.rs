//! DFF-backed architectural gate model of one OpenSPARC T1 strand.

mod engine;
pub mod gates;
mod state;

pub use engine::{
    assemble, classify_word, encoding, ConditionCodeRegister, ConditionCodes, ExecutionResult,
    InstructionClass, OpenSparcT1Error, OpenSparcT1State, ProcessorState, StepTrace,
    MAX_RETAINED_STEPS, MEMORY_SIZE, NUM_PHYSICAL_REGISTERS,
};

use state::{clock_bit, clock_bits, read_bits, DffMemory};

/// Exact number of persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize = 534_813;
/// Six primitive gates per master/slave DFF plus a stable combinational budget.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 262_144;

/// Complete architectural state clocked through simulated D flip-flops.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenSparcT1GateLevel {
    memory: DffMemory,
    registers: [[u8; 64]; NUM_PHYSICAL_REGISTERS],
    pc: [u8; 64],
    npc: [u8; 64],
    icc: [u8; 4],
    xcc: [u8; 4],
    y: [u8; 32],
    asi: [u8; 8],
    pstate: [u8; 3],
    gl: [u8; 2],
    cwp: [u8; 3],
    cansave: [u8; 3],
    canrestore: [u8; 3],
    cleanwin: [u8; 3],
    otherwin: [u8; 3],
    wstate: [u8; 6],
    halted: u8,
    delay_slot: u8,
    loaded_origin: [u8; 64],
    loaded_len: [u8; 17],
}

impl Default for OpenSparcT1GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenSparcT1GateLevel {
    /// Construct the deterministic privileged reset state.
    #[must_use]
    pub fn new() -> Self {
        let mut cpu = Self {
            memory: DffMemory::new(MEMORY_SIZE),
            registers: [[0; 64]; NUM_PHYSICAL_REGISTERS],
            pc: [0; 64],
            npc: [0; 64],
            icc: [0; 4],
            xcc: [0; 4],
            y: [0; 32],
            asi: [0; 8],
            pstate: [0; 3],
            gl: [0; 2],
            cwp: [0; 3],
            cansave: [0; 3],
            canrestore: [0; 3],
            cleanwin: [0; 3],
            otherwin: [0; 3],
            wstate: [0; 6],
            halted: 0,
            delay_slot: 0,
            loaded_origin: [0; 64],
            loaded_len: [0; 17],
        };
        cpu.restore_validated(&OpenSparcT1State::default());
        cpu
    }

    /// Clear every persistent bit through its DFF input.
    pub fn reset(&mut self) {
        self.memory.clear();
        self.restore_validated(&OpenSparcT1State::default());
    }

    /// Snapshot the complete architectural state.
    #[must_use]
    pub fn get_state(&self) -> OpenSparcT1State {
        self.state_with_memory(self.memory.snapshot())
    }

    fn try_get_state(&self) -> Result<OpenSparcT1State, OpenSparcT1Error> {
        let memory = self
            .memory
            .try_snapshot()
            .map_err(|()| OpenSparcT1Error::AllocationFailed)?;
        Ok(self.state_with_memory(memory))
    }

    fn state_in_preallocated(&self, mut memory: Vec<u8>) -> OpenSparcT1State {
        self.memory.snapshot_into(&mut memory);
        self.state_with_memory(memory)
    }

    fn state_with_memory(&self, memory: Vec<u8>) -> OpenSparcT1State {
        OpenSparcT1State {
            registers: std::array::from_fn(|index| read_bits(&self.registers[index])),
            pc: read_bits(&self.pc),
            npc: read_bits(&self.npc),
            ccr: ConditionCodeRegister {
                icc: Self::read_codes(&self.icc),
                xcc: Self::read_codes(&self.xcc),
            },
            y: read_bits(&self.y) as u32,
            asi: read_bits(&self.asi) as u8,
            pstate: ProcessorState {
                privileged: self.pstate[0] == 1,
                cle: self.pstate[1] == 1,
                address_mask: self.pstate[2] == 1,
            },
            gl: read_bits(&self.gl) as u8,
            cwp: read_bits(&self.cwp) as u8,
            cansave: read_bits(&self.cansave) as u8,
            canrestore: read_bits(&self.canrestore) as u8,
            cleanwin: read_bits(&self.cleanwin) as u8,
            otherwin: read_bits(&self.otherwin) as u8,
            wstate: read_bits(&self.wstate) as u8,
            memory,
            halted: self.halted == 1,
            delay_slot: self.delay_slot == 1,
            loaded_origin: read_bits(&self.loaded_origin),
            loaded_len: read_bits(&self.loaded_len) as usize,
        }
    }

    fn read_codes(bits: &[u8; 4]) -> ConditionCodes {
        ConditionCodes {
            n: bits[0] == 1,
            z: bits[1] == 1,
            v: bits[2] == 1,
            c: bits[3] == 1,
        }
    }

    fn clock_codes(bits: &mut [u8; 4], codes: ConditionCodes) {
        clock_bits(
            bits,
            u64::from(codes.n)
                | (u64::from(codes.z) << 1)
                | (u64::from(codes.v) << 2)
                | (u64::from(codes.c) << 3),
        );
    }

    /// Validate and atomically clock a complete state into the fixed topology.
    pub fn restore(&mut self, snapshot: OpenSparcT1State) -> Result<(), OpenSparcT1Error> {
        engine::validate_state(&snapshot)?;
        self.restore_validated(&snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &OpenSparcT1State) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (target, value) in self.registers.iter_mut().zip(snapshot.registers) {
            clock_bits(target, value);
        }
        clock_bits(&mut self.pc, snapshot.pc);
        clock_bits(&mut self.npc, snapshot.npc);
        Self::clock_codes(&mut self.icc, snapshot.ccr.icc);
        Self::clock_codes(&mut self.xcc, snapshot.ccr.xcc);
        clock_bits(&mut self.y, u64::from(snapshot.y));
        clock_bits(&mut self.asi, u64::from(snapshot.asi));
        clock_bits(
            &mut self.pstate,
            u64::from(snapshot.pstate.privileged)
                | (u64::from(snapshot.pstate.cle) << 1)
                | (u64::from(snapshot.pstate.address_mask) << 2),
        );
        clock_bits(&mut self.gl, u64::from(snapshot.gl));
        clock_bits(&mut self.cwp, u64::from(snapshot.cwp));
        clock_bits(&mut self.cansave, u64::from(snapshot.cansave));
        clock_bits(&mut self.canrestore, u64::from(snapshot.canrestore));
        clock_bits(&mut self.cleanwin, u64::from(snapshot.cleanwin));
        clock_bits(&mut self.otherwin, u64::from(snapshot.otherwin));
        clock_bits(&mut self.wstate, u64::from(snapshot.wstate));
        clock_bit(&mut self.halted, snapshot.halted);
        clock_bit(&mut self.delay_slot, snapshot.delay_slot);
        clock_bits(&mut self.loaded_origin, snapshot.loaded_origin);
        clock_bits(&mut self.loaded_len, snapshot.loaded_len as u64);
    }

    fn commit_transition(&mut self, next: &OpenSparcT1State) {
        for (address, &new) in next.memory.iter().enumerate() {
            if gates::byte_changed(self.memory.read(address), new) == 1 {
                self.memory.write(address, new);
            }
        }
        self.restore_architecture_without_memory(next);
    }

    fn restore_architecture_without_memory(&mut self, snapshot: &OpenSparcT1State) {
        for (target, value) in self.registers.iter_mut().zip(snapshot.registers) {
            clock_bits(target, value);
        }
        clock_bits(&mut self.pc, snapshot.pc);
        clock_bits(&mut self.npc, snapshot.npc);
        Self::clock_codes(&mut self.icc, snapshot.ccr.icc);
        Self::clock_codes(&mut self.xcc, snapshot.ccr.xcc);
        clock_bits(&mut self.y, u64::from(snapshot.y));
        clock_bits(&mut self.asi, u64::from(snapshot.asi));
        clock_bits(
            &mut self.pstate,
            u64::from(snapshot.pstate.privileged)
                | (u64::from(snapshot.pstate.cle) << 1)
                | (u64::from(snapshot.pstate.address_mask) << 2),
        );
        clock_bits(&mut self.gl, u64::from(snapshot.gl));
        clock_bits(&mut self.cwp, u64::from(snapshot.cwp));
        clock_bits(&mut self.cansave, u64::from(snapshot.cansave));
        clock_bits(&mut self.canrestore, u64::from(snapshot.canrestore));
        clock_bits(&mut self.cleanwin, u64::from(snapshot.cleanwin));
        clock_bits(&mut self.otherwin, u64::from(snapshot.otherwin));
        clock_bits(&mut self.wstate, u64::from(snapshot.wstate));
        clock_bit(&mut self.halted, snapshot.halted);
        clock_bit(&mut self.delay_slot, snapshot.delay_slot);
        clock_bits(&mut self.loaded_origin, snapshot.loaded_origin);
        clock_bits(&mut self.loaded_len, snapshot.loaded_len as u64);
    }

    /// Install a checked, word-aligned, big-endian program image.
    pub fn load(&mut self, program: &[u8], origin: u64) -> Result<(), OpenSparcT1Error> {
        engine::validate_program(program, origin)?;
        self.reset();
        for (offset, value) in program.iter().copied().enumerate() {
            self.memory.write(origin as usize + offset, value);
        }
        clock_bits(&mut self.pc, origin);
        clock_bits(&mut self.npc, gates::add_64(origin, 4, 0).0);
        clock_bits(&mut self.loaded_origin, origin);
        clock_bits(&mut self.loaded_len, program.len() as u64);
        Ok(())
    }

    fn physical_register_index(&self, index: u8) -> Result<usize, OpenSparcT1Error> {
        if index >= 32 {
            return Err(OpenSparcT1Error::InvalidRegister { index });
        }
        let gl = read_bits(&self.gl) as usize;
        let cwp = read_bits(&self.cwp) as usize;
        Ok(match index {
            0..=7 => gl * 8 + usize::from(index),
            8..=23 => 32 + cwp * 16 + usize::from(index - 8),
            _ => 32 + ((cwp + 7) & 7) * 16 + usize::from(index - 24),
        })
    }

    pub fn read_register(&self, index: u8) -> Result<u64, OpenSparcT1Error> {
        if index == 0 {
            return Ok(0);
        }
        Ok(read_bits(
            &self.registers[self.physical_register_index(index)?],
        ))
    }

    pub fn write_register(&mut self, index: u8, value: u64) -> Result<(), OpenSparcT1Error> {
        if index == 0 {
            return Ok(());
        }
        let physical = self.physical_register_index(index)?;
        clock_bits(&mut self.registers[physical], value);
        Ok(())
    }

    fn checked_memory_range(
        address: u64,
        width: usize,
    ) -> Result<std::ops::Range<usize>, OpenSparcT1Error> {
        if !matches!(width, 1 | 2 | 4 | 8) {
            return Err(OpenSparcT1Error::DataOutOfRange { address, width });
        }
        if address & (width as u64 - 1) != 0 {
            return Err(OpenSparcT1Error::MisalignedData { address, width });
        }
        let start = usize::try_from(address)
            .map_err(|_| OpenSparcT1Error::DataOutOfRange { address, width })?;
        let end = start
            .checked_add(width)
            .filter(|end| *end <= MEMORY_SIZE)
            .ok_or(OpenSparcT1Error::DataOutOfRange { address, width })?;
        Ok(start..end)
    }

    pub fn read_memory(&self, address: u64, width: usize) -> Result<u64, OpenSparcT1Error> {
        Ok(
            Self::checked_memory_range(address, width)?.fold(0_u64, |value, index| {
                (value << 8) | u64::from(self.memory.read(index))
            }),
        )
    }

    pub fn write_memory(
        &mut self,
        address: u64,
        width: usize,
        value: u64,
    ) -> Result<(), OpenSparcT1Error> {
        for (offset, index) in Self::checked_memory_range(address, width)?.enumerate() {
            let shift = (width - offset - 1) * 8;
            self.memory.write(index, (value >> shift) as u8);
        }
        Ok(())
    }

    /// Execute one gate-derived transition and clock its settled result.
    pub fn step(&mut self) -> Result<StepTrace, OpenSparcT1Error> {
        let trace = engine::step_state(self.try_get_state()?)?;
        self.commit_transition(&trace.state_after);
        Ok(trace)
    }

    fn step_in_preallocated(&mut self, memory: Vec<u8>) -> Result<StepTrace, OpenSparcT1Error> {
        let trace = engine::step_state(self.state_in_preallocated(memory))?;
        self.commit_transition(&trace.state_after);
        Ok(trace)
    }

    /// Execute until halt or the supplied bound, rolling back on any failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, OpenSparcT1Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(OpenSparcT1Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let allocation_count = max_steps
            .checked_add(2)
            .ok_or(OpenSparcT1Error::AllocationFailed)?;
        let mut state_memories = Vec::new();
        state_memories
            .try_reserve_exact(allocation_count)
            .map_err(|_| OpenSparcT1Error::AllocationFailed)?;
        for _ in 0..allocation_count {
            let mut memory = Vec::new();
            memory
                .try_reserve_exact(MEMORY_SIZE)
                .map_err(|_| OpenSparcT1Error::AllocationFailed)?;
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
            .map_err(|_| OpenSparcT1Error::AllocationFailed)?;
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
        Err(OpenSparcT1Error::StepLimitExceeded { limit: max_steps })
    }

    /// Transactionally install and execute one image.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u64,
        max_steps: usize,
    ) -> Result<ExecutionResult, OpenSparcT1Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(OpenSparcT1Error::RetentionLimitExceeded {
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
