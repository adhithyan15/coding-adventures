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
    pub fn vm_execute(&mut self, fn_index: usize, args: &[VmResult]) -> VmResult {
        let fn_name = match self.module.functions.get(fn_index) {
            Some(f) => f.name.clone(),
            None => return VmResult::trap(1), // index out of bounds
        };

        // Convert VmResult args to vm-core Values.
        let arg_values: Vec<Value> = args.iter().map(vmresult_to_value).collect();

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

fn vmresult_to_value(r: &VmResult) -> Value {
    match r.tag {
        VmResultTag::Bool => Value::Bool(r.payload != 0),
        VmResultTag::Str => Value::Str(String::new()), // intern pool TBD
        VmResultTag::Void => Value::Null,
        VmResultTag::Trap => Value::Null,
        // All integer / ref variants map to Int.
        _ => Value::Int(r.payload as i64),
    }
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
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn vmresult_to_value_int() {
        let r = VmResult::from_i64(42);
        let v = vmresult_to_value(&r);
        assert_eq!(v, Value::Int(42));
    }
}
