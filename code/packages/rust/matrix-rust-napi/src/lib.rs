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

use matrix_ir_json::{decode, encode};
use node_bridge::{
    create_function, get_cb_info, napi_callback_info, napi_env, napi_value,
    set_named_property, str_from_js, str_to_js, throw_error, undefined,
};

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
    let f = create_function(
        env,
        "graphRoundTripJson",
        Some(napi_graph_round_trip_json),
    );
    set_named_property(env, exports, "graphRoundTripJson", f);
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
