//! LANG83 compile-time operation check: refuse what a backend cannot lower.
//!
//! # The problem this solves
//!
//! A frontend emits operations from the LANG83 standard registry
//! (`interpreter_ir::op_registry::STANDARD_OPS`); each backend lowers each one
//! itself, or cannot. Before LANG83, "cannot" surfaced however the backend
//! happened to notice: a validator message, a lowering-time `UnsupportedOp`, a
//! linker error for a missing helper, or — for an op whose name collided with a
//! prefix the native backends dispatch on — **nothing at all**, just wrong
//! machine code.
//!
//! This module makes the answer uniform and early. Every backend crate publishes
//! an `op_support(name)` declaration; [`check_module_for`] walks a module once,
//! asks the target backend about every registry operation it uses, and returns a
//! typed [`UnsupportedOps`] error listing every refusal — function, op and the
//! backend's own reason — **before any lowering pass runs**:
//!
//! ```text
//! IIRModule ──▶ check_module_for(backend) ──Ok──▶ lowering passes ──▶ backend
//!                        │
//!                        └──Err(UnsupportedOps)──▶ "llvm cannot lower …
//!                                                    `i64_div_floor` in `main`
//!                                                    (not yet lowered; LANG83 PR1b)"
//! ```
//!
//! Instructions that are not registry operations (control flow, calls, heap,
//! the lisp `dyn_*` builtins, …) are not gated here; each backend's own
//! validator still checks them.
//!
//! # Example
//!
//! ```
//! use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
//! use lang_aot::{check_module_for, Backend};
//!
//! let main = IIRFunction::new("main", vec![], "i64", vec![
//!     IIRInstr::new("const", Some("a".into()), vec![Operand::Int(-7)], "i64"),
//!     IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i64"),
//!     IIRInstr::new("i64_div_floor", Some("q".into()),
//!         vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
//!     IIRInstr::new("ret", None, vec![Operand::Var("q".into())], "i64"),
//! ]);
//! let mut module = IIRModule::new("m", "test");
//! module.add_or_replace(main);
//!
//! let err = check_module_for(Backend::Llvm, &module).unwrap_err();
//! assert_eq!(err.refusals[0].op, "i64_div_floor");
//! assert!(err.to_string().contains("not yet lowered; LANG83 PR1b"));
//! ```

use std::fmt;

use interpreter_ir::op_registry::{registry_entry, OpSupport};
use interpreter_ir::IIRModule;

/// The nine lang-aot backend columns, each backed by one crate's
/// `op_support` declaration (LANG83 "Backend contract" table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    /// Native x86-64 AOT (`twig-aot` → `x86_64-backend`): Linux and Windows hosts.
    X86_64,
    /// Native AArch64 AOT (`twig-aot` → `aarch64-backend`): the macOS Mach-O path.
    Aarch64,
    /// Textual LLVM IR (`iir-to-llvm`).
    Llvm,
    /// WebAssembly (`iir-to-wasm`).
    Wasm,
    /// JVM class files (`iir-to-jvm-class-file`).
    Jvm,
    /// CLR / CIL (`iir-to-cil-bytecode`, the textual real-CoreCLR path).
    Clr,
    /// BEAM / Erlang VM (`iir-to-beam`).
    Beam,
    /// The generic register VM (`vm-core`).
    Vm,
    /// The generic JIT (`jit-core`'s `GenericCirJit`), judged without its
    /// interpreter fallback.
    Jit,
}

impl Backend {
    /// Every backend, in the LANG83 contract-table order.
    pub const ALL: [Backend; 9] = [
        Backend::X86_64,
        Backend::Aarch64,
        Backend::Llvm,
        Backend::Wasm,
        Backend::Jvm,
        Backend::Clr,
        Backend::Beam,
        Backend::Vm,
        Backend::Jit,
    ];

    /// A short lowercase name, as used in error messages.
    pub fn name(self) -> &'static str {
        match self {
            Backend::X86_64 => "x86_64",
            Backend::Aarch64 => "aarch64",
            Backend::Llvm => "llvm",
            Backend::Wasm => "wasm",
            Backend::Jvm => "jvm",
            Backend::Clr => "clr",
            Backend::Beam => "beam",
            Backend::Vm => "vm",
            Backend::Jit => "jit",
        }
    }

    /// The native backend `compile_iir_to_host_executable` targets on this
    /// host: AArch64 Mach-O on macOS, x86-64 everywhere else.
    pub fn native_host() -> Backend {
        if cfg!(target_os = "macos") {
            Backend::Aarch64
        } else {
            Backend::X86_64
        }
    }

    /// What this backend does with the standard operation `name` — a direct
    /// call to the owning crate's `op_support`.
    pub fn op_support(self, name: &str) -> OpSupport {
        match self {
            Backend::X86_64 => x86_64_backend::op_support(name),
            Backend::Aarch64 => aarch64_backend::op_support(name),
            Backend::Llvm => iir_to_llvm::op_support(name),
            Backend::Wasm => iir_to_wasm::op_support(name),
            Backend::Jvm => iir_to_jvm_class_file::op_support(name),
            Backend::Clr => iir_to_cil_bytecode::op_support(name),
            Backend::Beam => iir_to_beam::op_support(name),
            Backend::Vm => vm_core::op_support(name),
            Backend::Jit => jit_core::op_support::op_support(name),
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One refused operation use: which function, which op, and the backend's reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpRefusal {
    /// The function containing the instruction.
    pub function: String,
    /// The registry name of the refused operation.
    pub op: &'static str,
    /// The backend's reason, verbatim from its `op_support` table.
    pub reason: &'static str,
}

/// A module uses standard operations the target backend refuses (LANG83).
///
/// Lists each distinct `(function, op)` pair once, in program order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedOps {
    /// The backend that refused.
    pub backend: Backend,
    /// Every refused `(function, op)` use.
    pub refusals: Vec<OpRefusal>,
}

impl fmt::Display for UnsupportedOps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} cannot lower {} standard operation use(s) (LANG83): ",
            self.backend,
            self.refusals.len()
        )?;
        for (i, r) in self.refusals.iter().enumerate() {
            if i > 0 {
                f.write_str("; ")?;
            }
            write!(f, "`{}` in function `{}` ({})", r.op, r.function, r.reason)?;
        }
        Ok(())
    }
}

impl std::error::Error for UnsupportedOps {}

/// Refuse, at compile time, every standard operation in `module` that
/// `backend` does not lower (LANG83). See the module docs.
///
/// # Errors
///
/// [`UnsupportedOps`] listing every refused `(function, op)` use.
pub fn check_module_for(backend: Backend, module: &IIRModule) -> Result<(), UnsupportedOps> {
    let mut refusals: Vec<OpRefusal> = Vec::new();
    for func in &module.functions {
        for instr in &func.instructions {
            let Some(spec) = registry_entry(instr) else { continue };
            if let OpSupport::Refused(reason) = backend.op_support(spec.name) {
                let seen = refusals.iter().any(|r| r.function == func.name && r.op == spec.name);
                if !seen {
                    refusals.push(OpRefusal { function: func.name.clone(), op: spec.name, reason });
                }
            }
        }
    }
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(UnsupportedOps { backend, refusals })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::op_registry::{registry_entry, OpKind, STANDARD_OPS};
    use interpreter_ir::{IIRFunction, IIRInstr, Operand};

    fn module_using(instrs: Vec<IIRInstr>) -> IIRModule {
        let mut body = instrs;
        body.push(IIRInstr::new("ret_void", None, vec![], "void"));
        let mut module = IIRModule::new("m", "test");
        module.add_or_replace(IIRFunction::new("main", vec![], "void", body));
        module
    }

    fn floor_div() -> IIRInstr {
        IIRInstr::new(
            "i64_div_floor",
            Some("q".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            "i64",
        )
    }

    /// `mod` — refused by the generic JIT (interpreter tier only), lowered
    /// everywhere else. Since LANG83 PR1b no backend refuses a PR1b op, so
    /// the refusal path is exercised through the JIT's honest refusals.
    fn trunc_mod() -> IIRInstr {
        IIRInstr::new(
            "mod",
            Some("r".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            "i64",
        )
    }

    #[test]
    fn every_backend_answers_for_every_registry_op_through_its_own_crate() {
        for backend in Backend::ALL {
            for spec in STANDARD_OPS {
                let support = backend.op_support(spec.name);
                if spec.is_pr1b() {
                    // LANG83 PR1b lowered all four everywhere.
                    assert_eq!(support, OpSupport::Lowered, "{backend} {}", spec.name);
                }
            }
            // The output builtins every existing I/O frontend relies on are
            // lowered by every column.
            for name in ["print_i64", "putchar", "write_i64"] {
                assert!(backend.op_support(name).is_lowered(), "{backend} {name}");
            }
        }
    }

    #[test]
    fn a_refused_op_is_reported_with_a_clear_message() {
        let module = module_using(vec![trunc_mod()]);
        let err = check_module_for(Backend::Jit, &module).unwrap_err();
        assert_eq!(err.backend, Backend::Jit);
        assert_eq!(err.refusals.len(), 1);
        assert_eq!((err.refusals[0].function.as_str(), err.refusals[0].op), ("main", "mod"));
        let msg = err.to_string();
        assert!(msg.starts_with(&format!("{} cannot lower 1 standard operation", Backend::Jit)), "{msg}");
        assert!(msg.contains("`mod` in function `main`"), "{msg}");
        for backend in Backend::ALL.into_iter().filter(|b| *b != Backend::Jit) {
            assert!(check_module_for(backend, &module).is_ok(), "{backend}");
        }
    }

    #[test]
    fn builtins_are_checked_by_their_call_builtin_name() {
        let write = IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("write_i64".into()), Operand::Var("x".into())],
            "void",
        );
        assert_eq!(registry_entry(&write).map(|s| s.name), Some("write_i64"));
        assert_eq!(
            STANDARD_OPS.iter().find(|s| s.name == "write_i64").unwrap().kind,
            OpKind::Builtin
        );
        for backend in Backend::ALL {
            assert!(check_module_for(backend, &module_using(vec![write.clone()])).is_ok(), "{backend}");
        }
    }

    #[test]
    fn refusals_are_deduplicated_per_function_and_listed_in_order() {
        let len = IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64");
        let module = module_using(vec![trunc_mod(), trunc_mod(), len]);
        let err = check_module_for(Backend::Jit, &module).unwrap_err();
        let ops: Vec<&str> = err.refusals.iter().map(|r| r.op).collect();
        assert_eq!(ops, ["mod", "str_len"]);
        assert!(err.to_string().contains("2 standard operation use(s)"));
    }

    #[test]
    fn pr1b_ops_pass_the_check_on_every_backend() {
        let trap = IIRInstr::new("trap", None, vec![Operand::Str("Trap.User".into())], "void");
        let module = module_using(vec![floor_div(), trap]);
        for backend in Backend::ALL {
            assert!(check_module_for(backend, &module).is_ok(), "{backend}");
        }
    }

    #[test]
    fn modules_using_only_lowered_or_unregistered_ops_pass() {
        let module = module_using(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("add", Some("y".into()), vec![Operand::Var("x".into()), Operand::Int(1)], "i64"),
            IIRInstr::new(
                "call_builtin",
                None,
                vec![Operand::Var("print_i64".into()), Operand::Var("y".into())],
                "void",
            ),
            // Not a registry op: never gated here.
            IIRInstr::new("call_builtin", Some("c".into()), vec![Operand::Var("getchar".into())], "i64"),
        ]);
        for backend in Backend::ALL {
            assert!(check_module_for(backend, &module).is_ok(), "{backend}");
        }
    }

    #[test]
    fn the_jit_is_judged_without_its_interpreter_fallback() {
        let module = module_using(vec![IIRInstr::new(
            "f64_sin",
            Some("s".into()),
            vec![Operand::Var("x".into())],
            "f64",
        )]);
        assert!(check_module_for(Backend::Vm, &module).is_ok());
        let err = check_module_for(Backend::Jit, &module).unwrap_err();
        assert!(err.refusals[0].reason.contains("interpreter tier"), "{err}");
    }

    #[test]
    fn backend_names_are_unique_and_native_host_is_a_native_column() {
        let mut names: Vec<&str> = Backend::ALL.iter().map(|b| b.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Backend::ALL.len());
        assert!(matches!(Backend::native_host(), Backend::X86_64 | Backend::Aarch64));
    }
}
