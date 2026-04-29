//! IR → BEAM lowering pass.
//!
//! This module converts an [`IrProgram`] into a [`BEAMModule`] that can then
//! be serialized by [`crate::encoder::encode_beam`].
//!
//! # BEAM execution model (mini-primer)
//!
//! BEAM is the virtual machine that runs Erlang and Elixir code.  It is a
//! **register machine** — each function has a set of *x-registers* (scratch
//! registers, numbered x0, x1, x2, …) plus *y-registers* on a per-call stack
//! frame.  This lowering uses only x-registers.
//!
//! Arithmetic is done via "BIFs" (built-in functions).  `gc_bif2` calls a
//! two-argument BIF from the import table and stores the result in an
//! x-register.  The GC variant may trigger garbage collection, hence the
//! `live` operand (number of live registers the GC needs to scan).
//!
//! # Supported IR opcodes (v1)
//!
//! | IR op      | BEAM instruction        | Notes |
//! |------------|-------------------------|-------|
//! | LABEL      | label {u,N}             | N from label map (starts at 3) |
//! | LOAD_IMM   | move {i,val} {x,r}      | |
//! | ADD        | gc_bif2 erlang:+/2      | |
//! | ADD_IMM    | move + gc_bif2          | synthesised via scratch register |
//! | SUB        | gc_bif2 erlang:-/2      | |
//! | AND        | gc_bif2 erlang:band/2   | |
//! | AND_IMM    | move + gc_bif2          | synthesised |
//! | JUMP       | jump {f,label}          | opcode 36 |
//! | BRANCH_Z   | is_ne_exact {f,L} r {i,0} | branch to L when reg == 0 |
//! | BRANCH_NZ  | is_eq_exact {f,L} r {i,0} | branch to L when reg != 0 |
//! | CALL       | call {u,0} {f,label}    | arity 0 in v1 |
//! | RET / HALT | return                  | opcode 19 |
//! | NOP        | (nothing)               | |
//! | COMMENT    | (nothing)               | |
//!
//! # Module structure
//!
//! The emitted BEAM bytecode follows the standard function layout:
//!
//! ```text
//! {label, 1}.                         ← func_info header label
//! {func_info, {a,Module}, {a,run}, 0}.
//! {label, 2}.                         ← exported code entry (in ExpT)
//!   … translated IR instructions …
//! {return}.                           ← from HALT / RET
//! {int_code_end}.
//! ```
//!
//! Export table references label 2 for `run/0`.
//!
//! # Unsupported ops (validation errors)
//!
//! LOAD_BYTE, STORE_BYTE, LOAD_WORD, STORE_WORD, LOAD_ADDR, SYSCALL,
//! CMP_EQ, CMP_NE, CMP_LT, CMP_GT.
//!
//! These operations have no direct BEAM equivalent in v1.  Future versions
//! may introduce NIF wrappers or other lowering strategies.

use std::collections::HashMap;

use compiler_ir::{IrOp, IrOperand, IrProgram};

use crate::encoder::{BEAMExport, BEAMImport, BEAMInstruction, BEAMModule, BEAMOperand};

// ===========================================================================
// BEAM opcode constants
// ===========================================================================

/// `{label, {u,N}}` — defines a label at this position.
const OP_LABEL: u8 = 1;
/// `{func_info, {a,Mod}, {a,Fun}, {u,Arity}}` — function entry metadata.
const OP_FUNC_INFO: u8 = 2;
/// `{int_code_end}` — marks the end of the code section.
const OP_INT_CODE_END: u8 = 3;
/// `{call, {u,Arity}, {f,Label}}` — local call.
const OP_CALL: u8 = 4;
/// `{return}` — return from current function.
const OP_RETURN: u8 = 19;
/// `{jump, {f,Label}}` — unconditional branch.
const OP_JUMP: u8 = 36;
/// `{is_eq_exact, {f,Fail}, A, B}` — fall through if A == B, branch if not.
const OP_IS_EQ_EXACT: u8 = 43;
/// `{is_ne_exact, {f,Fail}, A, B}` — fall through if A != B, branch if not.
const OP_IS_NE_EXACT: u8 = 44;
/// `{move, Src, Dst}` — copy operand to register.
const OP_MOVE: u8 = 64;
/// `{gc_bif2, Fail, Live, Bif, Arg1, Arg2, Dst}` — two-argument BIF call.
const OP_GC_BIF2: u8 = 125;

// ===========================================================================
// BEAMBackendConfig
// ===========================================================================

/// Configuration for the BEAM lowering pass.
///
/// Currently only the module name needs to be specified — BEAM modules are
/// identified by a single atom (e.g. `myapp`, `calc`).
#[derive(Debug, Clone)]
pub struct BEAMBackendConfig {
    /// The Erlang module name (must be a valid atom, e.g. `"mymod"`).
    pub module_name: String,
}

impl Default for BEAMBackendConfig {
    fn default() -> Self {
        Self { module_name: "main".to_string() }
    }
}

// ===========================================================================
// BEAMBackendError
// ===========================================================================

/// Errors that can occur during the IR → BEAM lowering pass.
#[derive(Debug)]
pub enum BEAMBackendError {
    /// The program failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IR opcode that is not supported by the BEAM backend.
    UnsupportedOp(String),
    /// An operand had an unexpected kind (e.g. register where label expected).
    InvalidOperand(String),
    /// A jump or branch targets a label that has no definition.
    UndefinedLabel(String),
}

impl std::fmt::Display for BEAMBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidationFailed(errs) => write!(f, "validation failed: {}", errs.join("; ")),
            Self::UnsupportedOp(op)      => write!(f, "unsupported BEAM op: {}", op),
            Self::InvalidOperand(msg)    => write!(f, "invalid operand: {}", msg),
            Self::UndefinedLabel(lbl)    => write!(f, "undefined label: {}", lbl),
        }
    }
}

impl std::error::Error for BEAMBackendError {}

// ===========================================================================
// _AtomTable — insertion-ordered, 1-based
// ===========================================================================

/// Maintains the BEAM atom table with insertion-order 1-based indices.
///
/// BEAM atom tables are 1-based: the first atom has index 1.  Interning
/// the same atom twice always returns the same index.
struct _AtomTable {
    atoms: Vec<String>,
    index: HashMap<String, u32>,
}

impl _AtomTable {
    fn new() -> Self {
        Self { atoms: Vec::new(), index: HashMap::new() }
    }

    /// Intern `atom`, returning its 1-based index.  Inserts if not present.
    fn intern(&mut self, atom: &str) -> u32 {
        if let Some(&idx) = self.index.get(atom) {
            return idx;
        }
        self.atoms.push(atom.to_string());
        // Checked cast: BEAM atom indices are stored as u32.  More than ~4 B
        // unique atoms is not realistic, but we make the overflow explicit
        // rather than silently wrapping and producing wrong indices.
        let idx = u32::try_from(self.atoms.len())
            .expect("atom table exceeded u32::MAX entries");
        self.index.insert(atom.to_string(), idx);
        idx
    }

    /// Return a reference to the ordered atom list (0-indexed internally).
    fn all(&self) -> &[String] {
        &self.atoms
    }
}

// ===========================================================================
// _ImportTable — deduplicating, 0-based
// ===========================================================================

/// Maintains the BEAM import table (ImpT chunk) with deduplication.
///
/// Each import is a `(module_atom_idx, function_atom_idx, arity)` triple.
/// The returned index is **0-based** — that is what BEAM opcodes reference
/// when calling external functions.
struct _ImportTable {
    imports: Vec<BEAMImport>,
    index: HashMap<(u32, u32, u32), u32>,
}

impl _ImportTable {
    fn new() -> Self {
        Self { imports: Vec::new(), index: HashMap::new() }
    }

    /// Intern an import, returning its 0-based index.
    fn intern(&mut self, module_idx: u32, fn_idx: u32, arity: u32) -> u32 {
        let key = (module_idx, fn_idx, arity);
        if let Some(&idx) = self.index.get(&key) {
            return idx;
        }
        let idx = self.imports.len() as u32;
        self.imports.push(BEAMImport { module_atom_index: module_idx, function_atom_index: fn_idx, arity });
        self.index.insert(key, idx);
        idx
    }
}

// ===========================================================================
// validate_for_beam
// ===========================================================================

/// The set of IR opcodes that the BEAM backend does not support in v1.
///
/// These require NIF wrappers, memory BIFs, or other mechanisms that are
/// out of scope for the first BEAM backend version.
const UNSUPPORTED_OPS: &[IrOp] = &[
    IrOp::LoadByte, IrOp::StoreByte,
    IrOp::LoadWord, IrOp::StoreWord,
    IrOp::LoadAddr,
    IrOp::Syscall,
    IrOp::CmpEq, IrOp::CmpNe, IrOp::CmpLt, IrOp::CmpGt,
];

/// Validate an `IrProgram` for BEAM lowering.
///
/// Returns a list of human-readable error strings.  An empty list means the
/// program is safe to lower.  Always call this before [`lower_ir_to_beam`].
///
/// # Checks performed
///
/// 1. Module name (from config) is non-empty — checked separately in
///    [`lower_ir_to_beam`].
/// 2. No instructions use opcodes unsupported by the BEAM backend.
/// 3. Entry label is non-empty.
///
/// # Example
///
/// ```
/// use compiler_ir::{IrProgram, IrInstruction, IrOp};
/// use ir_to_beam::backend::validate_for_beam;
///
/// let prog = IrProgram::new("_start");
/// let errs = validate_for_beam(&prog);
/// assert!(errs.is_empty());
/// ```
pub fn validate_for_beam(program: &IrProgram) -> Vec<String> {
    let mut errors = Vec::new();

    if program.entry_label.is_empty() {
        errors.push("entry label must not be empty".to_string());
    }

    for instr in &program.instructions {
        if UNSUPPORTED_OPS.contains(&instr.opcode) {
            errors.push(format!("opcode {} is not supported by the BEAM backend (v1)", instr.opcode));
        }
    }

    errors
}

// ===========================================================================
// Internal helpers
// ===========================================================================

/// First-pass: collect all LABEL instructions and assign BEAM label numbers.
///
/// BEAM label 1 and 2 are reserved for the `func_info` preamble, so IR
/// labels start at 3.
fn collect_labels(program: &IrProgram) -> (HashMap<String, u32>, u32) {
    let mut label_map: HashMap<String, u32> = HashMap::new();
    let mut next_label: u32 = 3; // 1 and 2 are for func_info preamble

    for instr in &program.instructions {
        if instr.opcode == IrOp::Label {
            if let Some(IrOperand::Label(name)) = instr.operands.first() {
                label_map.entry(name.clone()).or_insert_with(|| {
                    let l = next_label;
                    next_label += 1;
                    l
                });
            }
        }
    }
    (label_map, next_label)
}

/// Find the highest x-register index used across all operands.
/// Returns 0 if no registers are used.
fn max_register_used(program: &IrProgram) -> usize {
    program.instructions.iter()
        .flat_map(|i| i.operands.iter())
        .filter_map(|op| if let IrOperand::Register(idx) = op { Some(*idx) } else { None })
        .max()
        .unwrap_or(0)
}

/// Extract a label name from an operand, or return an error.
fn label_name(op: &IrOperand) -> Result<String, BEAMBackendError> {
    match op {
        IrOperand::Label(name) => Ok(name.clone()),
        other => Err(BEAMBackendError::InvalidOperand(
            format!("expected label operand, got {:?}", other),
        )),
    }
}

/// Extract a virtual-register index from an operand, or return an error.
fn reg_idx(op: &IrOperand) -> Result<usize, BEAMBackendError> {
    match op {
        IrOperand::Register(idx) => Ok(*idx),
        other => Err(BEAMBackendError::InvalidOperand(
            format!("expected register operand, got {:?}", other),
        )),
    }
}

/// Extract an integer immediate from an operand, or return an error.
fn imm_val(op: &IrOperand) -> Result<i64, BEAMBackendError> {
    match op {
        IrOperand::Immediate(val) => Ok(*val),
        other => Err(BEAMBackendError::InvalidOperand(
            format!("expected immediate operand, got {:?}", other),
        )),
    }
}

/// Build a `gc_bif2` instruction.
///
/// Format: `{gc_bif2, {f,0}, {u,live}, {u,import_idx}, {x,a1}, {x,a2}, {x,dst}}`
///
/// `{f,0}` means "no failure label" — let the BEAM VM raise an exception
/// instead of branching to an error handler.
fn emit_gc_bif2(import_idx: u32, live: u64, arg1: u8, arg2: u8, dst: u8) -> BEAMInstruction {
    BEAMInstruction::new(OP_GC_BIF2, vec![
        BEAMOperand::f(0),                   // no explicit fail label
        BEAMOperand::u(live),                // live x-register count for GC
        BEAMOperand::u(import_idx as u64),   // import table index (0-based)
        BEAMOperand::x(arg1),                // first argument
        BEAMOperand::x(arg2),                // second argument
        BEAMOperand::x(dst),                 // destination
    ])
}

/// Produce the minimal Attr and CInf chunks (BERT nil list `[0x83, 0x6a]`).
///
/// BEAM loaders that validate the module file expect these chunks to contain
/// valid BERT-encoded terms.  The nil list `[]` is the simplest valid term.
fn beam_attr_cinf_chunks() -> Vec<([u8; 4], Vec<u8>)> {
    // 0x83 = BERT magic byte; 0x6A = nil list tag
    let bert_nil: Vec<u8> = vec![0x83, 0x6A];
    vec![
        (*b"Attr", bert_nil.clone()),
        (*b"CInf", bert_nil),
    ]
}

// ===========================================================================
// lower_ir_to_beam
// ===========================================================================

/// Lower an [`IrProgram`] to a [`BEAMModule`] ready for encoding.
///
/// This is the main entry point for the BEAM backend.  It:
///
/// 1. Validates the program with [`validate_for_beam`].
/// 2. Builds the atom and import tables.
/// 3. Emits the `func_info` preamble (labels 1 and 2).
/// 4. Translates each IR instruction to its BEAM equivalent.
/// 5. Appends `INT_CODE_END` and builds the final [`BEAMModule`].
///
/// # Errors
///
/// Returns [`BEAMBackendError::ValidationFailed`] if [`validate_for_beam`]
/// produces errors.  Returns other variants for malformed operands or
/// undefined labels.
///
/// # Example
///
/// ```
/// use compiler_ir::{IrProgram, IrInstruction, IrOperand, IrOp};
/// use ir_to_beam::backend::{BEAMBackendConfig, lower_ir_to_beam};
///
/// let mut prog = IrProgram::new("_start");
/// prog.add_instruction(IrInstruction::new(
///     IrOp::LoadImm,
///     vec![IrOperand::Register(0), IrOperand::Immediate(42)],
///     0,
/// ));
/// prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
///
/// let cfg = BEAMBackendConfig { module_name: "demo".to_string() };
/// let module = lower_ir_to_beam(&prog, &cfg).unwrap();
/// assert_eq!(module.name, "demo");
/// ```
pub fn lower_ir_to_beam(
    program: &IrProgram,
    config: &BEAMBackendConfig,
) -> Result<BEAMModule, BEAMBackendError> {
    // ── Pre-flight validation ──────────────────────────────────────────────
    let errs = validate_for_beam(program);
    if !errs.is_empty() {
        return Err(BEAMBackendError::ValidationFailed(errs));
    }
    if config.module_name.is_empty() {
        return Err(BEAMBackendError::ValidationFailed(
            vec!["module_name must not be empty".to_string()],
        ));
    }

    // ── Atom table setup ──────────────────────────────────────────────────
    let mut atoms = _AtomTable::new();
    let mod_idx    = atoms.intern(&config.module_name); // 1 — module name (must be first)
    let run_idx    = atoms.intern("run");               // 2 — exported function
    let erlang_idx = atoms.intern("erlang");            // 3 — for arithmetic BIFs
    let plus_idx   = atoms.intern("+");                 // 4
    let minus_idx  = atoms.intern("-");                 // 5
    let band_idx   = atoms.intern("band");              // 6
    let times_idx  = atoms.intern("*");                 // 7 — for Mul
    let div_idx    = atoms.intern("div");               // 8 — for Div (Erlang integer div)

    // ── Import table setup ────────────────────────────────────────────────
    // Pre-register arithmetic BIFs so their indices are stable.
    let mut imports = _ImportTable::new();
    let import_plus  = imports.intern(erlang_idx, plus_idx, 2);
    let import_minus = imports.intern(erlang_idx, minus_idx, 2);
    let import_band  = imports.intern(erlang_idx, band_idx, 2);
    let import_times = imports.intern(erlang_idx, times_idx, 2);
    let import_div   = imports.intern(erlang_idx, div_idx, 2);

    // ── Label first-pass ──────────────────────────────────────────────────
    // Assign BEAM label numbers starting at 3 (1 and 2 are reserved for
    // the func_info preamble).
    let (label_map, next_label) = collect_labels(program);

    // ── Register sizing ───────────────────────────────────────────────────
    // The scratch register is one above the highest user register so we
    // never clobber live user data when synthesising ADD_IMM / AND_IMM.
    let max_reg = max_register_used(program);
    // Use saturating_add before .min(255) to avoid overflow before the guard.
    let scratch: u8 = max_reg.saturating_add(1).min(255) as u8;
    let live: u64 = max_reg.saturating_add(2) as u64; // +2: include scratch

    // ── Instruction emission ──────────────────────────────────────────────
    let mut instrs: Vec<BEAMInstruction> = Vec::new();

    // Preamble: func_info block for run/0
    //   {label, 1}.
    //   {func_info, {a,Mod}, {a,run}, {u,0}}.
    //   {label, 2}.
    instrs.push(BEAMInstruction::new(OP_LABEL,     vec![BEAMOperand::u(1)]));
    instrs.push(BEAMInstruction::new(OP_FUNC_INFO, vec![
        BEAMOperand::a(mod_idx),
        BEAMOperand::a(run_idx),
        BEAMOperand::u(0),
    ]));
    instrs.push(BEAMInstruction::new(OP_LABEL,     vec![BEAMOperand::u(2)]));

    // Translate IR instructions
    for instr in &program.instructions {
        match instr.opcode {
            // ── No-ops ─────────────────────────────────────────────────────
            IrOp::Nop | IrOp::Comment => {
                // Produce no BEAM instructions — comments and no-ops vanish.
            }

            // ── Label definition ───────────────────────────────────────────
            IrOp::Label => {
                let name = label_name(instr.operands.first().ok_or_else(|| {
                    BEAMBackendError::InvalidOperand("LABEL requires one operand".into())
                })?)?;
                let beam_lbl = *label_map.get(&name)
                    .ok_or_else(|| BEAMBackendError::UndefinedLabel(name.clone()))?;
                instrs.push(BEAMInstruction::new(OP_LABEL, vec![BEAMOperand::u(beam_lbl as u64)]));
            }

            // ── Load immediate ─────────────────────────────────────────────
            // LOAD_IMM v_dst, imm  →  move {i,imm} {x,dst}
            IrOp::LoadImm => {
                let dst = reg_idx(&instr.operands[0])? as u8;
                let val = imm_val(&instr.operands[1])?;
                instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                    BEAMOperand::i(val as u64), // works for non-negative; bit-pattern for negative
                    BEAMOperand::x(dst),
                ]));
            }

            // ── Register-register addition ─────────────────────────────────
            // ADD v_dst, v_src1, v_src2  →  gc_bif2 erlang:+/2 src1 src2 dst
            IrOp::Add => {
                let dst  = reg_idx(&instr.operands[0])? as u8;
                let src1 = reg_idx(&instr.operands[1])? as u8;
                let src2 = reg_idx(&instr.operands[2])? as u8;
                instrs.push(emit_gc_bif2(import_plus, live, src1, src2, dst));
            }

            // ── Register-immediate addition ────────────────────────────────
            // ADD_IMM v_dst, v_src, imm  →  move {i,imm} scratch; gc_bif2 src scratch dst
            IrOp::AddImm => {
                let dst = reg_idx(&instr.operands[0])? as u8;
                let src = reg_idx(&instr.operands[1])? as u8;
                let val = imm_val(&instr.operands[2])?;
                instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                    BEAMOperand::i(val as u64),
                    BEAMOperand::x(scratch),
                ]));
                instrs.push(emit_gc_bif2(import_plus, live, src, scratch, dst));
            }

            // ── Subtraction ────────────────────────────────────────────────
            // SUB v_dst, v_src1, v_src2  →  gc_bif2 erlang:-/2 src1 src2 dst
            IrOp::Sub => {
                let dst  = reg_idx(&instr.operands[0])? as u8;
                let src1 = reg_idx(&instr.operands[1])? as u8;
                let src2 = reg_idx(&instr.operands[2])? as u8;
                instrs.push(emit_gc_bif2(import_minus, live, src1, src2, dst));
            }

            // ── Multiplication ─────────────────────────────────────────────
            // MUL v_dst, v_src1, v_src2  →  gc_bif2 erlang:*/2 src1 src2 dst
            IrOp::Mul => {
                let dst  = reg_idx(&instr.operands[0])? as u8;
                let src1 = reg_idx(&instr.operands[1])? as u8;
                let src2 = reg_idx(&instr.operands[2])? as u8;
                instrs.push(emit_gc_bif2(import_times, live, src1, src2, dst));
            }

            // ── Integer division ───────────────────────────────────────────
            // DIV v_dst, v_src1, v_src2  →  gc_bif2 erlang:div/2 src1 src2 dst
            // Erlang's `div` operator truncates toward zero, matching IR semantics.
            IrOp::Div => {
                let dst  = reg_idx(&instr.operands[0])? as u8;
                let src1 = reg_idx(&instr.operands[1])? as u8;
                let src2 = reg_idx(&instr.operands[2])? as u8;
                instrs.push(emit_gc_bif2(import_div, live, src1, src2, dst));
            }

            // ── Bitwise AND ────────────────────────────────────────────────
            // AND v_dst, v_src1, v_src2  →  gc_bif2 erlang:band/2 src1 src2 dst
            IrOp::And => {
                let dst  = reg_idx(&instr.operands[0])? as u8;
                let src1 = reg_idx(&instr.operands[1])? as u8;
                let src2 = reg_idx(&instr.operands[2])? as u8;
                instrs.push(emit_gc_bif2(import_band, live, src1, src2, dst));
            }

            // ── Bitwise AND with immediate ─────────────────────────────────
            // AND_IMM v_dst, v_src, imm  →  move {i,imm} scratch; gc_bif2 band src scratch dst
            IrOp::AndImm => {
                let dst = reg_idx(&instr.operands[0])? as u8;
                let src = reg_idx(&instr.operands[1])? as u8;
                let val = imm_val(&instr.operands[2])?;
                instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                    BEAMOperand::i(val as u64),
                    BEAMOperand::x(scratch),
                ]));
                instrs.push(emit_gc_bif2(import_band, live, src, scratch, dst));
            }

            // ── Unconditional jump ─────────────────────────────────────────
            // JUMP label  →  jump {f,beam_label}
            IrOp::Jump => {
                let name = label_name(&instr.operands[0])?;
                let beam_lbl = *label_map.get(&name)
                    .ok_or_else(|| BEAMBackendError::UndefinedLabel(name.clone()))?;
                instrs.push(BEAMInstruction::new(OP_JUMP, vec![BEAMOperand::f(beam_lbl)]));
            }

            // ── Branch if zero ─────────────────────────────────────────────
            // BRANCH_Z v_reg, label  →  branch to label when reg == 0
            //
            // BEAM: `is_ne_exact {f,Fail}, {x,Reg}, {i,0}`
            //   • if Reg != 0: test passes → fall through (skip the jump)
            //   • if Reg == 0: test fails  → jump to Fail (our target label)
            IrOp::BranchZ => {
                let reg = reg_idx(&instr.operands[0])? as u8;
                let name = label_name(&instr.operands[1])?;
                let beam_lbl = *label_map.get(&name)
                    .ok_or_else(|| BEAMBackendError::UndefinedLabel(name.clone()))?;
                instrs.push(BEAMInstruction::new(OP_IS_NE_EXACT, vec![
                    BEAMOperand::f(beam_lbl),
                    BEAMOperand::x(reg),
                    BEAMOperand::i(0),
                ]));
            }

            // ── Branch if non-zero ─────────────────────────────────────────
            // BRANCH_NZ v_reg, label  →  branch to label when reg != 0
            //
            // BEAM: `is_eq_exact {f,Fail}, {x,Reg}, {i,0}`
            //   • if Reg == 0: test passes → fall through (skip the jump)
            //   • if Reg != 0: test fails  → jump to Fail (our target label)
            IrOp::BranchNz => {
                let reg = reg_idx(&instr.operands[0])? as u8;
                let name = label_name(&instr.operands[1])?;
                let beam_lbl = *label_map.get(&name)
                    .ok_or_else(|| BEAMBackendError::UndefinedLabel(name.clone()))?;
                instrs.push(BEAMInstruction::new(OP_IS_EQ_EXACT, vec![
                    BEAMOperand::f(beam_lbl),
                    BEAMOperand::x(reg),
                    BEAMOperand::i(0),
                ]));
            }

            // ── Call ───────────────────────────────────────────────────────
            // CALL label  →  call {u,0} {f,label}
            //
            // v1 only supports arity-0 calls.  The CALL instruction saves the
            // return address on the BEAM call stack and jumps to the label.
            IrOp::Call => {
                let name = label_name(&instr.operands[0])?;
                let beam_lbl = *label_map.get(&name)
                    .ok_or_else(|| BEAMBackendError::UndefinedLabel(name.clone()))?;
                instrs.push(BEAMInstruction::new(OP_CALL, vec![
                    BEAMOperand::u(0),          // arity (always 0 in v1)
                    BEAMOperand::f(beam_lbl),   // call target
                ]));
            }

            // ── Return / halt ──────────────────────────────────────────────
            // Both RET and HALT lower to BEAM `return`.
            IrOp::Ret | IrOp::Halt => {
                instrs.push(BEAMInstruction::new(OP_RETURN, vec![]));
            }

            // ── Unsupported — caught by validate_for_beam ──────────────────
            IrOp::LoadByte | IrOp::StoreByte
            | IrOp::LoadWord | IrOp::StoreWord
            | IrOp::LoadAddr
            | IrOp::Syscall
            | IrOp::CmpEq | IrOp::CmpNe | IrOp::CmpLt | IrOp::CmpGt => {
                return Err(BEAMBackendError::UnsupportedOp(instr.opcode.to_string()));
            }
        }
    }

    // ── INT_CODE_END ──────────────────────────────────────────────────────
    instrs.push(BEAMInstruction::new(OP_INT_CODE_END, vec![]));

    // ── Module assembly ───────────────────────────────────────────────────
    // label_count = highest used label + 1 (for the BEAM loader's label table)
    let label_count = next_label; // next_label is already "max + 1"

    Ok(BEAMModule {
        name: config.module_name.clone(),
        atoms: atoms.all().to_vec(),
        instructions: instrs,
        imports: imports.imports,
        exports: vec![
            BEAMExport { function_atom_index: run_idx, arity: 0, label: 2 },
        ],
        locals: vec![],
        label_count,
        max_opcode: 0, // let encode_beam derive from instructions
        instruction_set_version: 0,
        extra_chunks: beam_attr_cinf_chunks(),
    })
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::{IrInstruction, IrOperand, IrProgram};

    /// Build a minimal valid program with just HALT.
    fn halt_prog() -> IrProgram {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
        p
    }

    fn default_cfg() -> BEAMBackendConfig {
        BEAMBackendConfig { module_name: "testmod".to_string() }
    }

    // ── Validation ────────────────────────────────────────────────────────

    #[test]
    fn validate_empty_program_is_ok() {
        let prog = IrProgram::new("_start");
        assert!(validate_for_beam(&prog).is_empty());
    }

    #[test]
    fn validate_empty_entry_label_errors() {
        let prog = IrProgram::new("");
        let errs = validate_for_beam(&prog);
        assert!(!errs.is_empty());
        assert!(errs[0].contains("entry label"));
    }

    #[test]
    fn validate_load_byte_unsupported() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadByte,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            0,
        ));
        let errs = validate_for_beam(&prog);
        assert!(!errs.is_empty());
        assert!(errs[0].contains("LOAD_BYTE"));
    }

    #[test]
    fn validate_syscall_unsupported() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(IrOp::Syscall, vec![], 0));
        let errs = validate_for_beam(&prog);
        assert!(!errs.is_empty());
    }

    #[test]
    fn validate_cmp_eq_unsupported() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::CmpEq,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            0,
        ));
        assert!(!validate_for_beam(&prog).is_empty());
    }

    #[test]
    fn empty_module_name_errors() {
        let prog = halt_prog();
        let cfg = BEAMBackendConfig { module_name: "".to_string() };
        assert!(lower_ir_to_beam(&prog, &cfg).is_err());
    }

    // ── Module structure ──────────────────────────────────────────────────

    #[test]
    fn module_name_is_set() {
        let module = lower_ir_to_beam(&halt_prog(), &default_cfg()).unwrap();
        assert_eq!(module.name, "testmod");
    }

    #[test]
    fn atom_table_first_entry_is_module_name() {
        let module = lower_ir_to_beam(&halt_prog(), &default_cfg()).unwrap();
        assert_eq!(module.atoms[0], "testmod"); // 0-indexed in Vec, 1-based in BEAM
    }

    #[test]
    fn atom_table_contains_erlang() {
        let module = lower_ir_to_beam(&halt_prog(), &default_cfg()).unwrap();
        assert!(module.atoms.contains(&"erlang".to_string()));
    }

    #[test]
    fn exports_run_at_label_2() {
        let module = lower_ir_to_beam(&halt_prog(), &default_cfg()).unwrap();
        assert_eq!(module.exports.len(), 1);
        assert_eq!(module.exports[0].label, 2);
        assert_eq!(module.exports[0].arity, 0);
        let run_atom_idx = module.exports[0].function_atom_index;
        assert_eq!(module.atoms[(run_atom_idx - 1) as usize], "run");
    }

    #[test]
    fn preamble_is_label_funcinfo_label() {
        let module = lower_ir_to_beam(&halt_prog(), &default_cfg()).unwrap();
        assert_eq!(module.instructions[0].opcode, OP_LABEL);    // {label,1}
        assert_eq!(module.instructions[1].opcode, OP_FUNC_INFO);
        assert_eq!(module.instructions[2].opcode, OP_LABEL);    // {label,2}
    }

    #[test]
    fn last_instruction_is_int_code_end() {
        let module = lower_ir_to_beam(&halt_prog(), &default_cfg()).unwrap();
        let last = module.instructions.last().unwrap();
        assert_eq!(last.opcode, OP_INT_CODE_END);
    }

    // ── Instruction lowering ──────────────────────────────────────────────

    #[test]
    fn load_imm_emits_move() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(42)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        let has_move = module.instructions.iter().any(|i| i.opcode == OP_MOVE);
        assert!(has_move, "expected MOVE instruction for LOAD_IMM");
    }

    #[test]
    fn add_emits_gc_bif2() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Add,
            vec![IrOperand::Register(2), IrOperand::Register(0), IrOperand::Register(1)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        assert!(module.instructions.iter().any(|i| i.opcode == OP_GC_BIF2));
    }

    #[test]
    fn sub_emits_gc_bif2_with_minus() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Sub,
            vec![IrOperand::Register(2), IrOperand::Register(0), IrOperand::Register(1)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        // erlang:- is in atom table
        assert!(module.atoms.contains(&"-".to_string()));
    }

    #[test]
    fn and_emits_gc_bif2_with_band() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::And,
            vec![IrOperand::Register(2), IrOperand::Register(0), IrOperand::Register(1)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        assert!(module.atoms.contains(&"band".to_string()));
    }

    #[test]
    fn add_imm_emits_move_then_gc_bif2() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::AddImm,
            vec![IrOperand::Register(0), IrOperand::Register(0), IrOperand::Immediate(1)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        // Should have at least one MOVE (preamble doesn't count) and one GC_BIF2
        let gc_count = module.instructions.iter().filter(|i| i.opcode == OP_GC_BIF2).count();
        assert_eq!(gc_count, 1);
    }

    #[test]
    fn label_emits_beam_label() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("loop".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        // labels 1, 2 from preamble, plus the "loop" label (≥ 3)
        let label_ops: Vec<_> = module.instructions.iter()
            .filter(|i| i.opcode == OP_LABEL)
            .collect();
        assert_eq!(label_ops.len(), 3); // 1, 2, plus IR "loop"
    }

    #[test]
    fn jump_emits_jump_instruction() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("end".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Jump,
            vec![IrOperand::Label("end".to_string())],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        assert!(module.instructions.iter().any(|i| i.opcode == OP_JUMP));
    }

    #[test]
    fn branch_z_emits_is_ne_exact() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("done".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::BranchZ,
            vec![IrOperand::Register(0), IrOperand::Label("done".to_string())],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        assert!(module.instructions.iter().any(|i| i.opcode == OP_IS_NE_EXACT));
    }

    #[test]
    fn branch_nz_emits_is_eq_exact() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("loop".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::BranchNz,
            vec![IrOperand::Register(0), IrOperand::Label("loop".to_string())],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        assert!(module.instructions.iter().any(|i| i.opcode == OP_IS_EQ_EXACT));
    }

    #[test]
    fn call_emits_call_instruction() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("sub".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Call,
            vec![IrOperand::Label("sub".to_string())],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        assert!(module.instructions.iter().any(|i| i.opcode == OP_CALL));
    }

    #[test]
    fn halt_emits_return() {
        let module = lower_ir_to_beam(&halt_prog(), &default_cfg()).unwrap();
        assert!(module.instructions.iter().any(|i| i.opcode == OP_RETURN));
    }

    #[test]
    fn ret_emits_return() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(IrOp::Ret, vec![], 0));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        assert!(module.instructions.iter().any(|i| i.opcode == OP_RETURN));
    }

    #[test]
    fn nop_emits_nothing_extra() {
        let mut prog_nop = IrProgram::new("_start");
        prog_nop.add_instruction(IrInstruction::new(IrOp::Nop, vec![], 0));
        prog_nop.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));

        let prog_bare = halt_prog();

        let m_nop  = lower_ir_to_beam(&prog_nop,  &default_cfg()).unwrap();
        let m_bare = lower_ir_to_beam(&prog_bare, &default_cfg()).unwrap();

        // NOP should produce no extra instructions
        assert_eq!(m_nop.instructions.len(), m_bare.instructions.len());
    }

    #[test]
    fn comment_emits_nothing_extra() {
        let mut prog_cmt = IrProgram::new("_start");
        prog_cmt.add_instruction(IrInstruction::new(
            IrOp::Comment,
            vec![IrOperand::Label("some comment".to_string())],
            0,
        ));
        prog_cmt.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let m_cmt  = lower_ir_to_beam(&prog_cmt,  &default_cfg()).unwrap();
        let m_bare = lower_ir_to_beam(&halt_prog(), &default_cfg()).unwrap();
        assert_eq!(m_cmt.instructions.len(), m_bare.instructions.len());
    }

    #[test]
    fn import_table_has_plus_for_add() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Add,
            vec![IrOperand::Register(2), IrOperand::Register(0), IrOperand::Register(1)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let module = lower_ir_to_beam(&prog, &default_cfg()).unwrap();
        // The import table must contain erlang:+/2
        let has_plus = module.imports.iter().any(|imp| {
            let m_atom = &module.atoms[(imp.module_atom_index - 1) as usize];
            let f_atom = &module.atoms[(imp.function_atom_index - 1) as usize];
            m_atom == "erlang" && f_atom == "+" && imp.arity == 2
        });
        assert!(has_plus, "erlang:+/2 not in import table");
    }

    #[test]
    fn encode_beam_round_trip_starts_with_for1() {
        use crate::encoder::encode_beam;
        let module = lower_ir_to_beam(&halt_prog(), &default_cfg()).unwrap();
        let bytes = encode_beam(&module);
        assert_eq!(&bytes[0..4], b"FOR1");
        assert_eq!(&bytes[8..12], b"BEAM");
    }
}
