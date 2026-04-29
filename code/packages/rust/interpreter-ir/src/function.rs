//! `IIRFunction` — a named, parameterised sequence of `IIRInstr`.
//!
//! A function is the **unit of compilation** in the LANG pipeline.  The JIT
//! compiles one function at a time; the profiler tracks call counts per
//! function.
//!
//! # Type status
//!
//! [`FunctionTypeStatus`] is shared across all LANG-pipeline languages:
//!
//! | Status | Meaning | JIT threshold |
//! |---|---|---|
//! | `FullyTyped` | Every instruction has a concrete `type_hint` | Compile before first call |
//! | `PartiallyTyped` | Some typed, some `"any"` | Compile after 10 calls |
//! | `Untyped` | All `"any"` | Compile after 100 calls |
//!
//! # Example
//!
//! ```
//! use interpreter_ir::function::{IIRFunction, FunctionTypeStatus};
//! use interpreter_ir::instr::{IIRInstr, Operand};
//!
//! let fn_ = IIRFunction {
//!     name: "add".into(),
//!     params: vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
//!     return_type: "u8".into(),
//!     instructions: vec![
//!         IIRInstr::new("add", Some("v0".into()),
//!             vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
//!         IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u8"),
//!     ],
//!     register_count: 8,
//!     type_status: FunctionTypeStatus::FullyTyped,
//!     call_count: 0,
//!     feedback_slots: std::collections::HashMap::new(),
//!     source_map: Vec::new(),
//! };
//! assert_eq!(fn_.param_names(), vec!["a", "b"]);
//! ```

use std::collections::HashMap;
use crate::instr::IIRInstr;
use crate::opcodes::is_concrete_type;

// ---------------------------------------------------------------------------
// FunctionTypeStatus
// ---------------------------------------------------------------------------

/// Compilation tier based on how much type information is available.
///
/// Derived from the function's parameter types and instruction `type_hint`s.
/// The JIT uses this to decide *when* to compile the function:
///
/// - `FullyTyped` → compile before the first interpreted call (threshold = 0)
/// - `PartiallyTyped` → compile after 10 interpreted calls
/// - `Untyped` → compile after 100 interpreted calls
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FunctionTypeStatus {
    /// All parameters and all instruction `type_hint`s are concrete types.
    FullyTyped,
    /// Some instructions are typed; others use `"any"`.
    PartiallyTyped,
    /// No static types; all type information must come from profiling.
    Untyped,
}

// ---------------------------------------------------------------------------
// IIRFunction
// ---------------------------------------------------------------------------

/// A single function in an [`IIRModule`](crate::module::IIRModule).
#[derive(Debug, Clone)]
pub struct IIRFunction {
    /// The function name, used as the key in the JIT cache and the vm-core
    /// call-count metrics table.
    pub name: String,

    /// Ordered list of `(param_name, type_hint)` pairs.
    ///
    /// The VM loads arguments into the first `params.len()` registers before
    /// the first instruction executes.
    pub params: Vec<(String, String)>,

    /// Declared return type (`"u8"`, `"void"`, `"any"`, …).
    pub return_type: String,

    /// The body of the function as a flat list of `IIRInstr`.
    ///
    /// Labels are also represented as instructions (op `"label"`) so the
    /// index of each instruction is its unique address within the function.
    pub instructions: Vec<IIRInstr>,

    /// Number of VM registers this function uses (including parameters).
    /// `vm-core` allocates exactly this many register slots per frame.
    pub register_count: usize,

    /// Compilation tier — determines when the JIT should compile this function.
    pub type_status: FunctionTypeStatus,

    /// Incremented by `vm-core` on each interpreted call.
    ///
    /// Read by `jit-core` to trigger tier promotion when `call_count` exceeds
    /// the function's threshold for its `type_status`.
    pub call_count: u32,

    // -----------------------------------------------------------------------
    // Optional frontend-owned side tables (LANG17)
    // -----------------------------------------------------------------------

    /// Optional: `slot_index → iir_instr_index` mapping.
    ///
    /// Frontends that allocate named feedback slots at compile time (Tetrad,
    /// SpiderMonkey-style) populate this dict so a slot index can be resolved
    /// back to the IIR instruction that owns it.
    pub feedback_slots: HashMap<usize, usize>,

    /// Optional: `(iir_index, source_a, source_b)` triples.
    ///
    /// Conventional use: `(iir_index, source_line, source_column)` for
    /// debugger source-mapping, or `(iir_index, original_bytecode_ip, 0)`
    /// for Tetrad's branch-profile re-keying.  The third field's meaning is
    /// frontend-defined; vm-core does not look at it.
    pub source_map: Vec<(usize, usize, usize)>,
}

impl IIRFunction {
    /// Create a new function with default profiling fields.
    pub fn new(
        name: impl Into<String>,
        params: Vec<(String, String)>,
        return_type: impl Into<String>,
        instructions: Vec<IIRInstr>,
    ) -> Self {
        let mut fn_ = IIRFunction {
            name: name.into(),
            params,
            return_type: return_type.into(),
            instructions,
            register_count: 8,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: HashMap::new(),
            source_map: Vec::new(),
        };
        fn_.type_status = fn_.infer_type_status();
        fn_
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Return just the parameter names in order.
    pub fn param_names(&self) -> Vec<&str> {
        self.params.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Return just the parameter type hints in order.
    pub fn param_types(&self) -> Vec<&str> {
        self.params.iter().map(|(_, t)| t.as_str()).collect()
    }

    /// Derive the type status from params and instruction type hints.
    ///
    /// A function is `FullyTyped` if every param type and every instruction
    /// `type_hint` is a concrete type (not `"any"`).  It is `Untyped` if none
    /// of them are.  Otherwise `PartiallyTyped`.
    pub fn infer_type_status(&self) -> FunctionTypeStatus {
        let all_hints: Vec<&str> = self.params.iter()
            .map(|(_, t)| t.as_str())
            .chain(self.instructions.iter().map(|i| i.type_hint.as_str()))
            .collect();

        if all_hints.is_empty() {
            return FunctionTypeStatus::Untyped;
        }

        let typed = all_hints.iter().filter(|h| is_concrete_type(h)).count();
        if typed == 0 {
            FunctionTypeStatus::Untyped
        } else if typed == all_hints.len() {
            FunctionTypeStatus::FullyTyped
        } else {
            FunctionTypeStatus::PartiallyTyped
        }
    }

    /// Return the instruction index of the named label, or `None`.
    pub fn label_index(&self, label_name: &str) -> Option<usize> {
        self.instructions.iter().position(|instr| {
            instr.op == "label"
                && instr.srcs.first().and_then(|s| s.as_var()) == Some(label_name)
        })
    }
}

impl std::fmt::Display for IIRFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<String> = self
            .params
            .iter()
            .map(|(n, t)| format!("{n}: {t}"))
            .collect();
        write!(
            f,
            "fn {}({}) -> {} [{} instrs, {:?}]",
            self.name,
            params.join(", "),
            self.return_type,
            self.instructions.len(),
            self.type_status,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instr::{Operand};

    fn make_typed_fn() -> IIRFunction {
        IIRFunction::new(
            "add",
            vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
            "u8",
            vec![
                IIRInstr::new("add", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u8"),
            ],
        )
    }

    fn make_untyped_fn() -> IIRFunction {
        IIRFunction::new(
            "sum_pair",
            vec![("a".into(), "any".into()), ("b".into(), "any".into())],
            "any",
            vec![
                IIRInstr::new("add", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "any"),
            ],
        )
    }

    #[test]
    fn infer_fully_typed() {
        let fn_ = make_typed_fn();
        assert_eq!(fn_.type_status, FunctionTypeStatus::FullyTyped);
    }

    #[test]
    fn infer_untyped() {
        let fn_ = make_untyped_fn();
        assert_eq!(fn_.type_status, FunctionTypeStatus::Untyped);
    }

    #[test]
    fn param_names() {
        let fn_ = make_typed_fn();
        assert_eq!(fn_.param_names(), vec!["a", "b"]);
    }

    #[test]
    fn label_index() {
        let mut fn_ = make_typed_fn();
        fn_.instructions.insert(
            0,
            IIRInstr::new("label", None, vec![Operand::Var("loop_start".into())], "void"),
        );
        assert_eq!(fn_.label_index("loop_start"), Some(0));
        assert_eq!(fn_.label_index("nonexistent"), None);
    }

    #[test]
    fn call_count_increments() {
        let mut fn_ = make_typed_fn();
        fn_.call_count += 1;
        assert_eq!(fn_.call_count, 1);
    }
}
