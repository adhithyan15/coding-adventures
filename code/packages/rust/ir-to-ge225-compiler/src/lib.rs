//! GE-225 three-pass code generator.
//!
//! # Overview
//!
//! This crate translates a target-independent [`IrProgram`] into a flat binary
//! image of GE-225 20-bit machine words. The GE-225 is the 1960-era General
//! Electric mainframe that ran Dartmouth's BASIC time-sharing system in 1964.
//!
//! # Architecture summary
//!
//! The GE-225 is a *word-addressed accumulator machine*:
//!
//! - Memory: 20-bit signed integers, addressed from 0.
//! - Accumulator (A): the sole arithmetic register. All computation routes through it.
//! - Q register: the lower half of the 40-bit double-word used by multiply/divide.
//! - N register: 6-bit typewriter code latch (loaded via SAN, printed via TYP).
//!
//! Every IR virtual register maps to a *spill slot* — a dedicated memory word
//! in the data segment. A three-register ADD lowered to GE-225 looks like:
//!
//! ```text
//! LDA [vA]      ; A = spill[vA]
//! ADD [vB]      ; A = A + spill[vB]
//! STA [vDst]    ; spill[vDst] = A
//! ```
//!
//! # Memory layout
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │ addr 0           : TON (prologue)         │
//! │ addr 1 … code_end-1 : IR code words       │
//! │ addr code_end    : BRU code_end (halt)    │
//! │ addr data_base … : spill slots (v0…vN)   │
//! │ addr …           : constants table        │
//! └──────────────────────────────────────────┘
//! ```
//!
//! - `data_base = code_end + 1`
//! - `spill_addr(N) = data_base + N`
//! - `const_addr(K) = data_base + n_regs + K`
//!
//! # Halt convention
//!
//! The GE-225 has no HALT instruction. We use a self-loop stub:
//! ```text
//! code_end: BRU code_end   ; spin forever
//! ```
//! The `HALT` IR opcode emits `BRU code_end`. The integration layer detects
//! halt by checking `trace.address == halt_address` after each `step()`.
//!
//! # Conditional branches
//!
//! GE-225 conditional branches (BZE/BNZ/BMI/BOD) use **inhibit** semantics:
//! the named condition *prevents* the skip; when the condition is FALSE the
//! next word is skipped.  Equivalently: "the BRU executes when the condition
//! is TRUE".
//!
//! | Instruction | Inhibit condition | BRU executes when… |
//! |-------------|-------------------|--------------------|
//! | `BZE`       | A == 0            | A == 0             |
//! | `BNZ`       | A ≠ 0             | A ≠ 0              |
//! | `BMI`       | A < 0             | A < 0              |
//! | `BOD`       | A is odd          | A is odd           |
//!
//! Far conditional jumps therefore require a two-word pair after the load.
//! For `BRANCH_Z` (jump when A == 0) use `BZE`:
//!
//! ```text
//! LDA  spill(vN)
//! BZE              ; inhibited-by-zero → skip BRU when A ≠ 0
//! BRU  target      ; executed only when A == 0
//! ```
//!
//! # Example
//!
//! ```
//! use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
//! use ir_to_ge225_compiler::{compile_to_ge225, validate_for_ge225};
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(
//!     IrOp::LoadImm,
//!     vec![IrOperand::Register(0), IrOperand::Immediate(42)],
//!     1,
//! ));
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
//!
//! assert!(validate_for_ge225(&prog).is_empty());
//! let result = compile_to_ge225(&prog).unwrap();
//! assert_eq!(result.binary.len() % 3, 0);  // 3 bytes per GE-225 word
//! assert_eq!(result.data_base, result.halt_address + 1);
//! ```

use std::collections::HashMap;

use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
use coding_adventures_ge225_simulator::{assemble_fixed, assemble_shift, encode_instruction, pack_words};

pub mod codegen;

// ---------------------------------------------------------------------------
// GE-225 base memory-reference opcodes (5-bit field, octal)
// ---------------------------------------------------------------------------

/// A = mem[ea]
const OP_LDA: i32 = 0o00;
/// A = A + mem[ea]
const OP_ADD: i32 = 0o01;
/// A = A - mem[ea]
const OP_SUB: i32 = 0o02;
/// mem[ea] = A
const OP_STA: i32 = 0o03;
/// A,Q = Q × mem[ea] + A  (40-bit accumulate multiply; reserved for future MUL opcode)
#[allow(dead_code)]
const OP_MPY: i32 = 0o15;
/// A = (A,Q) ÷ mem[ea],  Q = remainder  (reserved for future DIV opcode)
#[allow(dead_code)]
const OP_DVD: i32 = 0o16;
/// PC = ea  (unconditional branch)
const OP_BRU: i32 = 0o26;

// ---------------------------------------------------------------------------
// GE-225 word-size constraints
// ---------------------------------------------------------------------------
//
// The GE-225 uses 20-bit two's-complement words.
// Signed range: -524 288 (−2^19) to 524 287 (2^19 − 1).

/// Minimum value that fits in a 20-bit signed GE-225 word.
const GE225_WORD_MIN: i64 = -(1 << 19); // -524 288
/// Maximum value that fits in a 20-bit signed GE-225 word.
const GE225_WORD_MAX: i64 = (1 << 19) - 1; // 524 287

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Output of [`compile_to_ge225`].
///
/// Contains everything downstream code needs to load the program into a
/// GE-225 simulator and run it:
///
/// - `binary` — packed binary image (3 bytes per 20-bit word, big-endian).
/// - `halt_address` — word address of the halt stub (`BRU halt_address`).
/// - `data_base` — first data-segment word address (`= halt_address + 1`).
/// - `label_map` — maps IR label names → resolved code-word addresses.
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// Packed GE-225 binary image (3 bytes per 20-bit word, big-endian).
    pub binary: Vec<u8>,
    /// Word address of the halt stub (`BRU halt_address`). Spin here forever.
    pub halt_address: usize,
    /// First data-segment word address (`= halt_address + 1`).
    pub data_base: usize,
    /// Maps IR label names to their resolved code-word addresses.
    pub label_map: HashMap<String, usize>,
}

/// Returned by [`compile_to_ge225`] when the IR program cannot be translated.
///
/// Causes include unsupported IR opcodes, `AND_IMM` with a non-1 immediate, a
/// constant that does not fit in a GE-225 20-bit signed word, or a branch to
/// an undefined label.
#[derive(Debug, Clone)]
pub struct CodeGenError(pub String);

impl std::fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CodeGenError {}

// ---------------------------------------------------------------------------
// Pre-flight validator
// ---------------------------------------------------------------------------

/// The V1 GE-225 backend supports exactly these IR opcodes.
fn is_supported_opcode(op: IrOp) -> bool {
    matches!(
        op,
        IrOp::Label
            | IrOp::Comment
            | IrOp::Nop
            | IrOp::Halt
            | IrOp::LoadImm
            | IrOp::AddImm
            | IrOp::Add
            | IrOp::Sub
            | IrOp::Mul
            | IrOp::Div
            | IrOp::AndImm
            | IrOp::CmpEq
            | IrOp::CmpNe
            | IrOp::CmpLt
            | IrOp::CmpGt
            | IrOp::Jump
            | IrOp::BranchZ
            | IrOp::BranchNz
            | IrOp::Syscall
    )
}

/// Inspect `program` for GE-225 backend incompatibilities without generating
/// any code.
///
/// # Validation rules
///
/// 1. **Opcode support** — every opcode must be in the V1 supported set.
///    Opcodes absent (e.g. `LoadByte`, `StoreWord`, `Call`) are rejected
///    immediately so the caller gets a precise diagnostic.
///
/// 2. **Constant range** — every `IrOperand::Immediate` in `LoadImm` or
///    `AddImm` must fit in a GE-225 20-bit signed word (−524 288 to 524 287).
///    Values that overflow would be silently truncated by the 0xFFFFF mask.
///
/// 3. **SYSCALL number** — only `SYSCALL 1` (print character) is wired up
///    in the V1 backend. Any other syscall number is rejected.
///
/// 4. **AND_IMM immediate** — only `imm == 1` is supported; the GE-225 uses
///    the BOD (branch-if-odd) instruction for the parity/odd-bit test and has
///    no general bitwise-AND instruction.
///
/// Returns a list of human-readable error strings. An empty list means the
/// program is compatible with the GE-225 V1 backend.
///
/// # Example
///
/// ```
/// use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
/// use ir_to_ge225_compiler::validate_for_ge225;
///
/// let mut prog = IrProgram::new("_start");
/// prog.add_instruction(IrInstruction::new(
///     IrOp::LoadImm,
///     vec![IrOperand::Register(0), IrOperand::Immediate(1_000_000)],
///     1,
/// ));
/// let errors = validate_for_ge225(&prog);
/// assert!(!errors.is_empty());
/// assert!(errors[0].contains("overflow"));
/// ```
pub fn validate_for_ge225(program: &IrProgram) -> Vec<String> {
    let mut errors = Vec::new();

    for instr in &program.instructions {
        let op = instr.opcode;

        // Rule 1: opcode must be in the V1 supported set.
        if !is_supported_opcode(op) {
            errors.push(format!(
                "unsupported opcode {} in V1 GE-225 backend",
                op
            ));
            continue; // no point checking operands of an unsupported opcode
        }

        match op {
            // Rule 2: constant range on LOAD_IMM and ADD_IMM.
            IrOp::LoadImm | IrOp::AddImm => {
                for operand in &instr.operands {
                    if let IrOperand::Immediate(v) = operand {
                        if *v < GE225_WORD_MIN || *v > GE225_WORD_MAX {
                            errors.push(format!(
                                "{}: constant {} overflows GE-225 20-bit signed word \
                                 (valid range {} to {})",
                                op, v, GE225_WORD_MIN, GE225_WORD_MAX
                            ));
                        }
                    }
                }
            }

            // Rule 3: SYSCALL number must be 1.
            IrOp::Syscall => {
                let bad = instr.operands.iter().find_map(|o| {
                    if let IrOperand::Immediate(v) = o {
                        if *v != 1 { Some(*v) } else { None }
                    } else {
                        None
                    }
                });
                if let Some(n) = bad {
                    errors.push(format!(
                        "unsupported SYSCALL {}: only SYSCALL 1 (print char) \
                         is wired in the V1 GE-225 backend",
                        n
                    ));
                }
            }

            // Rule 4: AND_IMM must use immediate 1.
            IrOp::AndImm => {
                for operand in &instr.operands {
                    if let IrOperand::Immediate(v) = operand {
                        if *v != 1 {
                            errors.push(format!(
                                "unsupported AND_IMM immediate {}: \
                                 only AND_IMM 1 is supported (odd-bit test)",
                                v
                            ));
                            break;
                        }
                    }
                }
            }

            _ => {}
        }
    }

    errors
}

// ---------------------------------------------------------------------------
// Public compilation function
// ---------------------------------------------------------------------------

/// Compile an `IrProgram` to a GE-225 binary image.
///
/// This is a three-pass process:
///
/// - **Pre-flight**: [`validate_for_ge225`] inspects every instruction.
///   If any violation is found a [`CodeGenError`] is raised before any code
///   is generated.
/// - **Pass 0**: scan all instructions to collect virtual register indices and
///   build the constants table (unique integer values for `LoadImm` and
///   non-trivial `AddImm` immediates).
/// - **Pass 1**: walk the instruction list to compute each instruction's
///   code-word address; record label addresses in the label map.
/// - **Pass 2**: emit GE-225 machine words using the fully resolved addresses.
///
/// # Errors
///
/// Returns [`CodeGenError`] if pre-flight validation fails, or if a branch
/// target label is undefined during code generation.
pub fn compile_to_ge225(program: &IrProgram) -> Result<CompileResult, CodeGenError> {
    let errors = validate_for_ge225(program);
    if !errors.is_empty() {
        let joined = errors.join("; ");
        return Err(CodeGenError(format!(
            "IR program failed GE-225 pre-flight validation ({} error{}): {}",
            errors.len(),
            if errors.len() == 1 { "" } else { "s" },
            joined
        )));
    }
    CodeGen::new(program).compile()
}

// ---------------------------------------------------------------------------
// Internal three-pass assembler
// ---------------------------------------------------------------------------

/// Three-pass GE-225 assembler (internal).
///
/// ## Pass 0 — `pass0`
/// Scan every operand to find the maximum virtual register index (which
/// determines how many spill slots are needed) and to assign a consecutive
/// index to each unique constant value referenced by `LoadImm` and non-trivial
/// `AddImm` instructions.
///
/// ## Pass 1 — `pass1`
/// Walk the instruction list, computing the GE-225 word count for each IR
/// instruction and accumulating a `word_addr` counter. Labels record their
/// address in `label_map` at zero cost. After pass 1, `code_end` and
/// `data_base` are known.
///
/// ## Pass 2 — `pass2`
/// Walk the instruction list again, emitting the exact GE-225 words for each
/// instruction using the addresses computed in pass 1.
struct CodeGen<'a> {
    program: &'a IrProgram,
    /// Highest virtual register index seen in any operand. Determines spill size.
    max_reg: usize,
    /// Maps constant value → insertion-order index. Preserves order via `const_order`.
    const_index: HashMap<i64, usize>,
    /// Constants in insertion order (parallel to `const_index`).
    const_order: Vec<i64>,
    /// Maps IR label name → resolved code-word address (set in pass 1).
    label_map: HashMap<String, usize>,
    /// Word address immediately past all code words (set in pass 1).
    code_end: usize,
    /// First data-segment word address = `code_end + 1` (set in pass 1).
    data_base: usize,
}

impl<'a> CodeGen<'a> {
    fn new(program: &'a IrProgram) -> Self {
        Self {
            program,
            max_reg: 0,
            const_index: HashMap::new(),
            const_order: Vec::new(),
            label_map: HashMap::new(),
            code_end: 0,
            data_base: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Top-level
    // -----------------------------------------------------------------------

    fn compile(mut self) -> Result<CompileResult, CodeGenError> {
        self.pass0();
        self.pass1()?;
        let words = self.pass2()?;
        let binary = pack_words(&words);
        Ok(CompileResult {
            binary,
            halt_address: self.code_end,
            data_base: self.data_base,
            label_map: self.label_map,
        })
    }

    // -----------------------------------------------------------------------
    // Pass 0: collect registers and constants
    // -----------------------------------------------------------------------

    /// Scan all instructions to find the maximum register index and build the
    /// constants table.
    ///
    /// After this pass:
    /// - `max_reg`: highest virtual register index seen anywhere.
    /// - `const_index` / `const_order`: each unique LOAD_IMM / non-trivial
    ///   ADD_IMM constant value assigned a sequential index.
    fn pass0(&mut self) {
        for instr in &self.program.instructions {
            // Update max_reg from every Register operand.
            for operand in &instr.operands {
                if let IrOperand::Register(idx) = operand {
                    if *idx > self.max_reg {
                        self.max_reg = *idx;
                    }
                }
            }
            self.record_constants(instr);
        }
    }

    /// Register any immediate values that need a slot in the constants table.
    ///
    /// `LoadImm` always puts its immediate in the table (loaded via `LDA const_addr`).
    /// `AddImm` only needs the table for immediates other than 0, +1, and −1
    /// (those three use the copy, ADO, or SBO short paths).
    fn record_constants(&mut self, instr: &IrInstruction) {
        match instr.opcode {
            IrOp::LoadImm => {
                for operand in &instr.operands {
                    if let IrOperand::Immediate(v) = operand {
                        self.intern_const(*v);
                    }
                }
            }
            IrOp::AddImm => {
                for operand in &instr.operands {
                    if let IrOperand::Immediate(v) = operand {
                        if !matches!(v, 0 | 1 | -1) {
                            self.intern_const(*v);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Add `value` to the constants table if not already present.
    fn intern_const(&mut self, value: i64) {
        if !self.const_index.contains_key(&value) {
            let idx = self.const_order.len();
            self.const_order.push(value);
            self.const_index.insert(value, idx);
        }
    }

    // -----------------------------------------------------------------------
    // Derived layout helpers (depend on pass 0 results + pass 1 code_end)
    // -----------------------------------------------------------------------

    /// Number of spill slots = max_reg + 1 (covers v0 through v_max_reg).
    fn n_regs(&self) -> usize {
        self.max_reg + 1
    }

    /// Absolute word address of the spill slot for virtual register vN.
    ///
    /// Every IR virtual register gets its own dedicated memory word in the
    /// data segment. `spill(0)` is the argument/result register (used by
    /// SYSCALL 1 to pass the character code).
    fn spill(&self, reg_idx: usize) -> usize {
        self.data_base + reg_idx
    }

    /// Absolute word address of `value` in the constants table.
    ///
    /// The constants table sits immediately after the spill slots. Each
    /// unique constant seen in a `LOAD_IMM` or non-trivial `ADD_IMM` gets
    /// one slot, assigned in the order first encountered (pass 0).
    fn const_addr(&self, value: i64) -> usize {
        let idx = self.const_index[&value];
        self.data_base + self.n_regs() + idx
    }

    // -----------------------------------------------------------------------
    // Instruction encoding helpers
    // -----------------------------------------------------------------------
    //
    // These thin wrappers call `encode_instruction` with modifier=0 (no X
    // register group) and the given address. They panic on error because the
    // arguments are always compiler-generated valid values.

    fn lda(&self, addr: usize) -> i32 {
        encode_instruction(OP_LDA, 0, addr as i32).unwrap()
    }
    fn add(&self, addr: usize) -> i32 {
        encode_instruction(OP_ADD, 0, addr as i32).unwrap()
    }
    fn sub(&self, addr: usize) -> i32 {
        encode_instruction(OP_SUB, 0, addr as i32).unwrap()
    }
    fn sta(&self, addr: usize) -> i32 {
        encode_instruction(OP_STA, 0, addr as i32).unwrap()
    }
    fn mpy(&self, addr: usize) -> i32 {
        encode_instruction(OP_MPY, 0, addr as i32).unwrap()
    }
    fn dvd(&self, addr: usize) -> i32 {
        encode_instruction(OP_DVD, 0, addr as i32).unwrap()
    }
    fn bru(&self, addr: usize) -> i32 {
        encode_instruction(OP_BRU, 0, addr as i32).unwrap()
    }

    // -----------------------------------------------------------------------
    // Pass 1: label assignment and code-size calculation
    // -----------------------------------------------------------------------

    /// Return the number of GE-225 words this IR instruction occupies.
    ///
    /// Used in pass 1 to assign consecutive addresses to all labels and
    /// instructions before any words are actually emitted.
    fn word_count(&self, instr: &IrInstruction) -> Result<usize, CodeGenError> {
        Ok(match instr.opcode {
            // Zero-word pseudo-instructions.
            IrOp::Label | IrOp::Comment => 0,

            // Single-word instructions.
            IrOp::Nop | IrOp::Halt | IrOp::Jump => 1,

            // LOAD_IMM vDst, imm → LDA const_addr; STA spill(dst)  (2 words)
            IrOp::LoadImm => 2,

            // ADD_IMM has three specialisations:
            //   imm == 0  →  copy: LDA src; STA dst         (2 words)
            //   imm == ±1 →  inc/dec: LDA src; ADO/SBO; STA (3 words)
            //   other     →  add const: LDA src; ADD c; STA  (3 words)
            IrOp::AddImm => {
                let imm = self.get_imm(instr)?;
                if imm == 0 { 2 } else { 3 }
            }

            // LDA a; ADD/SUB b; STA dst  (3 words)
            IrOp::Add | IrOp::Sub => 3,

            // MUL vDst, vA, vB → LDA; LQA; LDZ; MPY; LAQ; STA  (6 words)
            IrOp::Mul => 6,
            // DIV vDst, vA, vB → LDA; LQA; LDZ; DVD; STA        (5 words)
            IrOp::Div => 5,

            // BOD-branch pattern for bit-0 extraction (7 words — see `emit_and_imm`)
            IrOp::AndImm => 7,

            // LDA a; SUB b; skip; BRU; result0; BRU; result1; STA  (8 words)
            IrOp::CmpEq | IrOp::CmpNe | IrOp::CmpLt | IrOp::CmpGt => 8,

            // LDA reg; BNZ/BZE; BRU target  (3 words)
            IrOp::BranchZ | IrOp::BranchNz => 3,

            // LDA spill(v0); SAN 6; TYP  (3 words)
            IrOp::Syscall => 3,

            op => {
                return Err(CodeGenError(format!(
                    "unsupported IR opcode in V1 GE-225 backend: {}",
                    op
                )));
            }
        })
    }

    /// Assign code addresses to all labels.
    ///
    /// The prologue (TON) occupies word 0. All IR instructions follow from
    /// word 1 onwards. After the loop, `code_end` is the address immediately
    /// past all code words; the halt stub lives there, and the data segment
    /// starts at `code_end + 1`.
    fn pass1(&mut self) -> Result<(), CodeGenError> {
        let mut word_addr: usize = 1; // word 0 = TON prologue
        for instr in &self.program.instructions {
            if instr.opcode == IrOp::Label {
                if let Some(IrOperand::Label(name)) = instr.operands.first() {
                    self.label_map.insert(name.clone(), word_addr);
                }
            }
            word_addr += self.word_count(instr)?;
        }
        self.code_end = word_addr;
        self.data_base = self.code_end + 1; // halt stub at code_end (one word)
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Pass 2: word emission
    // -----------------------------------------------------------------------

    /// Emit the complete GE-225 word list.
    ///
    /// Layout:
    /// ```text
    /// [TON, <IR code words>, <halt stub: BRU code_end>,
    ///  <n_regs zero-initialised spill slots>,
    ///  <constants table: values in pass-0 insertion order>]
    /// ```
    fn pass2(&self) -> Result<Vec<i32>, CodeGenError> {
        // Pre-assemble the fixed-word constants we use. These mnemonics are
        // all from the GE-225 manual; `assemble_fixed` only fails for unknown
        // mnemonics, so unwrap is correct here.
        // Pre-assemble fixed-word constants from the GE-225 manual.
        // These mnemonics are all documented; `assemble_fixed` only fails for
        // unknown mnemonics, so unwrap is safe here.
        let w_ton  = assemble_fixed("TON").unwrap();
        let w_nop  = assemble_fixed("NOP").unwrap();
        let w_ldz  = assemble_fixed("LDZ").unwrap();
        let w_ldo  = assemble_fixed("LDO").unwrap();
        let w_lqa  = assemble_fixed("LQA").unwrap(); // Q ← A (kept for future MUL/DIV)
        let w_laq  = assemble_fixed("LAQ").unwrap(); // A ← Q (kept for future MUL/DIV)
        let w_ado  = assemble_fixed("ADO").unwrap();
        let w_sbo  = assemble_fixed("SBO").unwrap();
        let w_bmi  = assemble_fixed("BMI").unwrap();
        let w_bpl  = assemble_fixed("BPL").unwrap();
        let w_bze  = assemble_fixed("BZE").unwrap();
        let w_bnz  = assemble_fixed("BNZ").unwrap();
        let w_bod  = assemble_fixed("BOD").unwrap();
        let w_san6 = assemble_shift("SAN", 6).unwrap();
        let w_typ  = assemble_fixed("TYP").unwrap();
        // BPL (branch on plus/non-negative) is assembled but replaced by BMI for
        // signed comparisons — suppress the unused-variable warning.
        let _ = w_bpl;

        let mut words: Vec<i32> = vec![w_ton]; // prologue at address 0
        let mut emit_addr: usize = 1; // address of the next word to be appended

        for instr in &self.program.instructions {
            let new_words = self.emit(
                instr,
                emit_addr,
                w_nop, w_ldz, w_ldo,
                w_ado, w_sbo, w_bmi, w_bze,
                w_bnz, w_bod, w_san6, w_typ,
                w_lqa, w_laq,
            )?;
            emit_addr += new_words.len();
            words.extend(new_words);
        }

        // Halt stub: self-referencing branch at code_end.
        words.push(self.bru(self.code_end));

        // Data section: n_regs zero-initialised spill slots.
        words.extend(std::iter::repeat(0).take(self.n_regs()));

        // Constants table (in pass-0 insertion order).
        // Values are guaranteed to be in the 20-bit signed range by
        // validate_for_ge225(), so the mask safely encodes negative values
        // as 20-bit two's-complement without silent truncation.
        for &value in &self.const_order {
            debug_assert!(
                value >= GE225_WORD_MIN && value <= GE225_WORD_MAX,
                "constant {} slipped past pre-flight validation",
                value
            );
            words.push((value as i32) & 0xFFFFF);
        }

        Ok(words)
    }

    /// Emit GE-225 words for a single IR instruction.
    ///
    /// `start_addr` is the code-word address at which the first emitted word
    /// will reside. Inline jump targets within a multi-word sequence (e.g.
    /// the compare and AND_IMM patterns) are computed relative to `start_addr`.
    ///
    /// The fixed-word constants are passed in to avoid repeated `assemble_fixed`
    /// calls on every instruction.
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &self,
        instr: &IrInstruction,
        start_addr: usize,
        w_nop: i32, w_ldz: i32, w_ldo: i32,
        w_ado: i32, w_sbo: i32, w_bmi: i32, w_bze: i32,
        w_bnz: i32, w_bod: i32, w_san6: i32, w_typ: i32,
        w_lqa: i32, w_laq: i32,
    ) -> Result<Vec<i32>, CodeGenError> {
        Ok(match instr.opcode {
            IrOp::Label | IrOp::Comment => vec![],
            IrOp::Nop => vec![w_nop],

            // HALT → BRU code_end  (self-loop to halt address)
            IrOp::Halt => vec![self.bru(self.code_end)],

            IrOp::LoadImm => self.emit_load_imm(instr)?,
            IrOp::AddImm => self.emit_add_imm(instr, w_ado, w_sbo)?,
            IrOp::Add => self.emit_binop(OP_ADD, instr)?,
            IrOp::Sub => self.emit_binop(OP_SUB, instr)?,
            IrOp::Mul => self.emit_mul(instr, w_lqa, w_laq, w_ldz)?,
            IrOp::Div => self.emit_div(instr, w_lqa, w_ldz)?,
            IrOp::AndImm => self.emit_and_imm(instr, start_addr, w_bod, w_ldz, w_ldo)?,
            IrOp::CmpEq => self.emit_cmp(instr, start_addr, true, false, w_bze, w_bnz, w_ldz, w_ldo)?,
            IrOp::CmpNe => self.emit_cmp(instr, start_addr, true, true, w_bze, w_bnz, w_ldz, w_ldo)?,
            IrOp::CmpLt => self.emit_cmp_signed(instr, start_addr, false, w_bmi, w_ldz, w_ldo)?,
            IrOp::CmpGt => self.emit_cmp_signed(instr, start_addr, true, w_bmi, w_ldz, w_ldo)?,
            IrOp::Jump => self.emit_jump(instr)?,
            IrOp::BranchZ => self.emit_branch(instr, true, w_bze, w_bnz)?,
            IrOp::BranchNz => self.emit_branch(instr, false, w_bze, w_bnz)?,
            IrOp::Syscall => self.emit_syscall(instr, w_san6, w_typ)?,

            op => {
                return Err(CodeGenError(format!(
                    "unsupported IR opcode in V1 GE-225 backend: {}",
                    op
                )));
            }
        })
    }

    // -----------------------------------------------------------------------
    // Per-opcode emitters
    // -----------------------------------------------------------------------

    /// `LOAD_IMM vDst, imm` → `LDA const_addr; STA spill(vDst)`  (2 words).
    ///
    /// The constant was pre-stored in the data segment during pass 0.
    fn emit_load_imm(&self, instr: &IrInstruction) -> Result<Vec<i32>, CodeGenError> {
        let dst = self.reg(instr, 0)?;
        let imm = self.get_imm(instr)?;
        Ok(vec![
            self.lda(self.const_addr(imm)),
            self.sta(self.spill(dst)),
        ])
    }

    /// `ADD_IMM vDst, vSrc, imm` — three specialisations.
    ///
    /// - `imm == 0`:  register copy  → `LDA; STA`            (2 words)
    /// - `imm == +1`: increment      → `LDA; ADO; STA`       (3 words)
    /// - `imm == -1`: decrement      → `LDA; SBO; STA`       (3 words)
    /// - other:       add constant   → `LDA; ADD const; STA` (3 words)
    ///
    /// The ADD-constant path requires the immediate to have been interned in
    /// the constants table during pass 0.
    fn emit_add_imm(
        &self, instr: &IrInstruction, w_ado: i32, w_sbo: i32,
    ) -> Result<Vec<i32>, CodeGenError> {
        let dst = self.reg(instr, 0)?;
        let src = self.reg(instr, 1)?;
        let imm = self.get_imm(instr)?;

        let lda = self.lda(self.spill(src));
        let sta = self.sta(self.spill(dst));

        Ok(match imm {
            0  => vec![lda, sta],
            1  => vec![lda, w_ado, sta],
            -1 => vec![lda, w_sbo, sta],
            _  => vec![lda, self.add(self.const_addr(imm)), sta],
        })
    }

    /// `ADD vDst, vA, vB` → `LDA spill(vA); ADD spill(vB); STA spill(vDst)` (3 words).
    ///
    /// Same pattern for `SUB` — just swap `OP_ADD` for `OP_SUB`.
    fn emit_binop(&self, ge225_op: i32, instr: &IrInstruction) -> Result<Vec<i32>, CodeGenError> {
        let dst   = self.reg(instr, 0)?;
        let reg_a = self.reg(instr, 1)?;
        let reg_b = self.reg(instr, 2)?;
        Ok(vec![
            self.lda(self.spill(reg_a)),
            encode_instruction(ge225_op, 0, self.spill(reg_b) as i32).unwrap(),
            self.sta(self.spill(dst)),
        ])
    }

    /// `MUL vDst, vA, vB` — signed multiply using MPY (6 words).
    ///
    /// The GE-225 `MPY` instruction computes `A,Q = Q × mem[ea] + A` (40-bit
    /// accumulate multiply).  To multiply `vA × vB` and store the lower 20
    /// bits of the product in `vDst`:
    ///
    /// ```text
    /// LDA  spill(vA)   ; A = vA
    /// LQA              ; Q = A = vA  (seed the Q register with vA)
    /// LDZ              ; A = 0       (zero the accumulator — we want Q*vB+0)
    /// MPY  spill(vB)   ; A,Q = Q*vB + A = vA*vB + 0  (40-bit result)
    /// LAQ              ; A = Q  (take the lower 20-bit word of the product)
    /// STA  spill(vDst)
    /// ```
    ///
    /// V1 ignores overflow (products larger than 2^19 − 1 wrap silently).
    fn emit_mul(
        &self,
        instr: &IrInstruction,
        w_lqa: i32, w_laq: i32, w_ldz: i32,
    ) -> Result<Vec<i32>, CodeGenError> {
        let dst   = self.reg(instr, 0)?;
        let reg_a = self.reg(instr, 1)?;
        let reg_b = self.reg(instr, 2)?;
        Ok(vec![
            self.lda(self.spill(reg_a)),  // A = vA
            w_lqa,                         // Q = A = vA
            w_ldz,                         // A = 0
            self.mpy(self.spill(reg_b)),  // A,Q = vA * vB
            w_laq,                         // A = Q  (lower 20 bits of product)
            self.sta(self.spill(dst)),    // spill(vDst) = A
        ])
    }

    /// `DIV vDst, vA, vB` — signed integer division using DVD (5 words).
    ///
    /// The GE-225 `DVD` instruction divides the 40-bit value `(A,Q)` by
    /// `mem[ea]`, placing the quotient in `A` and the remainder in `Q`.
    /// To compute `vA ÷ vB` (integer quotient, truncates toward zero):
    ///
    /// ```text
    /// LDA  spill(vA)   ; A = vA  (low word of 40-bit dividend)
    /// LQA              ; Q = A = vA  (copy into Q as the low dividend word)
    /// LDZ              ; A = 0       (high word — zero-extend the dividend)
    /// DVD  spill(vB)   ; A = quotient; Q = remainder
    /// STA  spill(vDst)
    /// ```
    ///
    /// Division by zero propagates a runtime trap from the GE-225 simulator.
    fn emit_div(
        &self,
        instr: &IrInstruction,
        w_lqa: i32, w_ldz: i32,
    ) -> Result<Vec<i32>, CodeGenError> {
        let dst   = self.reg(instr, 0)?;
        let reg_a = self.reg(instr, 1)?;
        let reg_b = self.reg(instr, 2)?;
        Ok(vec![
            self.lda(self.spill(reg_a)),  // A = vA
            w_lqa,                         // Q = A = vA  (dividend in Q)
            w_ldz,                         // A = 0       (high word = 0)
            self.dvd(self.spill(reg_b)),  // A = quotient
            self.sta(self.spill(dst)),    // spill(vDst) = quotient
        ])
    }

    /// `AND_IMM vDst, vSrc, 1` — extract parity bit using BOD (7 words).
    ///
    /// The GE-225 has no general bitwise-AND. We use the BOD (branch-if-odd)
    /// instruction to test bit 0.
    ///
    /// ## GE-225 branch-test "inhibit" semantics
    ///
    /// GE-225 skip instructions use **inhibit** semantics: the `cond` field says
    /// what *prevents* the skip, not what causes it.  Concretely:
    ///
    /// - `BOD` — inhibited by odd. When A is odd `cond = true`, so `!cond = false`
    ///   and the skip does **NOT** occur.  When A is even `!cond = true` and the
    ///   next word **is** skipped.
    ///
    /// Therefore: BOD **skips the next word when A is EVEN** and
    /// **does not skip when A is ODD**.
    ///
    /// ## Instruction layout
    ///
    /// ```text
    /// addr+0:  LDA  spill(vSrc)
    /// addr+1:  BOD               ; skip +2 when A is EVEN (inhibited-by-odd)
    /// addr+2:  BRU  addr+5       ; A is ODD (BOD did NOT skip) → jump to LDO
    /// addr+3:  LDZ               ; A is EVEN (BOD skipped +2) → A=0
    /// addr+4:  BRU  addr+6       ; jump to STA
    /// addr+5:  LDO               ; A is ODD → A=1
    /// addr+6:  STA  spill(vDst)
    /// ```
    fn emit_and_imm(
        &self, instr: &IrInstruction, start_addr: usize,
        w_bod: i32, w_ldz: i32, w_ldo: i32,
    ) -> Result<Vec<i32>, CodeGenError> {
        let dst = self.reg(instr, 0)?;
        let src = self.reg(instr, 1)?;
        let imm = self.get_imm(instr)?;
        if imm != 1 {
            return Err(CodeGenError(format!(
                "AND_IMM with immediate {} is not supported in V1; only imm=1 is allowed",
                imm
            )));
        }
        // BOD skips when A is EVEN (inhibit = odd).
        // Even path: BOD skips +2 → falls to LDZ (result=0) at +3.
        // Odd  path: BOD does NOT skip → executes BRU to LDO (result=1) at +5.
        let ldo_addr  = start_addr + 5; // LDO (result=1) for ODD inputs
        let done_addr = start_addr + 6; // STA
        Ok(vec![
            self.lda(self.spill(src)), // +0: load source
            w_bod,                      // +1: BOD — skip +2 when A is EVEN
            self.bru(ldo_addr),         // +2: A is ODD (not skipped) → jump to LDO
            w_ldz,                      // +3: A is EVEN (BOD skipped +2) → A=0
            self.bru(done_addr),        // +4: jump to STA
            w_ldo,                      // +5: A is ODD → A=1
            self.sta(self.spill(dst)),  // +6: store result
        ])
    }

    /// `CMP_EQ` or `CMP_NE` — compare two registers for equality (8 words).
    ///
    /// The GE-225 has no compare instruction. We subtract and test the result:
    ///
    /// ## GE-225 branch-test "inhibit" semantics (recap)
    ///
    /// Skip instructions use **inhibit** semantics: the named condition
    /// *prevents* the skip.  `BZE` is inhibited by zero (skip when A≠0);
    /// `BNZ` is inhibited by non-zero (skip when A==0).
    ///
    /// ## `CMP_EQ` layout (result = 1 when equal, 0 otherwise)
    ///
    /// ```text
    /// LDA  spill(vA)
    /// SUB  spill(vB)         ; A = vA - vB  (zero iff equal)
    /// BZE                    ; inhibited-by-zero → skip when A≠0
    /// BRU  addr+6 (__true)   ; executed when A==0 (equal) → jump to LDO
    /// LDZ                    ; A≠0 → result 0
    /// BRU  addr+7 (__done)
    /// LDO                    ; A==0 → result 1   [__true]
    /// STA  spill(vDst)                            [__done]
    /// ```
    ///
    /// For `CMP_NE`, swap the skip sense (`BNZ` instead of `BZE`) and the
    /// result labels.
    fn emit_cmp(
        &self, instr: &IrInstruction, start_addr: usize,
        eq: bool, negate: bool,
        w_bze: i32, w_bnz: i32, w_ldz: i32, w_ldo: i32,
    ) -> Result<Vec<i32>, CodeGenError> {
        let dst   = self.reg(instr, 0)?;
        let reg_a = self.reg(instr, 1)?;
        let reg_b = self.reg(instr, 2)?;

        let true_addr = start_addr + 6;
        let done_addr = start_addr + 7;

        // For CMP_EQ: BZE — inhibited by zero, so skips when A≠0;
        //   BRU to __true is NOT skipped when A==0 (equal → result 1).
        // For CMP_NE: BNZ — inhibited by non-zero, so skips when A==0;
        //   BRU to __true is NOT skipped when A≠0 (different → result 1).
        let skip_word = if eq { w_bze } else { w_bnz };

        // Result words: if negate (CMP_NE) swap 0 and 1.
        // zero_word = result when A==0 after SUB
        // one_word  = result when A!=0 after SUB
        let zero_word = if negate { w_ldo } else { w_ldz };
        let one_word  = if negate { w_ldz } else { w_ldo };

        Ok(vec![
            self.lda(self.spill(reg_a)), // +0
            self.sub(self.spill(reg_b)), // +1
            skip_word,                    // +2: conditional skip
            self.bru(true_addr),          // +3: jump to non-zero branch
            zero_word,                    // +4: result for A==0
            self.bru(done_addr),          // +5: jump past non-zero branch
            one_word,                     // +6: result for A!=0  [__true]
            self.sta(self.spill(dst)),   // +7  [__done]
        ])
    }

    /// `CMP_LT` or `CMP_GT` — signed integer comparison (8 words).
    ///
    /// For `CMP_LT` (vA < vB): compute vA − vB; negative result means vA < vB.
    ///
    /// ## GE-225 branch-test "inhibit" semantics (recap)
    ///
    /// `BMI` is **inhibited by minus** (inhibit = A<0): skip occurs when
    /// `!cond = (A≥0)`.  Therefore:
    /// - A < 0 (lhs < rhs → true): cond=true, skip **does NOT** occur → BRU
    ///   executes → jump to LDO → result 1.
    /// - A ≥ 0 (lhs ≥ rhs → false): cond=false, skip occurs → LDZ → result 0.
    ///
    /// ## Layout
    ///
    /// ```text
    /// LDA  spill(vA)
    /// SUB  spill(vB)         ; A = vA - vB
    /// BMI                    ; inhibited by minus → skip when A≥0 (not less than)
    /// BRU  addr+6 (__true)   ; A<0 → jump to LDO (result 1)
    /// LDZ                    ; A≥0 → result 0
    /// BRU  addr+7 (__done)
    /// LDO                    ; A<0 → result 1   [__true]
    /// STA  spill(vDst)                          [__done]
    /// ```
    ///
    /// For `CMP_GT` (vA > vB): swap vA and vB (vA > vB iff vB < vA).
    fn emit_cmp_signed(
        &self, instr: &IrInstruction, start_addr: usize,
        gt_mode: bool, w_bmi: i32, w_ldz: i32, w_ldo: i32,
    ) -> Result<Vec<i32>, CodeGenError> {
        let dst   = self.reg(instr, 0)?;
        let reg_a = self.reg(instr, 1)?;
        let reg_b = self.reg(instr, 2)?;

        let true_addr = start_addr + 6;
        let done_addr = start_addr + 7;

        // CMP_GT: swap operands so we compute vB - vA
        // (negative iff vB < vA, i.e., vA > vB)
        let lhs = if gt_mode { reg_b } else { reg_a };
        let rhs = if gt_mode { reg_a } else { reg_b };

        // BMI (inhibited-by-minus) skips when A≥0.
        // When A<0 (lhs < rhs) → no skip → BRU to __true → result 1.
        // When A≥0 (lhs ≥ rhs) → skip BRU → LDZ → result 0.
        Ok(vec![
            self.lda(self.spill(lhs)), // +0
            self.sub(self.spill(rhs)), // +1
            w_bmi,                      // +2: BMI — skip when A≥0 (not less)
            self.bru(true_addr),        // +3: A<0 → jump to LDO
            w_ldz,                      // +4: A≥0 → result 0
            self.bru(done_addr),        // +5
            w_ldo,                      // +6: A<0 → result 1  [__true]
            self.sta(self.spill(dst)), // +7  [__done]
        ])
    }

    /// `JUMP label` → `BRU label_addr`  (1 word).
    fn emit_jump(&self, instr: &IrInstruction) -> Result<Vec<i32>, CodeGenError> {
        let target = self.resolve_label(instr, 0)?;
        Ok(vec![self.bru(target)])
    }

    /// `BRANCH_Z` or `BRANCH_NZ` — conditional far branch (3 words).
    ///
    /// ## GE-225 branch-test "inhibit" semantics (recap)
    ///
    /// Skip instructions use **inhibit** semantics: the named condition
    /// *prevents* the skip.
    ///
    /// - `BZE` — inhibited by zero (cond = A==0).  Skips when A≠0
    ///   (i.e. the BRU at offset +1 is skipped when A is non-zero).
    ///   Therefore the BRU **executes** when A==0.
    /// - `BNZ` — inhibited by non-zero (cond = A≠0).  Skips when A==0.
    ///   Therefore the BRU **executes** when A≠0.
    ///
    /// ## Layout for `BRANCH_Z` (jump when A==0)
    ///
    /// ```text
    /// LDA  spill(vN)
    /// BZE               ; inhibited-by-zero → skip BRU when A≠0 (don't jump)
    /// BRU  target       ; executed only when A==0 → jump to target
    /// ; fall through when A≠0
    /// ```
    ///
    /// For `BRANCH_NZ` (jump when A≠0), swap `BZE → BNZ`.
    fn emit_branch(
        &self, instr: &IrInstruction, zero: bool, w_bze: i32, w_bnz: i32,
    ) -> Result<Vec<i32>, CodeGenError> {
        let reg_n  = self.reg(instr, 0)?;
        let target = self.resolve_label(instr, 1)?;
        // BRANCH_Z (jump when A==0): BZE — inhibited by zero → BRU executes when A==0.
        // BRANCH_NZ (jump when A≠0): BNZ — inhibited by non-zero → BRU executes when A≠0.
        let skip_word = if zero { w_bze } else { w_bnz };
        Ok(vec![
            self.lda(self.spill(reg_n)),
            skip_word,
            self.bru(target),
        ])
    }

    /// `SYSCALL 1` — print the character whose code is in spill_v0 (3 words).
    ///
    /// The GE-225 typewriter subsystem requires the character code in the N
    /// register (6 bits). The SAN 6 instruction loads N from the low 6 bits
    /// of A:
    ///
    /// ```text
    /// LDA  spill(v0)    ; load 6-bit typewriter code
    /// SAN  6            ; shift low 6 bits of A into N register
    /// TYP               ; print the character whose code is in N
    /// ```
    fn emit_syscall(
        &self, instr: &IrInstruction, w_san6: i32, w_typ: i32,
    ) -> Result<Vec<i32>, CodeGenError> {
        // Verify syscall number is 1 (pre-flight should have caught it, but
        // belt-and-suspenders check).
        let num = instr.operands.iter().find_map(|o| {
            if let IrOperand::Immediate(v) = o { Some(*v) } else { None }
        });
        if let Some(n) = num {
            if n != 1 {
                return Err(CodeGenError(format!(
                    "only SYSCALL 1 is supported in V1; got SYSCALL {}",
                    n
                )));
            }
        }
        Ok(vec![
            self.lda(self.spill(0)), // spill(v0) = syscall argument register
            w_san6,
            w_typ,
        ])
    }

    // -----------------------------------------------------------------------
    // Operand extraction helpers
    // -----------------------------------------------------------------------

    /// Extract the register index from the `idx`-th operand.
    fn reg(&self, instr: &IrInstruction, idx: usize) -> Result<usize, CodeGenError> {
        match instr.operands.get(idx) {
            Some(IrOperand::Register(r)) => Ok(*r),
            Some(other) => Err(CodeGenError(format!(
                "expected Register at operand {} of {}, got {:?}",
                idx, instr.opcode, other
            ))),
            None => Err(CodeGenError(format!(
                "missing operand {} of {}",
                idx, instr.opcode
            ))),
        }
    }

    /// Extract the integer value from the last `IrOperand::Immediate` operand.
    fn get_imm(&self, instr: &IrInstruction) -> Result<i64, CodeGenError> {
        instr.operands.iter().rev().find_map(|o| {
            if let IrOperand::Immediate(v) = o { Some(Ok(*v)) } else { None }
        }).unwrap_or_else(|| {
            Err(CodeGenError(format!(
                "no Immediate operand in {} instruction",
                instr.opcode
            )))
        })
    }

    /// Resolve a label operand to its absolute code address.
    ///
    /// Raises `CodeGenError` if the label has no entry in the label map
    /// (undefined forward reference).
    fn resolve_label(&self, instr: &IrInstruction, idx: usize) -> Result<usize, CodeGenError> {
        match instr.operands.get(idx) {
            Some(IrOperand::Label(name)) => {
                self.label_map.get(name).copied().ok_or_else(|| {
                    CodeGenError(format!("undefined label: {:?}", name))
                })
            }
            Some(other) => Err(CodeGenError(format!(
                "expected Label at operand {} of {}, got {:?}",
                idx, instr.opcode, other
            ))),
            None => Err(CodeGenError(format!(
                "missing operand {} of {}",
                idx, instr.opcode
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

    // -----------------------------------------------------------------------
    // Helper: minimal valid program skeleton
    // -----------------------------------------------------------------------

    /// Build a minimal `IrProgram` with a LABEL + HALT.
    fn base_prog() -> IrProgram {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("_start".to_string())],
            -1,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        p
    }

    /// Append a single instruction to a program (returns the program for chaining).
    fn with(mut prog: IrProgram, instr: IrInstruction) -> IrProgram {
        prog.add_instruction(instr);
        prog
    }

    // -----------------------------------------------------------------------
    // Validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_valid_program() {
        // A program with only supported opcodes should pass validation.
        let prog = base_prog();
        assert!(validate_for_ge225(&prog).is_empty());
    }

    #[test]
    fn test_validate_rejects_unsupported_opcode() {
        // LOAD_BYTE is not in the V1 GE-225 supported set.
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::LoadByte,
            vec![IrOperand::Register(0), IrOperand::Register(1)],
            2,
        ));
        let errors = validate_for_ge225(&prog);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("LOAD_BYTE") || errors[0].contains("unsupported opcode"));
    }

    #[test]
    fn test_validate_rejects_constant_overflow() {
        // 1_000_000 exceeds GE225_WORD_MAX (524_287).
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(1_000_000)],
            2,
        ));
        let errors = validate_for_ge225(&prog);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("overflow") || errors[0].contains("overflow"));
    }

    #[test]
    fn test_validate_rejects_negative_overflow() {
        // -600_000 is below GE225_WORD_MIN (-524_288).
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(-600_000)],
            2,
        ));
        let errors = validate_for_ge225(&prog);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_validate_rejects_syscall_not_1() {
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Register(0), IrOperand::Immediate(2)],
            2,
        ));
        let errors = validate_for_ge225(&prog);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("SYSCALL 2") || errors[0].contains("unsupported SYSCALL"));
    }

    #[test]
    fn test_validate_allows_syscall_1() {
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Register(0), IrOperand::Immediate(1)],
            2,
        ));
        assert!(validate_for_ge225(&prog).is_empty());
    }

    #[test]
    fn test_validate_rejects_and_imm_not_1() {
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::AndImm,
            vec![
                IrOperand::Register(0),
                IrOperand::Register(1),
                IrOperand::Immediate(3), // only 1 is allowed
            ],
            2,
        ));
        let errors = validate_for_ge225(&prog);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("AND_IMM"));
    }

    #[test]
    fn test_validate_allows_and_imm_1() {
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::AndImm,
            vec![
                IrOperand::Register(0),
                IrOperand::Register(1),
                IrOperand::Immediate(1),
            ],
            2,
        ));
        assert!(validate_for_ge225(&prog).is_empty());
    }

    // -----------------------------------------------------------------------
    // Compilation: structural checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_binary_is_multiple_of_3() {
        // Every GE-225 word is packed into 3 bytes (big-endian).
        let result = compile_to_ge225(&base_prog()).unwrap();
        assert_eq!(result.binary.len() % 3, 0);
    }

    #[test]
    fn test_data_base_equals_halt_address_plus_one() {
        let result = compile_to_ge225(&base_prog()).unwrap();
        assert_eq!(result.data_base, result.halt_address + 1);
    }

    #[test]
    fn test_label_is_recorded_in_map() {
        let result = compile_to_ge225(&base_prog()).unwrap();
        assert!(result.label_map.contains_key("_start"));
    }

    #[test]
    fn test_label_address_follows_ton() {
        // TON occupies word 0; the first LABEL maps to word 1.
        let result = compile_to_ge225(&base_prog()).unwrap();
        assert_eq!(result.label_map["_start"], 1);
    }

    // -----------------------------------------------------------------------
    // Compilation: word-count per opcode
    // -----------------------------------------------------------------------

    /// Insert an instruction *before* the HALT in base_prog.
    fn prog_with(instr: IrInstruction) -> IrProgram {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("_start".to_string())],
            -1,
        ));
        p.add_instruction(instr);
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        p
    }

    #[test]
    fn test_nop_is_one_word() {
        let r1 = compile_to_ge225(&base_prog()).unwrap();
        let r2 = compile_to_ge225(&prog_with(
            IrInstruction::new(IrOp::Nop, vec![], 2),
        )).unwrap();
        let words1 = r1.binary.len() / 3;
        let words2 = r2.binary.len() / 3;
        // NOP adds 1 word to the code section
        assert_eq!(words2 - words1, 1);
    }

    #[test]
    fn test_load_imm_is_two_words() {
        let r1 = compile_to_ge225(&base_prog()).unwrap();
        let r2 = compile_to_ge225(&prog_with(
            IrInstruction::new(
                IrOp::LoadImm,
                vec![IrOperand::Register(0), IrOperand::Immediate(99)],
                2,
            ),
        )).unwrap();
        assert_eq!(r2.binary.len() / 3 - r1.binary.len() / 3, 3);
        // 2 code words + 1 constant slot in data section
        // Actually the binary is code + data, so total words increase by 3 (2 code + 1 const)
    }

    #[test]
    fn test_add_imm_zero_is_two_words() {
        // ADD_IMM with imm=0 is a register copy: LDA src; STA dst
        let r1 = compile_to_ge225(&base_prog()).unwrap();
        // We need v0 and v1, so first define them with LOAD_IMM.
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label, vec![IrOperand::Label("_start".to_string())], -1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(5)],
            1,
        ));
        let halt_only = {
            let mut p2 = p.clone();
            p2.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
            p2
        };
        p.add_instruction(IrInstruction::new(
            IrOp::AddImm,
            vec![IrOperand::Register(1), IrOperand::Register(0), IrOperand::Immediate(0)],
            2,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        let r_halt_only = compile_to_ge225(&halt_only).unwrap();
        let r_with_copy = compile_to_ge225(&p).unwrap();
        // The copy instruction adds 2 code words and 0 constants (imm=0 not interned)
        // halt_only has halt_address N; r_with_copy has halt_address N+2
        assert_eq!(
            r_with_copy.halt_address - r_halt_only.halt_address,
            2
        );
        let _ = r1;
    }

    #[test]
    fn test_add_imm_plus_one_is_three_words() {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label, vec![IrOperand::Label("_start".to_string())], -1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(5)],
            1,
        ));
        let halt_only = {
            let mut p2 = p.clone();
            p2.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
            p2
        };
        p.add_instruction(IrInstruction::new(
            IrOp::AddImm,
            vec![IrOperand::Register(1), IrOperand::Register(0), IrOperand::Immediate(1)],
            2,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        let r1 = compile_to_ge225(&halt_only).unwrap();
        let r2 = compile_to_ge225(&p).unwrap();
        assert_eq!(r2.halt_address - r1.halt_address, 3);
    }

    #[test]
    fn test_add_is_three_words() {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label, vec![IrOperand::Label("_start".to_string())], -1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm, vec![IrOperand::Register(0), IrOperand::Immediate(1)], 1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm, vec![IrOperand::Register(1), IrOperand::Immediate(2)], 2,
        ));
        let halt_only = {
            let mut p2 = p.clone();
            p2.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
            p2
        };
        p.add_instruction(IrInstruction::new(
            IrOp::Add,
            vec![IrOperand::Register(2), IrOperand::Register(0), IrOperand::Register(1)],
            3,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        let r1 = compile_to_ge225(&halt_only).unwrap();
        let r2 = compile_to_ge225(&p).unwrap();
        assert_eq!(r2.halt_address - r1.halt_address, 3);
    }

    #[test]
    fn test_and_imm_is_seven_words() {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label, vec![IrOperand::Label("_start".to_string())], -1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm, vec![IrOperand::Register(0), IrOperand::Immediate(7)], 1,
        ));
        let halt_only = {
            let mut p2 = p.clone();
            p2.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
            p2
        };
        p.add_instruction(IrInstruction::new(
            IrOp::AndImm,
            vec![IrOperand::Register(1), IrOperand::Register(0), IrOperand::Immediate(1)],
            2,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        let r1 = compile_to_ge225(&halt_only).unwrap();
        let r2 = compile_to_ge225(&p).unwrap();
        assert_eq!(r2.halt_address - r1.halt_address, 7);
    }

    #[test]
    fn test_cmp_eq_is_eight_words() {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label, vec![IrOperand::Label("_start".to_string())], -1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm, vec![IrOperand::Register(0), IrOperand::Immediate(5)], 1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm, vec![IrOperand::Register(1), IrOperand::Immediate(5)], 2,
        ));
        let halt_only = {
            let mut p2 = p.clone();
            p2.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
            p2
        };
        p.add_instruction(IrInstruction::new(
            IrOp::CmpEq,
            vec![IrOperand::Register(2), IrOperand::Register(0), IrOperand::Register(1)],
            3,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        let r1 = compile_to_ge225(&halt_only).unwrap();
        let r2 = compile_to_ge225(&p).unwrap();
        assert_eq!(r2.halt_address - r1.halt_address, 8);
    }

    #[test]
    fn test_branch_z_is_three_words() {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label, vec![IrOperand::Label("_start".to_string())], -1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm, vec![IrOperand::Register(0), IrOperand::Immediate(0)], 1,
        ));
        let halt_only = {
            let mut p2 = p.clone();
            p2.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
            p2
        };
        p.add_instruction(IrInstruction::new(
            IrOp::BranchZ,
            vec![IrOperand::Register(0), IrOperand::Label("_start".to_string())],
            2,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        let r1 = compile_to_ge225(&halt_only).unwrap();
        let r2 = compile_to_ge225(&p).unwrap();
        assert_eq!(r2.halt_address - r1.halt_address, 3);
    }

    #[test]
    fn test_syscall_is_three_words() {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label, vec![IrOperand::Label("_start".to_string())], -1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm, vec![IrOperand::Register(0), IrOperand::Immediate(65)], 1,
        ));
        let halt_only = {
            let mut p2 = p.clone();
            p2.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
            p2
        };
        p.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Register(0), IrOperand::Immediate(1)],
            2,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        let r1 = compile_to_ge225(&halt_only).unwrap();
        let r2 = compile_to_ge225(&p).unwrap();
        assert_eq!(r2.halt_address - r1.halt_address, 3);
    }

    // -----------------------------------------------------------------------
    // Compilation: error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_compile_fails_on_unsupported_opcode() {
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::LoadByte,
            vec![IrOperand::Register(0), IrOperand::Register(1)],
            2,
        ));
        assert!(compile_to_ge225(&prog).is_err());
    }

    #[test]
    fn test_compile_fails_on_constant_overflow() {
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(600_000)],
            2,
        ));
        assert!(compile_to_ge225(&prog).is_err());
    }

    #[test]
    fn test_compile_fails_on_undefined_label() {
        // JUMP to a label that doesn't exist in the program.
        let prog = with(base_prog(), IrInstruction::new(
            IrOp::Jump,
            vec![IrOperand::Label("nonexistent".to_string())],
            2,
        ));
        // Note: validate_for_ge225 won't catch this (it only checks opcodes/immediates).
        // The error will surface during pass 2.
        assert!(compile_to_ge225(&prog).is_err());
    }

    // -----------------------------------------------------------------------
    // End-to-end: run on simulator
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_imm_and_halt_runs_on_simulator() {
        use coding_adventures_ge225_simulator::Simulator;

        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label, vec![IrOperand::Label("_start".to_string())], -1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(42)],
            1,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));

        let result = compile_to_ge225(&p).unwrap();

        let mut sim = Simulator::new(4096);
        sim.load_words(
            &coding_adventures_ge225_simulator::unpack_words(&result.binary).unwrap(),
            0,
        ).unwrap();

        // Run until we hit the halt stub (self-loop).
        let halt = result.halt_address as i32;
        for _ in 0..1000 {
            let trace = sim.step().unwrap();
            if trace.address == halt {
                break;
            }
        }
        assert_eq!(sim.get_state().halted || sim.get_state().pc == halt, true);
    }

    #[test]
    fn test_add_two_constants_runs_on_simulator() {
        use coding_adventures_ge225_simulator::Simulator;

        // Compute 3 + 4 = 7 and store in v2; read v2's spill slot.
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::Label, vec![IrOperand::Label("_start".to_string())], -1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm, vec![IrOperand::Register(0), IrOperand::Immediate(3)], 1,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm, vec![IrOperand::Register(1), IrOperand::Immediate(4)], 2,
        ));
        p.add_instruction(IrInstruction::new(
            IrOp::Add,
            vec![IrOperand::Register(2), IrOperand::Register(0), IrOperand::Register(1)],
            3,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 4));

        let result = compile_to_ge225(&p).unwrap();

        let mut sim = Simulator::new(4096);
        let words = coding_adventures_ge225_simulator::unpack_words(&result.binary).unwrap();
        sim.load_words(&words, 0).unwrap();

        let halt = result.halt_address as i32;
        for _ in 0..1000 {
            let trace = sim.step().unwrap();
            if trace.address == halt { break; }
        }

        // v2 spill slot = data_base + 2
        let v2_addr = (result.data_base + 2) as i32;
        let val = sim.read_word(v2_addr).unwrap();
        // GE-225 words are 20-bit signed; 7 in 20-bit two's complement is just 7.
        assert_eq!(val & 0xFFFFF, 7);
    }
}
