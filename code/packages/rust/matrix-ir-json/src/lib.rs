//! # `matrix-ir-json` — JSON wire format for matrix-ir Graph values
//!
//! **ARCH02 Phase 1.**  Provides `Graph::to_json` / `Graph::from_json`
//! as a sibling to the binary wire format that already lives inside
//! the [`matrix_ir`] crate.
//!
//! ## Why a separate crate
//!
//! `matrix-ir` follows the spec MX00 zero-dependency mandate — its
//! `Cargo.toml` `[dependencies]` section is empty, and a CI check
//! fails the build if that ever changes.  The hand-rolled binary
//! [`matrix_ir::Graph::to_bytes`] / [`matrix_ir::Graph::from_bytes`]
//! lives there because it can be expressed in pure core / alloc / std.
//!
//! A JSON encoder/decoder cannot reasonably be hand-rolled inside
//! `matrix-ir` without reinventing the workspace's existing
//! `coding-adventures-json-*` crates.  Rather than duplicate that
//! work (or relax MX00), this sibling crate `matrix-ir-json` depends
//! on `matrix-ir` plus the workspace JSON crates and exposes the
//! same shape — two functions, [`encode`] and [`decode`], plus an
//! [`Error`] enum.
//!
//! Consumers that only need binary use [`matrix_ir`] directly.
//! Consumers that need JSON (browser DevTools inspection,
//! cross-language ports, test fixtures, schema documentation) pull
//! in this crate.
//!
//! ## Isomorphism with the binary format
//!
//! Every value representable in the binary format is representable
//! in JSON, and round-trip through either one is lossless.  The two
//! share [`matrix_ir::WIRE_FORMAT_VERSION`] — the binary format
//! writes it as a u32 prefix; the JSON format writes it as the
//! `"matrix_ir_version"` field of the top-level object.
//!
//! The test `binary_and_json_round_trip_through_each_other` in this
//! crate goes `Graph -> bytes -> Graph -> json -> Graph` and asserts
//! equality at the start and end, pinning down the isomorphism.
//!
//! ## Schema
//!
//! ```json
//! {
//!   "matrix_ir_version": 1,
//!   "tensors": [
//!     {"id": 0, "dtype": "f32", "shape": [1, 4]},
//!     {"id": 1, "dtype": "f32", "shape": [4, 2]}
//!   ],
//!   "inputs": [0, 1],
//!   "outputs": [3],
//!   "ops": [
//!     {"kind": "MatMul", "a": 0, "b": 1, "output": 3}
//!   ],
//!   "constants": [
//!     {"tensor_id": 4, "dtype": "f32", "shape": [1, 2], "bytes_hex": "00000000"}
//!   ]
//! }
//! ```
//!
//! - `matrix_ir_version` — positive integer; readers reject any
//!   version they don't know.
//! - `tensors` — array of `{id, dtype, shape}` objects.  `dtype` is
//!   the lowercase mnemonic (`"f32"`, `"u8"`, `"i32"`); `shape` is an
//!   array of non-negative integers.
//! - `inputs` and `outputs` — arrays of tensor ids (u32).
//! - `ops` — array of op objects.  Every op has `"kind"` (string)
//!   and `"output"` (tensor id); other fields vary per variant.
//!   See `encode_op` / `decode_op` for the per-variant layout
//!   — it mirrors [`matrix_ir::Op`] 1:1.
//! - `constants` — literal-data tensors.  `bytes_hex` is a
//!   lowercase hex string with `2 * byte_count` characters.
//!
//! ## Encoding choices
//!
//! - Hex for bytes, not base64.  Hex has no padding ambiguity, every
//!   debug tool speaks it natively (`xxd`, Wireshark, GDB), and it
//!   round-trips trivially.  The 2× size cost vs base64 is irrelevant
//!   for the cases JSON serves; the binary format is the right
//!   choice when size matters.
//! - Compact output by default (no whitespace).  [`encode_pretty`]
//!   is available for human inspection.
//! - Lossless across the full op surface — every variant of
//!   [`matrix_ir::Op`] has an explicit encoder and decoder.

#![warn(rust_2018_idioms)]

use coding_adventures_json_serializer::{serialize, serialize_pretty, SerializerConfig};
use coding_adventures_json_value::{JsonNumber, JsonValue};
use matrix_ir::{Constant, DType, Graph, Op, Shape, Tensor, TensorId, WIRE_FORMAT_VERSION};

// ════════════════════════════════════════════════════════════════════
//                              ERRORS
// ════════════════════════════════════════════════════════════════════

/// Errors produced by the JSON encoder/decoder.
///
/// Encoder errors are essentially impossible (every `Graph` produced
/// by `matrix-ir` is encodable), so `encode` returns `String` directly;
/// `Error` is only used by [`decode`].
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// JSON text was malformed at the syntax level (parser-side
    /// error).  Wraps the underlying [`coding_adventures_json_value`]
    /// error message.
    JsonSyntax { message: String },
    /// JSON parsed successfully but doesn't match the matrix-ir
    /// schema — a required key is missing, a value has the wrong
    /// type, etc.  `path` is a slash-separated breadcrumb to the
    /// offending field (e.g. `"tensors[3]/dtype"`).
    SchemaMismatch { path: String, reason: String },
    /// The `"matrix_ir_version"` field was not the version this
    /// crate writes.  Carries the version we saw.
    UnsupportedVersion { saw: u64 },
    /// A dtype mnemonic in the JSON was not one of `"f32"`, `"u8"`,
    /// or `"i32"`.
    UnknownDType { saw: String, path: String },
    /// An op `"kind"` value was not one of the 29 known op names.
    UnknownOpKind { saw: String, path: String },
    /// A `bytes_hex` string had an odd character count or contained
    /// a non-hex character.
    BadHex { reason: &'static str, path: String },
    /// An integer value in the JSON was negative, fractional, or
    /// larger than `u32::MAX` when a u32 was expected.
    BadInteger { reason: &'static str, path: String },
    /// The `inputs` array references a tensor id that doesn't appear
    /// in the `tensors` array.
    InputTensorMissing { id: u32 },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::JsonSyntax { message } => {
                write!(f, "JSON syntax error: {}", message)
            }
            Error::SchemaMismatch { path, reason } => {
                write!(f, "schema mismatch at {}: {}", path, reason)
            }
            Error::UnsupportedVersion { saw } => {
                write!(
                    f,
                    "unsupported matrix_ir_version: saw {}, this crate writes {}",
                    saw, WIRE_FORMAT_VERSION
                )
            }
            Error::UnknownDType { saw, path } => {
                write!(f, "unknown dtype {:?} at {}", saw, path)
            }
            Error::UnknownOpKind { saw, path } => {
                write!(f, "unknown op kind {:?} at {}", saw, path)
            }
            Error::BadHex { reason, path } => {
                write!(f, "bad bytes_hex at {}: {}", path, reason)
            }
            Error::BadInteger { reason, path } => {
                write!(f, "bad integer at {}: {}", path, reason)
            }
            Error::InputTensorMissing { id } => {
                write!(
                    f,
                    "input references tensor id {} which is not in the tensors array",
                    id
                )
            }
        }
    }
}

impl std::error::Error for Error {}

// ════════════════════════════════════════════════════════════════════
//                              PUBLIC API
// ════════════════════════════════════════════════════════════════════

/// Encode a [`Graph`] as a compact JSON string (no whitespace).
///
/// The output starts with `{"matrix_ir_version":1,"tensors":[...` and
/// can be fed directly into [`decode`] to recover the same `Graph`
/// value.  See [`encode_pretty`] for an indented variant useful when
/// inspecting graphs by hand.
pub fn encode(g: &Graph) -> String {
    let v = graph_to_json_value(g);
    // The serializer can only fail on non-finite floats — we don't
    // emit any floats, so this `unwrap` is sound.
    serialize(&v).expect("matrix-ir-json encoder never produces non-finite floats")
}

/// Encode a [`Graph`] as a pretty-printed JSON string (2-space indent).
pub fn encode_pretty(g: &Graph) -> String {
    let v = graph_to_json_value(g);
    let config = SerializerConfig {
        indent_size: 2,
        indent_char: ' ',
        sort_keys: false,
        trailing_newline: false,
    };
    serialize_pretty(&v, &config).expect("matrix-ir-json encoder never produces non-finite floats")
}

/// Decode a [`Graph`] from a JSON string.  Performs structural
/// decoding only — does NOT call [`Graph::validate`].
///
/// Callers that want semantic validation must run `validate()`
/// themselves, mirroring [`Graph::from_bytes`].
pub fn decode(s: &str) -> Result<Graph, Error> {
    let value = coding_adventures_json_value::parse(s)
        .map_err(|e| Error::JsonSyntax { message: e.message })?;
    let obj = expect_object(&value, "/")?;

    // matrix_ir_version
    let version = expect_u64(
        get_field(obj, "matrix_ir_version", "/")?,
        "/matrix_ir_version",
    )?;
    if version != WIRE_FORMAT_VERSION as u64 {
        return Err(Error::UnsupportedVersion { saw: version });
    }

    // tensors
    let tensors_value = get_field(obj, "tensors", "/")?;
    let tensors = decode_tensor_array(tensors_value, "/tensors")?;

    // inputs
    let inputs_value = get_field(obj, "inputs", "/")?;
    let input_ids = decode_u32_array(inputs_value, "/inputs")?;

    // outputs
    let outputs_value = get_field(obj, "outputs", "/")?;
    let output_ids = decode_u32_array(outputs_value, "/outputs")?;

    // ops
    let ops_value = get_field(obj, "ops", "/")?;
    let ops = decode_op_array(ops_value, "/ops")?;

    // constants
    let constants_value = get_field(obj, "constants", "/")?;
    let constants = decode_constant_array(constants_value, "/constants")?;

    // Resolve input ids to full Tensor records.
    let mut inputs = Vec::with_capacity(input_ids.len());
    for id in input_ids {
        if (id as usize) >= tensors.len() {
            return Err(Error::InputTensorMissing { id });
        }
        inputs.push(tensors[id as usize].clone());
    }
    let outputs = output_ids.into_iter().map(TensorId).collect();

    Ok(Graph {
        inputs,
        outputs,
        ops,
        tensors,
        constants,
    })
}

// ════════════════════════════════════════════════════════════════════
//                              ENCODER
// ════════════════════════════════════════════════════════════════════

fn graph_to_json_value(g: &Graph) -> JsonValue {
    JsonValue::Object(vec![
        ("matrix_ir_version".to_owned(), u32_v(WIRE_FORMAT_VERSION)),
        ("tensors".to_owned(), tensors_to_json(&g.tensors)),
        (
            "inputs".to_owned(),
            tensor_id_array_to_json(g.inputs.iter().map(|t| t.id)),
        ),
        (
            "outputs".to_owned(),
            tensor_id_array_to_json(g.outputs.iter().copied()),
        ),
        ("ops".to_owned(), ops_to_json(&g.ops)),
        ("constants".to_owned(), constants_to_json(&g.constants)),
    ])
}

fn u32_v(n: u32) -> JsonValue {
    JsonValue::Number(JsonNumber::Integer(n as i64))
}

fn dtype_to_json(dt: DType) -> JsonValue {
    JsonValue::String(
        match dt {
            DType::F32 => "f32",
            DType::F64 => "f64",
            DType::U8 => "u8",
            DType::I32 => "i32",
        }
        .to_owned(),
    )
}

fn shape_to_json(s: &Shape) -> JsonValue {
    JsonValue::Array(s.dims.iter().map(|&d| u32_v(d)).collect())
}

fn tensor_to_json(t: &Tensor) -> JsonValue {
    JsonValue::Object(vec![
        ("id".to_owned(), u32_v(t.id.0)),
        ("dtype".to_owned(), dtype_to_json(t.dtype)),
        ("shape".to_owned(), shape_to_json(&t.shape)),
    ])
}

fn tensors_to_json(ts: &[Tensor]) -> JsonValue {
    JsonValue::Array(ts.iter().map(tensor_to_json).collect())
}

fn tensor_id_array_to_json(ids: impl Iterator<Item = TensorId>) -> JsonValue {
    JsonValue::Array(ids.map(|i| u32_v(i.0)).collect())
}

fn u32_array_to_json(xs: &[u32]) -> JsonValue {
    JsonValue::Array(xs.iter().map(|&x| u32_v(x)).collect())
}

/// Encode a byte slice as a lowercase hex JSON string.
fn bytes_to_hex_json(bytes: &[u8]) -> JsonValue {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0F) as usize] as char);
    }
    JsonValue::String(s)
}

fn constant_to_json(c: &Constant) -> JsonValue {
    JsonValue::Object(vec![
        ("tensor_id".to_owned(), u32_v(c.tensor.id.0)),
        ("dtype".to_owned(), dtype_to_json(c.tensor.dtype)),
        ("shape".to_owned(), shape_to_json(&c.tensor.shape)),
        ("bytes_hex".to_owned(), bytes_to_hex_json(&c.bytes)),
    ])
}

fn constants_to_json(cs: &[Constant]) -> JsonValue {
    JsonValue::Array(cs.iter().map(constant_to_json).collect())
}

fn ops_to_json(ops: &[Op]) -> JsonValue {
    JsonValue::Array(ops.iter().map(op_to_json).collect())
}

fn op_to_json(op: &Op) -> JsonValue {
    // Build (kind, [(field, value), ...]) pairs per variant.
    // Each variant follows the same shape as the binary encoder.
    let (kind, mut fields): (&'static str, Vec<(String, JsonValue)>) = match op {
        // unary
        Op::Neg { input, output } => (
            "Neg",
            vec![
                ("input".into(), u32_v(input.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Abs { input, output } => (
            "Abs",
            vec![
                ("input".into(), u32_v(input.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Sqrt { input, output } => (
            "Sqrt",
            vec![
                ("input".into(), u32_v(input.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Exp { input, output } => (
            "Exp",
            vec![
                ("input".into(), u32_v(input.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Log { input, output } => (
            "Log",
            vec![
                ("input".into(), u32_v(input.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Tanh { input, output } => (
            "Tanh",
            vec![
                ("input".into(), u32_v(input.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Recip { input, output } => (
            "Recip",
            vec![
                ("input".into(), u32_v(input.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        // binary
        Op::Add { lhs, rhs, output } => (
            "Add",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Sub { lhs, rhs, output } => (
            "Sub",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Mul { lhs, rhs, output } => (
            "Mul",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Div { lhs, rhs, output } => (
            "Div",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Max { lhs, rhs, output } => (
            "Max",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Min { lhs, rhs, output } => (
            "Min",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Pow { lhs, rhs, output } => (
            "Pow",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        // reductions
        Op::ReduceSum {
            input,
            axes,
            keep_dims,
            output,
        } => (
            "ReduceSum",
            vec![
                ("input".into(), u32_v(input.0)),
                ("axes".into(), u32_array_to_json(axes)),
                ("keep_dims".into(), JsonValue::Bool(*keep_dims)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::ReduceMax {
            input,
            axes,
            keep_dims,
            output,
        } => (
            "ReduceMax",
            vec![
                ("input".into(), u32_v(input.0)),
                ("axes".into(), u32_array_to_json(axes)),
                ("keep_dims".into(), JsonValue::Bool(*keep_dims)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::ReduceMean {
            input,
            axes,
            keep_dims,
            output,
        } => (
            "ReduceMean",
            vec![
                ("input".into(), u32_v(input.0)),
                ("axes".into(), u32_array_to_json(axes)),
                ("keep_dims".into(), JsonValue::Bool(*keep_dims)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        // shape
        Op::Reshape {
            input,
            new_shape,
            output,
        } => (
            "Reshape",
            vec![
                ("input".into(), u32_v(input.0)),
                ("new_shape".into(), shape_to_json(new_shape)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Transpose {
            input,
            perm,
            output,
        } => (
            "Transpose",
            vec![
                ("input".into(), u32_v(input.0)),
                ("perm".into(), u32_array_to_json(perm)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Broadcast {
            input,
            target_shape,
            output,
        } => (
            "Broadcast",
            vec![
                ("input".into(), u32_v(input.0)),
                ("target_shape".into(), shape_to_json(target_shape)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Slice {
            input,
            axis,
            start,
            end,
            step,
            output,
        } => (
            "Slice",
            vec![
                ("input".into(), u32_v(input.0)),
                ("axis".into(), u32_v(*axis)),
                ("start".into(), u32_v(*start)),
                ("end".into(), u32_v(*end)),
                ("step".into(), u32_v(*step)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Concat {
            inputs,
            axis,
            output,
        } => (
            "Concat",
            vec![
                (
                    "inputs".into(),
                    tensor_id_array_to_json(inputs.iter().copied()),
                ),
                ("axis".into(), u32_v(*axis)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        // linear algebra
        Op::MatMul { a, b, output } => (
            "MatMul",
            vec![
                ("a".into(), u32_v(a.0)),
                ("b".into(), u32_v(b.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        // comparison
        Op::Equal { lhs, rhs, output } => (
            "Equal",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Less { lhs, rhs, output } => (
            "Less",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        Op::Greater { lhs, rhs, output } => (
            "Greater",
            vec![
                ("lhs".into(), u32_v(lhs.0)),
                ("rhs".into(), u32_v(rhs.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        // selection
        Op::Where {
            predicate,
            true_value,
            false_value,
            output,
        } => (
            "Where",
            vec![
                ("predicate".into(), u32_v(predicate.0)),
                ("true_value".into(), u32_v(true_value.0)),
                ("false_value".into(), u32_v(false_value.0)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        // conversion
        Op::Cast {
            input,
            dtype,
            output,
        } => (
            "Cast",
            vec![
                ("input".into(), u32_v(input.0)),
                ("dtype".into(), dtype_to_json(*dtype)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
        // constants
        Op::Const { constant, output } => (
            "Const",
            vec![
                ("constant".into(), u32_v(*constant)),
                ("output".into(), u32_v(output.0)),
            ],
        ),
    };
    // Prepend kind so the object always starts with "kind".  This is
    // a convention for human-readable output, not a wire requirement.
    let mut pairs = Vec::with_capacity(fields.len() + 1);
    pairs.push(("kind".to_owned(), JsonValue::String(kind.to_owned())));
    pairs.append(&mut fields);
    JsonValue::Object(pairs)
}

// ════════════════════════════════════════════════════════════════════
//                              DECODER
// ════════════════════════════════════════════════════════════════════

fn expect_object<'a>(v: &'a JsonValue, path: &str) -> Result<&'a [(String, JsonValue)], Error> {
    match v {
        JsonValue::Object(pairs) => Ok(pairs),
        _ => Err(Error::SchemaMismatch {
            path: path.to_owned(),
            reason: format!("expected object, found {}", value_kind(v)),
        }),
    }
}

fn expect_array<'a>(v: &'a JsonValue, path: &str) -> Result<&'a [JsonValue], Error> {
    match v {
        JsonValue::Array(items) => Ok(items),
        _ => Err(Error::SchemaMismatch {
            path: path.to_owned(),
            reason: format!("expected array, found {}", value_kind(v)),
        }),
    }
}

fn expect_string<'a>(v: &'a JsonValue, path: &str) -> Result<&'a str, Error> {
    match v {
        JsonValue::String(s) => Ok(s.as_str()),
        _ => Err(Error::SchemaMismatch {
            path: path.to_owned(),
            reason: format!("expected string, found {}", value_kind(v)),
        }),
    }
}

fn expect_bool(v: &JsonValue, path: &str) -> Result<bool, Error> {
    match v {
        JsonValue::Bool(b) => Ok(*b),
        _ => Err(Error::SchemaMismatch {
            path: path.to_owned(),
            reason: format!("expected bool, found {}", value_kind(v)),
        }),
    }
}

fn expect_u64(v: &JsonValue, path: &str) -> Result<u64, Error> {
    match v {
        JsonValue::Number(JsonNumber::Integer(i)) => {
            if *i < 0 {
                Err(Error::BadInteger {
                    reason: "negative value where unsigned was expected",
                    path: path.to_owned(),
                })
            } else {
                Ok(*i as u64)
            }
        }
        JsonValue::Number(JsonNumber::Float(_)) => Err(Error::BadInteger {
            reason: "fractional value where integer was expected",
            path: path.to_owned(),
        }),
        _ => Err(Error::SchemaMismatch {
            path: path.to_owned(),
            reason: format!("expected integer, found {}", value_kind(v)),
        }),
    }
}

fn expect_u32(v: &JsonValue, path: &str) -> Result<u32, Error> {
    let u = expect_u64(v, path)?;
    if u > u32::MAX as u64 {
        Err(Error::BadInteger {
            reason: "value exceeds u32::MAX",
            path: path.to_owned(),
        })
    } else {
        Ok(u as u32)
    }
}

fn get_field<'a>(
    obj: &'a [(String, JsonValue)],
    key: &str,
    object_path: &str,
) -> Result<&'a JsonValue, Error> {
    obj.iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
        .ok_or_else(|| Error::SchemaMismatch {
            path: object_path.to_owned(),
            reason: format!("missing required field {:?}", key),
        })
}

fn get_field_opt<'a>(obj: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn value_kind(v: &JsonValue) -> &'static str {
    match v {
        JsonValue::Object(_) => "object",
        JsonValue::Array(_) => "array",
        JsonValue::String(_) => "string",
        JsonValue::Number(_) => "number",
        JsonValue::Bool(_) => "bool",
        JsonValue::Null => "null",
    }
}

fn decode_dtype(v: &JsonValue, path: &str) -> Result<DType, Error> {
    let s = expect_string(v, path)?;
    match s {
        "f32" => Ok(DType::F32),
        "f64" => Ok(DType::F64),
        "u8" => Ok(DType::U8),
        "i32" => Ok(DType::I32),
        other => Err(Error::UnknownDType {
            saw: other.to_owned(),
            path: path.to_owned(),
        }),
    }
}

fn decode_shape(v: &JsonValue, path: &str) -> Result<Shape, Error> {
    let items = expect_array(v, path)?;
    let mut dims = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let item_path = format!("{}[{}]", path, i);
        dims.push(expect_u32(item, &item_path)?);
    }
    Ok(Shape { dims })
}

fn decode_u32_array(v: &JsonValue, path: &str) -> Result<Vec<u32>, Error> {
    let items = expect_array(v, path)?;
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let item_path = format!("{}[{}]", path, i);
        out.push(expect_u32(item, &item_path)?);
    }
    Ok(out)
}

fn decode_tensor(v: &JsonValue, path: &str) -> Result<Tensor, Error> {
    let obj = expect_object(v, path)?;
    let id = expect_u32(get_field(obj, "id", path)?, &format!("{}/id", path))?;
    let dtype = decode_dtype(get_field(obj, "dtype", path)?, &format!("{}/dtype", path))?;
    let shape = decode_shape(get_field(obj, "shape", path)?, &format!("{}/shape", path))?;
    Ok(Tensor {
        id: TensorId(id),
        dtype,
        shape,
    })
}

fn decode_tensor_array(v: &JsonValue, path: &str) -> Result<Vec<Tensor>, Error> {
    let items = expect_array(v, path)?;
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let item_path = format!("{}[{}]", path, i);
        out.push(decode_tensor(item, &item_path)?);
    }
    Ok(out)
}

fn decode_hex(v: &JsonValue, path: &str) -> Result<Vec<u8>, Error> {
    let s = expect_string(v, path)?;
    let bytes = s.as_bytes();
    if bytes.len() % 2 != 0 {
        return Err(Error::BadHex {
            reason: "odd character count",
            path: path.to_owned(),
        });
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.as_chunks::<2>().0 {
        let hi = hex_nibble(chunk[0], path)?;
        let lo = hex_nibble(chunk[1], path)?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(c: u8, path: &str) -> Result<u8, Error> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(Error::BadHex {
            reason: "non-hex character",
            path: path.to_owned(),
        }),
    }
}

fn decode_constant(v: &JsonValue, path: &str) -> Result<Constant, Error> {
    let obj = expect_object(v, path)?;
    let id = expect_u32(
        get_field(obj, "tensor_id", path)?,
        &format!("{}/tensor_id", path),
    )?;
    let dtype = decode_dtype(get_field(obj, "dtype", path)?, &format!("{}/dtype", path))?;
    let shape = decode_shape(get_field(obj, "shape", path)?, &format!("{}/shape", path))?;
    let bytes = decode_hex(
        get_field(obj, "bytes_hex", path)?,
        &format!("{}/bytes_hex", path),
    )?;
    Ok(Constant {
        tensor: Tensor {
            id: TensorId(id),
            dtype,
            shape,
        },
        bytes,
    })
}

fn decode_constant_array(v: &JsonValue, path: &str) -> Result<Vec<Constant>, Error> {
    let items = expect_array(v, path)?;
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let item_path = format!("{}[{}]", path, i);
        out.push(decode_constant(item, &item_path)?);
    }
    Ok(out)
}

fn decode_op_array(v: &JsonValue, path: &str) -> Result<Vec<Op>, Error> {
    let items = expect_array(v, path)?;
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let item_path = format!("{}[{}]", path, i);
        out.push(decode_op(item, &item_path)?);
    }
    Ok(out)
}

fn decode_op(v: &JsonValue, path: &str) -> Result<Op, Error> {
    let obj = expect_object(v, path)?;
    let kind = expect_string(get_field(obj, "kind", path)?, &format!("{}/kind", path))?.to_owned();

    // Helper for reading a tensor-id field by name into a TensorId.
    let tid = |name: &str| -> Result<TensorId, Error> {
        let field_path = format!("{}/{}", path, name);
        let v = get_field(obj, name, path)?;
        Ok(TensorId(expect_u32(v, &field_path)?))
    };
    let u32f = |name: &str| -> Result<u32, Error> {
        let field_path = format!("{}/{}", path, name);
        let v = get_field(obj, name, path)?;
        expect_u32(v, &field_path)
    };
    let boolf = |name: &str| -> Result<bool, Error> {
        let field_path = format!("{}/{}", path, name);
        let v = get_field(obj, name, path)?;
        expect_bool(v, &field_path)
    };
    let shapef = |name: &str| -> Result<Shape, Error> {
        let field_path = format!("{}/{}", path, name);
        let v = get_field(obj, name, path)?;
        decode_shape(v, &field_path)
    };
    let u32arrayf = |name: &str| -> Result<Vec<u32>, Error> {
        let field_path = format!("{}/{}", path, name);
        let v = get_field(obj, name, path)?;
        decode_u32_array(v, &field_path)
    };
    let dtypef = |name: &str| -> Result<DType, Error> {
        let field_path = format!("{}/{}", path, name);
        let v = get_field(obj, name, path)?;
        decode_dtype(v, &field_path)
    };

    let op = match kind.as_str() {
        "Neg" => Op::Neg {
            input: tid("input")?,
            output: tid("output")?,
        },
        "Abs" => Op::Abs {
            input: tid("input")?,
            output: tid("output")?,
        },
        "Sqrt" => Op::Sqrt {
            input: tid("input")?,
            output: tid("output")?,
        },
        "Exp" => Op::Exp {
            input: tid("input")?,
            output: tid("output")?,
        },
        "Log" => Op::Log {
            input: tid("input")?,
            output: tid("output")?,
        },
        "Tanh" => Op::Tanh {
            input: tid("input")?,
            output: tid("output")?,
        },
        "Recip" => Op::Recip {
            input: tid("input")?,
            output: tid("output")?,
        },
        "Add" => Op::Add {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "Sub" => Op::Sub {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "Mul" => Op::Mul {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "Div" => Op::Div {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "Max" => Op::Max {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "Min" => Op::Min {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "Pow" => Op::Pow {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "ReduceSum" => Op::ReduceSum {
            input: tid("input")?,
            axes: u32arrayf("axes")?,
            keep_dims: boolf("keep_dims")?,
            output: tid("output")?,
        },
        "ReduceMax" => Op::ReduceMax {
            input: tid("input")?,
            axes: u32arrayf("axes")?,
            keep_dims: boolf("keep_dims")?,
            output: tid("output")?,
        },
        "ReduceMean" => Op::ReduceMean {
            input: tid("input")?,
            axes: u32arrayf("axes")?,
            keep_dims: boolf("keep_dims")?,
            output: tid("output")?,
        },
        "Reshape" => Op::Reshape {
            input: tid("input")?,
            new_shape: shapef("new_shape")?,
            output: tid("output")?,
        },
        "Transpose" => Op::Transpose {
            input: tid("input")?,
            perm: u32arrayf("perm")?,
            output: tid("output")?,
        },
        "Broadcast" => Op::Broadcast {
            input: tid("input")?,
            target_shape: shapef("target_shape")?,
            output: tid("output")?,
        },
        "Slice" => Op::Slice {
            input: tid("input")?,
            axis: u32f("axis")?,
            start: u32f("start")?,
            end: u32f("end")?,
            step: u32f("step")?,
            output: tid("output")?,
        },
        "Concat" => {
            let inputs_v = get_field(obj, "inputs", path)?;
            let ids = decode_u32_array(inputs_v, &format!("{}/inputs", path))?;
            Op::Concat {
                inputs: ids.into_iter().map(TensorId).collect(),
                axis: u32f("axis")?,
                output: tid("output")?,
            }
        }
        "MatMul" => Op::MatMul {
            a: tid("a")?,
            b: tid("b")?,
            output: tid("output")?,
        },
        "Equal" => Op::Equal {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "Less" => Op::Less {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "Greater" => Op::Greater {
            lhs: tid("lhs")?,
            rhs: tid("rhs")?,
            output: tid("output")?,
        },
        "Where" => Op::Where {
            predicate: tid("predicate")?,
            true_value: tid("true_value")?,
            false_value: tid("false_value")?,
            output: tid("output")?,
        },
        "Cast" => Op::Cast {
            input: tid("input")?,
            dtype: dtypef("dtype")?,
            output: tid("output")?,
        },
        "Const" => Op::Const {
            constant: u32f("constant")?,
            output: tid("output")?,
        },
        other => {
            return Err(Error::UnknownOpKind {
                saw: other.to_owned(),
                path: format!("{}/kind", path),
            });
        }
    };
    // Defensive: warn (in debug) if the op object carried unknown
    // extra fields.  We tolerate them at runtime — JSON consumers
    // commonly add their own metadata — but a strict mode could
    // reject them.  For now, just touch get_field_opt to keep the
    // helper referenced.
    let _ = get_field_opt(obj, "_unknown_marker_for_strict_mode");
    Ok(op)
}

// ════════════════════════════════════════════════════════════════════
//                              TESTS
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_ir::GraphBuilder;

    fn build_relu_layer() -> Graph {
        // y = max(x @ w + b, 0) — the canonical mini-graph from the
        // matrix-ir crate-level doc example.
        let mut g = GraphBuilder::new();
        let x = g.input(DType::F32, Shape::from(&[1, 4]));
        let w = g.input(DType::F32, Shape::from(&[4, 2]));
        let b = g.input(DType::F32, Shape::from(&[1, 2]));
        let zero = g.constant(DType::F32, Shape::from(&[1, 2]), vec![0u8; 8]);
        let xw = g.matmul(&x, &w);
        let xwb = g.add(&xw, &b);
        let y = g.max(&xwb, &zero);
        g.output(&y);
        g.build().unwrap()
    }

    #[test]
    fn encode_decode_round_trips_relu_layer() {
        let g = build_relu_layer();
        let json = encode(&g);
        let g2 = decode(&json).expect("decode");
        assert_eq!(g, g2);
    }

    #[test]
    fn encode_starts_with_version_field() {
        let g = build_relu_layer();
        let json = encode(&g);
        assert!(
            json.starts_with("{\"matrix_ir_version\":1"),
            "got: {}",
            &json[..80.min(json.len())]
        );
    }

    #[test]
    fn encode_pretty_is_multi_line() {
        let g = build_relu_layer();
        let pretty = encode_pretty(&g);
        assert!(pretty.contains('\n'));
        let g2 = decode(&pretty).expect("pretty decode");
        assert_eq!(g, g2);
    }

    #[test]
    fn decode_rejects_unsupported_version() {
        let json = r#"{"matrix_ir_version":999,"tensors":[],"inputs":[],"outputs":[],"ops":[],"constants":[]}"#;
        let err = decode(json).unwrap_err();
        assert_eq!(err, Error::UnsupportedVersion { saw: 999 });
    }

    #[test]
    fn decode_rejects_unknown_op_kind() {
        let json = r#"{"matrix_ir_version":1,"tensors":[],"inputs":[],"outputs":[],"ops":[{"kind":"Bogus","output":0}],"constants":[]}"#;
        let err = decode(json).unwrap_err();
        assert!(matches!(err, Error::UnknownOpKind { .. }), "got {:?}", err);
    }

    #[test]
    fn decode_rejects_unknown_dtype() {
        // `f16` is not a supported dtype (f64 is, as of MX12).
        let json = r#"{"matrix_ir_version":1,"tensors":[{"id":0,"dtype":"f16","shape":[1]}],"inputs":[],"outputs":[],"ops":[],"constants":[]}"#;
        let err = decode(json).unwrap_err();
        assert!(matches!(err, Error::UnknownDType { .. }), "got {:?}", err);
    }

    #[test]
    fn decode_rejects_odd_hex_length() {
        let json = r#"{"matrix_ir_version":1,"tensors":[{"id":0,"dtype":"u8","shape":[1]}],"inputs":[],"outputs":[],"ops":[],"constants":[{"tensor_id":0,"dtype":"u8","shape":[1],"bytes_hex":"abc"}]}"#;
        let err = decode(json).unwrap_err();
        assert!(
            matches!(
                err,
                Error::BadHex {
                    reason: "odd character count",
                    ..
                }
            ),
            "got {:?}",
            err
        );
    }

    #[test]
    fn decode_rejects_invalid_hex_char() {
        let json = r#"{"matrix_ir_version":1,"tensors":[{"id":0,"dtype":"u8","shape":[1]}],"inputs":[],"outputs":[],"ops":[],"constants":[{"tensor_id":0,"dtype":"u8","shape":[1],"bytes_hex":"zz"}]}"#;
        let err = decode(json).unwrap_err();
        assert!(
            matches!(
                err,
                Error::BadHex {
                    reason: "non-hex character",
                    ..
                }
            ),
            "got {:?}",
            err
        );
    }

    #[test]
    fn decode_rejects_negative_id() {
        let json = r#"{"matrix_ir_version":1,"tensors":[{"id":-1,"dtype":"f32","shape":[1]}],"inputs":[],"outputs":[],"ops":[],"constants":[]}"#;
        let err = decode(json).unwrap_err();
        assert!(matches!(err, Error::BadInteger { .. }), "got {:?}", err);
    }

    #[test]
    fn decode_rejects_fractional_id() {
        let json = r#"{"matrix_ir_version":1,"tensors":[{"id":1.5,"dtype":"f32","shape":[1]}],"inputs":[],"outputs":[],"ops":[],"constants":[]}"#;
        let err = decode(json).unwrap_err();
        assert!(matches!(err, Error::BadInteger { .. }), "got {:?}", err);
    }

    #[test]
    fn decode_rejects_missing_required_field() {
        // Missing "outputs" field.
        let json = r#"{"matrix_ir_version":1,"tensors":[],"inputs":[],"ops":[],"constants":[]}"#;
        let err = decode(json).unwrap_err();
        assert!(matches!(err, Error::SchemaMismatch { .. }), "got {:?}", err);
    }

    #[test]
    fn decode_rejects_input_tensor_id_out_of_range() {
        let json = r#"{"matrix_ir_version":1,"tensors":[{"id":0,"dtype":"f32","shape":[1]}],"inputs":[5],"outputs":[0],"ops":[],"constants":[]}"#;
        let err = decode(json).unwrap_err();
        assert_eq!(err, Error::InputTensorMissing { id: 5 });
    }

    #[test]
    fn decode_rejects_garbage_json() {
        let err = decode("not json at all").unwrap_err();
        assert!(matches!(err, Error::JsonSyntax { .. }), "got {:?}", err);
    }

    #[test]
    fn hex_round_trips_through_bytes() {
        let bytes_in: Vec<u8> = vec![0x00, 0x01, 0xAB, 0xCD, 0xEF, 0xFF];
        let v = bytes_to_hex_json(&bytes_in);
        if let JsonValue::String(s) = &v {
            // Always lowercase.
            assert_eq!(s, "0001abcdefff");
        } else {
            panic!("expected String");
        }
        let bytes_out = decode_hex(&v, "/test").unwrap();
        assert_eq!(bytes_in, bytes_out);
    }

    #[test]
    fn encoded_json_round_trips_for_every_op_family() {
        // Construct a graph that touches every op family so we
        // exercise every match arm of encode_op and decode_op.
        let mut g = GraphBuilder::new();
        let x = g.input(DType::F32, Shape::from(&[2, 3]));
        let y = g.input(DType::F32, Shape::from(&[2, 3]));

        // unary
        let neg = g.neg(&x);
        let abs = g.abs(&neg);
        let sqrt = g.sqrt(&abs);
        let exp = g.exp(&sqrt);
        let log = g.log(&exp);
        let tanh = g.tanh(&log);
        let recip = g.recip(&tanh);

        // binary
        let add = g.add(&recip, &y);
        let sub = g.sub(&add, &y);
        let mul = g.mul(&sub, &y);
        let div = g.div(&mul, &y);
        let max = g.max(&div, &y);
        let min = g.min(&max, &y);
        let pow = g.pow(&min, &y);

        // reductions
        let _sum = g.reduce_sum(&pow, vec![0], true);
        let _maxr = g.reduce_max(&pow, vec![1], false);
        let _meanr = g.reduce_mean(&pow, vec![], true);

        // shape
        let resh = g.reshape(&pow, Shape::from(&[1, 6]));
        let trans = g.transpose(&pow, vec![1, 0]);
        let _bcast = g.broadcast(&resh, Shape::from(&[2, 6]));
        let _slice = g.slice(&pow, 1, 0, 3, 1);
        let _concat = g.concat(&[&pow, &pow], 0);

        // linear algebra
        let a = g.input(DType::F32, Shape::from(&[2, 4]));
        let b = g.input(DType::F32, Shape::from(&[4, 3]));
        let _mm = g.matmul(&a, &b);

        // comparison
        let _eq = g.equal(&pow, &y);
        let _lt = g.less(&pow, &y);
        let _gt = g.greater(&pow, &y);

        // selection
        let pred = g.input(DType::U8, Shape::from(&[2, 3]));
        let _wher = g.where_(&pred, &pow, &y);

        // conversion
        let _cast = g.cast(&pow, DType::I32);

        // constant
        let kc = g.constant(DType::F32, Shape::from(&[2, 3]), vec![0u8; 24]);

        g.output(&trans);
        g.output(&kc);
        let graph = g.build().unwrap();

        let json = encode(&graph);
        let decoded = decode(&json).expect("round-trip");
        assert_eq!(graph, decoded);
    }

    #[test]
    fn binary_and_json_round_trip_through_each_other() {
        // The binary and JSON formats must be isomorphic over the
        // Graph value.  This test goes
        //   Graph -> bytes -> Graph -> json -> Graph
        // and asserts equality at the start and end.
        let g_in = build_relu_layer();
        let bytes = g_in.to_bytes();
        let g_mid = Graph::from_bytes(&bytes).expect("binary decode");
        assert_eq!(g_in, g_mid);
        let json = encode(&g_mid);
        let g_out = decode(&json).expect("json decode");
        assert_eq!(g_in, g_out);
    }
}
