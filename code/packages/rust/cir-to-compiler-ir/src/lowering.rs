//! # Lowering — `CIRInstr` list → `IrProgram`
//!
//! This module implements the two-pass algorithm that converts a flat list of
//! CIR instructions (produced by `jit_core::specialise`) into an `IrProgram`
//! ready for any of the AOT backends (`ir-to-wasm-compiler`, `ir-to-jvm-class-file`,
//! `ir-to-intel-4004-compiler`, etc.).
//!
//! ## Two-pass algorithm
//!
//! ### Pass 1 — collect variables
//!
//! Walk every `CIRInstr` and assign a unique virtual register index to each
//! unique variable name seen in `dest` or `srcs`. This gives us a stable
//! `name → register` map before we emit a single instruction.
//!
//! Why do this in a separate pass? The IR needs to know register indices
//! before it emits any instruction. Doing it lazily in a single pass would
//! require backpatching operand lists, which is error-prone.
//!
//! ### Pass 2 — emit IR instructions
//!
//! Walk the CIR instructions again. For each one, dispatch on the `op` prefix
//! and emit one or more `IrInstruction` objects.
//!
//! ## Op mapping
//!
//! | CIR op | IrOp(s) emitted | Notes |
//! |---|---|---|
//! | `const_{int/bool}` | `LoadImm` | Immediate from src[0] |
//! | `add_{int}` | `Add` / `AddImm` | `AddImm` when rhs is literal |
//! | `sub_{int}` | `Sub` | rhs immediates are pre-loaded |
//! | `and_{int}` | `And` / `AndImm` | |
//! | `neg_{int}` | `LoadImm(0)` + `Sub` | 0 − src |
//! | `mul_*`, `div_*`, `or_*`, `xor_*`, `not_*` | **error** | Not in v1 `IrOp` |
//! | Float ops | **error** | No float `IrOp` variants in v1 |
//! | `cmp_eq/ne/lt/gt_{int}` | `CmpEq/Ne/Lt/Gt` | Direct mapping |
//! | `cmp_le_{int}` | `CmpGt` + `LoadImm(1)` + `Sub` | 1 − (a > b) |
//! | `cmp_ge_{int}` | `CmpLt` + `LoadImm(1)` + `Sub` | 1 − (a < b) |
//! | `label` | `Label` | dest = label name |
//! | `jmp` | `Jump` | src[0] = label name |
//! | `jmp_if_true`, `br_true_bool` | `BranchNz` | |
//! | `jmp_if_false`, `br_false_bool` | `BranchZ` | |
//! | `ret_void` / `ret_{type}` | `Halt` | Return value ignored in v1 |
//! | `type_assert` | `Comment` | Guards fired at runtime by vm-core |
//! | `call` | `Call` | src[0] = callee label |
//! | `tetrad.move` | `AddImm(_, src, 0)` | Copy via identity add |
//! | `load_mem` | `LoadWord` | dest, base, offset |
//! | `store_mem` | `StoreWord` | val, base, offset |
//! | `call_builtin` | `Syscall` | putchar/getchar via OS syscall |
//! | `call_runtime`, `io_in`, `io_out` | **error** | No v1 equivalent |
//! | unknown | **error** | |

use std::collections::HashMap;

use compiler_ir::{
    types::{IrProgram, IrInstruction, IrOperand, IdGenerator},
    opcodes::IrOp,
};
use jit_core::{CIRInstr, CIROperand};

use crate::errors::CIRLoweringError;

// ===========================================================================
// CIRLowerer — internal two-pass lowering state
// ===========================================================================

/// Internal state for the two-pass CIR → IrProgram lowering.
///
/// This is not part of the public API. Callers use `lower_cir_to_ir_program`.
struct CIRLowerer {
    /// Maps variable names to virtual register indices.
    ///
    /// All indices are ≥ 0 and assigned monotonically in pass 1. The same
    /// name always maps to the same register — this is the key invariant that
    /// makes the lowering correct.
    regs: HashMap<String, usize>,

    /// The next register index to assign.
    next_reg: usize,

    /// Monotonic ID generator for IR instructions.
    id_gen: IdGenerator,

    /// The instruction stream being built.
    instrs: Vec<IrInstruction>,
}

impl CIRLowerer {
    // ── Constructor ─────────────────────────────────────────────────────────

    fn new() -> Self {
        CIRLowerer {
            regs: HashMap::new(),
            next_reg: 0,
            id_gen: IdGenerator::new(),
            instrs: Vec::new(),
        }
    }

    // ── Register allocation ─────────────────────────────────────────────────

    /// Look up the register for `name`, allocating a new index if first seen.
    ///
    /// # Guarantees
    /// - The same `name` always returns the same index.
    /// - Indices are assigned in the order first-seen during pass 1.
    fn var(&mut self, name: &str) -> usize {
        if let Some(&idx) = self.regs.get(name) {
            return idx;
        }
        let idx = self.next_reg;
        self.regs.insert(name.to_string(), idx);
        self.next_reg += 1;
        idx
    }

    /// Allocate a fresh scratch register that no variable name will alias.
    ///
    /// Scratch registers are used for synthesised multi-instruction sequences
    /// (e.g., `neg` = `LoadImm(0)` + `Sub`, `cmp_le` = `CmpGt` + `LoadImm(1)` + `Sub`).
    fn fresh(&mut self) -> usize {
        let idx = self.next_reg;
        self.next_reg += 1;
        idx
    }

    // ── Instruction emission ────────────────────────────────────────────────

    /// Emit an IR instruction with a unique monotonic ID.
    fn emit(&mut self, opcode: IrOp, operands: Vec<IrOperand>) {
        let id = self.id_gen.next();
        self.instrs.push(IrInstruction::new(opcode, operands, id));
    }

    /// Emit a `Label` pseudo-instruction with id = −1 (labels produce no code).
    fn emit_label(&mut self, name: &str) {
        self.instrs.push(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label(name.to_string())],
            -1,
        ));
    }

    // ── Pass 1 — variable collection ────────────────────────────────────────

    /// Walk the instruction list and pre-allocate a register for every
    /// variable name seen. After this pass, `self.regs` is complete.
    fn collect_vars(&mut self, instrs: &[CIRInstr]) {
        for instr in instrs {
            if let Some(dest) = &instr.dest {
                self.var(dest);
            }
            for src in &instr.srcs {
                if let CIROperand::Var(name) = src {
                    self.var(name);
                }
            }
        }
    }

    // ── Operand helpers ─────────────────────────────────────────────────────

    /// Convert a `CIROperand` to an `IrOperand`.
    ///
    /// - `Var(name)` → `Register(index)` — looks up (or allocates) the register.
    /// - `Int(v)` → `Immediate(v)`.
    /// - `Bool(b)` → `Immediate(1 or 0)`.
    /// - `Float(_)` → error (no float `IrOperand` in v1).
    fn cir_op_to_ir(&mut self, op: &CIROperand) -> Result<IrOperand, CIRLoweringError> {
        match op {
            CIROperand::Var(name) => Ok(IrOperand::Register(self.var(name))),
            CIROperand::Int(v) => Ok(IrOperand::Immediate(*v)),
            CIROperand::Bool(b) => Ok(IrOperand::Immediate(if *b { 1 } else { 0 })),
            CIROperand::Float(_) => Err(CIRLoweringError::new(
                "float operands are not supported in v1 IR lowering",
            )),
        }
    }

    /// Get the `dest` field or return a `CIRLoweringError`.
    fn require_dest<'a>(&self, instr: &'a CIRInstr) -> Result<&'a str, CIRLoweringError> {
        instr
            .dest
            .as_deref()
            .ok_or_else(|| CIRLoweringError::new(format!("op '{}' requires a dest", instr.op)))
    }

    /// Get `srcs[i]` or return a `CIRLoweringError`.
    fn require_src<'a>(&self, instr: &'a CIRInstr, i: usize) -> Result<&'a CIROperand, CIRLoweringError> {
        instr.srcs.get(i).ok_or_else(|| {
            CIRLoweringError::new(format!("op '{}' requires srcs[{}]", instr.op, i))
        })
    }

    /// Get `srcs[i]` as a variable name (register) or return a `CIRLoweringError`.
    fn require_var_src(&mut self, instr: &CIRInstr, i: usize) -> Result<usize, CIRLoweringError> {
        let src = self.require_src(instr, i)?;
        match src {
            CIROperand::Var(name) => Ok(self.var(name)),
            other => Err(CIRLoweringError::new(format!(
                "op '{}' srcs[{}] expected Var, got {:?}",
                instr.op, i, other
            ))),
        }
    }

    /// Get `srcs[i]` as a label string or return a `CIRLoweringError`.
    fn require_label_src(&self, instr: &CIRInstr, i: usize) -> Result<String, CIRLoweringError> {
        let src = self.require_src(instr, i)?;
        match src {
            CIROperand::Var(name) => Ok(name.clone()),
            other => Err(CIRLoweringError::new(format!(
                "op '{}' srcs[{}] expected label (Var), got {:?}",
                instr.op, i, other
            ))),
        }
    }

    /// Get `srcs[i]` as an immediate integer or return a `CIRLoweringError`.
    fn require_int_or_bool_src(&self, instr: &CIRInstr, i: usize) -> Result<i64, CIRLoweringError> {
        let src = self.require_src(instr, i)?;
        match src {
            CIROperand::Int(v) => Ok(*v),
            CIROperand::Bool(b) => Ok(if *b { 1 } else { 0 }),
            other => Err(CIRLoweringError::new(format!(
                "op '{}' srcs[{}] expected Int/Bool, got {:?}",
                instr.op, i, other
            ))),
        }
    }

    // ── Helpers for common patterns ─────────────────────────────────────────

    /// Emit a binary integer op: `dest = op(lhs, rhs)`.
    ///
    /// Automatically chooses the immediate-form opcode (`AddImm`, `AndImm`)
    /// when `rhs` is a literal integer. Falls back to the register form for
    /// `Sub` (which has no immediate variant) by pre-loading the immediate.
    fn emit_binary_int(
        &mut self,
        instr: &CIRInstr,
        reg_op: IrOp,
        imm_op: Option<IrOp>,
    ) -> Result<(), CIRLoweringError> {
        if instr.ty == "f32" || instr.ty == "f64" {
            return Err(CIRLoweringError::new(format!(
                "float arithmetic op '{}' is not supported in v1 IR lowering",
                instr.op
            )));
        }
        let dest = self.require_dest(instr)?;
        let dest_name = dest.to_string();
        let dreg = self.var(&dest_name);

        let lhs_op = {
            let src = self.require_src(instr, 0)?;
            self.cir_op_to_ir(src)?
        };
        let rhs_op = {
            let src = self.require_src(instr, 1)?;
            self.cir_op_to_ir(src)?
        };

        match (&rhs_op, imm_op) {
            (IrOperand::Immediate(_), Some(iop)) => {
                // Use the immediate form if available.
                self.emit(iop, vec![IrOperand::Register(dreg), lhs_op, rhs_op]);
            }
            (IrOperand::Immediate(v), None) => {
                // No immediate form — pre-load the constant into a scratch register.
                let scratch = self.fresh();
                let v = *v;
                self.emit(
                    IrOp::LoadImm,
                    vec![IrOperand::Register(scratch), IrOperand::Immediate(v)],
                );
                self.emit(
                    reg_op,
                    vec![
                        IrOperand::Register(dreg),
                        lhs_op,
                        IrOperand::Register(scratch),
                    ],
                );
            }
            _ => {
                self.emit(reg_op, vec![IrOperand::Register(dreg), lhs_op, rhs_op]);
            }
        }
        Ok(())
    }

    /// Emit a comparison: `dest = cmp_op(lhs, rhs)`.
    ///
    /// All four basic comparisons (`CmpEq`, `CmpNe`, `CmpLt`, `CmpGt`) follow
    /// the same three-operand form: `[dest, lhs, rhs]`.
    fn emit_cmp(&mut self, instr: &CIRInstr, cmp_op: IrOp) -> Result<(), CIRLoweringError> {
        if instr.ty == "f32" || instr.ty == "f64" {
            return Err(CIRLoweringError::new(format!(
                "float comparison '{}' not supported in v1 IR lowering",
                instr.op
            )));
        }
        let dest = self.require_dest(instr)?;
        let dest_name = dest.to_string();
        let dreg = self.var(&dest_name);

        let lhs_op = {
            let src = self.require_src(instr, 0)?;
            self.cir_op_to_ir(src)?
        };
        let rhs_op = {
            let src = self.require_src(instr, 1)?;
            self.cir_op_to_ir(src)?
        };

        self.emit(cmp_op, vec![IrOperand::Register(dreg), lhs_op, rhs_op]);
        Ok(())
    }

    /// Emit `cmp_le(a, b)` as `1 − CmpGt(a, b)`.
    ///
    /// # Why this synthesis?
    ///
    /// The v1 `IrOp` set has `CmpGt` but no `CmpLe` and no `Not`. However,
    /// because `CmpGt` returns exactly 0 or 1, we can compute the logical NOT
    /// using integer subtraction: `1 − x` flips 0↔1. This requires three
    /// IR instructions and two scratch registers.
    fn emit_cmp_le(&mut self, instr: &CIRInstr) -> Result<(), CIRLoweringError> {
        if instr.ty == "f32" || instr.ty == "f64" {
            return Err(CIRLoweringError::new(
                "float cmp_le not supported in v1 IR lowering",
            ));
        }
        let dest = self.require_dest(instr)?;
        let dest_name = dest.to_string();

        let lhs_op = {
            let src = self.require_src(instr, 0)?;
            self.cir_op_to_ir(src)?
        };
        let rhs_op = {
            let src = self.require_src(instr, 1)?;
            self.cir_op_to_ir(src)?
        };

        // Allocate scratch registers before dreg to avoid borrow conflict.
        let scratch_gt = self.fresh();
        let scratch_one = self.fresh();
        let dreg = self.var(&dest_name);

        // scratch_gt = CmpGt(lhs, rhs)
        self.emit(
            IrOp::CmpGt,
            vec![IrOperand::Register(scratch_gt), lhs_op, rhs_op],
        );
        // scratch_one = 1
        self.emit(
            IrOp::LoadImm,
            vec![
                IrOperand::Register(scratch_one),
                IrOperand::Immediate(1),
            ],
        );
        // dest = 1 − scratch_gt  (= NOT(a > b) = a ≤ b)
        self.emit(
            IrOp::Sub,
            vec![
                IrOperand::Register(dreg),
                IrOperand::Register(scratch_one),
                IrOperand::Register(scratch_gt),
            ],
        );
        Ok(())
    }

    /// Emit `cmp_ge(a, b)` as `1 − CmpLt(a, b)`.
    ///
    /// Mirrors `emit_cmp_le` but flips the inner comparison to `CmpLt`.
    fn emit_cmp_ge(&mut self, instr: &CIRInstr) -> Result<(), CIRLoweringError> {
        if instr.ty == "f32" || instr.ty == "f64" {
            return Err(CIRLoweringError::new(
                "float cmp_ge not supported in v1 IR lowering",
            ));
        }
        let dest = self.require_dest(instr)?;
        let dest_name = dest.to_string();

        let lhs_op = {
            let src = self.require_src(instr, 0)?;
            self.cir_op_to_ir(src)?
        };
        let rhs_op = {
            let src = self.require_src(instr, 1)?;
            self.cir_op_to_ir(src)?
        };

        let scratch_lt = self.fresh();
        let scratch_one = self.fresh();
        let dreg = self.var(&dest_name);

        // scratch_lt = CmpLt(lhs, rhs)
        self.emit(
            IrOp::CmpLt,
            vec![IrOperand::Register(scratch_lt), lhs_op, rhs_op],
        );
        // scratch_one = 1
        self.emit(
            IrOp::LoadImm,
            vec![
                IrOperand::Register(scratch_one),
                IrOperand::Immediate(1),
            ],
        );
        // dest = 1 − scratch_lt  (= NOT(a < b) = a ≥ b)
        self.emit(
            IrOp::Sub,
            vec![
                IrOperand::Register(dreg),
                IrOperand::Register(scratch_one),
                IrOperand::Register(scratch_lt),
            ],
        );
        Ok(())
    }

    // ── Pass 2 — single instruction lowering ────────────────────────────────

    /// Lower one CIR instruction, appending IR instructions to `self.instrs`.
    ///
    /// Returns `Ok(())` on success or a `CIRLoweringError` for unsupported ops.
    #[allow(clippy::too_many_lines)] // necessary: each arm is short but there are many ops
    fn lower_one(&mut self, instr: &CIRInstr) -> Result<(), CIRLoweringError> {
        let op = instr.op.as_str();

        // ── Constants ───────────────────────────────────────────────────────
        if op.starts_with("const_") {
            if instr.ty == "f32" || instr.ty == "f64" {
                return Err(CIRLoweringError::new(format!(
                    "float constants ('{}') not supported in v1 IR lowering",
                    op
                )));
            }
            let dest = self.require_dest(instr)?;
            let dest_name = dest.to_string();
            let dreg = self.var(&dest_name);
            let val = self.require_int_or_bool_src(instr, 0)?;
            self.emit(
                IrOp::LoadImm,
                vec![IrOperand::Register(dreg), IrOperand::Immediate(val)],
            );
            return Ok(());
        }

        // ── Integer / bitwise arithmetic ────────────────────────────────────

        if op.starts_with("add_") {
            return self.emit_binary_int(instr, IrOp::Add, Some(IrOp::AddImm));
        }
        if op.starts_with("sub_") {
            // Sub has no immediate form — emit_binary_int will pre-load constants.
            return self.emit_binary_int(instr, IrOp::Sub, None);
        }
        if op.starts_with("and_") {
            return self.emit_binary_int(instr, IrOp::And, Some(IrOp::AndImm));
        }
        if op.starts_with("mul_") {
            return Err(CIRLoweringError::new(format!(
                "op '{}' (MUL) is not in the v1 IrOp set; \
                 add Mul to compiler-ir and the AOT backends first",
                op
            )));
        }
        if op.starts_with("div_") {
            return Err(CIRLoweringError::new(format!(
                "op '{}' (DIV) is not in the v1 IrOp set",
                op
            )));
        }
        if op.starts_with("or_") {
            return Err(CIRLoweringError::new(format!(
                "op '{}' (OR) is not in the v1 IrOp set",
                op
            )));
        }
        if op.starts_with("xor_") {
            return Err(CIRLoweringError::new(format!(
                "op '{}' (XOR) is not in the v1 IrOp set",
                op
            )));
        }
        if op.starts_with("not_") {
            return Err(CIRLoweringError::new(format!(
                "op '{}' (NOT) is not in the v1 IrOp set",
                op
            )));
        }

        // neg: 0 − src  (two instructions, one scratch register)
        if op.starts_with("neg_") {
            if instr.ty == "f32" || instr.ty == "f64" {
                return Err(CIRLoweringError::new("float neg not supported in v1 IR lowering"));
            }
            let dest = self.require_dest(instr)?;
            let dest_name = dest.to_string();
            let src_reg = self.require_var_src(instr, 0)?;
            let scratch = self.fresh();
            let dreg = self.var(&dest_name);

            // scratch = 0
            self.emit(
                IrOp::LoadImm,
                vec![IrOperand::Register(scratch), IrOperand::Immediate(0)],
            );
            // dest = 0 − src
            self.emit(
                IrOp::Sub,
                vec![
                    IrOperand::Register(dreg),
                    IrOperand::Register(scratch),
                    IrOperand::Register(src_reg),
                ],
            );
            return Ok(());
        }

        // ── Comparisons ─────────────────────────────────────────────────────

        if op.starts_with("cmp_eq_") { return self.emit_cmp(instr, IrOp::CmpEq); }
        if op.starts_with("cmp_ne_") { return self.emit_cmp(instr, IrOp::CmpNe); }
        if op.starts_with("cmp_lt_") { return self.emit_cmp(instr, IrOp::CmpLt); }
        if op.starts_with("cmp_gt_") { return self.emit_cmp(instr, IrOp::CmpGt); }
        if op.starts_with("cmp_le_") { return self.emit_cmp_le(instr); }
        if op.starts_with("cmp_ge_") { return self.emit_cmp_ge(instr); }

        // Float comparisons (handled after integer prefix checks to be safe)
        if op.starts_with("cmp_") && (instr.ty == "f32" || instr.ty == "f64") {
            return Err(CIRLoweringError::new(format!(
                "float comparison '{}' not supported in v1 IR lowering",
                op
            )));
        }

        // ── Control flow ─────────────────────────────────────────────────────

        match op {
            "label" => {
                // `dest` holds the label name; `srcs` is empty.
                let name = self.require_dest(instr)?;
                let name = name.to_string();
                self.emit_label(&name);
            }

            "jmp" => {
                // src[0] is the target label name (as a Var operand).
                let label = self.require_label_src(instr, 0)?;
                self.emit(IrOp::Jump, vec![IrOperand::Label(label)]);
            }

            "jmp_if_true" | "br_true_bool" => {
                // src[0] = condition register, src[1] = target label.
                let cond = self.require_var_src(instr, 0)?;
                let label = self.require_label_src(instr, 1)?;
                self.emit(
                    IrOp::BranchNz,
                    vec![IrOperand::Register(cond), IrOperand::Label(label)],
                );
            }

            "jmp_if_false" | "br_false_bool" => {
                // src[0] = condition register, src[1] = target label.
                let cond = self.require_var_src(instr, 0)?;
                let label = self.require_label_src(instr, 1)?;
                self.emit(
                    IrOp::BranchZ,
                    vec![IrOperand::Register(cond), IrOperand::Label(label)],
                );
            }

            "ret_void" => {
                // Function returns nothing. Emit HALT (end of program in v1).
                self.emit(IrOp::Halt, vec![]);
            }

            // ret_{type}: return value is ignored in v1. Just halt.
            o if o.starts_with("ret_") => {
                self.emit(IrOp::Halt, vec![]);
            }

            // ── Call ─────────────────────────────────────────────────────────

            "call" => {
                // src[0] = callee label name.
                let label = self.require_label_src(instr, 0)?;
                self.emit(IrOp::Call, vec![IrOperand::Label(label)]);
            }

            // ── Type guards ──────────────────────────────────────────────────

            "type_assert" => {
                // Guards are checked at runtime by vm-core during specialisation.
                // In the AOT IR we emit a human-readable comment so the disassembler
                // can show which guard this was.
                let comment = format!("type_assert: {:?}", instr.srcs);
                self.emit(IrOp::Comment, vec![IrOperand::Label(comment)]);
            }

            // ── Move / copy ──────────────────────────────────────────────────

            "tetrad.move" => {
                // dest = src  (a register copy or constant load).
                let dest = self.require_dest(instr)?;
                let dest_name = dest.to_string();
                let dreg = self.var(&dest_name);
                let src = self.require_src(instr, 0)?;
                match src {
                    CIROperand::Var(name) => {
                        let name = name.clone();
                        let sreg = self.var(&name);
                        // Copy: dest = src + 0
                        self.emit(
                            IrOp::AddImm,
                            vec![
                                IrOperand::Register(dreg),
                                IrOperand::Register(sreg),
                                IrOperand::Immediate(0),
                            ],
                        );
                    }
                    CIROperand::Int(v) => {
                        let v = *v;
                        self.emit(
                            IrOp::LoadImm,
                            vec![IrOperand::Register(dreg), IrOperand::Immediate(v)],
                        );
                    }
                    CIROperand::Bool(b) => {
                        let v = if *b { 1i64 } else { 0 };
                        self.emit(
                            IrOp::LoadImm,
                            vec![IrOperand::Register(dreg), IrOperand::Immediate(v)],
                        );
                    }
                    CIROperand::Float(_) => {
                        return Err(CIRLoweringError::new(
                            "tetrad.move with float src not supported in v1 IR lowering",
                        ));
                    }
                }
            }

            // ── Memory ───────────────────────────────────────────────────────

            "load_mem" => {
                // dest = mem[base + offset]
                let dest = self.require_dest(instr)?;
                let dest_name = dest.to_string();
                let dreg = self.var(&dest_name);
                let base = self.require_var_src(instr, 0)?;
                let offset = {
                    let src = self.require_src(instr, 1)?;
                    self.cir_op_to_ir(src)?
                };
                self.emit(
                    IrOp::LoadWord,
                    vec![
                        IrOperand::Register(dreg),
                        IrOperand::Register(base),
                        offset,
                    ],
                );
            }

            "store_mem" => {
                // mem[base + offset] = val
                let val = self.require_var_src(instr, 0)?;
                let base = self.require_var_src(instr, 1)?;
                let offset = {
                    let src = self.require_src(instr, 2)?;
                    self.cir_op_to_ir(src)?
                };
                self.emit(
                    IrOp::StoreWord,
                    vec![
                        IrOperand::Register(val),
                        IrOperand::Register(base),
                        offset,
                    ],
                );
            }

            // ── Builtin calls ─────────────────────────────────────────────────

            "call_builtin" => {
                // putchar / getchar map to a platform Syscall in v1.
                // The specific syscall number is left for the backend to fill in.
                self.emit(IrOp::Syscall, vec![]);
            }

            // ── Unsupported ──────────────────────────────────────────────────

            "call_runtime" => {
                return Err(CIRLoweringError::new(
                    "call_runtime is not supported in v1 IR lowering; \
                     use specialised runtime ABI support (future LANG)",
                ));
            }
            "io_in" => {
                return Err(CIRLoweringError::new(
                    "io_in is not supported in v1 IR lowering; \
                     use a platform-specific backend instead",
                ));
            }
            "io_out" => {
                return Err(CIRLoweringError::new(
                    "io_out is not supported in v1 IR lowering; \
                     use a platform-specific backend instead",
                ));
            }

            unknown => {
                return Err(CIRLoweringError::new(format!(
                    "unknown or unsupported CIR op: '{}'",
                    unknown
                )));
            }
        }

        Ok(())
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Lower a list of CIR instructions into an `IrProgram`.
///
/// This is the main entry point for the LANG21 bridge. Call
/// `validate_cir_for_lowering` first to get early, clear error messages;
/// `lower_cir_to_ir_program` also validates internally and returns errors for
/// unsupported ops encountered during lowering.
///
/// # Arguments
///
/// * `instrs`      — flat list of typed CIR instructions from `jit_core::specialise`.
/// * `entry_label` — the label to use as the program's entry point (e.g. `"_start"`).
///
/// # Returns
///
/// `Ok(IrProgram)` on success.
/// `Err(CIRLoweringError)` if any instruction is unsupported or malformed.
///
/// # Example
///
/// ```
/// use jit_core::{CIRInstr, CIROperand};
/// use cir_to_compiler_ir::lower_cir_to_ir_program;
/// use compiler_ir::opcodes::IrOp;
///
/// let instrs = vec![
///     CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(40)], "i32"),
///     CIRInstr::new("const_i32", Some("y".to_string()), vec![CIROperand::Int(2)],  "i32"),
///     CIRInstr::new("add_i32", Some("z".to_string()),
///                   vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
///     CIRInstr::new("ret_void", None::<String>, vec![], "void"),
/// ];
/// let prog = lower_cir_to_ir_program(&instrs, "_start").unwrap();
///
/// // The first instruction is always the entry LABEL.
/// assert_eq!(prog.instructions[0].opcode, IrOp::Label);
/// assert_eq!(prog.entry_label, "_start");
/// // LOAD_IMM x=40, LOAD_IMM y=2, ADD z=x+y, HALT
/// // (plus 1 LABEL at front → 5 total instructions)
/// assert_eq!(prog.instructions.len(), 5);
/// ```
pub fn lower_cir_to_ir_program(
    instrs: &[CIRInstr],
    entry_label: &str,
) -> Result<IrProgram, CIRLoweringError> {
    let mut lowerer = CIRLowerer::new();

    // ── Pass 1: collect all variable names → register indices ───────────────
    lowerer.collect_vars(instrs);

    // ── Emit entry LABEL ────────────────────────────────────────────────────
    // The JVM and CIL backends require a LABEL as the very first instruction.
    lowerer.emit_label(entry_label);

    // ── Pass 2: lower each CIR instruction ──────────────────────────────────
    for instr in instrs {
        lowerer.lower_one(instr)?;
    }

    // ── Assemble IrProgram ───────────────────────────────────────────────────
    let mut prog = IrProgram::new(entry_label);
    for ir_instr in lowerer.instrs {
        prog.add_instruction(ir_instr);
    }
    Ok(prog)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::opcodes::IrOp;
    use jit_core::{CIRInstr, CIROperand};

    // ── Convenience helpers ──────────────────────────────────────────────────

    fn lower(instrs: Vec<CIRInstr>) -> IrProgram {
        lower_cir_to_ir_program(&instrs, "_start").expect("lowering should succeed")
    }

    fn lower_err(instrs: Vec<CIRInstr>) -> CIRLoweringError {
        lower_cir_to_ir_program(&instrs, "_start").expect_err("expected error")
    }

    fn opcodes(prog: &IrProgram) -> Vec<IrOp> {
        prog.instructions.iter().map(|i| i.opcode).collect()
    }

    // ── Constants ────────────────────────────────────────────────────────────

    #[test]
    fn test_const_i32_emits_load_imm() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(42)], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        // [LABEL, LOAD_IMM, HALT]
        assert!(opcodes(&prog).contains(&IrOp::LoadImm));
    }

    #[test]
    fn test_const_bool_true_emits_load_imm_1() {
        let prog = lower(vec![
            CIRInstr::new("const_bool", Some("b".to_string()), vec![CIROperand::Bool(true)], "bool"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        let load = prog.instructions.iter()
            .find(|i| i.opcode == IrOp::LoadImm)
            .expect("LOAD_IMM expected");
        assert_eq!(load.operands[1], IrOperand::Immediate(1));
    }

    #[test]
    fn test_const_bool_false_emits_load_imm_0() {
        let prog = lower(vec![
            CIRInstr::new("const_bool", Some("b".to_string()), vec![CIROperand::Bool(false)], "bool"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        let load = prog.instructions.iter()
            .find(|i| i.opcode == IrOp::LoadImm)
            .expect("LOAD_IMM expected");
        assert_eq!(load.operands[1], IrOperand::Immediate(0));
    }

    #[test]
    fn test_const_f64_raises_error() {
        let err = lower_err(vec![
            CIRInstr::new("const_f64", Some("f".to_string()), vec![CIROperand::Float(3.14)], "f64"),
        ]);
        assert!(err.message.contains("float"), "expected float error, got: {}", err.message);
    }

    // ── Integer arithmetic ────────────────────────────────────────────────────

    #[test]
    fn test_add_i32_emits_add() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(1)], "i32"),
            CIRInstr::new("const_i32", Some("y".to_string()), vec![CIROperand::Int(2)], "i32"),
            CIRInstr::new("add_i32", Some("z".to_string()),
                vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::Add));
    }

    #[test]
    fn test_add_with_immediate_emits_add_imm() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(10)], "i32"),
            CIRInstr::new("add_i32", Some("z".to_string()),
                vec![CIROperand::Var("x".into()), CIROperand::Int(5)], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::AddImm));
    }

    #[test]
    fn test_sub_i32_emits_sub() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(10)], "i32"),
            CIRInstr::new("const_i32", Some("y".to_string()), vec![CIROperand::Int(3)],  "i32"),
            CIRInstr::new("sub_i32", Some("z".to_string()),
                vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::Sub));
    }

    #[test]
    fn test_and_i32_emits_and() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(0xFF)], "i32"),
            CIRInstr::new("const_i32", Some("y".to_string()), vec![CIROperand::Int(0x0F)], "i32"),
            CIRInstr::new("and_i32", Some("z".to_string()),
                vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::And));
    }

    #[test]
    fn test_neg_i32_emits_load_imm_then_sub() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(5)], "i32"),
            CIRInstr::new("neg_i32", Some("y".to_string()), vec![CIROperand::Var("x".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        let ops = opcodes(&prog);
        // Must contain both LOAD_IMM (for 0) and SUB (for 0 − x).
        assert!(ops.contains(&IrOp::LoadImm));
        assert!(ops.contains(&IrOp::Sub));
        // LOAD_IMM for 0 must appear before SUB
        let li_pos = ops.iter().rposition(|&o| o == IrOp::LoadImm).unwrap();
        let sub_pos = ops.iter().rposition(|&o| o == IrOp::Sub).unwrap();
        assert!(li_pos < sub_pos, "LOAD_IMM must precede SUB in neg synthesis");
    }

    #[test]
    fn test_mul_raises_error() {
        let err = lower_err(vec![
            CIRInstr::new("mul_i32", Some("z".to_string()),
                vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
        ]);
        assert!(err.message.contains("MUL") || err.message.contains("mul"),
            "unexpected: {}", err.message);
    }

    #[test]
    fn test_or_raises_error() {
        let err = lower_err(vec![
            CIRInstr::new("or_i32", Some("z".to_string()),
                vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
        ]);
        assert!(err.message.contains("OR") || err.message.contains("or"),
            "unexpected: {}", err.message);
    }

    #[test]
    fn test_xor_raises_error() {
        let err = lower_err(vec![
            CIRInstr::new("xor_i32", Some("z".to_string()),
                vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
        ]);
        assert!(err.message.contains("XOR") || err.message.contains("xor"),
            "unexpected: {}", err.message);
    }

    #[test]
    fn test_div_raises_error() {
        let err = lower_err(vec![
            CIRInstr::new("div_i32", Some("z".to_string()),
                vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
        ]);
        assert!(err.message.contains("DIV") || err.message.contains("div"),
            "unexpected: {}", err.message);
    }

    // ── Comparisons ─────────────────────────────────────────────────────────

    #[test]
    fn test_cmp_eq_i32_emits_cmp_eq() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("a".to_string()), vec![CIROperand::Int(1)], "i32"),
            CIRInstr::new("const_i32", Some("b".to_string()), vec![CIROperand::Int(1)], "i32"),
            CIRInstr::new("cmp_eq_i32", Some("r".to_string()),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::CmpEq));
    }

    #[test]
    fn test_cmp_ne_i32_emits_cmp_ne() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("a".to_string()), vec![CIROperand::Int(1)], "i32"),
            CIRInstr::new("const_i32", Some("b".to_string()), vec![CIROperand::Int(2)], "i32"),
            CIRInstr::new("cmp_ne_i32", Some("r".to_string()),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::CmpNe));
    }

    #[test]
    fn test_cmp_lt_i32_emits_cmp_lt() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("a".to_string()), vec![CIROperand::Int(1)], "i32"),
            CIRInstr::new("const_i32", Some("b".to_string()), vec![CIROperand::Int(2)], "i32"),
            CIRInstr::new("cmp_lt_i32", Some("r".to_string()),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::CmpLt));
    }

    #[test]
    fn test_cmp_gt_i32_emits_cmp_gt() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("a".to_string()), vec![CIROperand::Int(3)], "i32"),
            CIRInstr::new("const_i32", Some("b".to_string()), vec![CIROperand::Int(2)], "i32"),
            CIRInstr::new("cmp_gt_i32", Some("r".to_string()),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::CmpGt));
    }

    #[test]
    fn test_cmp_le_i32_synthesizes_three_instructions() {
        // cmp_le(a, b) = 1 − CmpGt(a, b)  → CmpGt + LoadImm(1) + Sub
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("a".to_string()), vec![CIROperand::Int(2)], "i32"),
            CIRInstr::new("const_i32", Some("b".to_string()), vec![CIROperand::Int(3)], "i32"),
            CIRInstr::new("cmp_le_i32", Some("r".to_string()),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        let ops = opcodes(&prog);
        assert!(ops.contains(&IrOp::CmpGt), "missing CmpGt in cmp_le synthesis");
        assert!(ops.contains(&IrOp::Sub),   "missing Sub in cmp_le synthesis");
    }

    #[test]
    fn test_cmp_ge_i32_synthesizes_three_instructions() {
        // cmp_ge(a, b) = 1 − CmpLt(a, b) → CmpLt + LoadImm(1) + Sub
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("a".to_string()), vec![CIROperand::Int(3)], "i32"),
            CIRInstr::new("const_i32", Some("b".to_string()), vec![CIROperand::Int(2)], "i32"),
            CIRInstr::new("cmp_ge_i32", Some("r".to_string()),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        let ops = opcodes(&prog);
        assert!(ops.contains(&IrOp::CmpLt), "missing CmpLt in cmp_ge synthesis");
        assert!(ops.contains(&IrOp::Sub),   "missing Sub in cmp_ge synthesis");
    }

    // ── Control flow ─────────────────────────────────────────────────────────

    #[test]
    fn test_label_emits_label_instruction() {
        let prog = lower(vec![
            CIRInstr::new("label", Some("loop_start".to_string()), vec![], "void"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        let has_label = prog.instructions.iter().any(|i| {
            i.opcode == IrOp::Label
                && i.operands.first() == Some(&IrOperand::Label("loop_start".to_string()))
        });
        assert!(has_label, "expected LABEL loop_start");
    }

    #[test]
    fn test_jmp_emits_jump() {
        let prog = lower(vec![
            CIRInstr::new("jmp", None::<String>, vec![CIROperand::Var("loop_start".into())], "void"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::Jump));
    }

    #[test]
    fn test_jmp_if_true_emits_branch_nz() {
        let prog = lower(vec![
            CIRInstr::new("const_bool", Some("c".to_string()), vec![CIROperand::Bool(true)], "bool"),
            CIRInstr::new("jmp_if_true", None::<String>,
                vec![CIROperand::Var("c".into()), CIROperand::Var("target".into())], "void"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::BranchNz));
    }

    #[test]
    fn test_jmp_if_false_emits_branch_z() {
        let prog = lower(vec![
            CIRInstr::new("const_bool", Some("c".to_string()), vec![CIROperand::Bool(false)], "bool"),
            CIRInstr::new("jmp_if_false", None::<String>,
                vec![CIROperand::Var("c".into()), CIROperand::Var("done".into())], "void"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::BranchZ));
    }

    #[test]
    fn test_ret_void_emits_halt() {
        let prog = lower(vec![
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::Halt));
    }

    #[test]
    fn test_type_assert_emits_comment() {
        let prog = lower(vec![
            CIRInstr::new("type_assert", None::<String>, vec![CIROperand::Var("x".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::Comment));
    }

    #[test]
    fn test_call_emits_call() {
        let prog = lower(vec![
            CIRInstr::new("call", Some("result".to_string()),
                vec![CIROperand::Var("my_func".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::Call));
    }

    // ── Unsupported ops ───────────────────────────────────────────────────────

    #[test]
    fn test_call_runtime_raises_error() {
        let err = lower_err(vec![
            CIRInstr::new("call_runtime", None::<String>, vec![], "void"),
        ]);
        assert!(err.message.contains("call_runtime"));
    }

    #[test]
    fn test_io_in_raises_error() {
        let err = lower_err(vec![
            CIRInstr::new("io_in", Some("x".to_string()), vec![], "i32"),
        ]);
        assert!(err.message.contains("io_in"));
    }

    #[test]
    fn test_io_out_raises_error() {
        let err = lower_err(vec![
            CIRInstr::new("io_out", None::<String>, vec![CIROperand::Var("x".into())], "void"),
        ]);
        assert!(err.message.contains("io_out"));
    }

    #[test]
    fn test_unknown_op_raises_error() {
        let err = lower_err(vec![
            CIRInstr::new("zap_quux", None::<String>, vec![], "void"),
        ]);
        assert!(err.message.contains("zap_quux"));
    }

    // ── Register allocation ───────────────────────────────────────────────────

    #[test]
    fn test_same_variable_gets_same_register() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(1)], "i32"),
            CIRInstr::new("add_i32", Some("y".to_string()),
                vec![CIROperand::Var("x".into()), CIROperand::Var("x".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        // The ADD instruction operands: [dest_reg, src1_reg, src2_reg].
        // src1 and src2 both reference 'x', so they should be the same register.
        let add = prog.instructions.iter()
            .find(|i| i.opcode == IrOp::Add)
            .expect("ADD expected");
        assert_eq!(add.operands[1], add.operands[2],
            "both srcs should be the same register for 'x + x'");
    }

    // ── Program structure ─────────────────────────────────────────────────────

    #[test]
    fn test_entry_label_correct() {
        let prog = lower_cir_to_ir_program(
            &[CIRInstr::new("ret_void", None::<String>, vec![], "void")],
            "my_entry",
        ).unwrap();
        assert_eq!(prog.entry_label, "my_entry");
    }

    #[test]
    fn test_first_instruction_is_entry_label() {
        let prog = lower(vec![CIRInstr::new("ret_void", None::<String>, vec![], "void")]);
        assert_eq!(prog.instructions[0].opcode, IrOp::Label);
        assert_eq!(
            prog.instructions[0].operands[0],
            IrOperand::Label("_start".to_string())
        );
    }

    #[test]
    fn test_label_instruction_has_id_minus_one() {
        let prog = lower(vec![CIRInstr::new("ret_void", None::<String>, vec![], "void")]);
        // The entry LABEL always has id = -1.
        assert_eq!(prog.instructions[0].id, -1);
    }

    #[test]
    fn test_non_label_instructions_have_unique_ids() {
        let prog = lower(vec![
            CIRInstr::new("const_i32", Some("a".to_string()), vec![CIROperand::Int(1)], "i32"),
            CIRInstr::new("const_i32", Some("b".to_string()), vec![CIROperand::Int(2)], "i32"),
            CIRInstr::new("add_i32", Some("c".to_string()),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        let non_label_ids: Vec<_> = prog.instructions.iter()
            .filter(|i| i.opcode != IrOp::Label)
            .map(|i| i.id)
            .collect();
        let unique: std::collections::HashSet<_> = non_label_ids.iter().collect();
        assert_eq!(unique.len(), non_label_ids.len(), "duplicate instruction IDs");
    }

    // ── Br alias ─────────────────────────────────────────────────────────────

    #[test]
    fn test_br_true_bool_alias_works() {
        let prog = lower(vec![
            CIRInstr::new("const_bool", Some("c".to_string()), vec![CIROperand::Bool(true)], "bool"),
            CIRInstr::new("br_true_bool", None::<String>,
                vec![CIROperand::Var("c".into()), CIROperand::Var("target".into())], "void"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::BranchNz));
    }

    #[test]
    fn test_br_false_bool_alias_works() {
        let prog = lower(vec![
            CIRInstr::new("const_bool", Some("c".to_string()), vec![CIROperand::Bool(false)], "bool"),
            CIRInstr::new("br_false_bool", None::<String>,
                vec![CIROperand::Var("c".into()), CIROperand::Var("done".into())], "void"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ]);
        assert!(opcodes(&prog).contains(&IrOp::BranchZ));
    }
}
