//! Dispatch — walks a `ComputeGraph` and executes each `Compute` op.
//!
//! Walks `graph.ops` in order, performs the per-op eval against the
//! buffer store, and returns per-op timings (with measured `ns: 0`
//! since we don't time CPU ops in V1).

use crate::buffers::BufferStore;
use crate::eval;
use compute_ir::{ComputeGraph, PlacedOp, Residency, CPU_EXECUTOR};
use executor_protocol::OpTiming;
use matrix_ir::{DType, Op, Shape, TensorId};
use std::collections::HashMap;

/// Run the graph and return per-op timings.  Returns `Err(message)`
/// on the first op that fails (out-of-bounds, missing buffer, etc.).
pub fn run(buffers: &mut BufferStore, graph: &ComputeGraph) -> Result<Vec<OpTiming>, String> {
    // Track which BufferId currently holds each TensorId.
    let mut residency: HashMap<TensorId, Residency> = HashMap::new();
    for inp in &graph.inputs {
        residency.insert(inp.id, inp.residency);
    }
    for c in &graph.constants {
        residency.insert(c.tensor, c.residency);
        // Upload constant bytes to its declared buffer.
        // (The runtime would normally do this via UploadBuffer, but we
        // honour it here when the constant's bytes are still in the graph.)
        if !buffers.contains(c.residency.buffer) {
            buffers.alloc(c.residency.buffer, c.bytes.len());
        }
        buffers.write(c.residency.buffer, 0, &c.bytes)?;
    }

    let mut timings = Vec::new();

    for (op_idx, pop) in graph.ops.iter().enumerate() {
        match pop {
            PlacedOp::Alloc { residency: r, bytes } => {
                buffers.alloc(r.buffer, *bytes as usize);
            }
            PlacedOp::Free { residency: r } => {
                buffers.free(r.buffer);
            }
            PlacedOp::Transfer {
                tensor,
                src,
                dst,
                bytes,
                ..
            } => {
                // Copy bytes from src buffer to dst buffer.  Since CPU
                // is the only executor here, src and dst are both in
                // our local store.
                let src_bytes = buffers.read(src.buffer, 0, *bytes as usize)?;
                if !buffers.contains(dst.buffer) {
                    buffers.alloc(dst.buffer, *bytes as usize);
                }
                buffers.write(dst.buffer, 0, &src_bytes)?;
                residency.insert(*tensor, *dst);
            }
            PlacedOp::Compute {
                op,
                executor,
                timing: _,
            } => {
                if *executor != CPU_EXECUTOR {
                    return Err(format!(
                        "op {} routed to non-CPU executor {} but reached CpuExecutor",
                        op_idx, executor.0
                    ));
                }
                exec_compute(buffers, graph, op)?;
                timings.push(OpTiming {
                    op_index: op_idx as u32,
                    ns: 0,
                });
            }
        }
    }

    Ok(timings)
}

/// Execute a single matrix-ir `Op` against the buffer store.  The
/// output buffer must already be allocated by a prior `PlacedOp::Alloc`.
fn exec_compute(buffers: &mut BufferStore, graph: &ComputeGraph, op: &Op) -> Result<(), String> {
    match op {
        // ──── Elementwise unary ────
        Op::Neg { input, output } => unary_op(buffers, graph, *input, *output, |dt, x, y, n| match dt {
            DType::F32 => eval::unary_f32(x, y, n, |v| -v),
            DType::I32 => eval::unary_i32(x, y, n, |v| v.wrapping_neg()),
            DType::U8 => eval::unary_u8(x, y, |v| v.wrapping_neg()),
        }),
        Op::Abs { input, output } => unary_op(buffers, graph, *input, *output, |dt, x, y, n| match dt {
            DType::F32 => eval::unary_f32(x, y, n, |v| v.abs()),
            DType::I32 => eval::unary_i32(x, y, n, |v| v.wrapping_abs()),
            DType::U8 => eval::unary_u8(x, y, |v| v),
        }),
        Op::Sqrt { input, output } => unary_op(buffers, graph, *input, *output, |dt, x, y, n| match dt {
            DType::F32 => eval::unary_f32(x, y, n, |v| v.sqrt()),
            _ => unreachable!("sqrt is float-only per validator"),
        }),
        Op::Exp { input, output } => unary_op(buffers, graph, *input, *output, |dt, x, y, n| match dt {
            DType::F32 => eval::unary_f32(x, y, n, |v| v.exp()),
            _ => unreachable!("exp is float-only"),
        }),
        Op::Log { input, output } => unary_op(buffers, graph, *input, *output, |dt, x, y, n| match dt {
            DType::F32 => eval::unary_f32(x, y, n, |v| v.ln()),
            _ => unreachable!("log is float-only"),
        }),
        Op::Tanh { input, output } => unary_op(buffers, graph, *input, *output, |dt, x, y, n| match dt {
            DType::F32 => eval::unary_f32(x, y, n, |v| v.tanh()),
            _ => unreachable!("tanh is float-only"),
        }),
        Op::Recip { input, output } => unary_op(buffers, graph, *input, *output, |dt, x, y, n| match dt {
            DType::F32 => eval::unary_f32(x, y, n, |v| 1.0 / v),
            _ => unreachable!("recip is float-only"),
        }),

        // ──── Elementwise binary ────
        Op::Add { lhs, rhs, output } => bin_op(buffers, graph, *lhs, *rhs, *output, |dt, a, b, c, n| match dt {
            DType::F32 => eval::binary_f32(a, b, c, n, |x, y| x + y),
            DType::I32 => eval::binary_i32(a, b, c, n, |x, y| x.wrapping_add(y)),
            DType::U8 => eval::binary_u8(a, b, c, |x, y| x.wrapping_add(y)),
        }),
        Op::Sub { lhs, rhs, output } => bin_op(buffers, graph, *lhs, *rhs, *output, |dt, a, b, c, n| match dt {
            DType::F32 => eval::binary_f32(a, b, c, n, |x, y| x - y),
            DType::I32 => eval::binary_i32(a, b, c, n, |x, y| x.wrapping_sub(y)),
            DType::U8 => eval::binary_u8(a, b, c, |x, y| x.wrapping_sub(y)),
        }),
        Op::Mul { lhs, rhs, output } => bin_op(buffers, graph, *lhs, *rhs, *output, |dt, a, b, c, n| match dt {
            DType::F32 => eval::binary_f32(a, b, c, n, |x, y| x * y),
            DType::I32 => eval::binary_i32(a, b, c, n, |x, y| x.wrapping_mul(y)),
            DType::U8 => eval::binary_u8(a, b, c, |x, y| x.wrapping_mul(y)),
        }),
        Op::Div { lhs, rhs, output } => bin_op(buffers, graph, *lhs, *rhs, *output, |dt, a, b, c, n| match dt {
            DType::F32 => eval::binary_f32(a, b, c, n, |x, y| x / y),
            _ => unreachable!("div is float-only in V1"),
        }),
        Op::Max { lhs, rhs, output } => bin_op(buffers, graph, *lhs, *rhs, *output, |dt, a, b, c, n| match dt {
            DType::F32 => eval::binary_f32(a, b, c, n, |x, y| x.max(y)),
            DType::I32 => eval::binary_i32(a, b, c, n, |x, y| x.max(y)),
            DType::U8 => eval::binary_u8(a, b, c, |x, y| x.max(y)),
        }),
        Op::Min { lhs, rhs, output } => bin_op(buffers, graph, *lhs, *rhs, *output, |dt, a, b, c, n| match dt {
            DType::F32 => eval::binary_f32(a, b, c, n, |x, y| x.min(y)),
            DType::I32 => eval::binary_i32(a, b, c, n, |x, y| x.min(y)),
            DType::U8 => eval::binary_u8(a, b, c, |x, y| x.min(y)),
        }),
        Op::Pow { lhs, rhs, output } => bin_op(buffers, graph, *lhs, *rhs, *output, |dt, a, b, c, n| match dt {
            DType::F32 => eval::binary_f32(a, b, c, n, |x, y| x.powf(y)),
            _ => unreachable!("pow is float-only"),
        }),

        // ──── Comparison (output is U8) ────
        Op::Equal { lhs, rhs, output } => compare_op(buffers, graph, *lhs, *rhs, *output, |a, b| a == b),
        Op::Less { lhs, rhs, output } => compare_op(buffers, graph, *lhs, *rhs, *output, |a, b| a < b),
        Op::Greater { lhs, rhs, output } => compare_op(buffers, graph, *lhs, *rhs, *output, |a, b| a > b),

        // ──── Where ────
        Op::Where {
            predicate,
            true_value,
            false_value,
            output,
        } => where_op(buffers, graph, *predicate, *true_value, *false_value, *output),

        // ──── Reductions ────
        Op::ReduceSum {
            input,
            axes,
            keep_dims,
            output,
        } => reduce_op(buffers, graph, *input, axes, *keep_dims, *output, ReduceKind::Sum),
        Op::ReduceMax {
            input,
            axes,
            keep_dims,
            output,
        } => reduce_op(buffers, graph, *input, axes, *keep_dims, *output, ReduceKind::Max),
        Op::ReduceMean {
            input,
            axes,
            keep_dims,
            output,
        } => reduce_op(buffers, graph, *input, axes, *keep_dims, *output, ReduceKind::Mean),

        // ──── Shape ops ────
        Op::Reshape {
            input,
            new_shape: _,
            output,
        } => {
            // Reshape is a memcpy in CPU-land — same byte layout, new
            // shape interpretation.
            let in_t = lookup_meta(graph, *input)?;
            let out_t = lookup_meta(graph, *output)?;
            let in_buf = lookup_buffer(buffers, graph, in_t.id)?.to_vec();
            let out_residency = lookup_residency(graph, out_t.id)?;
            buffers.write(out_residency.buffer, 0, &in_buf)?;
            Ok(())
        }
        Op::Transpose {
            input,
            perm,
            output,
        } => {
            let in_t = lookup_meta(graph, *input)?;
            let out_t = lookup_meta(graph, *output)?;
            let in_buf = lookup_buffer(buffers, graph, in_t.id)?.to_vec();
            let elem = in_t.dtype.size_bytes();
            let (out_bytes, _out_dims) =
                eval::transpose_bytes(&in_buf, &in_t.shape.dims, perm, elem);
            let out_residency = lookup_residency(graph, out_t.id)?;
            buffers.write(out_residency.buffer, 0, &out_bytes)?;
            Ok(())
        }
        Op::Broadcast {
            input,
            target_shape,
            output,
        } => {
            let in_t = lookup_meta(graph, *input)?;
            let out_t = lookup_meta(graph, *output)?;
            let in_buf = lookup_buffer(buffers, graph, in_t.id)?.to_vec();
            let elem = in_t.dtype.size_bytes();
            let out_bytes =
                eval::broadcast_bytes(&in_buf, &in_t.shape.dims, &target_shape.dims, elem);
            let out_residency = lookup_residency(graph, out_t.id)?;
            buffers.write(out_residency.buffer, 0, &out_bytes)?;
            Ok(())
        }

        // ──── MatMul ────
        Op::MatMul { a, b, output } => {
            let ta = lookup_meta(graph, *a)?;
            let tb = lookup_meta(graph, *b)?;
            let tc = lookup_meta(graph, *output)?;
            let m = ta.shape.dims[0] as usize;
            let k = ta.shape.dims[1] as usize;
            let n = tb.shape.dims[1] as usize;
            let a_buf = lookup_buffer(buffers, graph, ta.id)?.to_vec();
            let b_buf = lookup_buffer(buffers, graph, tb.id)?.to_vec();
            let elem = tc.dtype.size_bytes();
            let mut c_bytes = vec![0u8; m * n * elem];
            match ta.dtype {
                DType::F32 => eval::matmul_f32(&a_buf, &b_buf, &mut c_bytes, m, k, n),
                DType::I32 => eval::matmul_i32(&a_buf, &b_buf, &mut c_bytes, m, k, n),
                DType::U8 => eval::matmul_u8(&a_buf, &b_buf, &mut c_bytes, m, k, n),
            }
            let out_residency = lookup_residency(graph, tc.id)?;
            buffers.write(out_residency.buffer, 0, &c_bytes)?;
            Ok(())
        }

        // ──── Cast ────
        Op::Cast {
            input,
            dtype,
            output,
        } => {
            let in_t = lookup_meta(graph, *input)?;
            let out_t = lookup_meta(graph, *output)?;
            let in_buf = lookup_buffer(buffers, graph, in_t.id)?.to_vec();
            let n = in_t.shape.numel().unwrap_or(0) as usize;
            let out_bytes = eval::cast(&in_buf, in_t.dtype, *dtype, n);
            let out_residency = lookup_residency(graph, out_t.id)?;
            buffers.write(out_residency.buffer, 0, &out_bytes)?;
            Ok(())
        }

        // ──── Const ────
        Op::Const {
            constant,
            output: _,
        } => {
            // Already uploaded above when we initialised constants.
            // Sanity-check the index.
            if (*constant as usize) >= graph.constants.len() {
                return Err(format!("constant index {} out of range", constant));
            }
            Ok(())
        }
    }
}

// ──────────────────────────── helpers ────────────────────────────

/// Snapshot of a tensor's metadata.  Returned by [`lookup_meta`] so
/// callers don't need to navigate the placed graph or handle lifetimes.
struct Meta {
    id: TensorId,
    dtype: DType,
    shape: Shape,
}

fn lookup_meta(graph: &ComputeGraph, id: TensorId) -> Result<Meta, String> {
    let pt = graph
        .tensor(id)
        .ok_or_else(|| format!("tensor {} not in graph", id.0))?;
    Ok(Meta {
        id: pt.id,
        dtype: pt.dtype,
        shape: pt.shape.clone(),
    })
}

fn lookup_residency(graph: &ComputeGraph, id: TensorId) -> Result<Residency, String> {
    let pt = graph
        .tensor(id)
        .ok_or_else(|| format!("tensor {} not in graph", id.0))?;
    Ok(pt.residency)
}

fn lookup_buffer<'b>(
    buffers: &'b BufferStore,
    graph: &ComputeGraph,
    id: TensorId,
) -> Result<&'b [u8], String> {
    let r = lookup_residency(graph, id)?;
    buffers.get(r.buffer)
}

fn unary_op(
    buffers: &mut BufferStore,
    graph: &ComputeGraph,
    input: TensorId,
    output: TensorId,
    f: impl FnOnce(DType, &[u8], &mut [u8], usize),
) -> Result<(), String> {
    let in_t = lookup_meta(graph, input)?;
    let out_t = lookup_meta(graph, output)?;
    let n = in_t.shape.numel().unwrap_or(0) as usize;
    let elem = in_t.dtype.size_bytes();
    let in_buf = lookup_buffer(buffers, graph, in_t.id)?.to_vec();
    let mut out_bytes = vec![0u8; n * elem];
    f(in_t.dtype, &in_buf, &mut out_bytes, n);
    let out_residency = lookup_residency(graph, out_t.id)?;
    buffers.write(out_residency.buffer, 0, &out_bytes)?;
    Ok(())
}

fn bin_op(
    buffers: &mut BufferStore,
    graph: &ComputeGraph,
    lhs: TensorId,
    rhs: TensorId,
    output: TensorId,
    f: impl FnOnce(DType, &[u8], &[u8], &mut [u8], usize),
) -> Result<(), String> {
    let l_t = lookup_meta(graph, lhs)?;
    let r_t = lookup_meta(graph, rhs)?;
    let out_t = lookup_meta(graph, output)?;
    let n = l_t.shape.numel().unwrap_or(0) as usize;
    let elem = l_t.dtype.size_bytes();
    let l_buf = lookup_buffer(buffers, graph, l_t.id)?.to_vec();
    let r_buf = lookup_buffer(buffers, graph, r_t.id)?.to_vec();
    let mut out_bytes = vec![0u8; n * elem];
    f(l_t.dtype, &l_buf, &r_buf, &mut out_bytes, n);
    let out_residency = lookup_residency(graph, out_t.id)?;
    buffers.write(out_residency.buffer, 0, &out_bytes)?;
    Ok(())
}

fn compare_op(
    buffers: &mut BufferStore,
    graph: &ComputeGraph,
    lhs: TensorId,
    rhs: TensorId,
    output: TensorId,
    f: impl Fn(f64, f64) -> bool,
) -> Result<(), String> {
    let l_t = lookup_meta(graph, lhs)?;
    let r_t = lookup_meta(graph, rhs)?;
    let out_t = lookup_meta(graph, output)?;
    let n = l_t.shape.numel().unwrap_or(0) as usize;
    let l_buf = lookup_buffer(buffers, graph, l_t.id)?.to_vec();
    let r_buf = lookup_buffer(buffers, graph, r_t.id)?.to_vec();
    let mut out_bytes = vec![0u8; n];
    match l_t.dtype {
        DType::F32 => {
            let xs = eval::read_f32_vec(&l_buf, n);
            let ys = eval::read_f32_vec(&r_buf, n);
            for i in 0..n {
                out_bytes[i] = if f(xs[i] as f64, ys[i] as f64) { 1 } else { 0 };
            }
        }
        DType::I32 => {
            let xs = eval::read_i32_vec(&l_buf, n);
            let ys = eval::read_i32_vec(&r_buf, n);
            for i in 0..n {
                out_bytes[i] = if f(xs[i] as f64, ys[i] as f64) { 1 } else { 0 };
            }
        }
        DType::U8 => {
            for i in 0..n {
                out_bytes[i] = if f(l_buf[i] as f64, r_buf[i] as f64) { 1 } else { 0 };
            }
        }
    }
    let _ = out_t;
    let out_residency = lookup_residency(graph, output)?;
    buffers.write(out_residency.buffer, 0, &out_bytes)?;
    Ok(())
}

fn where_op(
    buffers: &mut BufferStore,
    graph: &ComputeGraph,
    predicate: TensorId,
    true_value: TensorId,
    false_value: TensorId,
    output: TensorId,
) -> Result<(), String> {
    let p_t = lookup_meta(graph, predicate)?;
    let t_t = lookup_meta(graph, true_value)?;
    let f_t = lookup_meta(graph, false_value)?;
    let n = p_t.shape.numel().unwrap_or(0) as usize;
    let p_buf = lookup_buffer(buffers, graph, p_t.id)?.to_vec();
    let t_buf = lookup_buffer(buffers, graph, t_t.id)?.to_vec();
    let f_buf = lookup_buffer(buffers, graph, f_t.id)?.to_vec();
    let elem = t_t.dtype.size_bytes();
    let mut out_bytes = vec![0u8; n * elem];
    for i in 0..n {
        let src = if p_buf[i] != 0 { &t_buf } else { &f_buf };
        out_bytes[i * elem..(i + 1) * elem].copy_from_slice(&src[i * elem..(i + 1) * elem]);
    }
    let out_residency = lookup_residency(graph, output)?;
    buffers.write(out_residency.buffer, 0, &out_bytes)?;
    Ok(())
}

#[derive(Copy, Clone)]
enum ReduceKind {
    Sum,
    Max,
    Mean,
}

fn reduce_op(
    buffers: &mut BufferStore,
    graph: &ComputeGraph,
    input: TensorId,
    axes: &[u32],
    keep_dims: bool,
    output: TensorId,
    kind: ReduceKind,
) -> Result<(), String> {
    let in_t = lookup_meta(graph, input)?;
    let in_buf = lookup_buffer(buffers, graph, in_t.id)?.to_vec();
    let in_dims = &in_t.shape.dims;

    // Number of elements being reduced into each output element (for Mean).
    let reduced_count: usize = if axes.is_empty() {
        eval::numel(in_dims)
    } else {
        axes.iter().map(|&a| in_dims[a as usize] as usize).product()
    };

    let (out_bytes, _out_dims) = match in_t.dtype {
        DType::F32 => {
            let (init, fold): (f32, fn(f32, f32) -> f32) = match kind {
                ReduceKind::Sum | ReduceKind::Mean => (0.0, |a, b| a + b),
                ReduceKind::Max => (f32::NEG_INFINITY, |a, b| a.max(b)),
            };
            let (mut bytes, dims) = eval::reduce_f32(&in_buf, in_dims, axes, keep_dims, init, fold);
            if matches!(kind, ReduceKind::Mean) && reduced_count > 0 {
                let n_out = bytes.len() / 4;
                let mut vals = eval::read_f32_vec(&bytes, n_out);
                for v in vals.iter_mut() {
                    *v /= reduced_count as f32;
                }
                eval::write_f32_vec(&mut bytes, &vals);
            }
            (bytes, dims)
        }
        DType::I32 => {
            let (init, fold): (i32, fn(i32, i32) -> i32) = match kind {
                ReduceKind::Sum | ReduceKind::Mean => (0, |a, b| a.wrapping_add(b)),
                ReduceKind::Max => (i32::MIN, |a, b| a.max(b)),
            };
            let (mut bytes, dims) = eval::reduce_i32(&in_buf, in_dims, axes, keep_dims, init, fold);
            if matches!(kind, ReduceKind::Mean) && reduced_count > 0 {
                let n_out = bytes.len() / 4;
                let mut vals = eval::read_i32_vec(&bytes, n_out);
                for v in vals.iter_mut() {
                    *v /= reduced_count as i32;
                }
                eval::write_i32_vec(&mut bytes, &vals);
            }
            (bytes, dims)
        }
        DType::U8 => {
            let (init, fold): (u8, fn(u8, u8) -> u8) = match kind {
                ReduceKind::Sum | ReduceKind::Mean => (0, |a, b| a.wrapping_add(b)),
                ReduceKind::Max => (0, |a, b| a.max(b)),
            };
            let (mut bytes, dims) = eval::reduce_u8(&in_buf, in_dims, axes, keep_dims, init, fold);
            if matches!(kind, ReduceKind::Mean) && reduced_count > 0 {
                for v in bytes.iter_mut() {
                    *v /= reduced_count as u8;
                }
            }
            (bytes, dims)
        }
    };

    let _ = output;
    let out_residency = lookup_residency(graph, output)?;
    if !buffers.contains(out_residency.buffer) {
        buffers.alloc(out_residency.buffer, out_bytes.len());
    }
    buffers.write(out_residency.buffer, 0, &out_bytes)?;
    Ok(())
}
