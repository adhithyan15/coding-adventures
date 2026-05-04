//! `CIRInstr` — the CompilerIR instruction that flows through the JIT pipeline.
//!
//! # Design
//!
//! CIR (Compiler IR) is the typed intermediate form between the dynamically-typed
//! IIR bytecode and the final backend-specific output (native bytes, WASM, JVM
//! class file, …).  Every CIR instruction has a **concrete type** embedded in
//! its mnemonic — e.g. `"add_u8"`, `"cmp_lt_i32"`, `"const_bool"`.
//!
//! # Mnemonic conventions
//!
//! | Category | Pattern | Examples |
//! |---|---|---|
//! | Integer arithmetic | `{op}_{type}` | `add_u8`, `sub_i32`, `mul_u64` |
//! | Float arithmetic | `{op}_f64` | `add_f64`, `div_f64` |
//! | Comparisons | `cmp_{rel}_{type}` | `cmp_eq_u8`, `cmp_lt_f64` |
//! | Constants | `const_{type}` | `const_u8`, `const_bool`, `const_f64` |
//! | Control flow | unchanged | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
//! | Returns | `ret_{type}` / `ret_void` | `ret_u8`, `ret_void` |
//! | Runtime calls | `call_runtime` | generic fallback; `srcs[0]` = runtime name |
//! | Type guards | `type_assert` | guard emitted by specialiser; `deopt_to` set |
//!
//! # Operands
//!
//! `CIRInstr` reuses [`interpreter_ir::instr::Operand`] for `srcs`:
//!
//! - `Operand::Var(name)` — a named virtual register
//! - `Operand::Int(n)`   — a 64-bit integer literal
//! - `Operand::Float(v)` — an IEEE 754 double literal
//! - `Operand::Bool(b)`  — a boolean literal
//!
//! For `call_runtime`, `srcs[0]` is always `Operand::Var(runtime_fn_name)` —
//! Rust has no separate "string literal" operand type since variable names are
//! also strings.
//!
//! # Example
//!
//! ```
//! use jit_core::cir::{CIRInstr, CIROperand};
//!
//! // A typed add: v0 = add_u8 a, b  [u8]
//! let instr = CIRInstr {
//!     op: "add_u8".into(),
//!     dest: Some("v0".into()),
//!     srcs: vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
//!     ty: "u8".into(),
//!     deopt_to: None,
//! };
//! assert_eq!(instr.to_string(), "v0 = add_u8 a, b  [u8]");
//!
//! // A type guard:
//! let guard = CIRInstr {
//!     op: "type_assert".into(),
//!     dest: None,
//!     srcs: vec![CIROperand::Var("x".into()), CIROperand::Var("u8".into())],
//!     ty: "void".into(),
//!     deopt_to: Some(3),
//! };
//! assert!(guard.is_type_guard());
//! ```

use std::fmt;

// ---------------------------------------------------------------------------
// CIROperand
// ---------------------------------------------------------------------------

/// A source operand in a CIR instruction.
///
/// Mirrors [`interpreter_ir::instr::Operand`] but is defined here so
/// `jit-core` does not re-export a type from `interpreter-ir` through its
/// public API in a way that ties the two crates' versions together.
///
/// Conversions between `Operand` and `CIROperand` are provided for the
/// specialiser, which reads `Operand`s from `IIRInstr` and emits `CIROperand`s.
#[derive(Debug, Clone, PartialEq)]
pub enum CIROperand {
    /// A named virtual register, or a runtime-function name for `call_runtime`.
    Var(String),
    /// A 64-bit integer literal (covers u8/u16/u32/u64/i8…i64 via cast).
    Int(i64),
    /// An IEEE 754 double literal.
    Float(f64),
    /// A boolean literal.
    Bool(bool),
}

impl CIROperand {
    /// Return the variable / name string if this is a `Var` operand.
    pub fn as_var(&self) -> Option<&str> {
        match self {
            CIROperand::Var(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Return `true` if this operand is a literal (Int, Float, or Bool) —
    /// i.e. not a variable reference.
    pub fn is_literal(&self) -> bool {
        !matches!(self, CIROperand::Var(_))
    }
}

impl fmt::Display for CIROperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CIROperand::Var(s)   => write!(f, "{s}"),
            CIROperand::Int(n)   => write!(f, "{n}"),
            CIROperand::Float(v) => write!(f, "{v}"),
            CIROperand::Bool(b)  => write!(f, "{b}"),
        }
    }
}

/// Convert from `interpreter_ir::instr::Operand` to `CIROperand`.
///
/// Used by the specialiser to lift IIR operands into CIR without cloning
/// the whole IIR instruction.
impl From<interpreter_ir::instr::Operand> for CIROperand {
    fn from(op: interpreter_ir::instr::Operand) -> Self {
        match op {
            interpreter_ir::instr::Operand::Var(s)   => CIROperand::Var(s),
            interpreter_ir::instr::Operand::Int(n)   => CIROperand::Int(n),
            interpreter_ir::instr::Operand::Float(v) => CIROperand::Float(v),
            interpreter_ir::instr::Operand::Bool(b)  => CIROperand::Bool(b),
        }
    }
}

impl From<&interpreter_ir::instr::Operand> for CIROperand {
    fn from(op: &interpreter_ir::instr::Operand) -> Self {
        match op {
            interpreter_ir::instr::Operand::Var(s)   => CIROperand::Var(s.clone()),
            interpreter_ir::instr::Operand::Int(n)   => CIROperand::Int(*n),
            interpreter_ir::instr::Operand::Float(v) => CIROperand::Float(*v),
            interpreter_ir::instr::Operand::Bool(b)  => CIROperand::Bool(*b),
        }
    }
}

// ---------------------------------------------------------------------------
// CIRInstr
// ---------------------------------------------------------------------------

/// A single typed instruction in the CompilerIR stream.
///
/// # Field summary
///
/// | Field | Type | Purpose |
/// |---|---|---|
/// | `op` | `String` | Typed mnemonic (`"add_u8"`, `"label"`, …) |
/// | `dest` | `Option<String>` | Destination virtual register (`None` for void ops) |
/// | `srcs` | `Vec<CIROperand>` | Source operands (variables or literals) |
/// | `ty` | `String` | Concrete type of `dest` (or `"void"`) |
/// | `deopt_to` | `Option<usize>` | IIR instruction index to jump to if a guard fails |
///
/// Note: the field is called `ty` (not `type`) because `type` is a Rust keyword.
#[derive(Debug, Clone, PartialEq)]
pub struct CIRInstr {
    /// Typed mnemonic, e.g. `"add_u8"`, `"const_bool"`, `"jmp"`, `"type_assert"`.
    pub op: String,

    /// Name of the virtual register written by this instruction, or `None`
    /// for void operations (branches, stores, type guards, returns).
    pub dest: Option<String>,

    /// Source operands in declaration order.
    ///
    /// For `call_runtime`: `srcs[0]` is the runtime function name as
    /// `CIROperand::Var(name)`, followed by the actual arguments.
    pub srcs: Vec<CIROperand>,

    /// Concrete type string of the result.
    ///
    /// The specialiser guarantees this is never `"any"` for arithmetic and
    /// comparison ops.  Control-flow and call instructions may carry `"any"`
    /// or `"void"`.
    pub ty: String,

    /// IIR instruction index to revert to if a runtime type guard fails.
    ///
    /// `Some(i)` — the backend must implement a guard stub that jumps back
    /// to IIR instruction `i` on failure.
    ///
    /// `None` — this instruction carries no deopt payload.
    pub deopt_to: Option<usize>,
}

impl CIRInstr {
    /// Build a simple CIR instruction without a deopt anchor.
    ///
    /// The `dest` parameter accepts anything that implements `Into<String>` for
    /// the `Some` case; pass `None::<&str>` (or `None::<String>`) when the
    /// instruction has no destination register.
    ///
    /// # Example
    ///
    /// ```
    /// use jit_core::cir::{CIRInstr, CIROperand};
    ///
    /// let i = CIRInstr::new("add_u8", Some("v0"), vec![
    ///     CIROperand::Var("a".into()),
    ///     CIROperand::Var("b".into()),
    /// ], "u8");
    /// assert_eq!(i.op, "add_u8");
    ///
    /// // Void instruction — no destination:
    /// let r = CIRInstr::new("ret_void", None::<&str>, vec![], "void");
    /// assert!(r.dest.is_none());
    /// ```
    pub fn new(
        op: impl Into<String>,
        dest: Option<impl Into<String>>,
        srcs: Vec<CIROperand>,
        ty: impl Into<String>,
    ) -> Self {
        CIRInstr {
            op: op.into(),
            dest: dest.map(Into::into),
            srcs,
            ty: ty.into(),
            deopt_to: None,
        }
    }

    /// Build a CIR instruction with a deopt anchor (for type guards).
    ///
    /// Pass `None::<&str>` when the instruction has no destination register.
    pub fn new_with_deopt(
        op: impl Into<String>,
        dest: Option<impl Into<String>>,
        srcs: Vec<CIROperand>,
        ty: impl Into<String>,
        deopt_to: usize,
    ) -> Self {
        CIRInstr {
            op: op.into(),
            dest: dest.map(Into::into),
            srcs,
            ty: ty.into(),
            deopt_to: Some(deopt_to),
        }
    }

    // ------------------------------------------------------------------
    // Classification predicates
    // ------------------------------------------------------------------

    /// Return `true` if this is a type guard (`type_assert` with `deopt_to`
    /// set).
    ///
    /// The backend must implement a guard check and deopt on failure.
    pub fn is_type_guard(&self) -> bool {
        self.op == "type_assert" && self.deopt_to.is_some()
    }

    /// Return `true` if this is a generic runtime call (`call_runtime`).
    ///
    /// These are emitted when the specialiser cannot determine a concrete
    /// type.  Backends handle them via a slow-path interpreter call.
    pub fn is_generic(&self) -> bool {
        self.op == "call_runtime"
    }

    /// Return `true` if this instruction has no observable side effects
    /// (i.e. it is safe to remove if the result is unused).
    ///
    /// Used by the dead-code elimination pass.  A safe pruning heuristic:
    /// any instruction whose `op` does NOT appear in the side-effect set.
    ///
    /// # Security note
    ///
    /// Typed return mnemonics (`ret_u8`, `ret_i32`, `ret_f64`, …) and typed
    /// branch mnemonics (`br_false_bool`, `br_true_bool`) are always impure
    /// even though they do not appear verbatim in the match arm list.  The
    /// prefix check below guards against future DCE passes that might tighten
    /// the no-dest rule and accidentally eliminate typed returns.
    pub fn is_pure(&self) -> bool {
        let op = self.op.as_str();
        // All return instructions — bare `ret`, `ret_void`, and typed `ret_*` —
        // have observable effects (they terminate the function).
        if op.starts_with("ret") {
            return false;
        }
        // Typed conditional branches (br_false_bool, br_true_bool, …) have
        // observable control-flow effects.
        if op.starts_with("br_") {
            return false;
        }
        !matches!(
            op,
            "call_runtime" | "call" | "call_builtin"
            | "io_out" | "store_mem" | "store_reg"
            | "type_assert"
            | "jmp" | "jmp_if_true" | "jmp_if_false"
            | "label"
        )
    }
}

impl fmt::Display for CIRInstr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(dest) = &self.dest {
            write!(f, "{dest} = ")?;
        }
        write!(f, "{}", self.op)?;
        for (i, src) in self.srcs.iter().enumerate() {
            if i == 0 {
                write!(f, " {src}")?;
            } else {
                write!(f, ", {src}")?;
            }
        }
        write!(f, "  [{}]", self.ty)?;
        if let Some(d) = self.deopt_to {
            write!(f, "  [deopt→{d}]")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_u8_display() {
        let instr = CIRInstr::new(
            "add_u8",
            Some("v0"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "u8",
        );
        assert_eq!(instr.to_string(), "v0 = add_u8 a, b  [u8]");
    }

    #[test]
    fn void_instr_display() {
        let instr = CIRInstr::new(
            "ret_void",
            None::<&str>,
            vec![],
            "void",
        );
        assert_eq!(instr.to_string(), "ret_void  [void]");
    }

    #[test]
    fn type_guard_with_deopt() {
        let guard = CIRInstr::new_with_deopt(
            "type_assert",
            None::<&str>,
            vec![CIROperand::Var("x".into()), CIROperand::Var("u8".into())],
            "void",
            3,
        );
        assert!(guard.is_type_guard());
        assert!(guard.to_string().contains("deopt→3"));
    }

    #[test]
    fn is_pure_arithmetic() {
        let add = CIRInstr::new("add_u8", Some("v0"), vec![], "u8");
        assert!(add.is_pure());
    }

    #[test]
    fn is_pure_side_effect_ops() {
        for op in &["call_runtime", "call", "ret_void", "jmp", "label", "type_assert", "io_out"] {
            let i = CIRInstr::new(*op, None::<&str>, vec![], "void");
            assert!(!i.is_pure(), "expected impure: {op}");
        }
    }

    #[test]
    fn is_generic() {
        let i = CIRInstr::new(
            "call_runtime",
            Some("r"),
            vec![CIROperand::Var("generic_add".into())],
            "any",
        );
        assert!(i.is_generic());
        let j = CIRInstr::new("add_u8", Some("r"), vec![], "u8");
        assert!(!j.is_generic());
    }

    #[test]
    fn operand_from_iir_operand() {
        use interpreter_ir::instr::Operand;
        let ir_ops = vec![
            Operand::Var("x".into()),
            Operand::Int(42),
            Operand::Float(3.14),
            Operand::Bool(true),
        ];
        let cir_ops: Vec<CIROperand> = ir_ops.iter().map(CIROperand::from).collect();
        assert_eq!(cir_ops[0], CIROperand::Var("x".into()));
        assert_eq!(cir_ops[1], CIROperand::Int(42));
        assert!(matches!(cir_ops[2], CIROperand::Float(_)));
        assert_eq!(cir_ops[3], CIROperand::Bool(true));
    }

    #[test]
    fn operand_as_var() {
        assert_eq!(CIROperand::Var("a".into()).as_var(), Some("a"));
        assert!(CIROperand::Int(1).as_var().is_none());
    }

    #[test]
    fn operand_is_literal() {
        assert!(!CIROperand::Var("a".into()).is_literal());
        assert!(CIROperand::Int(1).is_literal());
        assert!(CIROperand::Float(1.0).is_literal());
        assert!(CIROperand::Bool(false).is_literal());
    }

    #[test]
    fn instr_with_literal_src_display() {
        let i = CIRInstr::new(
            "const_u8",
            Some("c"),
            vec![CIROperand::Int(42)],
            "u8",
        );
        assert_eq!(i.to_string(), "c = const_u8 42  [u8]");
    }

    #[test]
    fn type_assert_without_deopt_not_a_guard() {
        // type_assert without deopt_to is just a plain annotation.
        let i = CIRInstr {
            op: "type_assert".into(),
            dest: None,
            srcs: vec![],
            ty: "void".into(),
            deopt_to: None,
        };
        assert!(!i.is_type_guard());
    }
}
