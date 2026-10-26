//! Educational DFF-backed ARMv4T ARM7TDMI gate simulator.
//!
//! The transition engine is local to this package.  It uses repository gate
//! networks for decode predicates, conditions, bitwise datapaths, shifts,
//! add/subtract flags, and persistent storage.  The functional simulator is a
//! development-only differential oracle and is never linked into production.

mod engine;
pub mod gates;
mod state;

pub use engine::{
    Arm7TdmiError, Arm7TdmiState, ExecutionResult, InstructionSet, Mode, PhysicalRegisters,
    StepTrace, CPSR_C, CPSR_F, CPSR_I, CPSR_N, CPSR_T, CPSR_V, CPSR_Z, MAX_RETAINED_STEPS,
    MEMORY_SIZE,
};

use state::{clock_bit, clock_bits, read_bits, DffMemory};

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
static FAIL_MID_COMMIT: AtomicBool = AtomicBool::new(false);

const PHYSICAL_GPR_COUNT: usize = 31;
const STATUS_REGISTER_COUNT: usize = 6;
const IMAGE_LENGTH_BITS: usize = 17;

/// Exact stable-Q architectural storage inventory from Spec 07ax2.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8
    + PHYSICAL_GPR_COUNT * 32
    + STATUS_REGISTER_COUNT * 32
    + 1
    + 32
    + IMAGE_LENGTH_BITS;

/// Conservative primitive-gate budget: six gates per DFF plus combinational
/// ARM/Thumb decode, mux, shifter, multiplier, ALU, and control networks.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 196_608;

/// Stable-Q-backed complete ARM7TDMI architectural state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arm7TdmiGateLevel {
    memory: DffMemory,
    registers: [[u8; 32]; PHYSICAL_GPR_COUNT],
    cpsr: [u8; 32],
    spsr: [[u8; 32]; 5],
    halted: u8,
    loaded_origin: [u8; 32],
    loaded_len: [u8; IMAGE_LENGTH_BITS],
}

impl Default for Arm7TdmiGateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Arm7TdmiGateLevel {
    /// Construct the deterministic Supervisor/ARM reset state.
    #[must_use]
    pub fn new() -> Self {
        let mut cpu = Self {
            memory: DffMemory::new(MEMORY_SIZE),
            registers: [[0; 32]; PHYSICAL_GPR_COUNT],
            cpsr: [0; 32],
            spsr: [[0; 32]; 5],
            halted: 0,
            loaded_origin: [0; 32],
            loaded_len: [0; IMAGE_LENGTH_BITS],
        };
        cpu.clock_complete_state(&Arm7TdmiState::default());
        cpu
    }

    /// Clock every owned bit to the canonical reset state.
    pub fn reset(&mut self) {
        self.clock_complete_state(&Arm7TdmiState::default());
    }

    /// Snapshot the complete stable-Q state.
    #[must_use]
    pub fn get_state(&self) -> Arm7TdmiState {
        self.state_with_memory(self.memory.snapshot())
    }

    /// Validate then atomically clock a complete state.
    pub fn restore(&mut self, snapshot: Arm7TdmiState) -> Result<(), Arm7TdmiError> {
        let mut validator = engine::Arm7TdmiSimulator::new();
        validator.restore(snapshot.clone())?;
        self.commit_transition(&snapshot)?;
        Ok(())
    }

    /// Install a checked little-endian image and select ARM or Thumb fetch.
    pub fn load(
        &mut self,
        program: &[u8],
        origin: u32,
        instruction_set: InstructionSet,
    ) -> Result<(), Arm7TdmiError> {
        let mut candidate = engine::Arm7TdmiSimulator::new();
        candidate.load(program, origin, instruction_set)?;
        self.commit_transition(&candidate.get_state())?;
        Ok(())
    }

    /// Read the current banked logical register view from stable Q.
    pub fn read_register(&self, register: usize) -> Result<u32, Arm7TdmiError> {
        self.engine()?.read_register(register)
    }

    /// Clock one logical-register write after local validation.
    pub fn write_register(&mut self, register: usize, value: u32) -> Result<(), Arm7TdmiError> {
        let mut candidate = self.engine()?;
        candidate.write_register(register, value)?;
        self.commit_transition(&candidate.get_state())?;
        Ok(())
    }

    /// Read a checked byte range from stable-Q memory.
    pub fn read_memory(&self, address: u32, width: usize) -> Result<Vec<u8>, Arm7TdmiError> {
        self.engine()?.read_memory(address, width)
    }

    /// Clock a checked byte-range write into DFF memory.
    pub fn write_memory(&mut self, address: u32, bytes: &[u8]) -> Result<(), Arm7TdmiError> {
        let mut candidate = self.engine()?;
        candidate.write_memory(address, bytes)?;
        self.commit_transition(&candidate.get_state())?;
        Ok(())
    }

    /// Execute one independently decoded transition, then clock one commit.
    pub fn step(&mut self) -> Result<StepTrace, Arm7TdmiError> {
        let mut candidate = self.engine()?;
        let trace = candidate.step()?;
        self.commit_transition(&trace.state_after)?;
        Ok(trace)
    }

    /// Execute a bounded run transactionally and commit only its final state.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Arm7TdmiError> {
        let mut candidate = self.engine()?;
        let result = candidate.run(max_steps)?;
        self.commit_transition(&result.final_state)?;
        Ok(result)
    }

    /// Load and execute transactionally, preserving entry Q on any failure.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        instruction_set: InstructionSet,
        max_steps: usize,
    ) -> Result<ExecutionResult, Arm7TdmiError> {
        let mut candidate = self.engine()?;
        let result = candidate.execute(program, origin, instruction_set, max_steps)?;
        self.commit_transition(&result.final_state)?;
        Ok(result)
    }

    /// Published exact stable-Q storage count.
    #[must_use]
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    /// Published educational primitive-gate estimate.
    #[must_use]
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }

    fn engine(&self) -> Result<engine::Arm7TdmiSimulator, Arm7TdmiError> {
        let mut engine = engine::Arm7TdmiSimulator::new();
        engine.restore(self.get_state())?;
        Ok(engine)
    }

    fn commit_transition(&mut self, next: &Arm7TdmiState) -> Result<(), Arm7TdmiError> {
        #[cfg(test)]
        if FAIL_MID_COMMIT.swap(false, Ordering::SeqCst) {
            let before = self.get_state();
            clock_bits(
                &mut self.registers[0],
                u64::from(next.registers.common_r0_r7[0]),
            );
            self.clock_complete_state(&before);
            return Err(Arm7TdmiError::CommitFailed);
        }
        self.clock_complete_state(next);
        Ok(())
    }

    fn clock_complete_state(&mut self, next: &Arm7TdmiState) {
        let flat = flatten_registers(&next.registers);
        for (target, value) in self.registers.iter_mut().zip(flat) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.cpsr, u64::from(next.cpsr));
        for (target, value) in self.spsr.iter_mut().zip([
            next.spsr_fiq,
            next.spsr_irq,
            next.spsr_supervisor,
            next.spsr_abort,
            next.spsr_undefined,
        ]) {
            clock_bits(target, u64::from(value));
        }
        self.memory.restore_snapshot(&next.memory);
        clock_bit(&mut self.halted, next.halted);
        clock_bits(&mut self.loaded_origin, u64::from(next.loaded_origin));
        clock_bits(&mut self.loaded_len, next.loaded_len as u64);
    }

    fn state_with_memory(&self, memory: Vec<u8>) -> Arm7TdmiState {
        let flat = std::array::from_fn(|index| read_bits(&self.registers[index]) as u32);
        let registers = unflatten_registers(flat);
        Arm7TdmiState {
            registers,
            cpsr: read_bits(&self.cpsr) as u32,
            spsr_fiq: read_bits(&self.spsr[0]) as u32,
            spsr_irq: read_bits(&self.spsr[1]) as u32,
            spsr_supervisor: read_bits(&self.spsr[2]) as u32,
            spsr_abort: read_bits(&self.spsr[3]) as u32,
            spsr_undefined: read_bits(&self.spsr[4]) as u32,
            memory,
            halted: self.halted == 1,
            loaded_origin: read_bits(&self.loaded_origin) as u32,
            loaded_len: read_bits(&self.loaded_len) as usize,
        }
    }
}

fn flatten_registers(registers: &PhysicalRegisters) -> [u32; PHYSICAL_GPR_COUNT] {
    let mut flat = [0; PHYSICAL_GPR_COUNT];
    flat[0..8].copy_from_slice(&registers.common_r0_r7);
    flat[8..13].copy_from_slice(&registers.normal_r8_r12);
    flat[13..20].copy_from_slice(&registers.fiq_r8_r14);
    flat[20..22].copy_from_slice(&registers.user_r13_r14);
    flat[22..24].copy_from_slice(&registers.irq_r13_r14);
    flat[24..26].copy_from_slice(&registers.supervisor_r13_r14);
    flat[26..28].copy_from_slice(&registers.abort_r13_r14);
    flat[28..30].copy_from_slice(&registers.undefined_r13_r14);
    flat[30] = registers.pc;
    flat
}

fn unflatten_registers(flat: [u32; PHYSICAL_GPR_COUNT]) -> PhysicalRegisters {
    PhysicalRegisters {
        common_r0_r7: flat[0..8].try_into().expect("eight shared registers"),
        normal_r8_r12: flat[8..13].try_into().expect("five normal registers"),
        fiq_r8_r14: flat[13..20].try_into().expect("seven FIQ registers"),
        user_r13_r14: flat[20..22].try_into().expect("two user registers"),
        irq_r13_r14: flat[22..24].try_into().expect("two IRQ registers"),
        supervisor_r13_r14: flat[24..26].try_into().expect("two SVC registers"),
        abort_r13_r14: flat[26..28].try_into().expect("two abort registers"),
        undefined_r13_r14: flat[28..30].try_into().expect("two undefined registers"),
        pc: flat[30],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_is_derived_from_owned_banks() {
        assert_eq!(FLIP_FLOP_COUNT, 525_522);
        assert_eq!(
            FLIP_FLOP_COUNT,
            MEMORY_SIZE * 8 + 31 * 32 + 6 * 32 + 1 + 32 + 17
        );
    }

    #[test]
    fn flattening_round_trips_every_physical_bank() {
        let mut registers = PhysicalRegisters::default();
        let mut value = 1;
        for slot in &mut registers.common_r0_r7 {
            *slot = value;
            value += 1;
        }
        for slot in &mut registers.normal_r8_r12 {
            *slot = value;
            value += 1;
        }
        for slot in &mut registers.fiq_r8_r14 {
            *slot = value;
            value += 1;
        }
        for bank in [
            &mut registers.user_r13_r14,
            &mut registers.irq_r13_r14,
            &mut registers.supervisor_r13_r14,
            &mut registers.abort_r13_r14,
            &mut registers.undefined_r13_r14,
        ] {
            for slot in bank {
                *slot = value;
                value += 1;
            }
        }
        registers.pc = 31;
        assert_eq!(
            unflatten_registers(flatten_registers(&registers)),
            registers
        );
    }

    #[test]
    fn injected_mid_commit_failure_restores_every_stable_q_bit() {
        let mut cpu = Arm7TdmiGateLevel::new();
        cpu.load(&0xe3a0_0001u32.to_le_bytes(), 0, InstructionSet::Arm)
            .unwrap();
        let before = cpu.get_state();
        FAIL_MID_COMMIT.store(true, Ordering::SeqCst);
        assert_eq!(cpu.step(), Err(Arm7TdmiError::CommitFailed));
        assert_eq!(cpu.get_state(), before);
    }
}
