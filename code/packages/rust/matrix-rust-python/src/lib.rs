//! # `matrix-rust-python` — Python C extension for the Rust matrix execution layer
//!
//! **Phase 1** of [MX09](../../../specs/MX09-matrix-rust-python.md).
//! This is the minimal-viable shape of the Python extension crate:
//! one exported function (`graph_round_trip_json`) that takes the
//! canonical JSON wire format for a `matrix_ir::Graph`, parses it via
//! `matrix-ir-json`, re-encodes it back to JSON, and returns the
//! result.
//!
//! The round-trip is intentionally trivial in v0.1 — it only proves
//! three things:
//!
//! 1. **The build pipeline works.**  We can compile a `cdylib` that
//!    depends on the matrix-ir-json crate and have it load as a
//!    Python extension (`PyInit_matrix_rust_python` symbol present,
//!    module imports cleanly).
//! 2. **The Python C API boundary works.**  Strings can move into the
//!    extension via `str_from_py`, get processed by Rust code, and
//!    come back out via `str_to_py` without losing data.
//! 3. **The matrix-ir-json schema is the right interop boundary.**
//!    A graph constructed anywhere — Rust, browser TS via a future
//!    `matrix-ir-ts`, hand-written JSON — can be checked for schema
//!    validity by round-tripping through this function.  Anything
//!    that fails to round-trip is a bug.
//!
//! **Phase 2** adds an envelope-shaped `run_graph_on_cpu` for
//! end-to-end execution; **Phase 2b** adds `Graph` and `Runtime`
//! Python classes backed by `PyCapsule`.  This file stays small and
//! grows gradually with each PR — same pattern as `matrix-rust-napi`.
//!
//! ## What Python sees
//!
//! ```python
//! import matrix_rust_python as m
//!
//! json_in = json.dumps({
//!   "matrix_ir_version": 1,
//!   "tensors": [
//!     {"id": 0, "dtype": "f32", "shape": [1, 4]},
//!     {"id": 1, "dtype": "f32", "shape": [4, 1]},
//!     {"id": 2, "dtype": "f32", "shape": [1, 1]},
//!   ],
//!   "inputs":  [0],
//!   "outputs": [2],
//!   "ops": [
//!     {"kind": "MatMul", "lhs": 0, "rhs": 1, "output": 2},
//!   ],
//!   "constants": [
//!     {"tensor_id": 1, "dtype": "f32", "shape": [4, 1],
//!      "bytes_hex": "00000000000000000000000000000000"},
//!   ],
//! })
//!
//! round_tripped = m.graph_round_trip_json(json_in)
//! # round_tripped is a JSON string that decodes to the same Graph value.
//! ```
//!
//! ## Internals
//!
//! Two layers — same shape as `matrix-rust-napi`:
//!
//! 1. **Pure Rust** (`pub fn round_trip_json`) — the actual work.
//!    Cargo-testable: no Python required.  This is the function that
//!    Phase 2 will compose into the envelope/class exports.
//! 2. **Python C API wrapper** (`extern "C" fn py_graph_round_trip_json`)
//!    — un-marshals the Python `str`, calls `round_trip_json`,
//!    marshals the result back.  Errors get raised as Python
//!    `ValueError`s via `python-bridge::set_error` +
//!    `value_error_class()`.

#![allow(non_snake_case)] // C API names are PascalCase / SCREAMING_SNAKE_CASE

mod classes;
mod exec;

use std::ffi::{c_char, c_int};
use std::ptr;

use coding_adventures_json_value::JsonValue;
use matrix_ir_json::{decode, encode};
use python_bridge::{
    set_error, str_from_py, str_to_py, value_error_class, PyErr_Clear, PyMethodDef,
    PyModuleDef, PyModuleDef_Base, PyModule_Create2, PyObjectPtr, PyTuple_GetItem,
    PYTHON_API_VERSION, METH_VARARGS,
};

pub use exec::run_graph_on_cpu;

// ─────────────────────────────────────────────────────────────────────────────
// Pure Rust core: the testable round-trip.
//
// Splitting this from the Python wrapper lets `cargo test` exercise
// the round-trip without any Python interpreter.  It also keeps the
// wrapper trivial — one un-marshal, one call, one marshal — so the
// `unsafe` surface stays small.
// ─────────────────────────────────────────────────────────────────────────────

/// Round-trip a `matrix_ir::Graph` through its JSON wire format.
///
/// Returns the re-encoded JSON string.  The output is byte-equal to
/// the input *modulo*:
///
/// * whitespace (input may be pretty-printed; output is compact);
/// * key ordering inside objects (input may be in any order; output
///   follows the canonical order from `matrix-ir-json`).
///
/// The decoded `Graph` value is semantically identical to one
/// constructed natively in Rust — that is the property we rely on
/// for Phase 2+ execution.
pub fn round_trip_json(input: &str) -> Result<String, String> {
    let graph = decode(input).map_err(|e| format!("matrix-ir-json decode failed: {:?}", e))?;
    Ok(encode(&graph))
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2: end-to-end execution on the CPU executor.
//
// The Rust helper `exec::run_graph_on_cpu` (ported from
// matrix-rust-napi's exec.rs) does the real work; this section adds
// a JSON-envelope wrapper so we can expose it over the existing
// string-in / string-out Python C API surface without yet wiring
// bytes marshalling into the wrapper.
//
// Envelope shape (in):
//   { "graph":  <matrix-ir-json schema>,
//     "inputs": [ "<lowercase-hex bytes>", ... ] }
//
// Envelope shape (out):
//   { "outputs": [ "<lowercase-hex bytes>", ... ] }
//
// Per-tensor byte strings use the same hex encoding the
// matrix-ir-json crate uses for constants — lowercase, no separator,
// no 0x prefix, length always `2 * num_bytes`.  Phase 2b will
// replace this with real `bytes` marshalling via python-bridge's
// `bytes_to_py` / `bytes_from_py` helpers (already shipped); for
// Phase 2 the JSON envelope keeps the Python C API surface tiny
// (one function, identical pattern to graph_round_trip_json) while
// still proving the full plan + execute + return path works.
//
// This is the exact mirror of matrix-rust-napi's Phase 2 surface —
// the envelope shape is bit-identical so the same envelope JSON
// works against either binding.  Useful for cross-binding
// equivalence testing.
// ─────────────────────────────────────────────────────────────────────────────

/// Hex-decode a lowercase string (no separators, no `0x` prefix)
/// into bytes.  Used to parse `inputs[i]` from the envelope JSON.
///
/// Rejects odd-length strings and any non-hex character with a
/// precise error — fail-closed at the trust boundary.
fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err(format!("hex string length must be even, got {}", s.len()));
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(10 + b - b'a'),
        b'A'..=b'F' => Ok(10 + b - b'A'),
        other => Err(format!("invalid hex character: 0x{:02X}", other)),
    }
}

/// Hex-encode a byte slice as a lowercase, separator-free,
/// 0x-prefix-free string.  Used to serialise `outputs[i]` in the
/// result envelope.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

/// Helper: get a required object field by name, return a clear error
/// if it's missing or the parent is not an object.
fn envelope_field<'a>(v: &'a JsonValue, key: &str) -> Result<&'a JsonValue, String> {
    match v {
        JsonValue::Object(fields) => fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
            .ok_or_else(|| format!("envelope missing required field '{}'", key)),
        _ => Err(format!("envelope must be a JSON object (looking for '{}')", key)),
    }
}

/// Pure-Rust entry point for the JSON-envelope-shaped `run_graph_on_cpu`.
/// Parses the envelope, hex-decodes inputs, decodes the graph,
/// executes via [`run_graph_on_cpu`], hex-encodes outputs, returns
/// the result envelope JSON.
///
/// All errors are stringified into `Err(String)` — the Python C API
/// wrapper turns them into `ValueError`.  Never panics on adversarial
/// input.
pub fn run_graph_on_cpu_via_json_envelope(envelope_json: &str) -> Result<String, String> {
    // ── parse envelope ──────────────────────────────────────────
    let env = coding_adventures_json_value::parse(envelope_json)
        .map_err(|e| format!("envelope parse failed: {:?}", e))?;

    // ── extract `graph` field, re-serialise it, pass to matrix-ir-json::decode ──
    //
    // We re-serialise rather than pass the JsonValue directly because
    // matrix-ir-json's public API is `&str -> Graph`, not
    // `JsonValue -> Graph`.  One extra serialise per call; cost is
    // negligible at typical graph sizes and the API is cleaner.
    let graph_value = envelope_field(&env, "graph")?;
    let graph_str = coding_adventures_json_serializer::serialize(graph_value)
        .map_err(|e| format!("graph re-serialise failed: {:?}", e))?;
    let graph = decode(&graph_str)
        .map_err(|e| format!("matrix-ir-json decode failed: {:?}", e))?;

    // ── extract `inputs` field — array of hex strings ──────────
    let inputs_value = envelope_field(&env, "inputs")?;
    let inputs_array = match inputs_value {
        JsonValue::Array(xs) => xs,
        _ => return Err("envelope.inputs must be a JSON array".to_string()),
    };
    let mut inputs: Vec<Vec<u8>> = Vec::with_capacity(inputs_array.len());
    for (i, item) in inputs_array.iter().enumerate() {
        match item {
            JsonValue::String(s) => inputs
                .push(hex_decode(s).map_err(|e| format!("envelope.inputs[{}]: {}", i, e))?),
            _ => return Err(format!("envelope.inputs[{}] must be a hex string", i)),
        }
    }

    // ── execute ─────────────────────────────────────────────────
    let outputs = run_graph_on_cpu(&graph, &inputs)?;

    // ── build result envelope ───────────────────────────────────
    let outputs_array = JsonValue::Array(
        outputs
            .iter()
            .map(|b| JsonValue::String(hex_encode(b)))
            .collect(),
    );
    let envelope_out = JsonValue::Object(vec![("outputs".to_string(), outputs_array)]);
    coding_adventures_json_serializer::serialize(&envelope_out)
        .map_err(|e| format!("output envelope serialise failed: {:?}", e))
}

/// `run_graph_on_cpu(envelope_json: str) -> str`
///
/// Python entry point for envelope-shaped one-shot execution on the
/// CPU executor.  Argument arity is 1 (the envelope JSON); any other
/// arity raises `ValueError`.  Malformed JSON, missing fields,
/// invalid hex, planner errors, executor errors, or oversized graphs
/// all raise `ValueError`.
///
/// # Safety
///
/// Called by Python via the registered module methods table.  `args`
/// is a valid `PyObject*` tuple for the duration of the call per the
/// Python C API contract.
unsafe extern "C" fn py_run_graph_on_cpu(
    _self: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    // Extract args[0] — the envelope JSON string.
    let arg0 = PyTuple_GetItem(args, 0);
    if arg0.is_null() {
        PyErr_Clear();
        set_error(
            value_error_class(),
            "run_graph_on_cpu: expected exactly 1 argument (envelope_json: str)",
        );
        return ptr::null_mut();
    }

    let envelope = match str_from_py(arg0) {
        Some(s) => s,
        None => {
            set_error(
                value_error_class(),
                "run_graph_on_cpu: argument 0 must be a str",
            );
            return ptr::null_mut();
        }
    };

    match run_graph_on_cpu_via_json_envelope(&envelope) {
        Ok(out) => str_to_py(&out),
        Err(msg) => {
            set_error(
                value_error_class(),
                &format!("run_graph_on_cpu: {}", msg),
            );
            ptr::null_mut()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Python C API wrapper
//
// Every METH_VARARGS callback has the same signature:
//   PyObject* fn(PyObject* _self, PyObject* args)
// where `args` is a tuple of the positional arguments.
//
// We extract args[0] as a Python str, call `round_trip_json`, and
// return a fresh Python str.  On error we raise a Python ValueError
// (via `set_error` + `value_error_class()`) and return null — Python
// treats a null return + active exception as "this function raised".
// ─────────────────────────────────────────────────────────────────────────────

/// `graph_round_trip_json(json_string: str) -> str`
///
/// Python entry point for the round-trip helper.  Argument arity is
/// 1 (the JSON string); any other arity raises `TypeError`.  Invalid
/// JSON (or schema-invalid JSON) raises `ValueError`.
///
/// # Safety
///
/// Called by Python via the registered module methods table.  `args`
/// is a valid `PyObject*` tuple for the duration of the call per the
/// Python C API contract.
unsafe extern "C" fn py_graph_round_trip_json(
    _self: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    // Extract args[0] — the JSON string.
    //
    // PyTuple_GetItem returns null + sets an IndexError on
    // out-of-range access.  We clear that pending error before
    // setting our own so we don't trip the "exception already set"
    // assertion in PyErr_SetString.
    let arg0 = PyTuple_GetItem(args, 0);
    if arg0.is_null() {
        PyErr_Clear();
        set_error(
            value_error_class(),
            "graph_round_trip_json: expected exactly 1 argument (json_string: str)",
        );
        return ptr::null_mut();
    }

    let json = match str_from_py(arg0) {
        Some(s) => s,
        None => {
            // str_from_py clears any pending exception itself, so it
            // is safe to set ours.
            set_error(
                value_error_class(),
                "graph_round_trip_json: argument 0 must be a str",
            );
            return ptr::null_mut();
        }
    };

    match round_trip_json(&json) {
        Ok(out) => str_to_py(&out),
        Err(msg) => {
            set_error(
                value_error_class(),
                &format!("graph_round_trip_json: {}", msg),
            );
            ptr::null_mut()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Module methods table
//
// `PyMethodDef` entries tell Python about each exported function.
// The table must be terminated with a sentinel (all-null entry); we
// build it inline to mirror font-parser-python's style.
// ─────────────────────────────────────────────────────────────────────────────

static mut MODULE_METHODS: [PyMethodDef; 3] = [
    PyMethodDef {
        ml_name: b"graph_round_trip_json\0".as_ptr() as *const c_char,
        ml_meth: Some(py_graph_round_trip_json),
        ml_flags: METH_VARARGS,
        ml_doc: b"graph_round_trip_json(json_string: str) -> str\n\n\
                  Decode a matrix-ir-json Graph and re-encode it.  \
                  Raises ValueError on malformed or schema-invalid JSON.\n\0"
            .as_ptr() as *const c_char,
    },
    PyMethodDef {
        ml_name: b"run_graph_on_cpu\0".as_ptr() as *const c_char,
        ml_meth: Some(py_run_graph_on_cpu),
        ml_flags: METH_VARARGS,
        ml_doc: b"run_graph_on_cpu(envelope_json: str) -> str\n\n\
                  Plan and execute a matrix-ir-json Graph on the CPU executor.  \
                  Envelope is `{\"graph\": <matrix-ir-json>, \"inputs\": [<hex>, ...]}`; \
                  result is `{\"outputs\": [<hex>, ...]}`.  \
                  Raises ValueError on malformed JSON, missing fields, invalid hex, \
                  planner errors, executor errors, or graphs exceeding the 4 GiB \
                  total-buffer cap.\n\0"
            .as_ptr() as *const c_char,
    },
    // Sentinel — terminates the methods array.
    PyMethodDef {
        ml_name: ptr::null(),
        ml_meth: None,
        ml_flags: 0,
        ml_doc: ptr::null(),
    },
];

// ─────────────────────────────────────────────────────────────────────────────
// Module definition
//
// `PyModuleDef` is the singleton descriptor for our module.  It must
// be a `static` because Python holds a pointer to it for the lifetime
// of the interpreter.  `m_size = -1` opts out of sub-interpreter
// reinitialisation (we have no per-interpreter state in Phase 1; the
// methods table is process-global).
// ─────────────────────────────────────────────────────────────────────────────

static mut MODULE_DEF: PyModuleDef = PyModuleDef {
    m_base: PyModuleDef_Base {
        // PyModuleDef_HEAD_INIT in C initialises ob_refcnt=1,
        // ob_type=NULL, m_init=NULL, m_index=0, m_copy=NULL.
        ob_base: [0u8; std::mem::size_of::<usize>() * 2],
        m_init: None,
        m_index: 0,
        m_copy: ptr::null_mut(),
    },
    m_name: b"matrix_rust_python\0".as_ptr() as *const c_char,
    m_doc: b"Rust-backed matrix execution layer - zero-dependency Python C extension.\0"
        .as_ptr() as *const c_char,
    m_size: -1,
    m_methods: &raw mut MODULE_METHODS as *mut PyMethodDef,
    m_slots: ptr::null_mut(),
    m_traverse: ptr::null_mut(),
    m_clear: ptr::null_mut(),
    m_free: ptr::null_mut(),
};

// ─────────────────────────────────────────────────────────────────────────────
// Module init — the entry point Python calls when it imports the module
//
// The name MUST be `PyInit_<module_name>` where `<module_name>` is
// the name of the .so/.pyd file (without the file extension and ABI
// tag).  `#[no_mangle]` prevents Rust from mangling the symbol —
// Python's import machinery looks for this exact name via `dlsym`
// (POSIX) or `GetProcAddress` (Windows).
// ─────────────────────────────────────────────────────────────────────────────

#[no_mangle]
pub unsafe extern "C" fn PyInit_matrix_rust_python() -> PyObjectPtr {
    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION as c_int);
    if module.is_null() {
        return module;
    }
    // Phase 2b — register Graph + Runtime classes on the module.
    classes::register(module);
    module
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
//
// The unit tests cover the pure-Rust `round_trip_json` helper without
// involving Python.  They prove that:
//
//  * a valid Graph JSON survives the round-trip;
//  * the output decodes to a Graph that's byte-equal under the
//    binary wire format (the cross-format equivalence we rely on for
//    Phase 2+);
//  * malformed JSON returns an Err, not a panic;
//  * schema-invalid JSON (wrong version) is rejected;
//  * the round-trip is idempotent (f(f(x)) == f(x)).
//
// The Python C API wrapper is exercised by Phase 4's end-to-end
// tests (`pytest` via the wrapper package).  Here we keep things
// pure-Rust so they run on `cargo test -p matrix-rust-python` with
// no Python interpreter required.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_ir::{DType, GraphBuilder, Shape};

    /// Build a small ReLU layer and confirm the JSON round-trip is
    /// semantically identical (binary wire format byte-equal).
    ///
    /// This is the canonical "round-trip preserves the graph"
    /// invariant: encode → decode → re-encode → decode must give a
    /// `Graph` whose binary wire-format representation matches the
    /// original.  Anything weaker (e.g. comparing JSON strings) is
    /// brittle to whitespace and field ordering — the binary wire
    /// format is the canonical equality test (see matrix-ir-json's
    /// README).
    #[test]
    fn round_trip_preserves_graph_under_binary_wire_format() {
        let mut g = GraphBuilder::new();
        let x = g.input(DType::F32, Shape::from(&[1, 4]));
        let w = g.constant(DType::F32, Shape::from(&[4, 2]), vec![0u8; 32]);
        let b = g.constant(DType::F32, Shape::from(&[1, 2]), vec![0u8; 8]);
        let zero = g.constant(DType::F32, Shape::from(&[1, 2]), vec![0u8; 8]);
        let xw = g.matmul(&x, &w);
        let xwb = g.add(&xw, &b);
        let relu = g.max(&xwb, &zero);
        g.output(&relu);
        let original = g.build().expect("graph builds");

        let json_in = encode(&original);
        let json_out = round_trip_json(&json_in).expect("round-trip succeeds");

        let round_tripped = decode(&json_out).expect("decode round-trip output");
        assert_eq!(
            original.to_bytes(),
            round_tripped.to_bytes(),
            "round-tripped graph must be byte-equal to the original under \
             the binary wire format"
        );
    }

    /// A graph with multiple Op families is non-trivial enough to be
    /// a stress test of `matrix-ir-json` end-to-end through our
    /// wrapper.  We don't replicate the full all-ops test from
    /// `matrix-ir-json` here (that's its job), but we do confirm a
    /// representative 3-op graph survives.
    #[test]
    fn round_trip_handles_multi_op_graph() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3]));
        let b = g.input(DType::F32, Shape::from(&[2, 3]));
        let sum = g.add(&a, &b);
        let prod = g.mul(&sum, &a);
        let neg = g.neg(&prod);
        g.output(&neg);
        let original = g.build().expect("graph builds");

        let json_in = encode(&original);
        let json_out = round_trip_json(&json_in).expect("round-trip succeeds");
        let round_tripped = decode(&json_out).expect("decode round-trip output");

        assert_eq!(original.to_bytes(), round_tripped.to_bytes());
    }

    /// Garbage JSON must produce an Err — not panic, not crash the
    /// extension, not produce a corrupted graph value.  This is the
    /// fail-closed property we need at the FFI boundary: a bad input
    /// from Python becomes a clean `ValueError`, never a SIGSEGV.
    #[test]
    fn round_trip_rejects_garbage_json() {
        let err = round_trip_json("not even close to json").expect_err("garbage rejected");
        assert!(err.contains("decode failed"), "got: {}", err);
    }

    /// JSON that's syntactically valid but schema-invalid (wrong
    /// matrix_ir_version) must also fail cleanly.  Same fail-closed
    /// property as garbage-JSON, but covers the "newer producer, older
    /// consumer" forward-compat path that's most likely to bite in
    /// practice.
    #[test]
    fn round_trip_rejects_unsupported_version() {
        let err = round_trip_json(
            r#"{"matrix_ir_version": 9999, "tensors": [], "inputs": [],
                 "outputs": [], "ops": [], "constants": []}"#,
        )
        .expect_err("unsupported version rejected");
        assert!(err.contains("decode failed"), "got: {}", err);
    }

    /// The output of the round-trip must itself be valid JSON that
    /// re-round-trips to a byte-identical string.  Idempotence:
    /// `f(f(x)) == f(x)`.  Useful to catch any drift introduced by
    /// the encoder (e.g. if it ever started canonicalising fields
    /// differently between runs).
    #[test]
    fn round_trip_is_idempotent() {
        let mut g = GraphBuilder::new();
        let x = g.input(DType::F32, Shape::from(&[3]));
        let y = g.input(DType::F32, Shape::from(&[3]));
        let z = g.add(&x, &y);
        g.output(&z);
        let graph = g.build().expect("graph builds");

        let once = round_trip_json(&encode(&graph)).unwrap();
        let twice = round_trip_json(&once).unwrap();
        assert_eq!(once, twice, "round_trip_json must be idempotent");
    }

    // ── Phase 2: envelope-shaped end-to-end execution ────────────

    /// Hex encoder round-trips through the decoder for every byte
    /// value 0x00..=0xFF.
    #[test]
    fn hex_round_trips() {
        let bytes = vec![0u8, 1, 2, 0xAB, 0xCD, 0xEF, 0xFF];
        let s = hex_encode(&bytes);
        assert_eq!(s, "000102abcdefff");
        assert_eq!(hex_decode(&s).unwrap(), bytes);
    }

    /// Hex decoder rejects odd-length input.
    #[test]
    fn hex_decoder_rejects_odd_length() {
        assert!(hex_decode("abc").is_err());
    }

    /// Hex decoder rejects non-hex characters.
    #[test]
    fn hex_decoder_rejects_bad_chars() {
        assert!(hex_decode("zz").is_err());
    }

    /// Headline envelope test: build an Add graph, package it + inputs
    /// into the envelope JSON, run through the string-in / string-out
    /// entry point, parse the result envelope, hex-decode outputs,
    /// assert.  This is the end-to-end property that proves the
    /// envelope path from string → Graph → execute → string works.
    #[test]
    fn envelope_runs_add_end_to_end() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[3]));
        let b = g.input(DType::F32, Shape::from(&[3]));
        let c = g.add(&a, &b);
        g.output(&c);
        let graph_json = encode(&g.build().expect("graph builds"));

        let envelope = format!(
            r#"{{ "graph": {}, "inputs": ["{}", "{}"] }}"#,
            graph_json,
            hex_encode(
                &[1.0f32, 2.0, 3.0]
                    .iter()
                    .flat_map(|f| f.to_le_bytes())
                    .collect::<Vec<u8>>()
            ),
            hex_encode(
                &[10.0f32, 20.0, 30.0]
                    .iter()
                    .flat_map(|f| f.to_le_bytes())
                    .collect::<Vec<u8>>()
            ),
        );

        let out_envelope = run_graph_on_cpu_via_json_envelope(&envelope)
            .expect("envelope execution succeeds");

        // Parse result envelope and read first output bytes.
        let parsed = coding_adventures_json_value::parse(&out_envelope)
            .expect("output envelope parses");
        let outputs = envelope_field(&parsed, "outputs").unwrap();
        let arr = match outputs {
            JsonValue::Array(xs) => xs,
            _ => panic!("outputs not array"),
        };
        let bytes = match &arr[0] {
            JsonValue::String(s) => hex_decode(s).unwrap(),
            _ => panic!("output not string"),
        };
        let floats: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(floats, vec![11.0, 22.0, 33.0]);
    }

    /// Envelope without a `graph` field must fail cleanly — never
    /// panic across the FFI boundary.
    #[test]
    fn envelope_rejects_missing_graph() {
        let err = run_graph_on_cpu_via_json_envelope(r#"{"inputs": []}"#)
            .expect_err("missing graph field rejected");
        assert!(err.contains("graph"), "got: {}", err);
    }

    /// Envelope where `inputs` is not an array must fail cleanly.
    #[test]
    fn envelope_rejects_non_array_inputs() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2]));
        g.output(&a);
        let graph_json = encode(&g.build().unwrap());
        let envelope = format!(
            r#"{{ "graph": {}, "inputs": "not-an-array" }}"#,
            graph_json
        );

        let err = run_graph_on_cpu_via_json_envelope(&envelope)
            .expect_err("inputs-not-array rejected");
        assert!(err.contains("inputs"), "got: {}", err);
    }

    /// Envelope where an entry of `inputs` contains invalid hex must
    /// fail cleanly with a precise error pointing at the bad index.
    #[test]
    fn envelope_rejects_invalid_hex_input() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[1]));
        g.output(&a);
        let graph_json = encode(&g.build().unwrap());
        let envelope = format!(
            r#"{{ "graph": {}, "inputs": ["zzzz"] }}"#,
            graph_json
        );

        let err = run_graph_on_cpu_via_json_envelope(&envelope)
            .expect_err("non-hex input rejected");
        assert!(
            err.contains("envelope.inputs[0]") && err.contains("hex"),
            "got: {}",
            err
        );
    }

    /// Garbage envelope JSON must fail with a parse error — never
    /// panic, never crash the interpreter.
    #[test]
    fn envelope_rejects_garbage_json() {
        let err = run_graph_on_cpu_via_json_envelope("not even close to json")
            .expect_err("garbage rejected");
        assert!(err.contains("parse"), "got: {}", err);
    }
}
