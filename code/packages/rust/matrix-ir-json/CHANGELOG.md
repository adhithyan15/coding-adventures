# Changelog

All notable changes to `matrix-ir-json` are documented here.  The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [unreleased] — MX12 f64 dtype

### Added — `f64` dtype mnemonic (MX12)

The JSON encoder/decoder maps `DType::F64` ↔ `"f64"`.

## [0.1.0] — 2026-05-18

### Added — initial release: JSON wire format for `matrix-ir`

First cut of **ARCH02 Phase 1**: a sibling crate to `matrix-ir` that
adds a human-readable JSON serialization of `Graph` values, alongside
the canonical binary wire format already shipping in `matrix-ir`.

Public API:

```rust
pub fn encode(g: &Graph) -> String;
pub fn encode_pretty(g: &Graph) -> String;
pub fn decode(s: &str) -> Result<Graph, Error>;
pub enum Error {
    JsonSyntax(String),
    SchemaMismatch { path: String, message: String },
    UnsupportedVersion { found: u32, expected: u32 },
    UnknownDType(String),
    UnknownOpKind(String),
    BadHex(String),
    BadInteger(String),
    InputTensorMissing(u32),
}
```

### Why a sibling crate?

`matrix-ir` is bound by the **MX00 zero-dependency mandate** (CI-enforced):
it may not depend on anything outside the Rust standard library.  That
mandate is what makes the IR layer trivially auditable and embeddable.

Putting a JSON encoder inside `matrix-ir` would force a choice between
(a) hand-rolling another ~1000 lines of JSON code right next to the IR,
or (b) pulling in `serde_json` and breaking MX00.  Both are bad.

A sibling crate cleanly resolves the tension:

* `matrix-ir` stays zero-dep and CI-clean.
* `matrix-ir-json` depends on `matrix-ir` plus the workspace's
  existing JSON crates (`coding-adventures-json-value`,
  `coding-adventures-json-serializer`), reusing infrastructure
  that's already written and tested.
* Anyone who needs binary-only wire (embedded backends, FFI hot paths)
  can depend on `matrix-ir` alone and pay nothing for JSON.

### Schema

Top-level shape:

```jsonc
{
  "matrix_ir_version": 1,
  "tensors":   [ { "id": 0, "dtype": "f32", "shape": [1, 4] }, … ],
  "inputs":    [ 0, … ],
  "outputs":   [ 2, … ],
  "ops":       [ { "kind": "Add", "lhs": 0, "rhs": 1, "output": 2 }, … ],
  "constants": [ { "tensor_id": 1, "dtype": "f32", "shape": [4],
                   "bytes_hex": "0000000000000000…" }, … ]
}
```

* `matrix_ir_version` is required and must equal `WIRE_FORMAT_VERSION`
  (currently `1`).  Decoders return `Error::UnsupportedVersion`
  otherwise — the same fail-closed posture as the binary wire format.
* `dtype` values: `"f32" | "f64" | "i32" | "i64" | "u8" | "u32"`.
  Unknown strings → `Error::UnknownDType`.
* `kind` values: the literal `Op` variant name (`"Add"`, `"MatMul"`,
  `"ReduceSum"`, `"Concat"`, `"Where"`, …).  Unknown kinds →
  `Error::UnknownOpKind`.
* Constant bytes use **lowercase hex** (`[0-9a-f]+`) with no separator
  and no `0x` prefix.  Length is always exactly `2 * num_bytes`
  characters — leading zeros are preserved.  Odd length or non-hex
  characters → `Error::BadHex`.

### Round-trip guarantee

The test suite covers all 29 `Op` variants and asserts:

```
graph -> JSON -> graph' -> binary -> graph''
=> graph.to_bytes() == graph''.to_bytes()
```

In other words, JSON and binary are interchangeable representations of
the same `Graph` value.  Test `binary_and_json_round_trip_through_each_other`
flips through both encodings and verifies byte-exact equality at the end.

### Encoding choices

* **Hex over base64** for constant bytes.  Trivial to hand-decode
  (one nibble per char), no padding ambiguity, debugger-friendly when
  reading raw IR dumps, and unambiguous in shell pipelines that may
  mangle `+`/`/`.
* **Object field order matches schema order** (`matrix_ir_version`
  first, then `tensors`, `inputs`, `outputs`, `ops`, `constants`).
  Useful for diffing committed JSON fixtures.
* **`encode_pretty` uses 2-space indent**, matching the rest of the
  repo's JSON conventions and aligning with how `json-serializer`
  pretty-prints by default.

### Tests (16)

* `encode_decode_round_trips_relu_layer` — small ReLU layer
  (`MatMul + Add + Max(0)`) round-trips.
* `encode_starts_with_version_field` — first key is
  `"matrix_ir_version"` for cheap version sniffing.
* `encode_pretty_is_multi_line` — pretty form contains newlines.
* `decode_rejects_unsupported_version` — `{matrix_ir_version: 999}`
  fails fast.
* `decode_rejects_unknown_op_kind` — unknown `"kind"` string fails.
* `decode_rejects_unknown_dtype` — unknown `"dtype"` string fails.
* `decode_rejects_odd_hex_length` / `decode_rejects_invalid_hex_char` —
  malformed constant bytes fail.
* `decode_rejects_negative_id` / `decode_rejects_fractional_id` —
  tensor ids must be non-negative integers.
* `decode_rejects_missing_required_field` — missing schema fields fail.
* `decode_rejects_input_tensor_id_out_of_range` — references to
  undeclared tensor ids fail.
* `decode_rejects_garbage_json` — non-JSON input fails cleanly.
* `hex_round_trips_through_bytes` — `bytes -> hex -> bytes` is identity.
* `encoded_json_round_trips_for_every_op_family` — every op variant
  shape (unary, binary, reduction, shape ops, linear algebra,
  comparison, selection, conversion, constant) round-trips.
* `binary_and_json_round_trip_through_each_other` — the headline
  guarantee.
