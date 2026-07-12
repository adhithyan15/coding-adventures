//! # `c-bridge` — Stable C ABI for matrix-cpu graph execution
//!
//! Single-function bridge crate.  Exposes the same
//! `run_graph_on_cpu_via_json_envelope` shape that `matrix-rust-python`
//! (Python C ext) and `matrix-rust-napi` (Node.js binding) offer to
//! their respective hosts — but here through a **language-agnostic
//! C ABI** that any FFI-capable language can drive.
//!
//! ## The contract — two C functions
//!
//! ```c
//! /* Run a matrix-ir-json envelope through matrix-cpu and return the
//!    output envelope as a malloc'd, null-terminated UTF-8 string.
//!
//!    On success: returns a non-NULL pointer; *err_out (if non-NULL) is
//!    set to NULL.  The caller MUST release the returned string with
//!    matrix_cpu_free_string().
//!
//!    On failure: returns NULL.  *err_out (if non-NULL) is set to a
//!    malloc'd UTF-8 error message which the caller MUST also release
//!    with matrix_cpu_free_string().  If err_out is NULL the error
//!    message is discarded (the caller only knows execution failed).
//!
//!    Never panics on adversarial input — all error paths return
//!    NULL + error string.  Safe to call from any thread; matrix-cpu's
//!    executor is fresh per call (no shared mutable state). */
//! char* matrix_cpu_run_graph(const char* envelope_json,
//!                            char**       err_out);
//!
//! /* Drop a string previously returned by matrix_cpu_run_graph
//!    (either via the return value or via *err_out).  Passing NULL is
//!    a no-op.  After this call the pointer is invalidated. */
//! void matrix_cpu_free_string(char* s);
//! ```
//!
//! ## Memory contract
//!
//! Both returned strings are allocated with Rust's `CString::into_raw`,
//! which uses the Rust allocator.  The caller cannot free them with
//! `libc::free()` — they MUST use `matrix_cpu_free_string`.  Mixing
//! allocators is undefined behaviour.
//!
//! ## Why JSON-envelope on the wire?
//!
//! Every language has JSON.  The matrix-ir-json crate already defines
//! a stable wire format for graphs and tensors.  Building or parsing a
//! few KB of JSON per FFI call is microseconds — negligible compared
//! to the actual matmul / reduce / activation work being done on the
//! other side of the boundary.
//!
//! Binary alternatives (FlatBuffers, Cap'n Proto, raw bytes) would
//! be faster on paper, but force every language binding to drag in a
//! schema compiler.  JSON keeps the per-language binding code tiny.
//!
//! ## Example consumers
//!
//! - `matrix-rust-ruby` (Ruby gem with rb-sys → c-bridge)
//! - `matrix-rust-lua` (LuaRocks module via LuaJIT FFI)
//! - `matrix-rust-go` (cgo binding)
//! - `matrix-rust-swift` (SwiftPM with @_cdecl)
//! - Any host that can load `libmatrix_c_bridge.{so,dylib,dll}`
//!
//! See the per-language packages for usage idioms.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, CStr, CString};
use std::ptr;

use coding_adventures_json_value::JsonValue;
use matrix_ir_json::decode;

mod exec;

pub use exec::run_graph_on_cpu;

// ─────────────────────────────────────────────────────────────────────
// Pure-Rust core: envelope helpers + the JSON-envelope-shaped entry
// point.  Identical shape to matrix-rust-python's
// `run_graph_on_cpu_via_json_envelope` — kept duplicated here for v1
// so c-bridge has zero coupling to a Python-binding crate.  A future
// refactor PR can DRY into a shared `matrix-rust-core` crate once
// ≥2 language bindings exist using the same shape.
// ─────────────────────────────────────────────────────────────────────

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Hex-decode one input string (case-insensitive, no separators).
/// Returns an error if the length is odd or any char is non-hex.
fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err(format!("hex string has odd length {}", s.len()));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = decode_hex_char(bytes[i])?;
        let lo = decode_hex_char(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn decode_hex_char(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("invalid hex char {:?}", b as char)),
    }
}

/// Hex-encode (lowercase, no separators) one byte slice.
fn hex_encode(bytes: &[u8]) -> String {
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
        _ => Err(format!(
            "envelope must be a JSON object (looking for '{}')",
            key
        )),
    }
}

/// Pure-Rust entry point for the JSON-envelope-shaped `run_graph_on_cpu`.
/// Parses the envelope, hex-decodes inputs, decodes the graph,
/// executes via [`run_graph_on_cpu`], hex-encodes outputs, returns
/// the result envelope JSON.
///
/// All errors are stringified into `Err(String)`.  Never panics on
/// adversarial input — this is the contract the C ABI relies on to
/// turn errors into clean error strings rather than aborts.
pub fn run_graph_on_cpu_via_json_envelope(envelope_json: &str) -> Result<String, String> {
    // ── parse envelope ──────────────────────────────────────────
    let env = coding_adventures_json_value::parse(envelope_json)
        .map_err(|e| format!("envelope parse failed: {:?}", e))?;

    // ── extract `graph` field, re-serialise it, pass to matrix-ir-json::decode ──
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

// ─────────────────────────────────────────────────────────────────────
// C ABI surface
// ─────────────────────────────────────────────────────────────────────

/// Convert a Rust `String` into a `CString` and leak it as a raw pointer.
/// The caller takes ownership and must free via [`matrix_cpu_free_string`].
///
/// If the string contains interior NUL bytes (legal in Rust UTF-8 but
/// illegal in C strings) we substitute a generic error message rather
/// than panicking — preserves the "never panic across the C boundary"
/// contract.
fn leak_cstring(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => CString::new("c-bridge: string contained interior NUL bytes")
            .expect("static literal has no NULs")
            .into_raw(),
    }
}

/// Run a matrix-ir-json envelope through matrix-cpu.
///
/// # Safety
///
/// - `envelope_json` must be a valid pointer to a null-terminated UTF-8 string.
/// - `err_out` may be NULL (errors then discarded) or a valid pointer to a
///   `*mut c_char` slot that we may write to.
/// - The returned pointer (if non-NULL) is owned by the caller and must be
///   freed via [`matrix_cpu_free_string`].
/// - The string written to `*err_out` (if non-NULL) is also caller-owned
///   and must be freed via [`matrix_cpu_free_string`].
///
/// Never panics on adversarial input — all error paths return NULL +
/// error string.
#[no_mangle]
pub unsafe extern "C" fn matrix_cpu_run_graph(
    envelope_json: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut c_char {
    // SAFETY: if err_out is non-NULL we clear it first so callers can
    // rely on "NULL out-param means success" without checking the
    // return value first.
    if !err_out.is_null() {
        unsafe {
            *err_out = ptr::null_mut();
        }
    }

    // NULL input → error, no execution.
    if envelope_json.is_null() {
        if !err_out.is_null() {
            unsafe {
                *err_out = leak_cstring("envelope_json is NULL".to_string());
            }
        }
        return ptr::null_mut();
    }

    // Convert the C string to a Rust &str.  CStr::from_ptr requires
    // the C string to be valid (we trust the caller — this is the FFI
    // contract).  to_str rejects non-UTF-8 with a clean error.
    let envelope: &str = match unsafe { CStr::from_ptr(envelope_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            if !err_out.is_null() {
                unsafe {
                    *err_out = leak_cstring(format!("envelope_json is not UTF-8: {}", e));
                }
            }
            return ptr::null_mut();
        }
    };

    // Run the pure-Rust core.  Catch ANY panic that escapes the
    // executor (shouldn't happen, but defence in depth — panicking
    // across an FFI boundary is undefined behaviour in some Rust
    // versions, so we contain it).
    let result = std::panic::catch_unwind(|| run_graph_on_cpu_via_json_envelope(envelope));

    match result {
        Ok(Ok(out)) => leak_cstring(out),
        Ok(Err(msg)) => {
            if !err_out.is_null() {
                unsafe {
                    *err_out = leak_cstring(msg);
                }
            }
            ptr::null_mut()
        }
        Err(_panic) => {
            if !err_out.is_null() {
                unsafe {
                    *err_out =
                        leak_cstring("c-bridge: matrix-cpu panicked (this is a bug)".to_string());
                }
            }
            ptr::null_mut()
        }
    }
}

/// Free a string returned by [`matrix_cpu_run_graph`] (either the
/// return value or via the `err_out` parameter).
///
/// # Safety
///
/// - `s` must either be NULL (no-op) or a pointer previously returned
///   by `matrix_cpu_run_graph` and not yet freed.
/// - After this call, `s` is invalidated and must not be used again.
/// - Do NOT pass pointers from other sources (`malloc`, etc.) —
///   Rust's allocator is the only valid owner of these strings.
#[no_mangle]
pub unsafe extern "C" fn matrix_cpu_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    // SAFETY: per the contract above, s was returned by
    // CString::into_raw — converting it back via from_raw is the
    // documented inverse and reclaims ownership.  The resulting
    // CString drops at end of scope, freeing the buffer with the
    // correct (Rust) allocator.
    unsafe {
        let _ = CString::from_raw(s);
    }
}

// ─────────────────────────────────────────────────────────────────────
// Tests — exercise the pure-Rust envelope round-trip and the C ABI
// directly via the rlib half of the crate.
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Smallest possible envelope: 1 input tensor → identity-shaped
    /// graph with no ops → 0 outputs.  Tests the envelope parsing
    /// machinery without depending on any specific matrix-cpu op.
    fn make_empty_envelope() -> String {
        // Single tensor, declared as input AND output (no ops needed).
        // This proves the round-trip through parse → decode → execute
        // → encode actually works end-to-end on the smallest possible
        // graph.
        r#"{
          "graph": {
            "matrix_ir_version": 1,
            "tensors": [{"id": 0, "dtype": "f32", "shape": [2]}],
            "inputs": [0],
            "outputs": [0],
            "ops": [],
            "constants": []
          },
          "inputs": ["0000803f0000004f"]
        }"#
        .to_string()
    }

    #[test]
    fn envelope_round_trip_succeeds_on_identity_graph() {
        let env = make_empty_envelope();
        let out = run_graph_on_cpu_via_json_envelope(&env)
            .expect("identity graph should succeed");
        // Output envelope is just `{"outputs": ["<hex>"]}` — check shape.
        assert!(out.starts_with("{"));
        assert!(out.contains("outputs"));
        // The output hex must be 16 chars (2 f32 cells × 4 bytes × 2 hex chars).
        assert!(out.contains("0000803f"));
    }

    #[test]
    fn malformed_json_envelope_returns_err_not_panic() {
        let bad = "{not valid json";
        let err = run_graph_on_cpu_via_json_envelope(bad)
            .expect_err("malformed JSON should return Err");
        assert!(err.contains("envelope parse failed"));
    }

    #[test]
    fn missing_graph_field_returns_err() {
        let env = r#"{"inputs": []}"#;
        let err = run_graph_on_cpu_via_json_envelope(env)
            .expect_err("missing 'graph' field should be an error");
        assert!(err.contains("graph"));
    }

    #[test]
    fn missing_inputs_field_returns_err() {
        let env = r#"{"graph": {"matrix_ir_version": 1, "tensors": [], "inputs": [], "outputs": [], "ops": [], "constants": []}}"#;
        let err = run_graph_on_cpu_via_json_envelope(env)
            .expect_err("missing 'inputs' field should be an error");
        assert!(err.contains("inputs"));
    }

    #[test]
    fn invalid_hex_in_inputs_returns_err() {
        let env = r#"{
          "graph": {
            "matrix_ir_version": 1,
            "tensors": [{"id": 0, "dtype": "f32", "shape": [2]}],
            "inputs": [0],
            "outputs": [0],
            "ops": [],
            "constants": []
          },
          "inputs": ["zzzz"]
        }"#;
        let err = run_graph_on_cpu_via_json_envelope(env)
            .expect_err("non-hex characters should be rejected");
        assert!(err.contains("hex"));
    }

    #[test]
    fn c_abi_round_trip_succeeds() {
        // Drive the C ABI directly from Rust to prove the FFI shim works.
        let env = make_empty_envelope();
        let env_c = CString::new(env).unwrap();
        let mut err_ptr: *mut c_char = ptr::null_mut();
        // SAFETY: pointers are valid for the duration of the call.
        let out_ptr =
            unsafe { matrix_cpu_run_graph(env_c.as_ptr(), &mut err_ptr as *mut *mut c_char) };
        assert!(!out_ptr.is_null(), "expected success");
        assert!(err_ptr.is_null(), "err_out should be NULL on success");
        // Read the returned string.
        let out = unsafe { CStr::from_ptr(out_ptr) }
            .to_str()
            .unwrap()
            .to_string();
        assert!(out.contains("outputs"));
        // Free it.
        unsafe {
            matrix_cpu_free_string(out_ptr);
        }
    }

    #[test]
    fn c_abi_null_envelope_returns_null_with_err() {
        let mut err_ptr: *mut c_char = ptr::null_mut();
        let out_ptr =
            unsafe { matrix_cpu_run_graph(ptr::null(), &mut err_ptr as *mut *mut c_char) };
        assert!(out_ptr.is_null());
        assert!(!err_ptr.is_null(), "expected error string");
        let err = unsafe { CStr::from_ptr(err_ptr) }
            .to_str()
            .unwrap()
            .to_string();
        assert!(err.contains("NULL"));
        unsafe {
            matrix_cpu_free_string(err_ptr);
        }
    }

    #[test]
    fn c_abi_malformed_envelope_returns_null_with_err() {
        let env_c = CString::new("{not valid json").unwrap();
        let mut err_ptr: *mut c_char = ptr::null_mut();
        let out_ptr =
            unsafe { matrix_cpu_run_graph(env_c.as_ptr(), &mut err_ptr as *mut *mut c_char) };
        assert!(out_ptr.is_null());
        assert!(!err_ptr.is_null());
        unsafe {
            matrix_cpu_free_string(err_ptr);
        }
    }

    #[test]
    fn free_null_string_is_noop() {
        // Must not crash, must not double-free.
        unsafe {
            matrix_cpu_free_string(ptr::null_mut());
        }
    }
}
