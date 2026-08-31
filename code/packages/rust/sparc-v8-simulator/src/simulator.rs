//! Top-level SPARC V8 simulator combining all components.
//!
//! Public API shape mirrors the repository's MIPS R2000 simulator
//! / `riscv_simulator::simulator::RiscVSimulator`: `new(memory_size)`,
//! public `regs`/`mem`/`pc`/`halted` fields, `load_program(&[u8])`,
//! `run(&[u8])`, `run_loaded_with_limit(max_steps)`, and
//! `step() -> String`.  Struct is named `SparcV8Simulator` following the
//! `sparc-v8-gatelevel::SparcCpu` naming precedent for this architecture
//! (both crates share the `SPARC`/`Sparc` capitalisation the gate-level
//! port already established in-tree).

use cpu_simulator::Memory;

use crate::decode;
use crate::encoding::assemble;
use crate::execute::{self, Psr};
use crate::registers::RegisterWindowFile;

/// Exact memory size of the educational Spec 07r machine.
pub const MEMORY_SIZE: usize = 65_536;

/// Complete owned SPARC V8 state at an instruction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparcState {
    pub pc: u32,
    pub npc: u32,
    pub regs: [u32; crate::registers::NUM_PHYS],
    pub cwp: u32,
    pub save_depth: u32,
    pub psr: Psr,
    pub y: u32,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

/// Typed checked lifecycle and execution failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SparcError {
    Halted,
    InvalidRegister {
        index: u32,
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
    DivideByZero {
        mnemonic: &'static str,
    },
    WindowOverflow {
        pc: u32,
    },
    Trap {
        condition: u32,
        pc: u32,
    },
    UnknownInstruction {
        raw: u32,
        pc: u32,
    },
}

impl std::fmt::Display for SparcError {
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
            Self::DivideByZero { mnemonic } => write!(f, "{mnemonic} by zero"),
            Self::WindowOverflow { pc } => write!(f, "register window overflow at {pc:#010x}"),
            Self::Trap { condition, pc } => write!(
                f,
                "Ticc condition {condition:#x} at {pc:#010x} is not the HALT sentinel"
            ),
            Self::UnknownInstruction { raw, pc } => {
                write!(f, "unknown instruction {raw:#010x} at {pc:#010x}")
            }
        }
    }
}

impl std::error::Error for SparcError {}

/// Complete checked instruction transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: u32,
    pub mnemonic: String,
    pub state_before: SparcState,
    pub state_after: SparcState,
}

/// Complete SPARC V8 behavioral simulator: a windowed register file
/// (8 globals + `NWINDOWS` x 16 outs/locals, see [`crate::registers`]),
/// PSR condition-code flags, the `Y` multiply/divide auxiliary register,
/// flat byte-addressable memory (big-endian), and a 32-bit PC.
pub struct SparcV8Simulator {
    /// Windowed register file (56 physical registers + CWP).
    pub regs: RegisterWindowFile,
    /// Flat byte-addressable memory, read/written big-endian by this
    /// crate's `execute` module.
    pub mem: Memory,
    /// PSR condition-code flags (N/Z/V/C).
    pub psr: Psr,
    /// `Y` register — multiply/divide auxiliary.
    pub y: u32,
    /// Program counter.
    pub pc: i32,
    /// Next program counter; maintained as PC+4 in this no-delay-slot model.
    pub npc: i32,
    /// True once `ta 0` (the HALT sentinel) or a fault (divide-by-zero,
    /// register-window overflow, or a non-`TA` `Ticc` trap) has
    /// executed.
    pub halted: bool,
    loaded_origin: u32,
    loaded_len: usize,
}

/// Observable outcome of a bounded simulator run.  Mirrors
/// `mips_r2000_simulator::simulator::ExecutionResult` field-for-field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: i32,
    pub final_state: SparcState,
    pub traces: Vec<StepTrace>,
}

impl SparcV8Simulator {
    /// Create a new simulator with the given memory size (in bytes).
    pub fn new(memory_size: usize) -> Self {
        let memory_size = if memory_size == 0 {
            MEMORY_SIZE
        } else {
            memory_size
        };
        Self {
            regs: RegisterWindowFile::new(),
            mem: Memory::new(memory_size),
            psr: Psr::default(),
            y: 0,
            pc: 0,
            npc: 4,
            halted: false,
            loaded_origin: 0,
            loaded_len: memory_size,
        }
    }

    /// Create the exact 64 KiB machine specified by Layer 07r.
    pub fn architectural() -> Self {
        Self::new(MEMORY_SIZE)
    }

    /// Reset registers, memory, PC/nPC, flags, Y, halt, and installed range.
    pub fn reset(&mut self) {
        let memory_size = self.mem.size();
        self.regs.reset();
        self.mem = Memory::new(memory_size);
        self.psr = Psr::default();
        self.y = 0;
        self.pc = 0;
        self.npc = 4;
        self.halted = false;
        self.loaded_origin = 0;
        self.loaded_len = memory_size;
    }

    /// Load a program (as raw big-endian bytes) into memory at address 0.
    pub fn load_program(&mut self, program: &[u8]) {
        if program.len() <= self.mem.size() {
            self.reset();
            self.mem.load_bytes(0, program);
            self.loaded_len = program.len();
        }
    }

    /// Return every register, memory byte, lifecycle field, and PC pipeline value.
    pub fn get_state(&self) -> SparcState {
        SparcState {
            pc: self.pc as u32,
            npc: self.npc as u32,
            regs: self.regs.physical_registers(),
            cwp: self.regs.cwp,
            save_depth: self.regs.save_depth,
            psr: self.psr,
            y: self.y,
            memory: (0..self.mem.size())
                .map(|index| self.mem.read_byte(index))
                .collect(),
            halted: self.halted,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    /// Atomically restore a validated complete state.
    pub fn restore(&mut self, state: &SparcState) -> Result<(), SparcError> {
        if state.memory.len() != self.mem.size() {
            return Err(SparcError::InvalidState(format!(
                "state memory has {} bytes; simulator requires {}",
                state.memory.len(),
                self.mem.size()
            )));
        }
        if state.regs[0] != 0 {
            return Err(SparcError::InvalidState(
                "R0 must remain hardwired to zero".to_string(),
            ));
        }
        if state.cwp >= crate::registers::NWINDOWS || state.save_depth >= crate::registers::NWINDOWS
        {
            return Err(SparcError::InvalidState(
                "register-window state is invalid".to_string(),
            ));
        }
        if state.pc as usize >= self.mem.size()
            || state.npc as usize >= self.mem.size()
            || state.pc & 3 != 0
            || state.npc != (state.pc + 4) % self.mem.size() as u32
        {
            return Err(SparcError::InvalidState(
                "PC/nPC pipeline state is invalid".to_string(),
            ));
        }
        let start = state.loaded_origin as usize;
        if state.loaded_origin & 3 != 0
            || start
                .checked_add(state.loaded_len)
                .is_none_or(|end| end > self.mem.size())
        {
            return Err(SparcError::InvalidState(
                "installed program range is invalid".to_string(),
            ));
        }
        let mut regs = RegisterWindowFile::new();
        regs.restore_physical(state.regs);
        regs.cwp = state.cwp;
        regs.save_depth = state.save_depth;
        let mut mem = Memory::new(state.memory.len());
        mem.load_bytes(0, &state.memory);
        self.regs = regs;
        self.mem = mem;
        self.psr = state.psr;
        self.y = state.y;
        self.pc = state.pc as i32;
        self.npc = state.npc as i32;
        self.halted = state.halted;
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
        Ok(())
    }

    /// Deterministically reset and load a program at address zero.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), SparcError> {
        self.load_at_checked(program, 0)
    }

    /// Deterministically reset and load a program at an aligned origin.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), SparcError> {
        if origin & 3 != 0 {
            return Err(SparcError::MisalignedProgram { origin });
        }
        let start = origin as usize;
        if start
            .checked_add(program.len())
            .is_none_or(|end| end > self.mem.size())
        {
            return Err(SparcError::ProgramOutOfRange {
                origin,
                length: program.len(),
                memory_size: self.mem.size(),
            });
        }
        self.reset();
        self.mem.load_bytes(start, program);
        self.pc = origin as i32;
        self.npc = ((origin + 4) % self.mem.size() as u32) as i32;
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        Ok(())
    }

    /// Read a logical register with a typed bounds check.
    pub fn read_register_checked(&self, index: u32) -> Result<u32, SparcError> {
        if index >= 32 {
            return Err(SparcError::InvalidRegister { index });
        }
        Ok(self.regs.read(index))
    }

    /// Write a logical register with a typed bounds check.
    pub fn write_register_checked(&mut self, index: u32, value: u32) -> Result<(), SparcError> {
        if index >= 32 {
            return Err(SparcError::InvalidRegister { index });
        }
        self.regs.write(index, value);
        Ok(())
    }

    /// Read one byte with a typed direct bounds check.
    pub fn read_byte_checked(&self, address: u32) -> Result<u8, SparcError> {
        if address as usize >= self.mem.size() {
            return Err(SparcError::MemoryOutOfRange { address, width: 1 });
        }
        Ok(self.mem.read_byte(address as usize))
    }

    /// Write one byte with a typed direct bounds check.
    pub fn write_byte_checked(&mut self, address: u32, value: u8) -> Result<(), SparcError> {
        if address as usize >= self.mem.size() {
            return Err(SparcError::MemoryOutOfRange { address, width: 1 });
        }
        self.mem.write_byte(address as usize, value);
        Ok(())
    }

    /// Read one aligned big-endian word with typed bounds checks.
    pub fn read_word_checked(&self, address: u32) -> Result<u32, SparcError> {
        self.check_direct(address, 4)?;
        Ok(execute::fetch_word_be(&self.mem, address as usize))
    }

    /// Write one aligned big-endian word with typed bounds checks.
    pub fn write_word_checked(&mut self, address: u32, value: u32) -> Result<(), SparcError> {
        self.check_direct(address, 4)?;
        for (offset, byte) in value.to_be_bytes().into_iter().enumerate() {
            self.mem.write_byte(address as usize + offset, byte);
        }
        Ok(())
    }

    fn check_direct(&self, address: u32, width: usize) -> Result<(), SparcError> {
        if address as usize & (width - 1) != 0 {
            return Err(SparcError::MisalignedAccess { address, width });
        }
        if (address as usize)
            .checked_add(width)
            .is_none_or(|end| end > self.mem.size())
        {
            return Err(SparcError::MemoryOutOfRange { address, width });
        }
        Ok(())
    }

    fn operand2(&self, decoded: &decode::DecodeResult) -> u32 {
        if decoded.fields.get("i").copied().unwrap_or(0) == 1 {
            decoded.fields.get("simm13").copied().unwrap_or(0) as u32
        } else {
            self.regs
                .read(decoded.fields.get("rs2").copied().unwrap_or(0) as u32)
        }
    }

    fn preflight(&self, raw: u32) -> Result<(), SparcError> {
        let decoded = decode::decode(raw);
        let pc = self.pc as u32;
        if decoded.mnemonic.starts_with("UNKNOWN")
            || decoded.mnemonic == "alu_unknown"
            || decoded.mnemonic == "mem_unknown"
        {
            return Err(SparcError::UnknownInstruction { raw, pc });
        }
        if (decoded.mnemonic == "udiv" || decoded.mnemonic == "udivcc")
            && self.operand2(&decoded) == 0
        {
            return Err(SparcError::DivideByZero { mnemonic: "UDIV" });
        }
        if (decoded.mnemonic == "sdiv" || decoded.mnemonic == "sdivcc")
            && self.operand2(&decoded) == 0
        {
            return Err(SparcError::DivideByZero { mnemonic: "SDIV" });
        }
        if decoded.mnemonic == "save" && self.regs.save_depth >= crate::registers::NWINDOWS - 1 {
            return Err(SparcError::WindowOverflow { pc });
        }
        if decoded.mnemonic == "ticc" {
            return Err(SparcError::Trap {
                condition: decoded.fields.get("cond").copied().unwrap_or(0) as u32,
                pc,
            });
        }
        let width = match decoded.mnemonic.as_str() {
            "ld" | "st" => 4,
            "lduh" | "ldsh" | "sth" => 2,
            "ldub" | "ldsb" | "stb" => 1,
            _ => 0,
        };
        if width > 1 {
            let rs1 = decoded.fields.get("rs1").copied().unwrap_or(0) as u32;
            let address =
                self.regs.read(rs1).wrapping_add(self.operand2(&decoded)) % self.mem.size() as u32;
            if address & (width as u32 - 1) != 0 {
                return Err(SparcError::MisalignedAccess { address, width });
            }
        }
        Ok(())
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
    /// A non-halting result means the budget was exhausted — this makes
    /// the execution limit visible to callers instead of silently
    /// treating an infinite loop as success.
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

        // Fetch — big-endian, per SPARC V8's default byte order.
        let raw = execute::fetch_word_be(&self.mem, self.pc as usize);

        // Decode
        let decoded = decode::decode(raw);
        let mnemonic = decoded.mnemonic.clone();

        // Execute
        let result = execute::execute(
            &decoded,
            &mut self.regs,
            &mut self.mem,
            &mut self.psr,
            &mut self.y,
            self.pc,
        );
        let memory_size = self.mem.size() as i32;
        self.pc = result.next_pc.rem_euclid(memory_size);
        self.npc = (self.pc + 4).rem_euclid(memory_size);
        self.halted = result.halted;

        mnemonic
    }

    /// Execute one instruction atomically with complete before/after state.
    pub fn step_checked(&mut self) -> Result<StepTrace, SparcError> {
        if self.halted {
            return Err(SparcError::Halted);
        }
        let pc_before = self.pc as u32;
        if pc_before & 3 != 0 {
            return Err(SparcError::MisalignedAccess {
                address: pc_before,
                width: 4,
            });
        }
        let start = self.loaded_origin as usize;
        let pc = pc_before as usize;
        if pc < start
            || pc
                .checked_add(4)
                .is_none_or(|end| end > start + self.loaded_len)
        {
            return Err(SparcError::TruncatedInstruction { pc: pc_before });
        }
        let raw = execute::fetch_word_be(&self.mem, pc);
        self.preflight(raw)?;
        let state_before = self.get_state();
        let mnemonic = self.step();
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
    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, SparcError> {
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
    ) -> Result<ExecutionResult, SparcError> {
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
    use crate::opcodes::*;
    use crate::registers::NWINDOWS;

    // Register-index constants for readability, in the virtual r0-r31
    // numbering (relative to the current window) -- mirror
    // `sparc_v8_simulator/state.py`'s `REG_*` aliases.
    const G0: u32 = 0;
    const G1: u32 = 1;
    const O0: u32 = 8;
    const O1: u32 = 9;
    const O2: u32 = 10;

    fn run_program(instructions: &[u32]) -> SparcV8Simulator {
        let mut sim = SparcV8Simulator::new(65536);
        sim.run_instructions(instructions);
        sim
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = SparcV8Simulator::new(65536);
        sim.load_program(&assemble(&[encode_add_imm(O0, G0, 42), encode_ta(0)]));
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(result.pc, 8);
    }

    // ── The trivial "load immediate into %o0 + halt" sequence the
    // sparc-v8-backend smoke test relies on: ADD %g0, 42, %o0; ta 0.
    #[test]
    fn load_immediate_then_halt() {
        let mut sim = SparcV8Simulator::new(65536);
        sim.load_program(&assemble(&[encode_add_imm(O0, G0, 42), encode_ta(0)]));
        let result = sim.run_loaded_with_limit(2);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.read(O0), 42);
        // ta 0 still advances pc by 4 (see execute.rs module docs) --
        // unlike mips-r2000-simulator's SYSCALL, which leaves pc
        // unchanged at the halting instruction.
        assert_eq!(result.pc, 8);
    }

    #[test]
    fn g0_is_hardwired_zero() {
        let sim = run_program(&[encode_add_imm(G0, G0, 42), encode_ta(0)]);
        assert_eq!(sim.regs.read(G0), 0);
    }

    #[test]
    fn test_add_sub() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 10),
            encode_add_imm(O1, G0, 20),
            encode_add(O2, O0, O1),
            encode_sub(G1, O0, O1),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(O2), 30);
        assert_eq!(sim.regs.read(G1) as i32, -10);
    }

    #[test]
    fn test_addcc_sets_flags() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 5),
            encode_addcc_imm(O1, O0, -5), // 5 + (-5) = 0
            encode_ta(0),
        ]);
        assert!(sim.psr.z);
        assert_eq!(sim.regs.read(O1), 0);
    }

    #[test]
    fn test_logic_ops() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 0xFF),
            encode_add_imm(O1, G0, 0x0F),
            encode_and(2, O0, O1),
            encode_or(3, O0, O1),
            encode_xor(4, O0, O1),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0x0F);
        assert_eq!(sim.regs.read(3), 0xFF);
        assert_eq!(sim.regs.read(4), 0xF0);
    }

    #[test]
    fn test_shifts() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 1),
            encode_sll(2, O0, 4),
            encode_add_imm(O1, G0, -1),
            encode_srl(3, O1, 28),
            encode_sra(4, O1, 2),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 16);
        assert_eq!(sim.regs.read(3), 0x0F);
        assert_eq!(sim.regs.read(4) as i32, -1);
    }

    #[test]
    fn test_sethi_upper_bits() {
        let sim = run_program(&[encode_sethi(O0, 0x1234), encode_ta(0)]);
        assert_eq!(sim.regs.read(O0), 0x1234 << 10);
    }

    #[test]
    fn test_umul_smul() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, -1), // 0xFFFFFFFF
            encode_add_imm(O1, G0, 2),
            encode_umul(2, O0, O1),
            encode_add_imm(3, G0, 0),
            encode_rdy(3),
            encode_smul(4, O0, O1),
            encode_ta(0),
        ]);
        // unsigned: 0xFFFFFFFF * 2 = 0x1_FFFFFFFE
        assert_eq!(sim.regs.read(2), 0xFFFF_FFFE);
        assert_eq!(sim.regs.read(3), 1);
        // signed: -1 * 2 = -2
        assert_eq!(sim.regs.read(4) as i32, -2);
    }

    #[test]
    fn test_udiv_sdiv() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 20),
            encode_add_imm(O1, G0, 6),
            encode_udiv(2, O0, O1),
            encode_add_imm(O0, G0, -20),
            // SDIV's dividend is Y:rs1 -- real SPARC code must sign-extend
            // the dividend into %y before a 32-bit signed divide.  SRA by
            // 31 broadcasts O0's sign bit; WRY (XOR with %g0) copies it
            // into Y.
            encode_sra(O2, O0, 31),
            encode_wry(O2, G0),
            encode_sdiv(3, O0, O1),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 3);
        assert_eq!(sim.regs.read(3) as i32, -3);
    }

    #[test]
    fn test_cc_multiply_encodings_update_nz_and_clear_vc() {
        let unsigned = run_program(&[
            encode_add_imm(O0, G0, -1),
            encode_add_imm(O1, G0, 2),
            encode_umulcc(2, O0, O1),
            encode_ta(0),
        ]);
        assert_eq!(unsigned.regs.read(2), 0xffff_fffe);
        assert!(unsigned.psr.n);
        assert!(!unsigned.psr.z);
        assert!(!unsigned.psr.v);
        assert!(!unsigned.psr.c);

        let signed_zero = run_program(&[
            encode_add_imm(O1, G0, 2),
            encode_smulcc(2, G0, O1),
            encode_ta(0),
        ]);
        assert_eq!(signed_zero.regs.read(2), 0);
        assert!(!signed_zero.psr.n);
        assert!(signed_zero.psr.z);
        assert!(!signed_zero.psr.v);
        assert!(!signed_zero.psr.c);
    }

    #[test]
    fn test_cc_divide_sets_manual_overflow_and_never_panics() {
        let ordinary = run_program(&[
            encode_add_imm(O0, G0, 20),
            encode_add_imm(O1, G0, 6),
            encode_udivcc(2, O0, O1),
            encode_ta(0),
        ]);
        assert_eq!(ordinary.regs.read(2), 3);
        assert!(!ordinary.psr.n);
        assert!(!ordinary.psr.z);
        assert!(!ordinary.psr.v);
        assert!(!ordinary.psr.c);

        let unsigned_overflow = run_program(&[
            encode_add_imm(O2, G0, 1),
            encode_wry(O2, G0),
            encode_add_imm(O1, G0, 1),
            encode_udivcc(2, G0, O1),
            encode_ta(0),
        ]);
        assert_eq!(unsigned_overflow.regs.read(2), u32::MAX);
        assert!(unsigned_overflow.psr.n);
        assert!(unsigned_overflow.psr.v);
        assert!(!unsigned_overflow.psr.c);

        // Y:rs1 is i64::MIN and the divisor is -1. Rust's native signed
        // division would overflow here; SPARC saturates to i32::MAX and sets V.
        let positive_overflow = run_program(&[
            encode_sethi(O2, 0x20_0000),
            encode_wry(O2, G0),
            encode_add_imm(O1, G0, -1),
            encode_sdivcc(2, G0, O1),
            encode_ta(0),
        ]);
        assert_eq!(positive_overflow.regs.read(2), i32::MAX as u32);
        assert!(!positive_overflow.psr.n);
        assert!(positive_overflow.psr.v);

        let negative_overflow = run_program(&[
            encode_add_imm(O2, G0, -1),
            encode_wry(O2, G0),
            encode_add_imm(O1, G0, 1),
            encode_sdivcc(2, G0, O1),
            encode_ta(0),
        ]);
        assert_eq!(negative_overflow.regs.read(2), i32::MIN as u32);
        assert!(negative_overflow.psr.n);
        assert!(negative_overflow.psr.v);
        assert!(!negative_overflow.psr.c);
    }

    #[test]
    fn test_udiv_by_zero_halts() {
        let mut sim = SparcV8Simulator::new(65536);
        sim.run_instructions(&[
            encode_add_imm(O0, G0, 10),
            encode_udiv(2, O0, G0),
            encode_ta(0),
        ]);
        assert!(sim.halted);
    }

    #[test]
    fn test_sw_lw() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 0x100),
            encode_add_imm(O1, G0, 0x42),
            encode_st(O1, O0, 0),
            encode_ld(2, O0, 0),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0x42);
    }

    #[test]
    fn test_stb_ldub_ldsb_sign() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 0x200),
            encode_add_imm(O1, G0, 0xFF),
            encode_stb(O1, O0, 0),
            encode_ldub(2, O0, 0),
            encode_ldsb(3, O0, 0),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0xFF);
        assert_eq!(sim.regs.read(3) as i32, -1);
    }

    #[test]
    fn test_sth_lduh_big_endian() {
        let mut sim = SparcV8Simulator::new(65536);
        sim.run_instructions(&[
            encode_add_imm(O0, G0, 0x200),
            encode_add_imm(O1, G0, 0x0678), // within the 13-bit signed imm range
            encode_sth(O1, O0, 0),
            encode_lduh(2, O0, 0),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0x0678);
        // Big-endian: high byte at the lower address.
        assert_eq!(sim.mem.read_byte(0x200), 0x06);
        assert_eq!(sim.mem.read_byte(0x201), 0x78);
    }

    // ── Branches ──
    #[test]
    fn test_be_taken() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 5),
            encode_subcc_imm(O1, O0, 5), // Z=1
            encode_bicc(COND_BE, 2),     // taken: skip the next instruction
            encode_add_imm(2, G0, 999),
            encode_add_imm(3, G0, 42),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_bne_not_taken() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 5),
            encode_subcc_imm(O1, O0, 5), // Z=1
            encode_bicc(COND_BNE, 3),    // not taken
            encode_add_imm(2, G0, 42),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 42);
    }

    #[test]
    fn test_branch_backward_loop() {
        // for (o0 = 0; o0 != 3; o0++) {}
        let sim = run_program(&[
            encode_add_imm(O0, G0, 0),
            encode_add_imm(O1, G0, 3),
            encode_add_imm(O0, O0, 1), // loop:
            encode_subcc(O2, O0, O1),  // O0 - O1 -> Z when equal
            encode_bicc(COND_BNE, -2),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(O0), 3);
    }

    // ── CALL / JMPL ──
    #[test]
    fn test_call_sets_o7_and_jumps() {
        // main (word 0): CALL sub (word index 2, byte 8)
        //                ta 0                       (word 1, byte 4 -- skipped)
        // sub  (word 2, byte 8): add %g0,7,%o0 ; ta 0
        let sim = run_program(&[
            encode_call(2),
            encode_ta(0),
            encode_add_imm(O0, G0, 7),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(15), 0); // %o7 = call's own pc (word 0)
        assert_eq!(sim.regs.read(O0), 7);
    }

    #[test]
    fn test_jmpl_absolute_jump() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 12), // target byte address
            encode_jmpl(15, O0, 0),
            encode_add_imm(2, G0, 999),
            encode_add_imm(3, G0, 42),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    // ── SAVE / RESTORE ──
    #[test]
    fn test_save_restore_round_trip() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 99), // %o0 = 99 in window 0
            encode_save(14, O0, 0),     // %sp' = %o0 + 0 = 99; rotates CWP
            encode_restore(14, G0, 0),  // rotates CWP back
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.cwp, 0);
        assert_eq!(sim.regs.read(O0), 99);
    }

    #[test]
    fn test_save_aliases_outs_to_ins_of_new_window() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 99), // %o0 = 99 (window 0)
            encode_save(14, G0, 0),     // rotate; %sp' = 0
            encode_ta(0),
        ]);
        // %i0 (virt 24) in the post-SAVE window aliases %o0 of window 0.
        assert_eq!(sim.regs.read(24), 99);
    }

    #[test]
    fn test_save_overflow_halts() {
        let mut instrs = Vec::new();
        for _ in 0..NWINDOWS {
            instrs.push(encode_save(14, G0, 0));
        }
        instrs.push(encode_ta(0));
        let sim = run_program(&instrs);
        assert!(sim.halted);
    }

    #[test]
    fn test_step_mnemonics() {
        let mut sim = SparcV8Simulator::new(65536);
        let prog = assemble(&[
            encode_add_imm(O0, G0, 1),
            encode_add_imm(O1, G0, 2),
            encode_ta(0),
        ]);
        sim.load_program(&prog);
        assert_eq!(sim.step(), "add");
        assert_eq!(sim.regs.read(O0), 1);
        assert_eq!(sim.step(), "add");
        assert_eq!(sim.regs.read(O1), 2);
    }

    // ── Integration: sum 1..=10 ──
    #[test]
    fn test_sum_loop() {
        // o0 = i = 1; o1 = sum = 0; o2 = 11
        // loop: o1 += o0; o0 += 1; subcc o3,o0,o2; bne loop
        let sim = run_program(&[
            encode_add_imm(O0, G0, 1),
            encode_add_imm(O1, G0, 0),
            encode_add_imm(O2, G0, 11),
            encode_add(O1, O1, O0), // loop:
            encode_add_imm(O0, O0, 1),
            encode_subcc(3, O0, O2),
            encode_bicc(COND_BNE, -3),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(O1), 55);
    }

    #[test]
    fn test_assemble_is_big_endian() {
        let bytes = assemble(&[0x1234_5678]);
        assert_eq!(bytes, vec![0x12, 0x34, 0x56, 0x78]);
    }
}
