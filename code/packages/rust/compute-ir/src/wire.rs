//! Hand-rolled binary wire format for [`ComputeGraph`].
//!
//! Layout follows the same primitives as `matrix-ir`'s wire format
//! (varint, length-prefixed bytes, tagged unions) — see spec MX03
//! §"Wire format primitives".  This file is the encoder/decoder only;
//! the format itself is the same one a Python or JavaScript client
//! would implement from the spec.
//!
//! ## Top-level `ComputeGraph` layout
//!
//! ```text
//! u32             format_version          (must equal WIRE_FORMAT_VERSION)
//! vec<PlacedTensor>  tensors
//! vec<PlacedTensor>  inputs
//! vec<PlacedTensor>  outputs
//! vec<PlacedConstant> constants
//! vec<PlacedOp>      ops
//! ```
//!
//! ## Security
//!
//! Every length-prefixed allocation is bounded against remaining input
//! bytes (see `bounded_capacity`).  An attacker varint claiming
//! `u64::MAX` entries cannot amplify into a huge initial allocation
//! because the loop will short-circuit on `WireUnexpectedEof` once the
//! buffer runs out.
//!
//! Reader::need uses `checked_add` to guard against `pos + n`
//! overflowing `usize` on 32-bit targets.  See the `matrix-ir` wire
//! module for the same pattern.

use crate::graph::ComputeGraph;
use crate::placement::{
    BufferId, ExecutorId, OpTiming, PlacedConstant, PlacedOp, PlacedTensor, Residency,
};
use crate::validate::ComputeIrError;
use crate::WIRE_FORMAT_VERSION;
use matrix_ir::{DType, Op, Shape, TensorId};

// ─────────────────────────── encoder ───────────────────────────

struct Writer<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> Writer<'a> {
    fn new(out: &'a mut Vec<u8>) -> Self {
        Writer { out }
    }
    fn u8(&mut self, v: u8) {
        self.out.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    fn uv64(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.out.push(((v as u8) & 0x7F) | 0x80);
            v >>= 7;
        }
        self.out.push(v as u8);
    }
    fn bytes(&mut self, b: &[u8]) {
        self.uv64(b.len() as u64);
        self.out.extend_from_slice(b);
    }
}

fn encode_residency(r: &Residency, w: &mut Writer<'_>) {
    w.u32(r.executor.0);
    w.u64(r.buffer.0);
}

fn encode_shape(s: &Shape, w: &mut Writer<'_>) {
    w.uv64(s.dims.len() as u64);
    for &d in &s.dims {
        w.u32(d);
    }
}

fn encode_placed_tensor(t: &PlacedTensor, w: &mut Writer<'_>) {
    w.u32(t.id.0);
    w.u8(t.dtype.wire_tag());
    encode_shape(&t.shape, w);
    encode_residency(&t.residency, w);
}

fn encode_placed_constant(c: &PlacedConstant, w: &mut Writer<'_>) {
    w.u32(c.tensor.0);
    encode_residency(&c.residency, w);
    w.bytes(&c.bytes);
}

fn encode_op(op: &Op, w: &mut Writer<'_>) {
    // We borrow matrix-ir's wire-tag table but encode a minimal version
    // here.  Re-implementing rather than depending on matrix-ir's
    // crate-private wire module keeps the boundary clean.
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

fn encode_placed_op(op: &PlacedOp, w: &mut Writer<'_>) {
    w.u8(op.wire_tag());
    match op {
        PlacedOp::Compute {
            op: under,
            executor,
            timing,
        } => {
            encode_op(under, w);
            w.u32(executor.0);
            w.u64(timing.estimated_ns);
        }
        PlacedOp::Transfer {
            tensor,
            src,
            dst,
            bytes,
            timing,
        } => {
            w.u32(tensor.0);
            encode_residency(src, w);
            encode_residency(dst, w);
            w.u64(*bytes);
            w.u64(timing.estimated_ns);
        }
        PlacedOp::Alloc { residency, bytes } => {
            encode_residency(residency, w);
            w.u64(*bytes);
        }
        PlacedOp::Free { residency } => {
            encode_residency(residency, w);
        }
    }
}

/// Serialise a complete placed graph to bytes.
pub(crate) fn encode_graph(g: &ComputeGraph) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    let mut w = Writer::new(&mut out);
    w.u32(g.format_version);

    w.uv64(g.tensors.len() as u64);
    for t in &g.tensors {
        encode_placed_tensor(t, &mut w);
    }

    w.uv64(g.inputs.len() as u64);
    for t in &g.inputs {
        encode_placed_tensor(t, &mut w);
    }

    w.uv64(g.outputs.len() as u64);
    for t in &g.outputs {
        encode_placed_tensor(t, &mut w);
    }

    w.uv64(g.constants.len() as u64);
    for c in &g.constants {
        encode_placed_constant(c, &mut w);
    }

    w.uv64(g.ops.len() as u64);
    for op in &g.ops {
        encode_placed_op(op, &mut w);
    }

    out
}

// ─────────────────────────── decoder ───────────────────────────

/// Cap a pre-allocation against the remaining input bytes.  See
/// `matrix-ir::wire::bounded_capacity` for the rationale.
fn bounded_capacity(n: u64, min_elem_bytes: usize, remaining: usize) -> usize {
    let max_possible = remaining / min_elem_bytes.max(1);
    if n > max_possible as u64 {
        max_possible
    } else {
        n as usize
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }
    fn need(&self, n: usize) -> Result<(), ComputeIrError> {
        let end = self.pos.checked_add(n).ok_or(ComputeIrError::WireUnexpectedEof)?;
        if end > self.buf.len() {
            Err(ComputeIrError::WireUnexpectedEof)
        } else {
            Ok(())
        }
    }
    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }
    fn u8(&mut self) -> Result<u8, ComputeIrError> {
        self.need(1)?;
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }
    fn u32(&mut self) -> Result<u32, ComputeIrError> {
        self.need(4)?;
        let v = u32::from_le_bytes(self.buf[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(v)
    }
    fn u64(&mut self) -> Result<u64, ComputeIrError> {
        self.need(8)?;
        let v = u64::from_le_bytes(self.buf[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        Ok(v)
    }
    fn uv64(&mut self) -> Result<u64, ComputeIrError> {
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
        Err(ComputeIrError::WireOversizedVarint)
    }
    fn bytes(&mut self) -> Result<Vec<u8>, ComputeIrError> {
        let len_u64 = self.uv64()?;
        if len_u64 > usize::MAX as u64 {
            return Err(ComputeIrError::WireUnexpectedEof);
        }
        let len = len_u64 as usize;
        self.need(len)?;
        let v = self.buf[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(v)
    }
    fn at_end(&self) -> bool {
        self.pos == self.buf.len()
    }
}

fn decode_residency(r: &mut Reader<'_>) -> Result<Residency, ComputeIrError> {
    let executor = ExecutorId(r.u32()?);
    let buffer = BufferId(r.u64()?);
    Ok(Residency { executor, buffer })
}

fn decode_shape(r: &mut Reader<'_>) -> Result<Shape, ComputeIrError> {
    let n = r.uv64()?;
    let cap = bounded_capacity(n, 4, r.remaining());
    let mut dims = Vec::with_capacity(cap);
    for _ in 0..n {
        dims.push(r.u32()?);
    }
    Ok(Shape { dims })
}

fn decode_placed_tensor(r: &mut Reader<'_>) -> Result<PlacedTensor, ComputeIrError> {
    let id = TensorId(r.u32()?);
    let dtype_tag = r.u8()?;
    let dtype = DType::from_wire_tag(dtype_tag).ok_or(ComputeIrError::WireUnknownTag {
        what: "dtype",
        tag: dtype_tag as u64,
    })?;
    let shape = decode_shape(r)?;
    let residency = decode_residency(r)?;
    Ok(PlacedTensor {
        id,
        dtype,
        shape,
        residency,
    })
}

fn decode_placed_constant(r: &mut Reader<'_>) -> Result<PlacedConstant, ComputeIrError> {
    let tensor = TensorId(r.u32()?);
    let residency = decode_residency(r)?;
    let bytes = r.bytes()?;
    Ok(PlacedConstant {
        tensor,
        bytes,
        residency,
    })
}

fn decode_op(r: &mut Reader<'_>) -> Result<Op, ComputeIrError> {
    let tag = r.u8()?;
    let op = match tag {
        0x00 => Op::Neg {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x01 => Op::Abs {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x02 => Op::Sqrt {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x03 => Op::Exp {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x04 => Op::Log {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x05 => Op::Tanh {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x06 => Op::Recip {
            input: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x07 => Op::Add {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x08 => Op::Sub {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x09 => Op::Mul {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x0A => Op::Div {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x0B => Op::Max {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x0C => Op::Min {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x0D => Op::Pow {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x0E => decode_reduction(r, |i, ax, kd, o| Op::ReduceSum {
            input: i,
            axes: ax,
            keep_dims: kd,
            output: o,
        })?,
        0x0F => decode_reduction(r, |i, ax, kd, o| Op::ReduceMax {
            input: i,
            axes: ax,
            keep_dims: kd,
            output: o,
        })?,
        0x10 => decode_reduction(r, |i, ax, kd, o| Op::ReduceMean {
            input: i,
            axes: ax,
            keep_dims: kd,
            output: o,
        })?,
        0x11 => {
            let input = TensorId(r.u32()?);
            let new_shape = decode_shape(r)?;
            let output = TensorId(r.u32()?);
            Op::Reshape {
                input,
                new_shape,
                output,
            }
        }
        0x12 => {
            let input = TensorId(r.u32()?);
            let n = r.uv64()?;
            let cap = bounded_capacity(n, 4, r.remaining());
            let mut perm = Vec::with_capacity(cap);
            for _ in 0..n {
                perm.push(r.u32()?);
            }
            let output = TensorId(r.u32()?);
            Op::Transpose {
                input,
                perm,
                output,
            }
        }
        0x13 => {
            let input = TensorId(r.u32()?);
            let target_shape = decode_shape(r)?;
            let output = TensorId(r.u32()?);
            Op::Broadcast {
                input,
                target_shape,
                output,
            }
        }
        0x15 => Op::MatMul {
            a: TensorId(r.u32()?),
            b: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x16 => Op::Equal {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x17 => Op::Less {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x18 => Op::Greater {
            lhs: TensorId(r.u32()?),
            rhs: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x19 => Op::Where {
            predicate: TensorId(r.u32()?),
            true_value: TensorId(r.u32()?),
            false_value: TensorId(r.u32()?),
            output: TensorId(r.u32()?),
        },
        0x1A => {
            let input = TensorId(r.u32()?);
            let dtype_tag = r.u8()?;
            let dtype = DType::from_wire_tag(dtype_tag).ok_or(
                ComputeIrError::WireUnknownTag {
                    what: "dtype",
                    tag: dtype_tag as u64,
                },
            )?;
            let output = TensorId(r.u32()?);
            Op::Cast {
                input,
                dtype,
                output,
            }
        }
        0x1B => Op::Const {
            constant: r.u32()?,
            output: TensorId(r.u32()?),
        },
        unknown => {
            return Err(ComputeIrError::WireUnknownTag {
                what: "matrix_ir::Op",
                tag: unknown as u64,
            })
        }
    };
    Ok(op)
}

fn decode_reduction(
    r: &mut Reader<'_>,
    ctor: fn(TensorId, Vec<u32>, bool, TensorId) -> Op,
) -> Result<Op, ComputeIrError> {
    let input = TensorId(r.u32()?);
    let n = r.uv64()?;
    let cap = bounded_capacity(n, 4, r.remaining());
    let mut axes = Vec::with_capacity(cap);
    for _ in 0..n {
        axes.push(r.u32()?);
    }
    let keep_dims = r.u8()? != 0;
    let output = TensorId(r.u32()?);
    Ok(ctor(input, axes, keep_dims, output))
}

fn decode_placed_op(r: &mut Reader<'_>) -> Result<PlacedOp, ComputeIrError> {
    let tag = r.u8()?;
    match tag {
        0x00 => {
            let op = decode_op(r)?;
            let executor = ExecutorId(r.u32()?);
            let estimated_ns = r.u64()?;
            Ok(PlacedOp::Compute {
                op,
                executor,
                timing: OpTiming { estimated_ns },
            })
        }
        0x01 => {
            let tensor = TensorId(r.u32()?);
            let src = decode_residency(r)?;
            let dst = decode_residency(r)?;
            let bytes = r.u64()?;
            let estimated_ns = r.u64()?;
            Ok(PlacedOp::Transfer {
                tensor,
                src,
                dst,
                bytes,
                timing: OpTiming { estimated_ns },
            })
        }
        0x02 => {
            let residency = decode_residency(r)?;
            let bytes = r.u64()?;
            Ok(PlacedOp::Alloc { residency, bytes })
        }
        0x03 => {
            let residency = decode_residency(r)?;
            Ok(PlacedOp::Free { residency })
        }
        unknown => Err(ComputeIrError::WireUnknownTag {
            what: "PlacedOp",
            tag: unknown as u64,
        }),
    }
}

/// Decode a complete placed graph from bytes.
pub(crate) fn decode_graph(buf: &[u8]) -> Result<ComputeGraph, ComputeIrError> {
    let mut r = Reader::new(buf);
    let format_version = r.u32()?;
    if format_version != WIRE_FORMAT_VERSION {
        return Err(ComputeIrError::WireUnsupportedVersion(format_version));
    }

    // Min sizes per element:
    //   PlacedTensor:   u32 id (4) + u8 dtype (1) + uv64 rank (≥1) + 12 residency = 18
    //   PlacedConstant: u32 tensor (4) + 12 residency + uv64 byte_len (≥1) = 17
    //   PlacedOp:       u8 tag (≥1) — minimum is Free with 12 residency = 13
    let n_t = r.uv64()?;
    let cap = bounded_capacity(n_t, 18, r.remaining());
    let mut tensors = Vec::with_capacity(cap);
    for _ in 0..n_t {
        tensors.push(decode_placed_tensor(&mut r)?);
    }

    let n_in = r.uv64()?;
    let cap = bounded_capacity(n_in, 18, r.remaining());
    let mut inputs = Vec::with_capacity(cap);
    for _ in 0..n_in {
        inputs.push(decode_placed_tensor(&mut r)?);
    }

    let n_out = r.uv64()?;
    let cap = bounded_capacity(n_out, 18, r.remaining());
    let mut outputs = Vec::with_capacity(cap);
    for _ in 0..n_out {
        outputs.push(decode_placed_tensor(&mut r)?);
    }

    let n_c = r.uv64()?;
    let cap = bounded_capacity(n_c, 17, r.remaining());
    let mut constants = Vec::with_capacity(cap);
    for _ in 0..n_c {
        constants.push(decode_placed_constant(&mut r)?);
    }

    let n_op = r.uv64()?;
    let cap = bounded_capacity(n_op, 13, r.remaining());
    let mut ops = Vec::with_capacity(cap);
    for _ in 0..n_op {
        ops.push(decode_placed_op(&mut r)?);
    }

    if !r.at_end() {
        return Err(ComputeIrError::WireTrailingBytes);
    }

    Ok(ComputeGraph {
        format_version,
        inputs,
        outputs,
        constants,
        ops,
        tensors,
    })
}

// ─────────────────────────── public API ───────────────────────────

impl ComputeGraph {
    /// Serialise the placed graph to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_graph(self)
    }

    /// Deserialise a placed graph from bytes.
    ///
    /// # Security note
    ///
    /// `from_bytes` performs **structural** decoding only — it does
    /// not call [`ComputeGraph::validate`].  When deserialising
    /// untrusted input:
    ///
    /// 1. Cap the size of `buf` *before* calling this function.  The
    ///    decoder is bounded but a 1 GB legitimate-looking payload
    ///    will allocate 1 GB.
    /// 2. Always call [`ComputeGraph::validate`] on the result.  An
    ///    attacker can construct a structurally-decodable graph that
    ///    fails semantic checks (e.g. transfer source not matching
    ///    current residency).
    pub fn from_bytes(buf: &[u8]) -> Result<ComputeGraph, ComputeIrError> {
        decode_graph(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_round_trip() {
        for v in [0u64, 1, 127, 128, 300, u32::MAX as u64, u64::MAX] {
            let mut buf = Vec::new();
            Writer::new(&mut buf).uv64(v);
            let mut r = Reader::new(&buf);
            assert_eq!(r.uv64().unwrap(), v);
        }
    }

    #[test]
    fn varint_oversized_errors() {
        let buf = vec![0xFF; 11];
        let mut r = Reader::new(&buf);
        assert!(matches!(r.uv64(), Err(ComputeIrError::WireOversizedVarint)));
    }

    #[test]
    fn unsupported_version_errors() {
        let mut buf = Vec::new();
        Writer::new(&mut buf).u32(99);
        assert!(matches!(
            decode_graph(&buf),
            Err(ComputeIrError::WireUnsupportedVersion(99))
        ));
    }
}
