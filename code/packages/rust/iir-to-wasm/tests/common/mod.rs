//! A WASM test host with the one import every string program needs since
//! LANG83 PR5a: `env.__trap(i32)`.
//!
//! An out-of-range `str_index` / `str_slice` raises `Trap.Bounds` and a
//! `str_concat` past the string ceiling raises `Trap.Overflow`, both through
//! `i32.const <id>; call $env.__trap; unreachable`. A WASM import is resolved
//! when the module is *instantiated*, not when the call runs, so a module that
//! merely contains one of these ops cannot load on a host without the import
//! (`WasmRuntime::new()` has none), even when it never fails. See the lesson
//! "Giving a WASM op a typed LANG83 trap makes every module that uses it
//! import env.__trap".
//!
//! The import records the kind id it receives and returns; the module's own
//! `unreachable` then stops the run, which is what the callers observe as an
//! `Err`.

#![allow(dead_code)] // each test file uses a different subset

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_execution::{GlobalStorage, HostFunction, HostInterface, LinearMemory, Table, TrapError, WasmValue};
use wasm_runtime::WasmRuntime;
use wasm_types::{FuncType, GlobalType, ValueType};

/// The kind id `env.__trap` received, if the program trapped.
pub type TrapSlot = Rc<Cell<Option<i32>>>;

struct TrapImport(FuncType, TrapSlot);

impl HostFunction for TrapImport {
    fn func_type(&self) -> &FuncType {
        &self.0
    }
    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        match args.first() {
            Some(WasmValue::I32(id)) => {
                self.1.set(Some(*id));
                Ok(vec![])
            }
            other => Err(TrapError::new(format!("env.__trap: expected an i32 kind id, got {other:?}"))),
        }
    }
}

struct TrapHost(TrapSlot);

impl HostInterface for TrapHost {
    fn resolve_function(&self, module: &str, name: &str) -> Option<Box<dyn HostFunction>> {
        (module == "env" && name == "__trap").then(|| {
            Box::new(TrapImport(FuncType { params: vec![ValueType::I32], results: vec![] }, Rc::clone(&self.0)))
                as Box<dyn HostFunction>
        })
    }
    fn resolve_global(&self, _: &str, _: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
        None
    }
    fn resolve_memory(&self, _: &str, _: &str) -> Option<LinearMemory> {
        None
    }
    fn resolve_table(&self, _: &str, _: &str) -> Option<Table> {
        None
    }
}

/// A runtime whose host provides `env.__trap`, and the slot it records into.
pub fn runtime_with_trap_slot() -> (WasmRuntime, TrapSlot) {
    let slot: TrapSlot = Rc::default();
    (WasmRuntime::with_host(Box::new(TrapHost(Rc::clone(&slot)))), slot)
}

/// A runtime whose host provides `env.__trap` (the kind is not inspected).
pub fn runtime() -> WasmRuntime {
    runtime_with_trap_slot().0
}
