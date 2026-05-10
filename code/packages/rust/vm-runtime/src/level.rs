//! `RuntimeLevel` — the four runtime-link levels for AOT binaries.
//!
//! Different AOT targets need different amounts of runtime support.  A fully
//! statically-typed Tetrad program running on bare Intel 4004 ROM needs
//! **no** runtime at all — every instruction is compiled to native code.  A
//! Lisp program with runtime polymorphism, closures, and garbage collection
//! needs the **full** runtime.
//!
//! LANG15 defines four levels so that AOT binaries link only the runtime
//! subset they actually require:
//!
//! ```text
//! Level 0 (None)     ── no runtime linked; pure native code
//! Level 1 (Minimal)  ── dispatch loop + arithmetic/control-flow handlers
//! Level 2 (Standard) ── Level 1 + builtins registry + I/O
//! Level 3 (Full)     ── Level 2 + profiler + shadow frames + GC hooks
//! ```
//!
//! The AOT compiler selects the **lowest** level that satisfies the program's
//! opcode mix.  Selection rules:
//!
//! - A module with no unspecialised functions: `None`.
//! - A module with unspecialised arithmetic/control-flow but no builtins or I/O:
//!   `Minimal`.
//! - A module that calls builtins or uses I/O opcodes: `Standard`.
//! - A module that uses a hybrid AOT+JIT strategy, closures, or GC allocation:
//!   `Full`.

use interpreter_ir::module::IIRModule;
use interpreter_ir::opcodes::DYNAMIC_TYPE;

/// The runtime-link level required by an AOT binary.
///
/// # Example
///
/// ```
/// use vm_runtime::level::RuntimeLevel;
///
/// assert_eq!(RuntimeLevel::None.level_number(), 0);
/// assert_eq!(RuntimeLevel::Full.level_number(), 3);
/// assert!(RuntimeLevel::Standard >= RuntimeLevel::Minimal);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeLevel {
    /// No runtime linked.  Every function in the module was fully
    /// specialised to native code; no interpreter fallback is needed.
    ///
    /// The pre-built library artefact for this level is never produced —
    /// there is nothing to link.
    None = 0,

    /// Minimal runtime: dispatch loop, arithmetic handlers, control-flow.
    ///
    /// Artefact: `vm_runtime_<target>_l1.a`
    Minimal = 1,

    /// Standard runtime: Level 1 + builtins registry + I/O handlers.
    ///
    /// Required when the module calls registered builtins (`call_builtin`)
    /// or uses `io_in` / `io_out` opcodes.
    ///
    /// Artefact: `vm_runtime_<target>_l2.a`
    Standard = 2,

    /// Full runtime: Level 2 + profiler + shadow frames + GC hooks.
    ///
    /// Required when the module uses a hybrid AOT+JIT strategy (shadow
    /// frames for deopt), closures, or GC allocation opcodes (`alloc`,
    /// `box`, `field_load`, `field_store`).
    ///
    /// Artefact: `vm_runtime_<target>_l3.a`
    Full = 3,
}

impl RuntimeLevel {
    /// Return the numeric level (0–3).
    ///
    /// ```
    /// use vm_runtime::level::RuntimeLevel;
    /// assert_eq!(RuntimeLevel::Minimal.level_number(), 1);
    /// ```
    pub fn level_number(self) -> u8 {
        self as u8
    }

    /// Return the artefact filename for the given target triple, or `None`
    /// for `RuntimeLevel::None` (no artefact produced).
    ///
    /// ```
    /// use vm_runtime::level::RuntimeLevel;
    ///
    /// assert_eq!(
    ///     RuntimeLevel::Minimal.artefact_name("riscv32"),
    ///     Some("vm_runtime_riscv32_l1.a".to_string()),
    /// );
    /// assert_eq!(RuntimeLevel::None.artefact_name("x86_64"), None);
    /// ```
    pub fn artefact_name(self, target: &str) -> Option<String> {
        match self {
            RuntimeLevel::None => None,
            l => Some(format!("vm_runtime_{}_l{}.a", target, l.level_number())),
        }
    }

    /// True when this level requires GC hook support (level 3).
    pub fn requires_gc(self) -> bool {
        self == RuntimeLevel::Full
    }

    /// True when this level includes builtin registry support (level 2+).
    pub fn includes_builtins(self) -> bool {
        self >= RuntimeLevel::Standard
    }
}

impl std::fmt::Display for RuntimeLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeLevel::None => write!(f, "None(l0)"),
            RuntimeLevel::Minimal => write!(f, "Minimal(l1)"),
            RuntimeLevel::Standard => write!(f, "Standard(l2)"),
            RuntimeLevel::Full => write!(f, "Full(l3)"),
        }
    }
}

// ---------------------------------------------------------------------------
// Automatic level selection
// ---------------------------------------------------------------------------

/// Compute the minimum `RuntimeLevel` required for `module`.
///
/// The selection scans the IIR instruction opcodes to find the highest
/// requirement:
///
/// | Opcodes present | Minimum level |
/// |-----------------|---------------|
/// | None (fully specialised or empty module) | `None` |
/// | Any `"any"` instruction (unspecialised) | `Minimal` |
/// | `call_builtin`, `io_in`, `io_out` | `Standard` |
/// | `alloc`, `box`, `unbox`, `field_load`, `field_store`, `safepoint` | `Full` |
///
/// The level is the **maximum** of all per-instruction requirements.
///
/// # Example
///
/// ```
/// use interpreter_ir::module::IIRModule;
/// use interpreter_ir::function::IIRFunction;
/// use interpreter_ir::instr::{IIRInstr, Operand};
/// use vm_runtime::level::{RuntimeLevel, required_level};
///
/// // Fully-typed module → no runtime needed.
/// let fn_ = IIRFunction::new("main", vec![], "void",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
/// let mut m = IIRModule::new("t", "tetrad");
/// m.functions.push(fn_);
/// assert_eq!(required_level(&m), RuntimeLevel::None);
/// ```
pub fn required_level(module: &IIRModule) -> RuntimeLevel {
    let mut level = RuntimeLevel::None;

    for func in &module.functions {
        for instr in &func.instructions {
            let op = instr.op.as_str();
            let th = instr.type_hint.as_str();

            // GC opcodes → Full
            let needs_full = matches!(
                op,
                "alloc" | "box" | "unbox"
                    | "field_load" | "field_store"
                    | "safepoint" | "is_null"
            );
            if needs_full {
                return RuntimeLevel::Full; // maximum possible — short-circuit
            }

            // Builtin / IO opcodes → Standard
            let needs_standard = matches!(op, "call_builtin" | "io_in" | "io_out");
            if needs_standard {
                level = level.max(RuntimeLevel::Standard);
            }

            // Any unspecialised instruction → at least Minimal
            if th == DYNAMIC_TYPE && level < RuntimeLevel::Minimal {
                level = RuntimeLevel::Minimal;
            }
        }
    }

    level
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "void", instrs);
        let mut m = IIRModule::new("test", "tetrad");
        m.functions.push(fn_);
        m
    }

    #[test]
    fn level_numbers() {
        assert_eq!(RuntimeLevel::None.level_number(), 0);
        assert_eq!(RuntimeLevel::Minimal.level_number(), 1);
        assert_eq!(RuntimeLevel::Standard.level_number(), 2);
        assert_eq!(RuntimeLevel::Full.level_number(), 3);
    }

    #[test]
    fn level_ordering() {
        assert!(RuntimeLevel::None < RuntimeLevel::Minimal);
        assert!(RuntimeLevel::Minimal < RuntimeLevel::Standard);
        assert!(RuntimeLevel::Standard < RuntimeLevel::Full);
    }

    #[test]
    fn artefact_name_none_is_none() {
        assert_eq!(RuntimeLevel::None.artefact_name("x86_64"), None);
    }

    #[test]
    fn artefact_name_minimal() {
        assert_eq!(
            RuntimeLevel::Minimal.artefact_name("riscv32"),
            Some("vm_runtime_riscv32_l1.a".to_string())
        );
    }

    #[test]
    fn artefact_name_full() {
        assert_eq!(
            RuntimeLevel::Full.artefact_name("wasm32"),
            Some("vm_runtime_wasm32_l3.a".to_string())
        );
    }

    #[test]
    fn requires_gc_only_full() {
        assert!(RuntimeLevel::Full.requires_gc());
        assert!(!RuntimeLevel::Standard.requires_gc());
    }

    #[test]
    fn includes_builtins_from_standard() {
        assert!(!RuntimeLevel::None.includes_builtins());
        assert!(!RuntimeLevel::Minimal.includes_builtins());
        assert!(RuntimeLevel::Standard.includes_builtins());
        assert!(RuntimeLevel::Full.includes_builtins());
    }

    // required_level tests

    #[test]
    fn empty_module_needs_no_runtime() {
        let m = IIRModule::new("empty", "tetrad");
        assert_eq!(required_level(&m), RuntimeLevel::None);
    }

    #[test]
    fn fully_typed_module_needs_no_runtime() {
        let module = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "u8"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        assert_eq!(required_level(&module), RuntimeLevel::None);
    }

    #[test]
    fn untyped_instruction_needs_minimal() {
        let module = make_module(vec![
            IIRInstr::new("add", Some("x".into()),
                vec![Operand::Int(1), Operand::Int(2)], "any"),
        ]);
        assert_eq!(required_level(&module), RuntimeLevel::Minimal);
    }

    #[test]
    fn call_builtin_needs_standard() {
        let module = make_module(vec![
            IIRInstr::new("call_builtin", None, vec![Operand::Var("print".into())], "any"),
        ]);
        assert_eq!(required_level(&module), RuntimeLevel::Standard);
    }

    #[test]
    fn alloc_needs_full() {
        let module = make_module(vec![
            IIRInstr::new("alloc", Some("obj".into()), vec![], "ref<Object>"),
        ]);
        assert_eq!(required_level(&module), RuntimeLevel::Full);
    }

    #[test]
    fn display_levels() {
        assert_eq!(RuntimeLevel::None.to_string(), "None(l0)");
        assert_eq!(RuntimeLevel::Minimal.to_string(), "Minimal(l1)");
        assert_eq!(RuntimeLevel::Standard.to_string(), "Standard(l2)");
        assert_eq!(RuntimeLevel::Full.to_string(), "Full(l3)");
    }
}
