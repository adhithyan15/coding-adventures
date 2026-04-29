//! `VmRuntime` — pre-compiled vm-runtime library for linking into AOT binaries.
//!
//! When a program uses dynamic features (runtime polymorphism, closures, string
//! operations, dynamic dispatch) that cannot be compiled to static native code,
//! `aot-core` falls back to including those functions in the **IIR table
//! section** of the `.aot` binary.  At execution time, the vm-runtime library
//! interprets those functions on demand.
//!
//! # What is a vm-runtime?
//!
//! The vm-runtime is a compiled, linkable form of `vm-core` — the same
//! interpreter dispatch loop that jit-core uses, but pre-compiled to the target
//! architecture as a static library embedded in the `.aot` binary.
//!
//! # Serialisation format
//!
//! The IIR table is **JSON-encoded**.  Each function is a JSON object:
//!
//! ```json
//! {
//!   "name": "helper",
//!   "params": [["x", "u8"], ["y", "u8"]],
//!   "instructions": [
//!     {"op": "add", "dest": "r0", "srcs": ["x", "y"], "type_hint": "any", "deopt_anchor": null}
//!   ],
//!   "type_status": 2
//! }
//! ```
//!
//! `type_status` numeric values:
//! | Value | Meaning |
//! |-------|---------|
//! | 0 | `FullyTyped` |
//! | 1 | `PartiallyTyped` |
//! | 2 | `Untyped` |
//!
//! # Example
//!
//! ```
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use aot_core::vm_runtime::VmRuntime;
//!
//! let fn_ = IIRFunction::new("f", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
//! let rt = VmRuntime::new(vec![]);
//! let bytes = rt.serialise_iir_table(&[fn_]);
//! assert!(bytes.starts_with(b"[{"));
//! let back = rt.deserialise_iir_table(&bytes).unwrap();
//! assert_eq!(back.len(), 1);
//! assert_eq!(back[0]["name"], "f");
//! ```

use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
use interpreter_ir::instr::{IIRInstr, Operand};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// VmRuntime
// ---------------------------------------------------------------------------

/// Wrapper for a pre-compiled vm-runtime library.
///
/// `library_bytes` can be empty when targeting simulators that run the IIR
/// table via the Python `vm-core` interpreter — no native library is needed
/// in that case.
#[derive(Debug, Clone, Default)]
pub struct VmRuntime {
    /// Pre-compiled vm-runtime static library bytes (may be empty).
    pub library_bytes: Vec<u8>,
}

impl VmRuntime {
    /// Create a `VmRuntime` with the given pre-compiled library bytes.
    ///
    /// Pass `vec![]` for simulator targets that use the Python interpreter.
    pub fn new(library_bytes: Vec<u8>) -> Self {
        VmRuntime { library_bytes }
    }

    /// True if no pre-compiled library was provided.
    pub fn is_empty(&self) -> bool {
        self.library_bytes.is_empty()
    }

    // ------------------------------------------------------------------
    // IIR table serialisation
    // ------------------------------------------------------------------

    /// Serialise a list of `IIRFunction` objects to IIR-table bytes.
    ///
    /// The result is suitable for embedding in the `.aot` snapshot's IIR
    /// table section via `snapshot::write(iir_table=Some(&bytes))`.
    ///
    /// # Format
    ///
    /// UTF-8 encoded compact JSON array of function records.
    ///
    /// # Panics
    ///
    /// Never panics — serde_json serialization of a `Value` is infallible.
    pub fn serialise_iir_table(&self, fns: &[IIRFunction]) -> Vec<u8> {
        let records: Vec<Value> = fns.iter().map(serialise_fn).collect();
        let json_str = serde_json::to_string(&records)
            .expect("serde_json::to_string(Value) is infallible");
        json_str.into_bytes()
    }

    /// Parse IIR-table bytes back to plain `serde_json::Value` records.
    ///
    /// Returns a `Vec<Value>` — one entry per function — or an error string
    /// if the bytes are not valid UTF-8 JSON.
    ///
    /// This method is primarily for testing and inspection; the actual runtime
    /// does not call it during normal execution.
    pub fn deserialise_iir_table(&self, data: &[u8]) -> Result<Vec<Value>, String> {
        let s = std::str::from_utf8(data)
            .map_err(|e| format!("invalid UTF-8: {}", e))?;
        serde_json::from_str(s)
            .map_err(|e| format!("invalid JSON: {}", e))
    }
}

// ---------------------------------------------------------------------------
// Serialisation helpers
// ---------------------------------------------------------------------------

/// Convert a `FunctionTypeStatus` enum variant to its integer encoding.
///
/// | Variant | Value |
/// |---------|-------|
/// | `FullyTyped` | 0 |
/// | `PartiallyTyped` | 1 |
/// | `Untyped` | 2 |
fn type_status_value(status: &FunctionTypeStatus) -> u8 {
    match status {
        FunctionTypeStatus::FullyTyped => 0,
        FunctionTypeStatus::PartiallyTyped => 1,
        FunctionTypeStatus::Untyped => 2,
    }
}

/// Convert a single `Operand` to a JSON `Value`.
///
/// | Rust variant | JSON encoding |
/// |---|---|
/// | `Operand::Var(s)` | `"s"` (string) |
/// | `Operand::Int(n)` | `n` (number) |
/// | `Operand::Float(f)` | `f` (number) |
/// | `Operand::Bool(b)` | `true`/`false` |
fn operand_to_json(op: &Operand) -> Value {
    match op {
        Operand::Var(s)    => Value::String(s.clone()),
        Operand::Int(n)    => json!(*n),
        Operand::Float(f)  => json!(*f),
        Operand::Bool(b)   => Value::Bool(*b),
    }
}

/// Serialise a single `IIRInstr` to a JSON object.
fn serialise_instr(instr: &IIRInstr) -> Value {
    let srcs: Vec<Value> = instr.srcs.iter().map(operand_to_json).collect();
    json!({
        "op":          instr.op,
        "dest":        instr.dest,
        "srcs":        srcs,
        "type_hint":   instr.type_hint,
        "deopt_anchor": instr.deopt_anchor,
    })
}

/// Serialise a whole `IIRFunction` to a JSON object.
fn serialise_fn(fn_: &IIRFunction) -> Value {
    let params: Vec<Value> = fn_.params.iter()
        .map(|(n, t)| json!([n, t]))
        .collect();
    let instrs: Vec<Value> = fn_.instructions.iter()
        .map(serialise_instr)
        .collect();
    json!({
        "name":         fn_.name,
        "params":       params,
        "instructions": instrs,
        "type_status":  type_status_value(&fn_.type_status),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn simple_fn() -> IIRFunction {
        IIRFunction::new(
            "f",
            vec![("a".into(), "u8".into())],
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        )
    }

    // ------------------------------------------------------------------
    // VmRuntime basics
    // ------------------------------------------------------------------

    #[test]
    fn is_empty_with_no_bytes() {
        let rt = VmRuntime::new(vec![]);
        assert!(rt.is_empty());
    }

    #[test]
    fn not_empty_with_bytes() {
        let rt = VmRuntime::new(vec![0xDE, 0xAD]);
        assert!(!rt.is_empty());
    }

    // ------------------------------------------------------------------
    // serialise_iir_table
    // ------------------------------------------------------------------

    #[test]
    fn serialise_empty_list() {
        let rt = VmRuntime::new(vec![]);
        let bytes = rt.serialise_iir_table(&[]);
        assert_eq!(bytes, b"[]");
    }

    #[test]
    fn serialise_single_fn_is_valid_json() {
        let rt = VmRuntime::new(vec![]);
        let bytes = rt.serialise_iir_table(&[simple_fn()]);
        assert!(serde_json::from_slice::<Value>(&bytes).is_ok());
    }

    #[test]
    fn serialise_fn_name_present() {
        let rt = VmRuntime::new(vec![]);
        let bytes = rt.serialise_iir_table(&[simple_fn()]);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v[0]["name"], "f");
    }

    #[test]
    fn serialise_params_present() {
        let rt = VmRuntime::new(vec![]);
        let bytes = rt.serialise_iir_table(&[simple_fn()]);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        let params = &v[0]["params"];
        assert_eq!(params[0][0], "a");
        assert_eq!(params[0][1], "u8");
    }

    #[test]
    fn serialise_instructions_present() {
        let rt = VmRuntime::new(vec![]);
        let bytes = rt.serialise_iir_table(&[simple_fn()]);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        let instrs = &v[0]["instructions"];
        assert_eq!(instrs[0]["op"], "ret_void");
    }

    #[test]
    fn serialise_type_status_numeric() {
        // simple_fn has no untyped instructions, so fully typed
        let rt = VmRuntime::new(vec![]);
        let fn_ = IIRFunction::new("g", vec![], "void", vec![]);
        let bytes = rt.serialise_iir_table(&[fn_]);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        // Any numeric value is valid; just assert it's a number.
        assert!(v[0]["type_status"].is_number());
    }

    #[test]
    fn serialise_operand_var() {
        let rt = VmRuntime::new(vec![]);
        let fn_ = IIRFunction::new("h", vec![], "void", vec![
            IIRInstr::new("jmp", None, vec![Operand::Var("loop".into())], "any"),
        ]);
        let bytes = rt.serialise_iir_table(&[fn_]);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v[0]["instructions"][0]["srcs"][0], "loop");
    }

    #[test]
    fn serialise_operand_int() {
        let rt = VmRuntime::new(vec![]);
        let fn_ = IIRFunction::new("h", vec![], "void", vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(42)], "any"),
        ]);
        let bytes = rt.serialise_iir_table(&[fn_]);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v[0]["instructions"][0]["srcs"][0], 42);
    }

    #[test]
    fn serialise_operand_bool() {
        let rt = VmRuntime::new(vec![]);
        let fn_ = IIRFunction::new("h", vec![], "void", vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Bool(true)], "any"),
        ]);
        let bytes = rt.serialise_iir_table(&[fn_]);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v[0]["instructions"][0]["srcs"][0], true);
    }

    // ------------------------------------------------------------------
    // deserialise_iir_table
    // ------------------------------------------------------------------

    #[test]
    fn round_trip() {
        let rt = VmRuntime::new(vec![]);
        let bytes = rt.serialise_iir_table(&[simple_fn()]);
        let back = rt.deserialise_iir_table(&bytes).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0]["name"], "f");
    }

    #[test]
    fn deserialise_invalid_json_returns_error() {
        let rt = VmRuntime::new(vec![]);
        let result = rt.deserialise_iir_table(b"not json");
        assert!(result.is_err());
    }

    #[test]
    fn deserialise_invalid_utf8_returns_error() {
        let rt = VmRuntime::new(vec![]);
        let result = rt.deserialise_iir_table(&[0xFF, 0xFE]);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // type_status_value helper
    // ------------------------------------------------------------------

    #[test]
    fn type_status_fully_typed() {
        assert_eq!(type_status_value(&FunctionTypeStatus::FullyTyped), 0);
    }

    #[test]
    fn type_status_partially_typed() {
        assert_eq!(type_status_value(&FunctionTypeStatus::PartiallyTyped), 1);
    }

    #[test]
    fn type_status_untyped() {
        assert_eq!(type_status_value(&FunctionTypeStatus::Untyped), 2);
    }
}
