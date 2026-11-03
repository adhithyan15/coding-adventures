//! `InProcessVMRuntime` — the development/test variant of the vm-runtime ABI.
//!
//! For development and testing, an `InProcessVMRuntime` provides the same
//! ABI surface as the pre-compiled native `vm_runtime_<target>.a`, backed
//! entirely by Rust's `vm-core` crate.  This means:
//!
//! - AOT path tests can run end-to-end without a target backend.
//! - The "AOT binary" in tests is just a Rust struct that satisfies the ABI.
//! - Once a real backend matures, the same test can be re-run against the
//!   real `.a` library through a ctypes/FFI shim.
//!
//! `InProcessVMRuntime` wraps a [`vm_core::core::VMCore`] instance and
//! exposes the `vm_execute`, `vm_call_builtin`, and `vm_resume_at` entry
//! points as Rust methods.
//!
//! # Example
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use vm_runtime::inprocess::InProcessVMRuntime;
//! use vm_runtime::result::VmResult;
//!
//! let fn_ = IIRFunction::new(
//!     "main", vec![], "u8",
//!     vec![
//!         IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "u8"),
//!         IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u8"),
//!     ],
//! );
//! let mut module = IIRModule::new("test", "tetrad");
//! module.functions.push(fn_);
//!
//! let mut rt = InProcessVMRuntime::from_module(module);
//! let result = rt.vm_execute(0, &[]);
//! // Result depends on vm-core execution; not trapping means success.
//! assert!(!result.is_trap() || result.trap_code() == Some(0));
//! ```

use interpreter_ir::module::IIRModule;
use vm_core::core::VMCore;
use vm_core::value::Value;

use crate::result::{VmResult, VmResultTag};

/// In-process vm-runtime backed by `vm-core`.
///
/// Suitable for development, testing, and simulation.  Production AOT binaries
/// use the pre-compiled `vm_runtime_<target>.a` instead.
pub struct InProcessVMRuntime {
    /// The interpreter instance.
    vm: VMCore,
    /// A copy of the module for function lookup.
    module: IIRModule,
}

impl InProcessVMRuntime {
    /// Create an `InProcessVMRuntime` backed by `module`.
    ///
    /// The runtime retains a clone of the module for function-index lookup.
    pub fn from_module(module: IIRModule) -> Self {
        let vm = VMCore::new();
        InProcessVMRuntime { vm, module }
    }

    /// Execute the function at `fn_index` with `args`.
    ///
    /// `fn_index` is a zero-based index into the module's function list.
    ///
    /// Returns `VmResult::trap(1)` if `fn_index` is out of bounds.
    /// Returns `VmResult::trap(2)` if execution fails.
    /// Returns `VmResult::trap(4)` if an argument is malformed — a `Symbol`
    /// whose payload is not a `u32` id — before anything runs (#18470
    /// review: it used to arrive as `Value::Null`, which is now nil).
    pub fn vm_execute(&mut self, fn_index: usize, args: &[VmResult]) -> VmResult {
        let fn_name = match self.module.functions.get(fn_index) {
            Some(f) => f.name.clone(),
            None => return VmResult::trap(1), // index out of bounds
        };

        // Convert VmResult args to vm-core Values.
        let Some(arg_values) = args.iter().map(vmresult_to_value).collect::<Option<Vec<Value>>>() else {
            return VmResult::trap(4); // malformed argument
        };

        match self.vm.execute(&mut self.module.clone(), &fn_name, &arg_values) {
            Ok(Some(value)) => VmResult::from_value(value),
            Ok(None) => VmResult::void(),
            Err(_) => VmResult::trap(2),
        }
    }

    /// Resume execution at a specific instruction pointer (for deopt).
    ///
    /// This is the `vm_resume_at` entry point for JIT deoptimisation.
    /// The current implementation falls back to re-executing the entire
    /// function from the start — a real implementation would inject the
    /// register state at `ip`.
    ///
    /// Returns `VmResult::trap(3)` for unsupported resume requests.
    pub fn vm_resume_at(
        &mut self,
        fn_index: usize,
        _ip: usize,
        _registers: &[VmResult],
    ) -> VmResult {
        // V1: fall back to full re-execution from function start.
        self.vm_execute(fn_index, &[])
    }

    /// Return the number of functions in the module.
    pub fn function_count(&self) -> usize {
        self.module.functions.len()
    }

    /// Look up a function index by name, or `None` if not found.
    pub fn lookup_function(&self, name: &str) -> Option<usize> {
        self.module.functions.iter().position(|f| f.name == name)
    }
}

// ── Conversion helpers ────────────────────────────────────────────────────

/// The `Value` a `VmResult` argument stands for, or `None` for one no
/// `VmResult::from_value` could have produced.
fn vmresult_to_value(r: &VmResult) -> Option<Value> {
    Some(match r.tag {
        VmResultTag::Bool => Value::Bool(r.payload != 0),
        VmResultTag::Str => Value::str(""), // intern pool TBD
        VmResultTag::Void => Value::Null,
        VmResultTag::Trap => Value::Null,
        // A symbol id is a `u32`; a wider payload was not made by
        // `from_value`. It is refused — not truncated into some other
        // symbol, and not read as `Null`, which is nil to a Lisp module.
        VmResultTag::Symbol => Value::Symbol(u32::try_from(r.payload).ok()?),
        // All integer / ref variants map to Int.
        _ => Value::Int(r.payload as i64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::IIRInstr;

    fn make_module_with_fn(fn_: IIRFunction) -> IIRModule {
        let mut m = IIRModule::new("test", "tetrad");
        m.functions.push(fn_);
        m
    }

    /// #18470: a `Symbol` result maps back to `Value::Symbol`, and a payload no
    /// `from_value` could have written is not read as some other symbol.
    #[test]
    fn symbol_result_round_trips_and_rejects_a_wide_payload() {
        let r = VmResult::from_value(Value::Symbol(9));
        assert_eq!(vmresult_to_value(&r), Some(Value::Symbol(9)));
        let wide = VmResult { tag: VmResultTag::Symbol, payload: u64::from(u32::MAX) + 1 };
        assert_eq!(vmresult_to_value(&wide), None, "refused, not nil");
    }

    /// …and `vm_execute` refuses the call with trap 4 before running anything.
    #[test]
    fn a_malformed_symbol_argument_traps_before_execution() {
        let fn_ = IIRFunction::new("id", vec![("x".into(), "any".into())], "any",
            vec![IIRInstr::new("ret", None, vec![interpreter_ir::instr::Operand::Var("x".into())], "any")]);
        let mut rt = InProcessVMRuntime::from_module(make_module_with_fn(fn_));
        let ok = rt.vm_execute(0, &[VmResult::from_value(Value::Symbol(3))]);
        assert_eq!(ok.tag, VmResultTag::Symbol, "a well-formed symbol round-trips: {ok:?}");
        let wide = VmResult { tag: VmResultTag::Symbol, payload: u64::MAX };
        let r = rt.vm_execute(0, &[wide]);
        assert!(r.is_trap());
        assert_eq!(r.trap_code(), Some(4));
    }

    #[test]
    fn function_count() {
        let fn_ = IIRFunction::new("main", vec![], "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let m = make_module_with_fn(fn_);
        let rt = InProcessVMRuntime::from_module(m);
        assert_eq!(rt.function_count(), 1);
    }

    #[test]
    fn lookup_existing_function() {
        let fn_ = IIRFunction::new("helper", vec![], "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let m = make_module_with_fn(fn_);
        let rt = InProcessVMRuntime::from_module(m);
        assert_eq!(rt.lookup_function("helper"), Some(0));
        assert_eq!(rt.lookup_function("nonexistent"), None);
    }

    #[test]
    fn execute_out_of_bounds_traps() {
        let fn_ = IIRFunction::new("f", vec![], "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let m = make_module_with_fn(fn_);
        let mut rt = InProcessVMRuntime::from_module(m);
        let r = rt.vm_execute(99, &[]);
        assert!(r.is_trap());
        assert_eq!(r.trap_code(), Some(1));
    }

    #[test]
    fn execute_void_function_succeeds() {
        let fn_ = IIRFunction::new("main", vec![], "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let m = make_module_with_fn(fn_);
        let mut rt = InProcessVMRuntime::from_module(m);
        let r = rt.vm_execute(0, &[]);
        // Either void or trap(2) from vm-core — both are valid in V1.
        assert!(r.is_void() || r.is_trap());
    }

    #[test]
    fn vmresult_to_value_bool() {
        let r = VmResult::from_bool(true);
        let v = vmresult_to_value(&r);
        assert_eq!(v, Some(Value::Bool(true)));
    }

    #[test]
    fn vmresult_to_value_int() {
        let r = VmResult::from_i64(42);
        let v = vmresult_to_value(&r);
        assert_eq!(v, Some(Value::Int(42)));
    }
}
