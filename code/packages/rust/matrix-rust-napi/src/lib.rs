//! # `matrix-rust-napi` — Node.js N-API addon for the Rust matrix execution layer
//!
//! **Phase 1** of [MX07](../../../specs/MX07-matrix-rust-napi.md).
//! This is the minimal-viable shape of the napi crate: one exported
//! function (`graphRoundTripJson`) that takes the canonical JSON wire
//! format for a `matrix_ir::Graph`, parses it via `matrix-ir-json`,
//! re-encodes it back to JSON, and returns the result.
//!
//! The round-trip is intentionally trivial in v0.1 — it only proves
//! three things:
//!
//! 1. **The build pipeline works.**  We can compile a `cdylib` that
//!    depends on the matrix-ir-json crate (which transitively pulls
//!    the workspace JSON tooling) and have it load as a Node addon.
//! 2. **The N-API boundary works.**  Strings can move into the addon
//!    via `str_from_js`, get processed by Rust code, and come back
//!    out via `str_to_js` without losing data.
//! 3. **The matrix-ir-json schema is the right interop boundary.**
//!    A graph constructed anywhere — Rust, browser TS via a future
//!    `matrix-ir-ts`, hand-written JSON — can be checked for schema
//!    validity by round-tripping through this function.  Anything
//!    that fails to round-trip is a bug.
//!
//! **Phase 2** adds `Graph` and `Runtime` classes and actual
//! execution on `matrix-cpu`.  This file stays small and grows
//! gradually with each PR.
//!
//! ## What Node sees
//!
//! ```javascript
//! const m = require("./matrix_rust_napi.node");
//!
//! const json = JSON.stringify({
//!   matrix_ir_version: 1,
//!   tensors: [
//!     { id: 0, dtype: "f32", shape: [1, 4] },
//!     { id: 1, dtype: "f32", shape: [4, 1] },
//!     { id: 2, dtype: "f32", shape: [1, 1] },
//!   ],
//!   inputs:  [0],
//!   outputs: [2],
//!   ops: [
//!     { kind: "MatMul", lhs: 0, rhs: 1, output: 2 }
//!   ],
//!   constants: [
//!     { tensor_id: 1, dtype: "f32", shape: [4, 1],
//!       bytes_hex: "00000000000000000000000000000000" }
//!   ],
//! });
//!
//! const roundTripped = m.graphRoundTripJson(json);
//! // roundTripped is a JSON string that decodes to the same Graph value.
//! ```
//!
//! ## Internals
//!
//! Two layers:
//!
//! 1. **Pure Rust** (`pub fn round_trip_json`) — the actual work.
//!    Cargo-testable: no Node required.  This is the function that
//!    Phase 2 will compose into `Graph` / `Runtime` exports.
//! 2. **N-API wrapper** (`extern "C" fn napi_graph_round_trip_json`)
//!    — un-marshals the JS string, calls `round_trip_json`, marshals
//!    the result back.  Errors get thrown as JS exceptions via
//!    `node-bridge::throw_error`.

#![allow(non_camel_case_types)]

mod classes;
mod exec;

use coding_adventures_json_value::{JsonNumber, JsonValue};
use matrix_ir_json::{decode, encode};
use node_bridge::{
    create_function, get_cb_info, napi_callback_info, napi_env, napi_value,
    set_named_property, str_from_js, str_to_js, throw_error, undefined,
};

pub use exec::run_graph_on_cpu;

// ─────────────────────────────────────────────────────────────────────────────
// Pure Rust core: the testable round-trip.
//
// Splitting this from the N-API wrapper lets `cargo test` exercise the
// round-trip without any Node toolchain.  It also keeps the wrapper
// trivial — one un-marshal, one call, one marshal — so the unsafe
// surface stays small.
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
/// constructed natively in Rust — that is the property we rely on for
/// Phase 2+ execution.
pub fn round_trip_json(input: &str) -> Result<String, String> {
    let graph = decode(input).map_err(|e| format!("matrix-ir-json decode failed: {:?}", e))?;
    Ok(encode(&graph))
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2: end-to-end execution on the CPU executor.
//
// The Rust helper `exec::run_graph_on_cpu` does the real work; this
// section adds a JSON-envelope wrapper so we can expose it over the
// existing string-in / string-out N-API surface without yet wiring
// Buffer marshalling into node-bridge.
//
// Envelope shape (in):
//   { "graph": <matrix-ir-json schema>,
//     "inputs": [ "<lowercase-hex bytes>", ... ] }
//
// Envelope shape (out):
//   { "outputs": [ "<lowercase-hex bytes>", ... ] }
//
// Per-tensor byte strings use the same hex encoding the matrix-ir-json
// crate uses for constants — lowercase, no separator, no 0x prefix,
// length always `2 * num_bytes`.  Phase 2b will replace this with
// real Buffer marshalling once node-bridge grows Buffer helpers; for
// Phase 2 the JSON envelope keeps the napi surface tiny (one
// function, identical pattern to graphRoundTripJson) while still
// proving the full plan + execute + return path works.
// ─────────────────────────────────────────────────────────────────────────────

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

/// Pure-Rust entry point for the JSON-envelope-shaped `runGraphOnCpu`.
/// Parses the envelope, hex-decodes inputs, decodes the graph,
/// executes via [`run_graph_on_cpu`], hex-encodes outputs, returns
/// the result envelope JSON.
pub fn run_graph_on_cpu_via_json_envelope(envelope_json: &str) -> Result<String, String> {
    // ── parse envelope ──────────────────────────────────────────
    let env =
        coding_adventures_json_value::parse(envelope_json).map_err(|e| format!("envelope parse failed: {:?}", e))?;

    // ── extract `graph` field, re-serialise it, pass to matrix-ir-json::decode ──
    //
    // We re-serialise rather than pass the JsonValue directly because
    // matrix-ir-json's public API is string -> Graph, not JsonValue ->
    // Graph.  One extra serialise per call; cost is negligible at
    // typical graph sizes and the API is cleaner.
    let graph_value = envelope_field(&env, "graph")?;
    let graph_str = coding_adventures_json_serializer::serialize(graph_value)
        .map_err(|e| format!("graph re-serialise failed: {:?}", e))?;
    let graph = decode(&graph_str).map_err(|e| format!("matrix-ir-json decode failed: {:?}", e))?;

    // ── extract `inputs` field — array of hex strings ──────────
    let inputs_value = envelope_field(&env, "inputs")?;
    let inputs_array = match inputs_value {
        JsonValue::Array(xs) => xs,
        _ => return Err("envelope.inputs must be a JSON array".to_string()),
    };
    let mut inputs: Vec<Vec<u8>> = Vec::with_capacity(inputs_array.len());
    for (i, item) in inputs_array.iter().enumerate() {
        match item {
            JsonValue::String(s) => inputs.push(
                hex_decode(s).map_err(|e| format!("envelope.inputs[{}]: {}", i, e))?,
            ),
            _ => return Err(format!("envelope.inputs[{}] must be a hex string", i)),
        }
    }

    // ── execute ─────────────────────────────────────────────────
    let outputs = run_graph_on_cpu(&graph, &inputs)?;

    // ── build result envelope ───────────────────────────────────
    let outputs_array = JsonValue::Array(outputs.iter().map(|b| JsonValue::String(hex_encode(b))).collect());
    let envelope_out = JsonValue::Object(vec![("outputs".to_string(), outputs_array)]);
    coding_adventures_json_serializer::serialize(&envelope_out)
        .map_err(|e| format!("output envelope serialise failed: {:?}", e))
}

/// `runGraphOnCpu(envelopeJson: string): string` — see
/// [`run_graph_on_cpu_via_json_envelope`] for the envelope shape.
///
/// # Safety
///
/// Invoked by Node; `env` and `info` valid for the duration of the call.
unsafe extern "C" fn napi_run_graph_on_cpu(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() != 1 {
        throw_error(
            env,
            &format!(
                "runGraphOnCpu: expected exactly 1 argument (envelopeJson), got {}",
                args.len()
            ),
        );
        return undefined(env);
    }
    let envelope = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "runGraphOnCpu: argument 0 must be a string");
            return undefined(env);
        }
    };
    match run_graph_on_cpu_via_json_envelope(&envelope) {
        Ok(out) => str_to_js(env, &out),
        Err(msg) => {
            throw_error(env, &format!("runGraphOnCpu: {}", msg));
            undefined(env)
        }
    }
}

// Silence the unused-import warning when JsonNumber isn't used in tests.
// (Reserved for future envelope fields like `version` that we may want
// to validate against an integer.)
#[allow(dead_code)]
fn _suppress_unused_jsonnumber() {
    let _ = JsonNumber::Integer(0);
}

// ─────────────────────────────────────────────────────────────────────────────
// N-API wrapper
//
// Every napi callback has the same signature:
//   fn(env, info) -> napi_value
//
// We extract args via `get_cb_info`, do work, and return a napi_value.
// On error we throw a JS exception (via `throw_error`) and return
// `undefined` — Node treats the throw as the actual return.
// ─────────────────────────────────────────────────────────────────────────────

/// `graphRoundTripJson(jsonString: string): string`
///
/// JS entry point.  Argument arity is 1; any other arity throws.
///
/// # Safety
///
/// This function is invoked by Node via the registered N-API binding.
/// `env` and `info` are valid for the duration of the call per N-API
/// contract.
unsafe extern "C" fn napi_graph_round_trip_json(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    // We request 2 arg slots so the wrapper can detect strict
    // over-arity ("you passed too many"); requesting exactly 1
    // would silently accept extras because `get_cb_info` truncates
    // `argv` to `min(actual_argc, max_args)`.
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() != 1 {
        throw_error(
            env,
            &format!(
                "graphRoundTripJson: expected exactly 1 argument (jsonString), got {}",
                args.len()
            ),
        );
        return undefined(env);
    }

    let json = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "graphRoundTripJson: argument 0 must be a string");
            return undefined(env);
        }
    };

    match round_trip_json(&json) {
        Ok(out) => str_to_js(env, &out),
        Err(msg) => {
            throw_error(env, &format!("graphRoundTripJson: {}", msg));
            undefined(env)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Module registration — the entry point Node.js calls on require()
//
// `napi_register_module_v1` is the standard symbol Node looks up when
// loading a `.node` file.  We attach our exported function(s) to the
// `exports` object and return it.
// ─────────────────────────────────────────────────────────────────────────────

/// Module entry point — called by Node.js when the addon is `require`d.
///
/// # Safety
///
/// Invoked exactly once per Node load, with `env` and `exports` valid
/// for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    // Phase 1 — JSON-string round-trip (validation utility).
    let f1 = create_function(env, "graphRoundTripJson", Some(napi_graph_round_trip_json));
    set_named_property(env, exports, "graphRoundTripJson", f1);

    // Phase 2 — JSON-envelope one-shot execution (kept as the
    // CLI-friendly / Node-Buffer-free alternative path).
    let f2 = create_function(env, "runGraphOnCpu", Some(napi_run_graph_on_cpu));
    set_named_property(env, exports, "runGraphOnCpu", f2);

    // Phase 2b — class-based API (Graph + Runtime, Buffer[] I/O).
    classes::register(env, exports);

    exports
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
//
// The unit tests cover the pure-Rust `round_trip_json` helper without
// involving Node.  They prove that:
//
//  * a valid Graph JSON survives the round-trip;
//  * the output decodes to a Graph that's byte-equal under the binary
//    wire format (the cross-format equivalence we rely on for Phase
//    2+);
//  * malformed JSON returns an Err, not a panic.
//
// The N-API wrapper is exercised by Phase 3+ end-to-end tests
// (`node --test`).  Here we keep things pure-Rust.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_ir::{DType, GraphBuilder, Shape};

    /// Build a small ReLU layer and confirm the JSON round-trip is
    /// semantically identical (binary wire format byte-equal).
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

        // Decode the output JSON and compare bytes with the original
        // graph's binary wire format.  Byte equality is the canonical
        // "same graph value" test (see matrix-ir-json README).
        let round_tripped = decode(&json_out).expect("decode round-trip output");
        assert_eq!(
            original.to_bytes(),
            round_tripped.to_bytes(),
            "round-tripped graph must be byte-equal to the original under \
             the binary wire format"
        );
    }

    /// A graph with every Op family is non-trivial enough to be a
    /// stress test of `matrix-ir-json` end-to-end through our wrapper.
    /// We don't replicate the full all-ops test from `matrix-ir-json`
    /// here (that's its job), but we do confirm a representative
    /// 3-op graph survives.
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
    /// addon, not produce a corrupted graph value.  This is the
    /// fail-closed property we need at the FFI boundary.
    #[test]
    fn round_trip_rejects_garbage_json() {
        let err = round_trip_json("not even close to json").expect_err("garbage rejected");
        assert!(err.contains("decode failed"), "got: {}", err);
    }

    /// JSON that's syntactically valid but schema-invalid (wrong
    /// version) must also fail cleanly.
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
    /// re-round-trips identically.  Idempotence: `f(f(x)) == f(x)`.
    /// (Useful to catch any drift introduced by the encoder — e.g.
    /// if it ever started canonicalising fields differently between
    /// runs.)
    // ── JSON envelope (Phase 2) ──────────────────────────────────

    /// Build an Add graph, package it + inputs into the envelope JSON,
    /// run through the napi-shaped string entry point, parse the
    /// result envelope, hex-decode outputs, assert.
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
            // 3.0_f32 .to_le_bytes() = [00, 00, 40, 40] etc.
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

        let out_envelope =
            run_graph_on_cpu_via_json_envelope(&envelope).expect("envelope execution succeeds");

        // Parse result envelope and read first output bytes.
        let parsed =
            coding_adventures_json_value::parse(&out_envelope).expect("output envelope parses");
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

    /// Envelope without a `graph` field must fail cleanly.
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
        let envelope = format!(r#"{{ "graph": {}, "inputs": "not-an-array" }}"#, graph_json);

        let err = run_graph_on_cpu_via_json_envelope(&envelope)
            .expect_err("inputs-not-array rejected");
        assert!(err.contains("inputs"), "got: {}", err);
    }

    /// Hex encoder round-trips through the decoder.
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
}
