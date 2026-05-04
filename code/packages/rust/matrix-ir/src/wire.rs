//! Hand-rolled binary wire format for [`Graph`].
//!
//! See spec MX03 §"Wire format primitives".  The format is implementation-
//! agnostic; this is just the Rust encoder/decoder.  A Python or
//! JavaScript port that follows the spec produces interchangeable bytes.
//!
//! ## Primitives
//!
//! | name   | encoding |
//! |--------|----------|
//! | `u8`   | 1 byte |
//! | `u16`  | 2 bytes LE |
//! | `u32`  | 4 bytes LE |
//! | `u64`  | 8 bytes LE |
//! | `uv64` | varint, 7 bits/byte, top bit means "more bytes follow" |
//! | `bytes`| `uv64` length + raw bytes |
//! | `str`  | `bytes`, validated UTF-8 |
//! | `enum` | `uv64` tag + variant payload |
//! | `vec<T>` | `uv64` count + count instances of T |
//!
//! ## Top-level `Graph` layout
//!
//! ```text
//! u32         format_version              // = WIRE_FORMAT_VERSION
//! vec<Tensor> tensors
//! vec<Tensor> inputs                       // (full Tensor records)
//! vec<TensorId> outputs                    // u32 ids
//! vec<Op>      ops
//! vec<Constant> constants
//! ```
//!
//! No frame header / no payload length — the caller supplies a slice
//! and we either parse it all or fail.  The MX03 message-level frame
//! lives in `executor-protocol` and adds versioning and length
//! prefixing on top of this.

use crate::graph::{Constant, Graph};
use crate::op::Op;
use crate::tensor::{DType, Shape, Tensor, TensorId};
use crate::validate::IrError;
use crate::WIRE_FORMAT_VERSION;

// ─────────────────────────── encoder ───────────────────────────

/// Streaming writer that appends to a `Vec<u8>`.  Owns no state beyond
/// the buffer; encoding is straight-line append.
pub(crate) struct Writer<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> Writer<'a> {
    pub(crate) fn new(out: &'a mut Vec<u8>) -> Self {
        Writer { out }
    }

    pub(crate) fn u8(&mut self, v: u8) {
        self.out.push(v);
    }
    pub(crate) fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn u64(&mut self, v: u64) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    /// LEB128-style unsigned varint.
    pub(crate) fn uv64(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.out.push(((v as u8) & 0x7F) | 0x80);
            v >>= 7;
        }
        self.out.push(v as u8);
    }
    pub(crate) fn bytes(&mut self, b: &[u8]) {
        self.uv64(b.len() as u64);
        self.out.extend_from_slice(b);
    }
}

/// Encode a complete graph into bytes.  Round-trips with [`decode_graph`].
pub(crate) fn encode_graph(g: &Graph, out: &mut Vec<u8>) {
    let mut w = Writer::new(out);
    w.u32(WIRE_FORMAT_VERSION);

    // tensors
    w.uv64(g.tensors.len() as u64);
    for t in &g.tensors {
        encode_tensor(t, &mut w);
    }

    // inputs (full Tensor records)
    w.uv64(g.inputs.len() as u64);
    for t in &g.inputs {
        encode_tensor(t, &mut w);
    }

    // outputs (ids only)
    w.uv64(g.outputs.len() as u64);
    for id in &g.outputs {
        w.u32(id.0);
    }

    // ops
    w.uv64(g.ops.len() as u64);
    for op in &g.ops {
        encode_op(op, &mut w);
    }

    // constants
    w.uv64(g.constants.len() as u64);
    for c in &g.constants {
        encode_constant(c, &mut w);
    }
}

fn encode_tensor(t: &Tensor, w: &mut Writer<'_>) {
    w.u32(t.id.0);
    w.u8(t.dtype.wire_tag());
    encode_shape(&t.shape, w);
}

fn encode_shape(s: &Shape, w: &mut Writer<'_>) {
    w.uv64(s.dims.len() as u64);
    for &d in &s.dims {
        w.u32(d);
    }
}

fn encode_constant(c: &Constant, w: &mut Writer<'_>) {
    encode_tensor(&c.tensor, w);
    w.bytes(&c.bytes);
}

fn encode_op(op: &Op, w: &mut Writer<'_>) {
    w.u8(op.wire_tag());
    match op {
        Op::Neg { input, output }
        | Op::Abs { input, output }
        | Op::Sqrt { input, output }
        | Op::Exp { input, output }
        | Op::Log { input, output }
        | Op::Tanh { input, output }
        | Op::Recip { input, output } => {
            w.u32(input.0);
            w.u32(output.0);
        }
        Op::Add { lhs, rhs, output }
        | Op::Sub { lhs, rhs, output }
        | Op::Mul { lhs, rhs, output }
        | Op::Div { lhs, rhs, output }
        | Op::Max { lhs, rhs, output }
        | Op::Min { lhs, rhs, output }
        | Op::Pow { lhs, rhs, output }
        | Op::Equal { lhs, rhs, output }
        | Op::Less { lhs, rhs, output }
        | Op::Greater { lhs, rhs, output } => {
            w.u32(lhs.0);
            w.u32(rhs.0);
            w.u32(output.0);
        }
        Op::ReduceSum {
            input,
            axes,
            keep_dims,
            output,
        }
        | Op::ReduceMax {
            input,
            axes,
            keep_dims,
            output,
        }
        | Op::ReduceMean {
            input,
            axes,
            keep_dims,
            output,
        } => {
            w.u32(input.0);
            w.uv64(axes.len() as u64);
            for &a in axes {
                w.u32(a);
            }
            w.u8(if *keep_dims { 1 } else { 0 });
            w.u32(output.0);
        }
        Op::Reshape {
            input,
            new_shape,
            output,
        } => {
            w.u32(input.0);
            encode_shape(new_shape, w);
            w.u32(output.0);
        }
        Op::Transpose {
            input,
            perm,
            output,
        } => {
            w.u32(input.0);
            w.uv64(perm.len() as u64);
            for &p in perm {
                w.u32(p);
            }
            w.u32(output.0);
        }
        Op::Broadcast {
            input,
            target_shape,
            output,
        } => {
            w.u32(input.0);
            encode_shape(target_shape, w);
            w.u32(output.0);
        }
        Op::MatMul { a, b, output } => {
            w.u32(a.0);
            w.u32(b.0);
            w.u32(output.0);
        }
        Op::Where {
            predicate,
            true_value,
            false_value,
            output,
        } => {
            w.u32(predicate.0);
            w.u32(true_value.0);
            w.u32(false_value.0);
            w.u32(output.0);
        }
        Op::Cast {
            input,
            dtype,
            output,
        } => {
            w.u32(input.0);
            w.u8(dtype.wire_tag());
            w.u32(output.0);
        }
        Op::Const { constant, output } => {
            w.u32(*constant);
            w.u32(output.0);
        }
    }
}

// ─────────────────────────── decoder ───────────────────────────

/// Cap a pre-allocation against what the input could possibly contain.
///
/// `n` is the attacker-supplied count.  `min_elem_bytes` is the smallest
/// number of payload bytes any single element of this kind can occupy.
/// `remaining` is the number of bytes still in the input buffer.
///
/// We return `min(n, remaining / min_elem_bytes)`.  This means an
/// attacker who claims `n = 2^60` but only sends 50 bytes can make us
/// allocate at most `50 / min_elem_bytes` slots — bounded by what the
/// input could actually fill.  Push() will still grow the Vec further
/// when the legitimate data exceeds the cap, but the initial allocation
/// can no longer be amplified arbitrarily.
fn bounded_capacity(n: u64, min_elem_bytes: usize, remaining: usize) -> usize {
    let max_possible = remaining / min_elem_bytes.max(1);
    if n > max_possible as u64 {
        max_possible
    } else {
        n as usize
    }
}

pub(crate) struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    fn need(&self, n: usize) -> Result<(), IrError> {
        // checked_add guards against pos+n overflowing usize on 32-bit
        // targets when n was decoded from an attacker-controlled varint.
        let end = self
            .pos
            .checked_add(n)
            .ok_or(IrError::WireUnexpectedEof)?;
        if end > self.buf.len() {
            Err(IrError::WireUnexpectedEof)
        } else {
            Ok(())
        }
    }

    /// Bytes remaining after the cursor.  Used by the bounded-capacity
    /// helper to cap pre-allocations against the actual payload size.
    pub(crate) fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, IrError> {
        self.need(1)?;
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }

    pub(crate) fn u32(&mut self) -> Result<u32, IrError> {
        self.need(4)?;
        let v = u32::from_le_bytes(self.buf[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(v)
    }

    /// Read an unsigned varint (LEB128).  Bounded at 10 bytes to prevent
    /// pathological inputs from causing unbounded reads.
    pub(crate) fn uv64(&mut self) -> Result<u64, IrError> {
        let mut acc: u64 = 0;
        let mut shift = 0u32;
        for _ in 0..10 {
            let b = self.u8()?;
            acc |= ((b & 0x7F) as u64) << shift;
            if b & 0x80 == 0 {
                return Ok(acc);
            }
            shift += 7;
        }
        Err(IrError::WireOversizedVarint)
    }

    pub(crate) fn bytes(&mut self) -> Result<Vec<u8>, IrError> {
        let len_u64 = self.uv64()?;
        // On 32-bit targets, a u64 length can exceed usize::MAX; reject
        // explicitly rather than silently truncating with `as usize`.
        if len_u64 > usize::MAX as u64 {
            return Err(IrError::WireUnexpectedEof);
        }
        let len = len_u64 as usize;
        self.need(len)?;
        let v = self.buf[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(v)
    }

    pub(crate) fn at_end(&self) -> bool {
        self.pos == self.buf.len()
    }
}

/// Decode a complete graph from bytes.  Errors out if the input is
/// truncated, contains unknown tags, or has trailing bytes after a
/// successful parse.
pub(crate) fn decode_graph(buf: &[u8]) -> Result<Graph, IrError> {
    let mut r = Reader::new(buf);
    let version = r.u32()?;
    if version != WIRE_FORMAT_VERSION {
        return Err(IrError::WireUnsupportedVersion(version));
    }

    // Each pre-allocation is capped against the bytes still available
    // in the buffer, so an attacker cannot amplify a small payload into
    // a huge initial Vec allocation.  Min element sizes:
    //   - Tensor:    u32 id (4) + u8 dtype (1) + uv64 rank (≥1) = 6 bytes
    //   - TensorId:  u32 = 4 bytes
    //   - Op:        u8 tag (≥1) — the smallest var, Const, is 9 bytes
    //   - Constant:  Tensor (6) + uv64 byte_length (≥1) = 7 bytes
    let n_tensors = r.uv64()?;
    let cap = bounded_capacity(n_tensors, 6, r.remaining());
    let mut tensors = Vec::with_capacity(cap);
    for _ in 0..n_tensors {
        tensors.push(decode_tensor(&mut r)?);
    }

    let n_inputs = r.uv64()?;
    let cap = bounded_capacity(n_inputs, 6, r.remaining());
    let mut inputs = Vec::with_capacity(cap);
    for _ in 0..n_inputs {
        inputs.push(decode_tensor(&mut r)?);
    }

    let n_outputs = r.uv64()?;
    let cap = bounded_capacity(n_outputs, 4, r.remaining());
    let mut outputs = Vec::with_capacity(cap);
    for _ in 0..n_outputs {
        outputs.push(TensorId(r.u32()?));
    }

    let n_ops = r.uv64()?;
    let cap = bounded_capacity(n_ops, 1, r.remaining());
    let mut ops = Vec::with_capacity(cap);
    for _ in 0..n_ops {
        ops.push(decode_op(&mut r)?);
    }

    let n_constants = r.uv64()?;
    let cap = bounded_capacity(n_constants, 7, r.remaining());
    let mut constants = Vec::with_capacity(cap);
    for _ in 0..n_constants {
        constants.push(decode_constant(&mut r)?);
    }

    if !r.at_end() {
        return Err(IrError::WireTrailingBytes);
    }

    Ok(Graph {
        inputs,
        outputs,
        ops,
        tensors,
        constants,
    })
}

fn decode_tensor(r: &mut Reader<'_>) -> Result<Tensor, IrError> {
    let id = TensorId(r.u32()?);
    let dtype_tag = r.u8()?;
    let dtype = DType::from_wire_tag(dtype_tag).ok_or(IrError::WireUnknownTag {
        what: "dtype",
        tag: dtype_tag as u64,
    })?;
    let shape = decode_shape(r)?;
    Ok(Tensor::new(id, dtype, shape))
}

fn decode_shape(r: &mut Reader<'_>) -> Result<Shape, IrError> {
    let n = r.uv64()?;
    // Each dim is u32 = 4 bytes.
    let cap = bounded_capacity(n, 4, r.remaining());
    let mut dims = Vec::with_capacity(cap);
    for _ in 0..n {
        dims.push(r.u32()?);
    }
    Ok(Shape { dims })
}

fn decode_constant(r: &mut Reader<'_>) -> Result<Constant, IrError> {
    let tensor = decode_tensor(r)?;
    let bytes = r.bytes()?;
    Ok(Constant { tensor, bytes })
}

fn decode_op(r: &mut Reader<'_>) -> Result<Op, IrError> {
    let tag = r.u8()?;
    match tag {
        // unary
        0x00 => Ok(Op::Neg {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x01 => Ok(Op::Abs {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x02 => Ok(Op::Sqrt {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x03 => Ok(Op::Exp {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x04 => Ok(Op::Log {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x05 => Ok(Op::Tanh {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x06 => Ok(Op::Recip {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        // binary
        0x07 => Ok(Op::Add {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x08 => Ok(Op::Sub {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x09 => Ok(Op::Mul {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x0A => Ok(Op::Div {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x0B => Ok(Op::Max {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x0C => Ok(Op::Min {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x0D => Ok(Op::Pow {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        // reductions
        0x0E => decode_reduction(r, |i, ax, kd, o| Op::ReduceSum {
            input: i,
            axes: ax,
            keep_dims: kd,
            output: o,
        }),
        0x0F => decode_reduction(r, |i, ax, kd, o| Op::ReduceMax {
            input: i,
            axes: ax,
            keep_dims: kd,
            output: o,
        }),
        0x10 => decode_reduction(r, |i, ax, kd, o| Op::ReduceMean {
            input: i,
            axes: ax,
            keep_dims: kd,
            output: o,
        }),
        // shape
        0x11 => {
            let input = TensorId(r.u32()?);
            let new_shape = decode_shape(r)?;
            let output = TensorId(r.u32()?);
            Ok(Op::Reshape {
                input,
                new_shape,
                output,
            })
        }
        0x12 => {
            let input = TensorId(r.u32()?);
            let n = r.uv64()?;
            // Each perm element is u32 = 4 bytes.
            let cap = bounded_capacity(n, 4, r.remaining());
            let mut perm = Vec::with_capacity(cap);
            for _ in 0..n {
                perm.push(r.u32()?);
            }
            let output = TensorId(r.u32()?);
            Ok(Op::Transpose {
                input,
                perm,
                output,
            })
        }
        0x13 => {
            let input = TensorId(r.u32()?);
            let target_shape = decode_shape(r)?;
            let output = TensorId(r.u32()?);
            Ok(Op::Broadcast {
                input,
                target_shape,
                output,
            })
        }
        // 0x14 reserved
        0x15 => Ok(Op::MatMul {
            a: TensorId(r.u32()?),
            b: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x16 => Ok(Op::Equal {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x17 => Ok(Op::Less {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x18 => Ok(Op::Greater {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x19 => Ok(Op::Where {
            predicate: TensorId(r.u32()?),
            true_value: TensorId(r.u32()?),
            false_value: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        }),
        0x1A => {
            let input = TensorId(r.u32()?);
            let dtype_tag = r.u8()?;
            let dtype = DType::from_wire_tag(dtype_tag).ok_or(
                IrError::WireUnknownTag {
                    what: "dtype",
                    tag: dtype_tag as u64,
                },
            )?;
            let output = TensorId(r.u32()?);
            Ok(Op::Cast {
                input,
                dtype,
                output,
            })
        }
        0x1B => Ok(Op::Const {
            constant: r.u32()?,
            output: TensorId(r.u32()?),
        }),
        unknown => Err(IrError::WireUnknownTag {
            what: "op",
            tag: unknown as u64,
        }),
    }
}

fn decode_reduction(
    r: &mut Reader<'_>,
    ctor: fn(TensorId, Vec<u32>, bool, TensorId) -> Op,
) -> Result<Op, IrError> {
    let input = TensorId(r.u32()?);
    let n_axes = r.uv64()?;
    // Each axis is u32 = 4 bytes.
    let cap = bounded_capacity(n_axes, 4, r.remaining());
    let mut axes = Vec::with_capacity(cap);
    for _ in 0..n_axes {
        axes.push(r.u32()?);
    }
    let keep_dims = r.u8()? != 0;
    let output = TensorId(r.u32()?);
    Ok(ctor(input, axes, keep_dims, output))
}

// ─────────────────────────── public API ───────────────────────────

impl Graph {
    /// Serialise the graph to bytes.  See spec MX03 for the byte
    /// layout.  Round-trips with [`from_bytes`](Graph::from_bytes).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64);
        encode_graph(self, &mut out);
        out
    }

    /// Deserialise a graph from bytes.  The bytes must consist of
    /// exactly one graph; trailing bytes are an error.
    ///
    /// # Security note
    ///
    /// `from_bytes` performs **structural** decoding only — it does
    /// not call [`Graph::validate`].  When deserialising untrusted
    /// input (e.g. a remote executor's wire payload):
    ///
    /// 1. Cap the size of `buf` *before* calling this function.  A
    ///    sensible default for V1 is `16 MiB`; tune up only if real
    ///    workloads need more.  Without a cap the decoder still
    ///    terminates in linear time (`Vec::with_capacity` is bounded
    ///    against the input length, and the per-element loop fails
    ///    fast on truncation), but a 1 GB legitimate-looking payload
    ///    will allocate 1 GB.
    /// 2. Always call [`Graph::validate`] on the result before using
    ///    it.  An attacker can construct a structurally-decodable
    ///    graph that fails semantic checks (e.g. constant byte length
    ///    not matching declared shape).
    pub fn from_bytes(buf: &[u8]) -> Result<Graph, IrError> {
        decode_graph(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_round_trip() {
        for v in [0u64, 1, 127, 128, 300, 16384, u32::MAX as u64, u64::MAX] {
            let mut buf = Vec::new();
            Writer::new(&mut buf).uv64(v);
            let mut r = Reader::new(&buf);
            assert_eq!(r.uv64().unwrap(), v);
            assert!(r.at_end());
        }
    }

    #[test]
    fn varint_oversized_errors() {
        // 11 bytes all with continuation bit -> WireOversizedVarint
        let buf = vec![0xFF; 11];
        let mut r = Reader::new(&buf);
        assert!(matches!(r.uv64(), Err(IrError::WireOversizedVarint)));
    }

    #[test]
    fn truncation_returns_eof() {
        // u32 needs 4 bytes; supplying 3 should fail.
        let buf = vec![0u8; 3];
        let mut r = Reader::new(&buf);
        assert!(matches!(r.u32(), Err(IrError::WireUnexpectedEof)));
    }

    #[test]
    fn decode_unsupported_version_errors() {
        // Encode a fake header with a future version.
        let mut buf = Vec::new();
        Writer::new(&mut buf).u32(99);
        let result = decode_graph(&buf);
        assert!(matches!(result, Err(IrError::WireUnsupportedVersion(99))));
    }
}
