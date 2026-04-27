//! # IR Types — operands, instructions, data declarations, and programs.
//!
//! This module defines the core data structures of the IR:
//!
//! - `IrOperand` — an operand to an IR instruction (register, immediate, label)
//! - `IrInstruction` — a single IR instruction with opcode + operands + ID
//! - `IrDataDecl` — a data segment declaration (label, size, init byte)
//! - `IrProgram` — a complete IR program (instructions + data + entry label)
//! - `IdGenerator` — a monotonic counter for assigning unique instruction IDs
//!
//! ## Why unique instruction IDs?
//!
//! Every instruction carries a monotonically-increasing integer `id`. This
//! ID is the key that connects the instruction to the source map chain:
//!
//! ```text
//! Source: "+" at line 1, column 3
//!   → AST node #42
//!   → IR instructions [#7, #8, #9, #10]  (LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE)
//!   → machine code bytes [0x20..0x28]
//! ```
//!
//! Without stable IDs, the source map chain would break every time an
//! optimiser reorders or rewrites instructions.
//!
//! ## Register naming
//!
//! Virtual registers are named `v0`, `v1`, `v2`, ... where the number is
//! the `index` field of `IrOperand::Register`. There are infinitely many —
//! the backend's register allocator maps them to physical registers.

use std::fmt;
use crate::opcodes::IrOp;

// ===========================================================================
// IrOperand — an operand to an IR instruction
// ===========================================================================

/// An operand to an IR instruction.
///
/// Three kinds of operand exist:
///
/// | Variant       | Example source  | Display |
/// |---------------|-----------------|---------|
/// | `Register(0)` | virtual reg v0  | `"v0"`  |
/// | `Immediate(42)` | literal 42    | `"42"`  |
/// | `Label("loop_0_start")` | jump target | `"loop_0_start"` |
///
/// Registers are "virtual" — there are infinitely many and they are mapped
/// to physical registers by the backend. Immediates are signed 64-bit
/// integers. Labels are UTF-8 strings that name jump targets or data regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrOperand {
    /// A virtual register, e.g. `v0`, `v1`, `v5`.
    ///
    /// # Example
    /// ```
    /// use compiler_ir::types::IrOperand;
    /// let r = IrOperand::Register(3);
    /// assert_eq!(r.to_string(), "v3");
    /// ```
    Register(usize),

    /// A literal integer value embedded in the instruction.
    ///
    /// # Example
    /// ```
    /// use compiler_ir::types::IrOperand;
    /// let imm = IrOperand::Immediate(255);
    /// assert_eq!(imm.to_string(), "255");
    /// ```
    Immediate(i64),

    /// A named jump target or data label.
    ///
    /// # Example
    /// ```
    /// use compiler_ir::types::IrOperand;
    /// let lbl = IrOperand::Label("loop_0_start".to_string());
    /// assert_eq!(lbl.to_string(), "loop_0_start");
    /// ```
    Label(String),
}

impl fmt::Display for IrOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrOperand::Register(idx) => write!(f, "v{}", idx),
            IrOperand::Immediate(val) => write!(f, "{}", val),
            IrOperand::Label(name) => write!(f, "{}", name),
        }
    }
}

// ===========================================================================
// IrInstruction — a single IR instruction
// ===========================================================================

/// A single IR instruction.
///
/// Every instruction has:
/// - `opcode`: what operation to perform (`ADD_IMM`, `BRANCH_Z`, etc.)
/// - `operands`: the arguments (registers, immediates, labels)
/// - `id`: a unique monotonic integer for source mapping
///
/// The `id` field is the key that connects this instruction to the source
/// map chain. Labels use id = -1 since they produce no machine code.
///
/// # Examples
///
/// ```text
/// { opcode: AddImm, operands: [v1, v1, 1], id: 3 }
///   →  ADD_IMM v1, v1, 1  ; #3
///
/// { opcode: BranchZ, operands: [v2, loop_0_end], id: 7 }
///   →  BRANCH_Z v2, loop_0_end  ; #7
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrInstruction {
    /// The opcode for this instruction.
    pub opcode: IrOp,
    /// The operands for this instruction (registers, immediates, labels).
    pub operands: Vec<IrOperand>,
    /// Unique monotonic ID. Labels use -1 (they produce no machine code).
    pub id: i64,
}

impl IrInstruction {
    /// Create a new instruction with the given opcode, operands, and ID.
    pub fn new(opcode: IrOp, operands: Vec<IrOperand>, id: i64) -> Self {
        IrInstruction { opcode, operands, id }
    }
}

// ===========================================================================
// IrDataDecl — a data segment declaration
// ===========================================================================

/// A data segment declaration: a named region of memory.
///
/// Declares a named region of memory with a given size and initial byte
/// value. For Brainfuck, this is the tape:
///
/// ```text
/// IrDataDecl { label: "tape", size: 30000, init: 0 }
///   →  .data tape 30000 0
/// ```
///
/// The `init` value is repeated for every byte in the region.
/// `init = 0` means zero-initialized (equivalent to `.bss` in most formats).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrDataDecl {
    /// The name of the data region (e.g., `"tape"`).
    pub label: String,
    /// The number of bytes to allocate.
    pub size: usize,
    /// The byte value used to initialize every cell.
    pub init: u8,
}

// ===========================================================================
// IrProgram — a complete IR program
// ===========================================================================

/// A complete IR program.
///
/// Contains:
/// - `instructions`: the linear sequence of IR instructions
/// - `data`: data segment declarations (`.bss`, `.data`)
/// - `entry_label`: the label where execution begins
/// - `version`: IR version number (1 = Brainfuck subset)
///
/// The `instructions` slice is ordered — execution flows from index 0
/// to `len - 1`, with jumps and branches altering the flow.
///
/// # Example
///
/// ```
/// use compiler_ir::types::IrProgram;
/// let prog = IrProgram::new("_start");
/// assert_eq!(prog.entry_label, "_start");
/// assert_eq!(prog.version, 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrProgram {
    /// Linear sequence of IR instructions.
    pub instructions: Vec<IrInstruction>,
    /// Data segment declarations.
    pub data: Vec<IrDataDecl>,
    /// Label where execution begins.
    pub entry_label: String,
    /// IR version (1 = v1 Brainfuck subset).
    pub version: u32,
}

impl IrProgram {
    /// Create a new, empty IR program with the given entry label and version 1.
    ///
    /// # Example
    ///
    /// ```
    /// use compiler_ir::types::IrProgram;
    /// let prog = IrProgram::new("_start");
    /// assert_eq!(prog.entry_label, "_start");
    /// assert_eq!(prog.version, 1);
    /// assert!(prog.instructions.is_empty());
    /// assert!(prog.data.is_empty());
    /// ```
    pub fn new(entry_label: &str) -> Self {
        IrProgram {
            instructions: Vec::new(),
            data: Vec::new(),
            entry_label: entry_label.to_string(),
            version: 1,
        }
    }

    /// Append an instruction to the program.
    ///
    /// # Example
    ///
    /// ```
    /// use compiler_ir::types::{IrProgram, IrInstruction, IrOperand};
    /// use compiler_ir::opcodes::IrOp;
    /// let mut prog = IrProgram::new("_start");
    /// prog.add_instruction(IrInstruction::new(
    ///     IrOp::Halt, vec![], 0
    /// ));
    /// assert_eq!(prog.instructions.len(), 1);
    /// ```
    pub fn add_instruction(&mut self, instr: IrInstruction) {
        self.instructions.push(instr);
    }

    /// Append a data declaration to the program.
    ///
    /// # Example
    ///
    /// ```
    /// use compiler_ir::types::{IrProgram, IrDataDecl};
    /// let mut prog = IrProgram::new("_start");
    /// prog.add_data(IrDataDecl { label: "tape".to_string(), size: 30000, init: 0 });
    /// assert_eq!(prog.data.len(), 1);
    /// ```
    pub fn add_data(&mut self, decl: IrDataDecl) {
        self.data.push(decl);
    }
}

// ===========================================================================
// IdGenerator — produces unique monotonic instruction IDs
// ===========================================================================

/// Produces unique monotonic instruction IDs.
///
/// Every IR instruction in the pipeline needs a unique ID for source
/// mapping. `IdGenerator` ensures no two instructions ever share an ID.
///
/// # Usage
///
/// ```
/// use compiler_ir::types::IdGenerator;
/// let mut gen = IdGenerator::new();
/// assert_eq!(gen.next(), 0);
/// assert_eq!(gen.next(), 1);
/// assert_eq!(gen.next(), 2);
/// assert_eq!(gen.current(), 3); // next value to be returned
/// ```
#[derive(Debug, Clone)]
pub struct IdGenerator {
    next: i64,
}

impl IdGenerator {
    /// Create a new `IdGenerator` starting at 0.
    pub fn new() -> Self {
        IdGenerator { next: 0 }
    }

    /// Create a new `IdGenerator` starting at `start`.
    ///
    /// This is useful when multiple compilers contribute instructions to the
    /// same program and IDs must not collide.
    pub fn from_start(start: i64) -> Self {
        IdGenerator { next: start }
    }

    /// Return the next unique ID and increment the counter.
    pub fn next(&mut self) -> i64 {
        let id = self.next;
        self.next += 1;
        id
    }

    /// Return the current counter value without incrementing.
    ///
    /// This is the ID that will be returned by the next call to `next()`.
    pub fn current(&self) -> i64 {
        self.next
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes::IrOp;

    // ── IrOperand Display ─────────────────────────────────────────────────

    #[test]
    fn test_register_display() {
        assert_eq!(IrOperand::Register(0).to_string(), "v0");
        assert_eq!(IrOperand::Register(5).to_string(), "v5");
        assert_eq!(IrOperand::Register(100).to_string(), "v100");
    }

    #[test]
    fn test_immediate_display() {
        assert_eq!(IrOperand::Immediate(42).to_string(), "42");
        assert_eq!(IrOperand::Immediate(-1).to_string(), "-1");
        assert_eq!(IrOperand::Immediate(0).to_string(), "0");
        assert_eq!(IrOperand::Immediate(255).to_string(), "255");
    }

    #[test]
    fn test_label_display() {
        assert_eq!(IrOperand::Label("_start".to_string()).to_string(), "_start");
        assert_eq!(IrOperand::Label("loop_0_end".to_string()).to_string(), "loop_0_end");
        assert_eq!(IrOperand::Label("tape".to_string()).to_string(), "tape");
    }

    // ── IrInstruction ─────────────────────────────────────────────────────

    #[test]
    fn test_instruction_new() {
        let instr = IrInstruction::new(
            IrOp::Halt,
            vec![],
            0,
        );
        assert_eq!(instr.opcode, IrOp::Halt);
        assert!(instr.operands.is_empty());
        assert_eq!(instr.id, 0);
    }

    #[test]
    fn test_instruction_with_operands() {
        let instr = IrInstruction::new(
            IrOp::AddImm,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(1),
                IrOperand::Immediate(1),
            ],
            3,
        );
        assert_eq!(instr.opcode, IrOp::AddImm);
        assert_eq!(instr.operands.len(), 3);
        assert_eq!(instr.id, 3);
    }

    // ── IrDataDecl ────────────────────────────────────────────────────────

    #[test]
    fn test_data_decl() {
        let decl = IrDataDecl {
            label: "tape".to_string(),
            size: 30000,
            init: 0,
        };
        assert_eq!(decl.label, "tape");
        assert_eq!(decl.size, 30000);
        assert_eq!(decl.init, 0);
    }

    // ── IrProgram ─────────────────────────────────────────────────────────

    #[test]
    fn test_new_program() {
        let prog = IrProgram::new("_start");
        assert_eq!(prog.entry_label, "_start");
        assert_eq!(prog.version, 1);
        assert!(prog.instructions.is_empty());
        assert!(prog.data.is_empty());
    }

    #[test]
    fn test_add_instruction() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
        assert_eq!(prog.instructions.len(), 1);
        assert_eq!(prog.instructions[0].opcode, IrOp::Halt);
    }

    #[test]
    fn test_add_data() {
        let mut prog = IrProgram::new("_start");
        prog.add_data(IrDataDecl { label: "tape".to_string(), size: 30000, init: 0 });
        assert_eq!(prog.data.len(), 1);
        assert_eq!(prog.data[0].label, "tape");
        assert_eq!(prog.data[0].size, 30000);
    }

    #[test]
    fn test_multiple_instructions() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadAddr,
            vec![IrOperand::Register(0), IrOperand::Label("tape".to_string())],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(0)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        assert_eq!(prog.instructions.len(), 3);
    }

    // ── IdGenerator ───────────────────────────────────────────────────────

    #[test]
    fn test_id_generator_starts_at_zero() {
        let mut gen = IdGenerator::new();
        assert_eq!(gen.next(), 0);
    }

    #[test]
    fn test_id_generator_increments() {
        let mut gen = IdGenerator::new();
        assert_eq!(gen.next(), 0);
        assert_eq!(gen.next(), 1);
        assert_eq!(gen.next(), 2);
    }

    #[test]
    fn test_id_generator_current() {
        let mut gen = IdGenerator::new();
        assert_eq!(gen.current(), 0);
        gen.next();
        assert_eq!(gen.current(), 1);
        gen.next();
        assert_eq!(gen.current(), 2);
    }

    #[test]
    fn test_id_generator_from_start() {
        let mut gen = IdGenerator::from_start(100);
        assert_eq!(gen.next(), 100);
        assert_eq!(gen.next(), 101);
        assert_eq!(gen.current(), 102);
    }

    #[test]
    fn test_id_generator_unique_ids() {
        let mut gen = IdGenerator::new();
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let id = gen.next();
            assert!(ids.insert(id), "duplicate ID: {}", id);
        }
    }

    // ── IrOperand Equality ────────────────────────────────────────────────

    #[test]
    fn test_operand_equality() {
        assert_eq!(IrOperand::Register(0), IrOperand::Register(0));
        assert_ne!(IrOperand::Register(0), IrOperand::Register(1));
        assert_eq!(IrOperand::Immediate(42), IrOperand::Immediate(42));
        assert_ne!(IrOperand::Immediate(42), IrOperand::Immediate(43));
        assert_eq!(
            IrOperand::Label("tape".to_string()),
            IrOperand::Label("tape".to_string())
        );
    }
}
