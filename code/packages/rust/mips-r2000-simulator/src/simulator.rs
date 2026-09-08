//! Top-level MIPS R2000 simulator combining all components.
//!
//! The compatibility API provides `new(memory_size)`, public
//! `regs`/`mem`/`pc`/`halted` fields,
//! `load_program(&[u8])`, `run(&[u8])`, `run_loaded_with_limit(max_steps)`,
//! and `step() -> String`.

use cpu_simulator::{Memory, RegisterFile};

use crate::decode;
use crate::encoding::assemble;
use crate::execute;
use crate::execute::read_word_be;

/// Exact memory size of the educational Spec 07q machine.
pub const MEMORY_SIZE: usize = 65_536;

/// Complete owned MIPS R2000 state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MipsState {
    pub pc: u32,
    pub regs: [u32; 32],
    pub hi: u32,
    pub lo: u32,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

/// Typed fail-closed lifecycle and execution errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MipsError {
    Halted,
    InvalidRegister {
        index: usize,
    },
    InvalidState(String),
    MisalignedProgram {
        origin: u32,
    },
    ProgramOutOfRange {
        origin: u32,
        length: usize,
        memory_size: usize,
    },
    TruncatedInstruction {
        pc: u32,
    },
    MemoryOutOfRange {
        address: u32,
        width: usize,
    },
    MisalignedAccess {
        address: u32,
        width: usize,
    },
    Break {
        pc: u32,
    },
    DivideByZero {
        mnemonic: &'static str,
    },
    SignedOverflow {
        mnemonic: &'static str,
    },
    UnknownInstruction {
        raw: u32,
        pc: u32,
    },
}

impl std::fmt::Display for MipsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Halted => f.write_str("CPU is halted"),
            Self::InvalidRegister { index } => {
                write!(f, "register index {index} is outside R0-R31")
            }
            Self::InvalidState(message) => f.write_str(message),
            Self::MisalignedProgram { origin } => {
                write!(f, "program origin {origin:#010x} is not word-aligned")
            }
            Self::ProgramOutOfRange {
                origin,
                length,
                memory_size,
            } => write!(
                f,
                "program of {length} bytes at {origin:#010x} exceeds {memory_size}-byte memory"
            ),
            Self::TruncatedInstruction { pc } => write!(
                f,
                "instruction at {pc:#010x} crosses the loaded program boundary"
            ),
            Self::MemoryOutOfRange { address, width } => {
                write!(f, "{width}-byte access at {address:#010x} exceeds memory")
            }
            Self::MisalignedAccess { address, width } => {
                write!(f, "misaligned {width}-byte access at {address:#010x}")
            }
            Self::Break { pc } => write!(f, "BREAK instruction at {pc:#010x}"),
            Self::DivideByZero { mnemonic } => write!(f, "{mnemonic} by zero"),
            Self::SignedOverflow { mnemonic } => write!(f, "{mnemonic} signed overflow"),
            Self::UnknownInstruction { raw, pc } => {
                write!(f, "unknown instruction {raw:#010x} at {pc:#010x}")
            }
        }
    }
}

impl std::error::Error for MipsError {}

/// Complete checked instruction transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: u32,
    pub mnemonic: String,
    pub state_before: MipsState,
    pub state_after: MipsState,
}

/// Complete MIPS R2000 simulator: 32 GPRs (R0 hardwired zero), HI/LO,
/// flat byte-addressable memory (big-endian), and a 32-bit PC.
pub struct MipsR2000Simulator {
    /// 32 general-purpose registers.  `R0` reads as zero and discards
    /// writes (`RegisterFile::new(32, true)`).
    pub regs: RegisterFile,
    /// Flat byte-addressable memory, read/written big-endian by this
    /// crate's `execute` module.
    pub mem: Memory,
    /// High word of `MULT`/`MULTU` results and the remainder of
    /// `DIV`/`DIVU`.
    pub hi: u32,
    /// Low word of `MULT`/`MULTU` results and the quotient of
    /// `DIV`/`DIVU`.
    pub lo: u32,
    /// Program counter.
    pub pc: i32,
    /// True once `SYSCALL` (the HALT sentinel) or a fault (`BREAK`,
    /// signed-overflow `ADD`/`ADDI`/`SUB`, or divide-by-zero) has executed.
    pub halted: bool,
    loaded_origin: u32,
    loaded_len: usize,
}

/// Observable outcome of a bounded simulator run.  Mirrors
/// `riscv_simulator::simulator::ExecutionResult` field-for-field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: i32,
    pub final_state: MipsState,
    pub traces: Vec<StepTrace>,
}

impl MipsR2000Simulator {
    /// Create a new simulator with the given memory size (in bytes).
    pub fn new(memory_size: usize) -> Self {
        let memory_size = if memory_size == 0 {
            MEMORY_SIZE
        } else {
            memory_size
        };
        Self {
            regs: RegisterFile::new(32, true),
            mem: Memory::new(memory_size),
            hi: 0,
            lo: 0,
            pc: 0,
            halted: false,
            loaded_origin: 0,
            loaded_len: memory_size,
        }
    }

    /// Create the exact 64 KiB machine specified by Layer 07q.
    pub fn architectural() -> Self {
        Self::new(MEMORY_SIZE)
    }

    /// Reset registers, memory, PC, HI/LO, halt, and installed range.
    pub fn reset(&mut self) {
        let memory_size = self.mem.size();
        self.regs = RegisterFile::new(32, true);
        self.mem = Memory::new(memory_size);
        self.hi = 0;
        self.lo = 0;
        self.pc = 0;
        self.halted = false;
        self.loaded_origin = 0;
        self.loaded_len = memory_size;
    }

    /// Load a program (as raw big-endian bytes) into memory at address 0.
    pub fn load_program(&mut self, program: &[u8]) {
        self.mem.load_bytes(0, program);
        self.loaded_origin = 0;
        self.loaded_len = program.len().min(self.mem.size());
    }

    /// Run until halted or 10000 steps (safety limit).
    pub fn run(&mut self, program: &[u8]) {
        self.load_program(program);
        self.run_loaded_with_limit(10000);
    }

    /// Run instructions from an already-loaded program.
    pub fn run_loaded(&mut self) {
        self.run_loaded_with_limit(10000);
    }

    /// Run the already-loaded program for at most `max_steps` instructions.
    ///
    /// A non-halting result means the budget was exhausted — this makes the
    /// execution limit visible to callers instead of silently treating an
    /// infinite loop (e.g. a `JR $ra` whose `$ra` was never set) as success.
    pub fn run_loaded_with_limit(&mut self, max_steps: usize) -> ExecutionResult {
        let mut steps = 0;
        while steps < max_steps {
            if self.halted {
                break;
            }
            self.step();
            steps += 1;
        }
        ExecutionResult {
            halted: self.halted,
            steps,
            pc: self.pc,
            final_state: self.get_state(),
            traces: Vec::new(),
        }
    }

    /// Execute a single instruction and return its mnemonic.
    pub fn step(&mut self) -> String {
        if self.halted {
            return "halted".to_string();
        }

        // Fetch — big-endian, per MIPS R2000's default byte order.
        let raw = read_word_be(&self.mem, self.pc as usize);

        // Decode
        let decoded = decode::decode(raw, self.pc);
        let mnemonic = decoded.mnemonic.clone();

        // Execute
        let result = execute::execute(
            &decoded,
            &mut self.regs,
            &mut self.mem,
            &mut self.hi,
            &mut self.lo,
            self.pc,
        );
        self.pc = result.next_pc.rem_euclid(self.mem.size() as i32);
        self.halted = result.halted;

        mnemonic
    }

    /// Return every register, memory byte, special register, and lifecycle bit.
    pub fn get_state(&self) -> MipsState {
        MipsState {
            pc: self.pc as u32,
            regs: std::array::from_fn(|index| self.regs.read(index)),
            hi: self.hi,
            lo: self.lo,
            memory: (0..self.mem.size())
                .map(|index| self.mem.read_byte(index))
                .collect(),
            halted: self.halted,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    /// Atomically restore a validated complete state.
    pub fn restore(&mut self, state: &MipsState) -> Result<(), MipsError> {
        if state.memory.len() != self.mem.size() {
            return Err(MipsError::InvalidState(format!(
                "state memory has {} bytes; simulator requires {}",
                state.memory.len(),
                self.mem.size()
            )));
        }
        if state.regs[0] != 0 {
            return Err(MipsError::InvalidState(
                "R0 must remain hardwired to zero".to_string(),
            ));
        }
        if state.pc as usize >= self.mem.size() {
            return Err(MipsError::InvalidState(format!(
                "PC {:#010x} exceeds memory",
                state.pc
            )));
        }
        let start = state.loaded_origin as usize;
        let end = start.checked_add(state.loaded_len);
        if state.loaded_origin & 3 != 0 || end.is_none_or(|value| value > self.mem.size()) {
            return Err(MipsError::InvalidState(
                "installed program range is invalid".to_string(),
            ));
        }
        self.regs = RegisterFile::new(32, true);
        for (index, value) in state.regs.iter().copied().enumerate() {
            self.regs.write(index, value);
        }
        self.mem = Memory::new(state.memory.len());
        self.mem.load_bytes(0, &state.memory);
        self.hi = state.hi;
        self.lo = state.lo;
        self.pc = state.pc as i32;
        self.halted = state.halted;
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
        Ok(())
    }

    /// Deterministically reset and load a program at address zero.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), MipsError> {
        self.load_at_checked(program, 0)
    }

    /// Deterministically reset and load a program at an explicit origin.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), MipsError> {
        if origin & 3 != 0 {
            return Err(MipsError::MisalignedProgram { origin });
        }
        let start = origin as usize;
        let end = start.checked_add(program.len());
        if end.is_none_or(|value| value > self.mem.size()) {
            return Err(MipsError::ProgramOutOfRange {
                origin,
                length: program.len(),
                memory_size: self.mem.size(),
            });
        }
        self.reset();
        self.mem.load_bytes(start, program);
        self.pc = origin as i32;
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        Ok(())
    }

    /// Read a register with a typed bounds check.
    pub fn read_register_checked(&self, index: usize) -> Result<u32, MipsError> {
        if index >= 32 {
            return Err(MipsError::InvalidRegister { index });
        }
        Ok(self.regs.read(index))
    }

    /// Write a register with a typed bounds check; writes to R0 are discarded.
    pub fn write_register_checked(&mut self, index: usize, value: u32) -> Result<(), MipsError> {
        if index >= 32 {
            return Err(MipsError::InvalidRegister { index });
        }
        self.regs.write(index, value);
        Ok(())
    }

    /// Read one byte with a typed direct bounds check.
    pub fn read_byte_checked(&self, address: u32) -> Result<u8, MipsError> {
        let address = address as usize;
        if address >= self.mem.size() {
            return Err(MipsError::MemoryOutOfRange {
                address: address as u32,
                width: 1,
            });
        }
        Ok(self.mem.read_byte(address))
    }

    /// Write one byte with a typed direct bounds check.
    pub fn write_byte_checked(&mut self, address: u32, value: u8) -> Result<(), MipsError> {
        let address = address as usize;
        if address >= self.mem.size() {
            return Err(MipsError::MemoryOutOfRange {
                address: address as u32,
                width: 1,
            });
        }
        self.mem.write_byte(address, value);
        Ok(())
    }

    /// Read one aligned big-endian word with typed bounds checks.
    pub fn read_word_checked(&self, address: u32) -> Result<u32, MipsError> {
        if address & 3 != 0 {
            return Err(MipsError::MisalignedAccess { address, width: 4 });
        }
        if (address as usize)
            .checked_add(4)
            .is_none_or(|end| end > self.mem.size())
        {
            return Err(MipsError::MemoryOutOfRange { address, width: 4 });
        }
        Ok(read_word_be(&self.mem, address as usize))
    }

    /// Write one aligned big-endian word with typed bounds checks.
    pub fn write_word_checked(&mut self, address: u32, value: u32) -> Result<(), MipsError> {
        if address & 3 != 0 {
            return Err(MipsError::MisalignedAccess { address, width: 4 });
        }
        if (address as usize)
            .checked_add(4)
            .is_none_or(|end| end > self.mem.size())
        {
            return Err(MipsError::MemoryOutOfRange { address, width: 4 });
        }
        for (offset, byte) in value.to_be_bytes().into_iter().enumerate() {
            self.mem.write_byte(address as usize + offset, byte);
        }
        Ok(())
    }

    fn preflight(&self, raw: u32) -> Result<(), MipsError> {
        let op = (raw >> 26) & 0x3f;
        let rs = ((raw >> 21) & 0x1f) as usize;
        let rt = ((raw >> 16) & 0x1f) as usize;
        let funct = raw & 0x3f;
        let simm = (raw as i16) as i32;
        let pc = self.pc as u32;

        const VALID_R: &[u32] = &[
            0x00, 0x02, 0x03, 0x04, 0x06, 0x07, 0x08, 0x09, 0x0c, 0x0d, 0x10, 0x11, 0x12, 0x13,
            0x18, 0x19, 0x1a, 0x1b, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2a, 0x2b,
        ];
        const VALID_OP: &[u32] = &[
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x28, 0x29, 0x2a, 0x2b, 0x2e,
        ];
        if !VALID_OP.contains(&op) || (op == 0 && !VALID_R.contains(&funct)) {
            return Err(MipsError::UnknownInstruction { raw, pc });
        }
        if op == 1 && ![0, 1, 0x10, 0x11].contains(&rt) {
            return Err(MipsError::UnknownInstruction { raw, pc });
        }
        if op == 0 && funct == 0x0d {
            return Err(MipsError::Break { pc });
        }
        if op == 0 && (funct == 0x1a || funct == 0x1b) && self.regs.read(rt) == 0 {
            return Err(MipsError::DivideByZero {
                mnemonic: if funct == 0x1a { "DIV" } else { "DIVU" },
            });
        }
        if op == 0 && (funct == 0x20 || funct == 0x22) {
            let a = self.regs.read(rs) as i32;
            let b = self.regs.read(rt) as i32;
            let overflow = if funct == 0x20 {
                a.checked_add(b).is_none()
            } else {
                a.checked_sub(b).is_none()
            };
            if overflow {
                return Err(MipsError::SignedOverflow {
                    mnemonic: if funct == 0x20 { "ADD" } else { "SUB" },
                });
            }
        }
        if op == 0x08 && (self.regs.read(rs) as i32).checked_add(simm).is_none() {
            return Err(MipsError::SignedOverflow { mnemonic: "ADDI" });
        }
        let width = match op {
            0x21 | 0x25 | 0x29 => 2,
            0x23 | 0x2b => 4,
            _ => 1,
        };
        if width > 1 {
            let address = self.regs.read(rs).wrapping_add(simm as u32) % self.mem.size() as u32;
            if address & (width as u32 - 1) != 0 {
                return Err(MipsError::MisalignedAccess { address, width });
            }
        }
        Ok(())
    }

    /// Execute one instruction atomically with complete before/after state.
    pub fn step_checked(&mut self) -> Result<StepTrace, MipsError> {
        if self.halted {
            return Err(MipsError::Halted);
        }
        let pc_before = self.pc as u32;
        let start = self.loaded_origin as usize;
        let pc = pc_before as usize;
        if pc < start
            || pc
                .checked_add(4)
                .is_none_or(|end| end > start + self.loaded_len)
        {
            return Err(MipsError::TruncatedInstruction { pc: pc_before });
        }
        let raw = read_word_be(&self.mem, pc);
        self.preflight(raw)?;
        let state_before = self.get_state();
        let mnemonic = self.step();
        if raw >> 26 == 0 && raw & 0x3f == 0x0c {
            self.pc = ((pc_before + 4) % self.mem.size() as u32) as i32;
        }
        let state_after = self.get_state();
        Ok(StepTrace {
            pc_before,
            pc_after: self.pc as u32,
            raw,
            mnemonic,
            state_before,
            state_after,
        })
    }

    /// Execute already-loaded code transactionally for at most `max_steps`.
    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, MipsError> {
        let original = self.get_state();
        let mut traces = Vec::new();
        while traces.len() < max_steps && !self.halted {
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.restore(&original)?;
                    return Err(error);
                }
            }
        }
        Ok(ExecutionResult {
            halted: self.halted,
            steps: traces.len(),
            pc: self.pc,
            final_state: self.get_state(),
            traces,
        })
    }

    /// Deterministically load and execute a program transactionally.
    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, MipsError> {
        let original = self.get_state();
        self.load_checked(program)?;
        match self.run_loaded_checked(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore(&original)?;
                Err(error)
            }
        }
    }

    /// Run a list of instruction words (convenience for tests).
    pub fn run_instructions(&mut self, instructions: &[u32]) {
        let program = assemble(instructions);
        self.run(&program);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::*;

    // Register-index constants for readability — mirror the psABI names
    // documented in `code/specs/07q-mips-r2000-simulator.md` /
    // `mips_r2000_simulator/state.py`.
    const ZERO: u32 = 0;
    const V0: u32 = 2;
    const T0: u32 = 8;
    const T1: u32 = 9;
    const T2: u32 = 10;
    const RA: u32 = 31;

    fn run_program(instructions: &[u32]) -> MipsR2000Simulator {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.run_instructions(instructions);
        sim
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.load_program(&assemble(&[encode_addiu(T0, ZERO, 42), encode_syscall()]));
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(result.pc, 4);
    }

    // ── the trivial "load immediate + jump-register-return" sequence ──
    // the mips-r2000-backend smoke test relies on: ADDIU $v0, $zero, 42;
    // JR $ra.  $ra was never set (starts at 0), so JR loops back to
    // address 0 — we only assert the register state after exactly two
    // steps, not that the program halts.
    #[test]
    fn load_immediate_then_jump_register_return() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.load_program(&assemble(&[encode_addiu(V0, ZERO, 42), encode_jr(RA)]));
        let result = sim.run_loaded_with_limit(2);
        assert!(!result.halted, "JR is not a halt instruction");
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.read(V0 as usize), 42);
        assert_eq!(
            result.pc, 0,
            "JR $ra jumps back to address 0 since $ra was never set"
        );
    }

    // ── ALU ops ──
    #[test]
    fn test_add_sub() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 10),
            encode_addiu(T1, ZERO, 20),
            encode_add(T2, T0, T1),
            encode_sub(3, T0, T1),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T2 as usize), 30);
        assert_eq!(sim.regs.read(3) as i32, -10);
    }

    #[test]
    fn test_add_overflow_halts_without_writing_rd() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.run_instructions(&[
            encode_lui(T0, 0x7FFF),
            encode_ori(T0, T0, 0xFFFF), // T0 = i32::MAX
            encode_addiu(T1, ZERO, 1),
            encode_addiu(3, ZERO, 999),
            encode_add(3, T0, T1), // overflow: i32::MAX + 1
            encode_syscall(),
        ]);
        assert!(sim.halted);
        // rd (register 3) must NOT have been overwritten by the faulting ADD.
        assert_eq!(sim.regs.read(3), 999);
    }

    #[test]
    fn test_addu_wraps_without_halting() {
        let sim = run_program(&[
            encode_lui(T0, 0xFFFF),
            encode_ori(T0, T0, 0xFFFF), // T0 = 0xFFFFFFFF (-1 as u32)
            encode_addiu(T1, ZERO, 1),
            encode_addu(T2, T0, T1), // wraps to 0
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T2 as usize), 0);
    }

    #[test]
    fn test_and_or_xor_nor() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0xFF),
            encode_addiu(T1, ZERO, 0x0F),
            encode_and(2, T0, T1),
            encode_or(3, T0, T1),
            encode_xor(4, T0, T1),
            encode_nor(5, ZERO, ZERO),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0x0F);
        assert_eq!(sim.regs.read(3), 0xFF);
        assert_eq!(sim.regs.read(4), 0xF0);
        assert_eq!(sim.regs.read(5), 0xFFFF_FFFF);
    }

    #[test]
    fn test_slt_sltu() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, (-5i32) as u16 as i32),
            encode_addiu(T1, ZERO, 3),
            encode_slt(2, T0, T1),
            encode_slt(3, T1, T0),
            encode_sltu(4, T0, T1), // unsigned(-5) is huge, not < 3
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 1);
        assert_eq!(sim.regs.read(3), 0);
        assert_eq!(sim.regs.read(4), 0);
    }

    #[test]
    fn test_shifts() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 1),
            encode_sll(2, T0, 4),
            encode_addiu(T1, ZERO, (-1i32) as u16 as i32),
            encode_srl(3, T1, 28),
            encode_addiu(T1, ZERO, (-16i32) as u16 as i32),
            encode_sra(4, T1, 2),
            encode_addiu(T0, ZERO, 2),
            encode_sllv(5, 2, T0), // (1<<4)<<2 = 64
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 16);
        assert_eq!(sim.regs.read(3), 0x0F);
        assert_eq!(sim.regs.read(4) as i32, -4);
        assert_eq!(sim.regs.read(5), 64);
    }

    #[test]
    fn test_mult_multu() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, (-1i32) as u16 as i32),
            encode_addiu(T1, ZERO, 2),
            encode_mult(T0, T1),
            encode_mflo(2),
            encode_mfhi(3),
            encode_multu(T0, T1),
            encode_mflo(4),
            encode_mfhi(5),
            encode_syscall(),
        ]);
        // signed: -1 * 2 = -2 -> LO=0xFFFFFFFE, HI=0xFFFFFFFF (sign-extended product)
        assert_eq!(sim.regs.read(2), 0xFFFF_FFFE);
        assert_eq!(sim.regs.read(3), 0xFFFF_FFFF);
        // unsigned: 0xFFFFFFFF * 2 = 0x1_FFFFFFFE
        assert_eq!(sim.regs.read(4), 0xFFFF_FFFE);
        assert_eq!(sim.regs.read(5), 1);
    }

    #[test]
    fn test_div() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, -20i32 & 0xFFFF),
            encode_addiu(T1, ZERO, 6),
            encode_div(T0, T1),
            encode_mflo(2), // quotient
            encode_mfhi(3), // remainder
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2) as i32, -3);
        assert_eq!(sim.regs.read(3) as i32, -2);
    }

    #[test]
    fn test_divu() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 20),
            encode_addiu(T1, ZERO, 6),
            encode_divu(T0, T1),
            encode_mflo(2), // quotient
            encode_mfhi(3), // remainder
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 3);
        assert_eq!(sim.regs.read(3), 2);
    }

    #[test]
    fn test_divu_by_zero_halts() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.run_instructions(&[
            encode_addiu(T0, ZERO, 10),
            encode_divu(T0, ZERO),
            encode_syscall(),
        ]);
        assert!(sim.halted);
    }

    // ── Loads & stores ──
    #[test]
    fn test_sw_lw() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0x100),
            encode_addiu(T1, ZERO, 0x42),
            encode_sw(T1, T0, 0),
            encode_lw(2, T0, 0),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0x42);
    }

    #[test]
    fn test_sb_lbu_lb_sign() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0x200),
            encode_addiu(T1, ZERO, 0xFF),
            encode_sb(T1, T0, 0),
            encode_lbu(2, T0, 0),
            encode_lb(3, T0, 0),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0xFF);
        assert_eq!(sim.regs.read(3) as i32, -1);
    }

    #[test]
    fn test_sh_lhu_big_endian() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.run_instructions(&[
            encode_addiu(T0, ZERO, 0x200),
            encode_addiu(T1, ZERO, 0x1234),
            encode_sh(T1, T0, 0),
            encode_lhu(2, T0, 0),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0x1234);
        // Big-endian: high byte at the lower address.
        assert_eq!(sim.mem.read_byte(0x200), 0x12);
        assert_eq!(sim.mem.read_byte(0x201), 0x34);
    }

    // ── Branches ──
    #[test]
    fn test_beq_taken() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 5),
            encode_addiu(T1, ZERO, 5),
            encode_beq(T0, T1, 1), // skip the next instruction
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_bne_not_taken() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 5),
            encode_addiu(T1, ZERO, 5),
            encode_bne(T0, T1, 2),
            encode_addiu(2, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 42);
    }

    #[test]
    fn test_blez_bgtz() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0),
            encode_blez(T0, 1),
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_addiu(T1, ZERO, 5),
            encode_bgtz(T1, 1),
            encode_addiu(4, ZERO, 999),
            encode_addiu(5, ZERO, 43),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
        assert_eq!(sim.regs.read(4), 0);
        assert_eq!(sim.regs.read(5), 43);
    }

    #[test]
    fn test_bltz_bgez() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, (-1i32) as u16 as i32),
            encode_bltz(T0, 1),
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_branch_backward_loop() {
        // for (t0 = 0; t0 != 3; t0++) {}
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0),
            encode_addiu(T1, ZERO, 3),
            encode_addiu(T0, T0, 1),
            encode_bne(T0, T1, -2),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T0 as usize), 3);
    }

    // ── Jumps ──
    #[test]
    fn test_j() {
        let sim = run_program(&[
            encode_j(3), // jump to word index 3 == byte address 12
            encode_addiu(2, ZERO, 999),
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_jal_jr_call_return() {
        // main: JAL sub; ADDIU $3,$zero,42; SYSCALL
        // sub (word index 3, byte 12): ADDIU $2,$zero,7; JR $ra
        let sim = run_program(&[
            encode_jal(3),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
            encode_addiu(2, ZERO, 7),
            encode_jr(RA),
        ]);
        assert_eq!(sim.regs.read(RA as usize), 4);
        assert_eq!(sim.regs.read(2), 7);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_jalr() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 12),
            encode_jalr(RA, T0),
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(RA as usize), 8);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    // ── I-type arithmetic / logic ──
    #[test]
    fn test_addi() {
        let sim = run_program(&[
            encode_addi(T0, ZERO, 42),
            encode_addi(T1, T0, 10),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T0 as usize), 42);
        assert_eq!(sim.regs.read(T1 as usize), 52);
    }

    #[test]
    fn test_andi_ori_xori_zero_extend() {
        let sim = run_program(&[
            encode_lui(T0, 0xFFFF),
            encode_ori(T0, T0, 0xFFFF), // T0 = 0xFFFFFFFF
            encode_andi(2, T0, 0x00FF),
            encode_ori(3, ZERO, 0x1234),
            encode_xori(4, T0, 0xFFFF),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0x00FF);
        assert_eq!(sim.regs.read(3), 0x1234);
        assert_eq!(sim.regs.read(4), 0xFFFF_0000);
    }

    #[test]
    fn test_lui() {
        let sim = run_program(&[encode_lui(T0, 0x1234), encode_syscall()]);
        assert_eq!(sim.regs.read(T0 as usize), 0x1234_0000);
    }

    // ── R0 hardwired ──
    #[test]
    fn test_r0_hardwired() {
        let sim = run_program(&[encode_addiu(ZERO, ZERO, 42), encode_syscall()]);
        assert_eq!(sim.regs.read(0), 0);
    }

    #[test]
    fn test_break_halts() {
        let sim = run_program(&[encode_break()]);
        assert!(sim.halted);
    }

    #[test]
    fn test_step_mnemonics() {
        let mut sim = MipsR2000Simulator::new(65536);
        let prog = assemble(&[
            encode_addiu(T0, ZERO, 1),
            encode_addiu(T1, ZERO, 2),
            encode_syscall(),
        ]);
        sim.load_program(&prog);
        assert_eq!(sim.step(), "addiu");
        assert_eq!(sim.regs.read(T0 as usize), 1);
        assert_eq!(sim.step(), "addiu");
        assert_eq!(sim.regs.read(T1 as usize), 2);
    }

    // ── Integration: sum 1..=10 ──
    #[test]
    fn test_sum_loop() {
        // t0 = i = 1; t1 = sum = 0; t2 = 11
        // loop: t1 += t0; t0 += 1; bne t0, t2, loop
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 1),
            encode_addiu(T1, ZERO, 0),
            encode_addiu(T2, ZERO, 11),
            encode_addu(T1, T1, T0), // loop:
            encode_addiu(T0, T0, 1),
            encode_bne(T0, T2, -3),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T1 as usize), 55);
    }

    // ── Encode-decode round trip ──
    #[test]
    fn test_round_trip() {
        let cases: Vec<(&str, u32)> = vec![
            ("sll", encode_sll(1, 2, 5)),
            ("srl", encode_srl(1, 2, 5)),
            ("sra", encode_sra(1, 2, 5)),
            ("sllv", encode_sllv(1, 2, 3)),
            ("srlv", encode_srlv(1, 2, 3)),
            ("srav", encode_srav(1, 2, 3)),
            ("jr", encode_jr(31)),
            ("jalr", encode_jalr(31, 2)),
            ("syscall", encode_syscall()),
            ("break", encode_break()),
            ("mfhi", encode_mfhi(1)),
            ("mthi", encode_mthi(1)),
            ("mflo", encode_mflo(1)),
            ("mtlo", encode_mtlo(1)),
            ("mult", encode_mult(1, 2)),
            ("multu", encode_multu(1, 2)),
            ("div", encode_div(1, 2)),
            ("divu", encode_divu(1, 2)),
            ("add", encode_add(1, 2, 3)),
            ("addu", encode_addu(1, 2, 3)),
            ("sub", encode_sub(1, 2, 3)),
            ("subu", encode_subu(1, 2, 3)),
            ("and", encode_and(1, 2, 3)),
            ("or", encode_or(1, 2, 3)),
            ("xor", encode_xor(1, 2, 3)),
            ("nor", encode_nor(1, 2, 3)),
            ("slt", encode_slt(1, 2, 3)),
            ("sltu", encode_sltu(1, 2, 3)),
            ("bltz", encode_bltz(1, 8)),
            ("bgez", encode_bgez(1, 8)),
            ("bltzal", encode_bltzal(1, 8)),
            ("bgezal", encode_bgezal(1, 8)),
            ("j", encode_j(100)),
            ("jal", encode_jal(100)),
            ("beq", encode_beq(1, 2, 8)),
            ("bne", encode_bne(1, 2, 8)),
            ("blez", encode_blez(1, 8)),
            ("bgtz", encode_bgtz(1, 8)),
            ("addi", encode_addi(1, 2, 42)),
            ("addiu", encode_addiu(1, 2, 42)),
            ("slti", encode_slti(1, 2, -5)),
            ("sltiu", encode_sltiu(1, 2, 5)),
            ("andi", encode_andi(1, 2, 0xFF)),
            ("ori", encode_ori(1, 2, 0xFF)),
            ("xori", encode_xori(1, 2, 0xFF)),
            ("lui", encode_lui(1, 0x1234)),
            ("lb", encode_lb(1, 2, 4)),
            ("lh", encode_lh(1, 2, 4)),
            ("lw", encode_lw(1, 2, 4)),
            ("lbu", encode_lbu(1, 2, 4)),
            ("lhu", encode_lhu(1, 2, 4)),
            ("sb", encode_sb(1, 2, 4)),
            ("sh", encode_sh(1, 2, 4)),
            ("sw", encode_sw(1, 2, 4)),
        ];
        for (name, encoded) in &cases {
            let result = decode::decode(*encoded, 0);
            assert_eq!(result.mnemonic, *name, "decode(0x{encoded:08x}) failed");
        }
    }

    #[test]
    fn test_assemble_is_big_endian() {
        let bytes = assemble(&[0x1234_5678]);
        assert_eq!(bytes, vec![0x12, 0x34, 0x56, 0x78]);
    }
}
