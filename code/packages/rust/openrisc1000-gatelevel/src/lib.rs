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
            memory: self.memory.snapshot(),
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
        let mut validator = engine::OpenRisc1000Simulator::new();
        validator.restore(snapshot.clone())?;
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

    fn commit_transition(&mut self, before: &OpenRisc1000State, next: &OpenRisc1000State) {
        for (address, (&old, &new)) in before.memory.iter().zip(&next.memory).enumerate() {
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
        let mut validator = engine::OpenRisc1000Simulator::new();
        validator.load(program, origin)?;
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
        let state = self.get_state();
        if let Some(reserved) = state.reservation {
            let write_end = u64::from(address) + bytes.len() as u64;
            let reservation_end = u64::from(reserved) + 4;
            if u64::from(address) < reservation_end && u64::from(reserved) < write_end {
                clock_bit(&mut self.reservation_valid, false);
            }
        }
        Ok(())
    }

    /// Execute one independent transition and clock its gate-validated result.
    pub fn step(&mut self) -> Result<StepTrace, OpenRisc1000Error> {
        let before = self.get_state();
        if before.halted {
            return Err(OpenRisc1000Error::Halted);
        }
        validate_gate_primary(&before)?;
        let mut engine = engine::OpenRisc1000Simulator::new();
        engine.restore(before.clone())?;
        let trace = engine.step()?;
        validate_gate_datapath(&before, &trace)?;
        self.commit_transition(&before, &trace.state_after);
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
        let before = self.get_state();
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| OpenRisc1000Error::AllocationFailed)?;
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

fn validate_gate_primary(state: &OpenRisc1000State) -> Result<(), OpenRisc1000Error> {
    if state.pc & 3 != 0 {
        return Ok(());
    }
    let start = state.pc as usize;
    let image_start = state.loaded_origin as usize;
    let image_end = image_start.saturating_add(state.loaded_len);
    let Some(end) = start.checked_add(4) else {
        return Ok(());
    };
    if start < image_start || end > image_end || end > state.memory.len() {
        return Ok(());
    }
    let word = u32::from_be_bytes(state.memory[start..end].try_into().expect("word slice"));
    let opcode = (word >> 26) as u8;
    let gate_implemented = gates::primary_is_implemented(opcode) == 1;
    let gate_unsupported = gates::primary_is_unsupported(opcode) == 1;
    let local_unsupported = matches!(
        opcode,
        0x0a | 0x1a | 0x20 | 0x32 | 0x1c..=0x1f | 0x3c..=0x3f
    );
    if gate_implemented != engine::primary_opcode_is_implemented(opcode) {
        return Err(OpenRisc1000Error::InvalidState {
            reason: "gate primary decoder disagrees with the local transition engine",
        });
    }
    if gate_unsupported != local_unsupported || (gate_implemented && gate_unsupported) {
        return Err(OpenRisc1000Error::InvalidState {
            reason: "gate unsupported decoder disagrees with the local transition engine",
        });
    }
    Ok(())
}

fn validate_gate_datapath(
    before: &OpenRisc1000State,
    trace: &StepTrace,
) -> Result<(), OpenRisc1000Error> {
    let word = trace.raw_word;
    let opcode = (word >> 26) as u8;
    let rd = ((word >> 21) & 0x1f) as usize;
    let ra = ((word >> 16) & 0x1f) as usize;
    let rb = ((word >> 11) & 0x1f) as usize;
    let immediate = word as u16;
    let actual = trace.state_after.gpr[rd];
    let expected = match opcode {
        0x27 => Some(gates::add_32(before.gpr[ra], (immediate as i16 as i32) as u32, 0).0),
        0x28 => Some(
            gates::add_32(
                before.gpr[ra],
                (immediate as i16 as i32) as u32,
                u8::from(before.sr & SR_CY != 0),
            )
            .0,
        ),
        0x29 => Some(gates::bit_and_32(before.gpr[ra], u32::from(immediate))),
        0x2a => Some(gates::bit_or_32(before.gpr[ra], u32::from(immediate))),
        0x2b => Some(gates::bit_xor_32(
            before.gpr[ra],
            (immediate as i16 as i32) as u32,
        )),
        0x2e => match (word >> 6) & 3 {
            0 => Some(gates::shift_left_32(before.gpr[ra], word & 0x1f)),
            1 => Some(gates::shift_right_32(before.gpr[ra], word & 0x1f)),
            _ => None,
        },
        0x38 => match word & 0x7ff {
            0x000 => Some(gates::add_32(before.gpr[ra], before.gpr[rb], 0).0),
            0x001 => Some(
                gates::add_32(
                    before.gpr[ra],
                    before.gpr[rb],
                    u8::from(before.sr & SR_CY != 0),
                )
                .0,
            ),
            0x002 => Some(gates::sub_32(before.gpr[ra], before.gpr[rb]).0),
            0x003 => Some(gates::bit_and_32(before.gpr[ra], before.gpr[rb])),
            0x004 => Some(gates::bit_or_32(before.gpr[ra], before.gpr[rb])),
            0x005 => Some(gates::bit_xor_32(before.gpr[ra], before.gpr[rb])),
            0x008 => Some(gates::shift_left_32(before.gpr[ra], before.gpr[rb] & 0x1f)),
            0x048 => Some(gates::shift_right_32(before.gpr[ra], before.gpr[rb] & 0x1f)),
            _ => None,
        },
        _ => None,
    };
    if expected.is_some_and(|value| value != actual) {
        return Err(OpenRisc1000Error::InvalidState {
            reason: "gate arithmetic network disagrees with the local transition engine",
        });
    }
    Ok(())
}
