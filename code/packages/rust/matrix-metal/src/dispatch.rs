//! Dispatch a `ComputeGraph` on Metal.
//!
//! Mirrors matrix-cpu's `dispatch::run`: walks `graph.ops` in order,
//! handles Alloc / Free / Transfer / Compute, and writes results to
//! the planner-assigned buffer IDs so downloads can find them.
//!
//! Compute ops route to MSL kernels via the pipeline cache.  Ops that
//! aren't supported on Metal V1 (anything other than F32 elementwise
//! and F32 matmul, plus Const) return Err — but in practice the
//! planner's capability filter prevents routing them here.

#![cfg(target_vendor = "apple")]

use crate::buffers::BufferStore;
use compute_ir::{ComputeGraph, PlacedOp, Residency, CPU_EXECUTOR};
use executor_protocol::OpTiming;
use matrix_ir::{Op, TensorId};
use metal_compute::{MetalCommandQueue, MetalComputePipeline, MetalDevice};
use std::collections::HashMap;

pub struct DispatchCtx<'a> {
    pub device: &'a MetalDevice,
    pub queue: &'a MetalCommandQueue,
    pub buffers: &'a mut BufferStore,
    pub pipelines: &'a HashMap<String, MetalComputePipeline>,
    pub our_id: compute_ir::ExecutorId,
}

pub fn run(ctx: &mut DispatchCtx<'_>, graph: &ComputeGraph) -> Result<Vec<OpTiming>, String> {
    // ──── Up-front validation pass (mirrors matrix-cpu) ────
    const MAX_TENSOR_BYTES: u64 = 16 * 1024 * 1024;
    for t in &graph.tensors {
        let bytes = t
            .shape
            .byte_size(t.dtype)
            .ok_or_else(|| format!("tensor {} byte_size overflows u64", t.id.0))?;
        if bytes > MAX_TENSOR_BYTES {
            return Err(format!(
                "tensor {} requires {} bytes, exceeds the {}-byte limit",
                t.id.0, bytes, MAX_TENSOR_BYTES
            ));
        }
    }

    // ──── Pre-upload constants ────
    let mut residency: HashMap<TensorId, Residency> = HashMap::new();
    for inp in &graph.inputs {
        residency.insert(inp.id, inp.residency);
    }
    for c in &graph.constants {
        residency.insert(c.tensor, c.residency);
        if !ctx.buffers.contains(c.residency.buffer) {
            ctx.buffers
                .alloc(ctx.device, c.residency.buffer, c.bytes.len())?;
        }
        ctx.buffers.write(c.residency.buffer, 0, &c.bytes)?;
    }

    let mut timings = Vec::new();

    for (op_idx, pop) in graph.ops.iter().enumerate() {
        match pop {
            PlacedOp::Alloc { residency: r, bytes } => {
                ctx.buffers.alloc(ctx.device, r.buffer, *bytes as usize)?;
            }
            PlacedOp::Free { residency: r } => {
                ctx.buffers.free(r.buffer);
            }
            PlacedOp::Transfer {
                tensor,
                src,
                dst,
                bytes,
                ..
            } => {
                let data = ctx.buffers.read(src.buffer, 0, *bytes as usize)?;
                if !ctx.buffers.contains(dst.buffer) {
                    ctx.buffers
                        .alloc(ctx.device, dst.buffer, *bytes as usize)?;
                }
                ctx.buffers.write(dst.buffer, 0, &data)?;
                residency.insert(*tensor, *dst);
            }
            PlacedOp::Compute {
                op,
                executor,
                timing: _,
            } => {
                if *executor == CPU_EXECUTOR {
                    return Err(format!(
                        "op {} routed to CPU but reached MetalExecutor",
                        op_idx
                    ));
                }
                // V1 ran a stricter `*executor != ctx.our_id` check
                // here, but the runtime never actually calls
                // `MetalExecutor::set_our_id`, so `our_id` stays at
                // `u32::MAX` and every dispatch failed once a runtime
                // *with* multiple executors started routing real work
                // to us.  In the V1 single-transport-per-executor
                // world we already know we're the executor that was
                // dispatched to (the transport made the call), so the
                // weaker "anything but CPU" check is sufficient.
                // V2 work: have the runtime push the assigned id into
                // each executor at registration time so this check
                // can come back as a real one.
                let _ = ctx.our_id;
                exec_compute(ctx, graph, op)?;
                timings.push(OpTiming {
                    op_index: op_idx as u32,
                    ns: 0,
                });
            }
        }
    }

    Ok(timings)
}

fn exec_compute(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    op: &Op,
) -> Result<(), String> {
    match op {
        Op::Const { constant, output } => {
            let c = graph
                .constants
                .get(*constant as usize)
                .ok_or_else(|| format!("constant index {} out of range", constant))?;
            let out_residency = lookup_residency(graph, *output)?;
            if !ctx.buffers.contains(out_residency.buffer) {
                ctx.buffers
                    .alloc(ctx.device, out_residency.buffer, c.bytes.len())?;
            }
            ctx.buffers.write(out_residency.buffer, 0, &c.bytes)?;
            Ok(())
        }

        // F32 elementwise unary
        Op::Neg { input, output } => unary_dispatch(ctx, graph, "neg_f32", *input, *output),
        Op::Abs { input, output } => unary_dispatch(ctx, graph, "abs_f32", *input, *output),
        Op::Sqrt { input, output } => unary_dispatch(ctx, graph, "sqrt_f32", *input, *output),
        Op::Exp { input, output } => unary_dispatch(ctx, graph, "exp_f32", *input, *output),
        Op::Log { input, output } => unary_dispatch(ctx, graph, "log_f32", *input, *output),
        Op::Tanh { input, output } => unary_dispatch(ctx, graph, "tanh_f32", *input, *output),
        Op::Recip { input, output } => unary_dispatch(ctx, graph, "recip_f32", *input, *output),

        // F32 elementwise binary
        Op::Add { lhs, rhs, output } => binary_dispatch(ctx, graph, "add_f32", *lhs, *rhs, *output),
        Op::Sub { lhs, rhs, output } => binary_dispatch(ctx, graph, "sub_f32", *lhs, *rhs, *output),
        Op::Mul { lhs, rhs, output } => binary_dispatch(ctx, graph, "mul_f32", *lhs, *rhs, *output),
        Op::Div { lhs, rhs, output } => binary_dispatch(ctx, graph, "div_f32", *lhs, *rhs, *output),
        Op::Max { lhs, rhs, output } => binary_dispatch(ctx, graph, "max_f32", *lhs, *rhs, *output),
        Op::Min { lhs, rhs, output } => binary_dispatch(ctx, graph, "min_f32", *lhs, *rhs, *output),
        Op::Pow { lhs, rhs, output } => binary_dispatch(ctx, graph, "pow_f32", *lhs, *rhs, *output),

        // F32 matmul
        Op::MatMul { a, b, output } => matmul_dispatch(ctx, graph, *a, *b, *output),

        // Reshape: metadata-only in SSA → same-size memcpy from input
        // buffer to output buffer.  No kernel needed.  Apple Silicon's
        // unified memory makes this essentially `memcpy`.
        Op::Reshape { input, output, .. } => reshape_dispatch(ctx, graph, *input, *output),

        // Transpose: general N-D permutation up to rank 4.  Single
        // MSL kernel walks the output linearly and reconstructs the
        // input multi-index by reversing the permutation.
        Op::Transpose {
            input,
            perm,
            output,
        } => transpose_dispatch(ctx, graph, *input, perm, *output),

        // Broadcast: replicate input across size-1 axes to match the
        // declared target shape.  Single MSL kernel reads from the
        // input by clamping size-1 axes to index 0.
        Op::Broadcast {
            input,
            output,
            ..
        } => broadcast_dispatch(ctx, graph, *input, *output),

        // Cast: dtype conversion.  matrix-metal advertises only F32
        // as a supported output dtype, so the planner only routes
        // Casts whose output is F32 to us — three input paths to
        // handle: U8→F32, I32→F32, F32→F32.
        Op::Cast {
            input,
            dtype,
            output,
        } => cast_dispatch(ctx, graph, *input, *dtype, *output),

        // Reductions: V1 supports single-axis only.  Multi-axis errors
        // at dispatch time and the runtime falls back to CPU.
        Op::ReduceSum {
            input,
            axes,
            keep_dims,
            output,
        } => reduce_dispatch(ctx, graph, "reduce_sum_f32", *input, axes, *keep_dims, *output),
        Op::ReduceMax {
            input,
            axes,
            keep_dims,
            output,
        } => reduce_dispatch(ctx, graph, "reduce_max_f32", *input, axes, *keep_dims, *output),
        Op::ReduceMean {
            input,
            axes,
            keep_dims,
            output,
        } => reduce_dispatch(ctx, graph, "reduce_mean_f32", *input, axes, *keep_dims, *output),

        _ => Err(format!(
            "matrix-metal V1 doesn't support op {}; the planner should have routed this to CPU",
            op_kind_name(op)
        )),
    }
}

fn unary_dispatch(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    kernel_name: &str,
    input: TensorId,
    output: TensorId,
) -> Result<(), String> {
    let in_t = graph
        .tensor(input)
        .ok_or_else(|| format!("input tensor {} not found", input.0))?;
    let in_residency = in_t.residency;
    let out_residency = lookup_residency(graph, output)?;
    let n = in_t
        .shape
        .numel()
        .ok_or_else(|| format!("input tensor {} numel overflow", input.0))?
        as u32;
    if n == 0 {
        return Ok(());
    }
    let pipeline = ctx
        .pipelines
        .get(kernel_name)
        .ok_or_else(|| format!("pipeline {} not in cache", kernel_name))?;

    let n_bytes = n.to_le_bytes();
    let in_buf = ctx.buffers.get(in_residency.buffer)?;
    let out_buf_ptr = ctx.buffers.get(out_residency.buffer)? as *const _;
    // SAFETY: we hold &BufferStore via ctx.buffers, so both buffers
    // live for the dispatch call.  Using a raw-pointer aliasing to
    // pass two buffer refs into the closure (HashMap::get returns refs
    // tied to the HashMap, so we can have two immutable refs alive).
    //
    // Cleaner alternative: introduce a `BufferStore::get_two(id1, id2)`
    // helper that returns a tuple — V2 polish.
    let out_buf = unsafe { &*out_buf_ptr };
    let tg = pipeline.preferred_threads_1d();

    ctx.queue.dispatch(|enc| {
        enc.set_pipeline(pipeline);
        enc.set_buffer(in_buf, 0);
        enc.set_buffer(out_buf, 1);
        enc.set_bytes(&n_bytes, 2);
        enc.dispatch_threads_1d(n, tg);
    });
    Ok(())
}

fn binary_dispatch(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    kernel_name: &str,
    lhs: TensorId,
    rhs: TensorId,
    output: TensorId,
) -> Result<(), String> {
    let lhs_t = graph
        .tensor(lhs)
        .ok_or_else(|| format!("lhs tensor {} not found", lhs.0))?;
    let rhs_t = graph
        .tensor(rhs)
        .ok_or_else(|| format!("rhs tensor {} not found", rhs.0))?;
    let out_residency = lookup_residency(graph, output)?;
    let n = lhs_t
        .shape
        .numel()
        .ok_or_else(|| "lhs numel overflow".to_string())? as u32;
    if n == 0 {
        return Ok(());
    }
    let pipeline = ctx
        .pipelines
        .get(kernel_name)
        .ok_or_else(|| format!("pipeline {} not in cache", kernel_name))?;

    let a_ptr = ctx.buffers.get(lhs_t.residency.buffer)? as *const _;
    let b_ptr = ctx.buffers.get(rhs_t.residency.buffer)? as *const _;
    let out_ptr = ctx.buffers.get(out_residency.buffer)? as *const _;
    // SAFETY: see unary_dispatch.
    let a_buf = unsafe { &*a_ptr };
    let b_buf = unsafe { &*b_ptr };
    let out_buf = unsafe { &*out_ptr };
    let n_bytes = n.to_le_bytes();
    let tg = pipeline.preferred_threads_1d();

    ctx.queue.dispatch(|enc| {
        enc.set_pipeline(pipeline);
        enc.set_buffer(a_buf, 0);
        enc.set_buffer(b_buf, 1);
        enc.set_buffer(out_buf, 2);
        enc.set_bytes(&n_bytes, 3);
        enc.dispatch_threads_1d(n, tg);
    });
    Ok(())
}

fn matmul_dispatch(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    a: TensorId,
    b: TensorId,
    output: TensorId,
) -> Result<(), String> {
    let a_t = graph.tensor(a).ok_or_else(|| format!("a tensor {} not found", a.0))?;
    let b_t = graph.tensor(b).ok_or_else(|| format!("b tensor {} not found", b.0))?;
    let out_residency = lookup_residency(graph, output)?;
    if a_t.shape.rank() != 2 || b_t.shape.rank() != 2 {
        return Err("matmul inputs must be rank 2".to_string());
    }
    let m = a_t.shape.dims[0] as u32;
    let k = a_t.shape.dims[1] as u32;
    let n = b_t.shape.dims[1] as u32;
    if m == 0 || k == 0 || n == 0 {
        return Ok(());
    }

    let pipeline = ctx
        .pipelines
        .get("matmul_f32")
        .ok_or_else(|| "matmul_f32 pipeline not in cache".to_string())?;

    let a_ptr = ctx.buffers.get(a_t.residency.buffer)? as *const _;
    let b_ptr = ctx.buffers.get(b_t.residency.buffer)? as *const _;
    let out_ptr = ctx.buffers.get(out_residency.buffer)? as *const _;
    // SAFETY: see unary_dispatch.
    let a_buf = unsafe { &*a_ptr };
    let b_buf = unsafe { &*b_ptr };
    let out_buf = unsafe { &*out_ptr };

    // dims is uint3 in MSL, padded to 16 bytes.
    let mut dims_bytes = [0u8; 16];
    dims_bytes[0..4].copy_from_slice(&m.to_le_bytes());
    dims_bytes[4..8].copy_from_slice(&k.to_le_bytes());
    dims_bytes[8..12].copy_from_slice(&n.to_le_bytes());

    // Pick a 2D tile size.  matmul_f32 doesn't have a special preferred
    // tile shape, so use 8x8 = 64 threads (a multiple of Apple Silicon's
    // execution width 32 and divides into the maxThreadsPerThreadgroup
    // for almost all hardware).
    let tg_x = 8u32;
    let tg_y = 8u32;

    ctx.queue.dispatch(|enc| {
        enc.set_pipeline(pipeline);
        enc.set_buffer(a_buf, 0);
        enc.set_buffer(b_buf, 1);
        enc.set_buffer(out_buf, 2);
        enc.set_bytes(&dims_bytes, 3);
        enc.dispatch_threads_2d(m, n, tg_x, tg_y);
    });
    Ok(())
}

/// Transpose: launch the generic permutation kernel.  Encodes the
/// rank, output numel, permutation vector, and input/output dims
/// into a 56-byte args struct (rounded up by MSL alignment).
///
/// V1 limit: rank ≤ 4, dtype F32.  These match this backend's
/// advertised `max_tensor_rank` and `supported_dtypes`.  Anything
/// else returns an error — the planner shouldn't route those to us
/// once it sees our profile, but the dispatch defends in depth.
fn transpose_dispatch(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    input: TensorId,
    perm: &[u32],
    output: TensorId,
) -> Result<(), String> {
    let in_t = graph
        .tensor(input)
        .ok_or_else(|| format!("transpose input tensor {} not found", input.0))?;
    let out_t = graph
        .tensor(output)
        .ok_or_else(|| format!("transpose output tensor {} not found", output.0))?;
    let in_residency = in_t.residency;
    let out_residency = out_t.residency;
    let rank = in_t.shape.rank();
    if rank == 0 {
        // Scalar transpose is a no-op (no axes to permute).  Just
        // copy the bytes through.
        let n_bytes = in_t
            .shape
            .byte_size(in_t.dtype)
            .ok_or_else(|| "transpose scalar byte_size overflow".to_string())?;
        if n_bytes == 0 {
            return Ok(());
        }
        let data = ctx.buffers.read(in_residency.buffer, 0, n_bytes as usize)?;
        if !ctx.buffers.contains(out_residency.buffer) {
            ctx.buffers
                .alloc(ctx.device, out_residency.buffer, n_bytes as usize)?;
        }
        ctx.buffers.write(out_residency.buffer, 0, &data)?;
        return Ok(());
    }
    if rank > 4 {
        return Err(format!(
            "matrix-metal transpose: rank {} exceeds the supported maximum of 4",
            rank
        ));
    }
    if perm.len() != rank {
        return Err(format!(
            "transpose perm has length {} but input rank is {}",
            perm.len(),
            rank
        ));
    }

    let numel = out_t
        .shape
        .numel()
        .ok_or_else(|| "transpose output numel overflow".to_string())? as u32;
    if numel == 0 {
        return Ok(());
    }

    let pipeline = ctx
        .pipelines
        .get("transpose_f32")
        .ok_or_else(|| "transpose_f32 pipeline not in cache".to_string())?;

    let in_buf_ptr = ctx.buffers.get(in_residency.buffer)? as *const _;
    let out_buf_ptr = ctx.buffers.get(out_residency.buffer)? as *const _;
    // SAFETY: we hold `&BufferStore` via ctx.buffers; both buffer
    // references live for the dispatch call.  See `unary_dispatch`
    // for the same pattern.
    let in_buf = unsafe { &*in_buf_ptr };
    let out_buf = unsafe { &*out_buf_ptr };

    // Encode TransposeArgs.  Field order matches the MSL struct:
    //   uint rank;
    //   uint numel;
    //   uint perm[4];
    //   uint in_dims[4];
    //   uint out_dims[4];
    // Total: 14 × 4 = 56 bytes.  MSL alignment rounds to 64.
    let mut args = [0u8; 64];
    let mut off = 0usize;
    let put_u32 = |v: u32, args: &mut [u8; 64], off: &mut usize| {
        args[*off..*off + 4].copy_from_slice(&v.to_le_bytes());
        *off += 4;
    };
    put_u32(rank as u32, &mut args, &mut off);
    put_u32(numel, &mut args, &mut off);
    for d in 0..4 {
        let p = if d < rank { perm[d] } else { 0 };
        put_u32(p, &mut args, &mut off);
    }
    for d in 0..4 {
        let v = if d < rank { in_t.shape.dims[d] } else { 0 };
        put_u32(v, &mut args, &mut off);
    }
    for d in 0..4 {
        let v = if d < rank { out_t.shape.dims[d] } else { 0 };
        put_u32(v, &mut args, &mut off);
    }
    let _ = off; // silence unused-mut warning if all branches were taken

    let tg = pipeline.preferred_threads_1d();

    ctx.queue.dispatch(|enc| {
        enc.set_pipeline(pipeline);
        enc.set_buffer(in_buf, 0);
        enc.set_buffer(out_buf, 1);
        enc.set_bytes(&args, 2);
        enc.dispatch_threads_1d(numel, tg);
    });
    Ok(())
}

/// Reshape: SSA produces a fresh output tensor, so we copy the input
/// buffer's bytes into the output buffer.  Same numel; only the shape
/// metadata differs, and shapes are tracked at the graph level (not in
/// the buffer itself), so a byte-level memcpy is the entire
/// Broadcast: launch the generic axis-replication kernel.  Each
/// input axis must equal the corresponding output (target) axis or be
/// 1; the matrix-ir validator enforces this so we don't re-check it
/// here at dispatch time.
///
/// V1 limit: rank ≤ 4, dtype F32 (matches `max_tensor_rank` and the
/// V1 dtype set advertised in this backend's profile).  Out-of-range
/// inputs return an error — defence in depth.
fn broadcast_dispatch(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    input: TensorId,
    output: TensorId,
) -> Result<(), String> {
    let in_t = graph
        .tensor(input)
        .ok_or_else(|| format!("broadcast input tensor {} not found", input.0))?;
    let out_t = graph
        .tensor(output)
        .ok_or_else(|| format!("broadcast output tensor {} not found", output.0))?;
    let in_residency = in_t.residency;
    let out_residency = out_t.residency;
    let rank = out_t.shape.rank();
    if in_t.shape.rank() != rank {
        return Err(format!(
            "broadcast: input rank {} doesn't match output rank {}",
            in_t.shape.rank(),
            rank
        ));
    }
    if rank == 0 {
        // Scalar broadcast → single-element copy.
        let n_bytes = in_t
            .shape
            .byte_size(in_t.dtype)
            .ok_or_else(|| "broadcast scalar byte_size overflow".to_string())?;
        if n_bytes == 0 {
            return Ok(());
        }
        let data = ctx.buffers.read(in_residency.buffer, 0, n_bytes as usize)?;
        if !ctx.buffers.contains(out_residency.buffer) {
            ctx.buffers
                .alloc(ctx.device, out_residency.buffer, n_bytes as usize)?;
        }
        ctx.buffers.write(out_residency.buffer, 0, &data)?;
        return Ok(());
    }
    if rank > 4 {
        return Err(format!(
            "matrix-metal broadcast: rank {} exceeds the supported maximum of 4",
            rank
        ));
    }

    let numel = out_t
        .shape
        .numel()
        .ok_or_else(|| "broadcast output numel overflow".to_string())? as u32;
    if numel == 0 {
        return Ok(());
    }

    let pipeline = ctx
        .pipelines
        .get("broadcast_f32")
        .ok_or_else(|| "broadcast_f32 pipeline not in cache".to_string())?;

    let in_buf_ptr = ctx.buffers.get(in_residency.buffer)? as *const _;
    let out_buf_ptr = ctx.buffers.get(out_residency.buffer)? as *const _;
    // SAFETY: we hold `&BufferStore` via ctx.buffers; both buffer
    // references live for the dispatch call.  Same pattern as
    // `unary_dispatch`.
    let in_buf = unsafe { &*in_buf_ptr };
    let out_buf = unsafe { &*out_buf_ptr };

    // Encode BroadcastArgs.  Field order matches the MSL struct:
    //   uint rank;
    //   uint numel;
    //   uint in_dims[4];
    //   uint out_dims[4];
    // Total: 10 × 4 = 40 bytes.  Round up to 48 (MSL alignment).
    let mut args = [0u8; 48];
    let mut off = 0usize;
    let put_u32 = |v: u32, args: &mut [u8; 48], off: &mut usize| {
        args[*off..*off + 4].copy_from_slice(&v.to_le_bytes());
        *off += 4;
    };
    put_u32(rank as u32, &mut args, &mut off);
    put_u32(numel, &mut args, &mut off);
    for d in 0..4 {
        let v = if d < rank { in_t.shape.dims[d] } else { 0 };
        put_u32(v, &mut args, &mut off);
    }
    for d in 0..4 {
        let v = if d < rank { out_t.shape.dims[d] } else { 0 };
        put_u32(v, &mut args, &mut off);
    }
    let _ = off;

    let tg = pipeline.preferred_threads_1d();

    ctx.queue.dispatch(|enc| {
        enc.set_pipeline(pipeline);
        enc.set_buffer(in_buf, 0);
        enc.set_buffer(out_buf, 1);
        enc.set_bytes(&args, 2);
        enc.dispatch_threads_1d(numel, tg);
    });
    Ok(())
}

/// implementation.
///
/// V2 polish: matrix-metal could expose this as a `BlitCommandEncoder`
/// copy (`MTLBlitCommandEncoder::copyFromBuffer:...:toBuffer:...`) to
/// keep the work entirely on the GPU.  V1 goes through `BufferStore`'s
/// host-side read/write which on Apple Silicon's unified memory is
/// essentially `memcpy` anyway.
fn reshape_dispatch(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    input: TensorId,
    output: TensorId,
) -> Result<(), String> {
    let in_t = graph
        .tensor(input)
        .ok_or_else(|| format!("input tensor {} not found", input.0))?;
    let in_residency = in_t.residency;
    let out_residency = lookup_residency(graph, output)?;
    let n_bytes = in_t
        .shape
        .byte_size(in_t.dtype)
        .ok_or_else(|| format!("reshape input tensor {} byte_size overflow", input.0))?;
    if n_bytes == 0 {
        return Ok(());
    }
    let data = ctx.buffers.read(in_residency.buffer, 0, n_bytes as usize)?;
    if !ctx.buffers.contains(out_residency.buffer) {
        ctx.buffers
            .alloc(ctx.device, out_residency.buffer, n_bytes as usize)?;
    }
    ctx.buffers.write(out_residency.buffer, 0, &data)?;
    Ok(())
}

/// Cast: dispatch a single-element-wise dtype conversion.  Only
/// F32-output paths reach us (the planner's capability filter
/// restricts on output dtype, and matrix-metal advertises F32 only).
/// Three input dtypes are handled: F32, U8, I32.
fn cast_dispatch(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    input: TensorId,
    output_dtype: matrix_ir::DType,
    output: TensorId,
) -> Result<(), String> {
    let in_t = graph
        .tensor(input)
        .ok_or_else(|| format!("cast input tensor {} not found", input.0))?;
    let out_residency = lookup_residency(graph, output)?;
    let n = in_t
        .shape
        .numel()
        .ok_or_else(|| format!("cast input tensor {} numel overflow", input.0))?
        as u32;
    if n == 0 {
        return Ok(());
    }

    // Defence in depth: matrix-metal only emits F32-output kernels;
    // routing a non-F32-output cast to us indicates a planner bug.
    if output_dtype != matrix_ir::DType::F32 {
        return Err(format!(
            "matrix-metal cast: unsupported output dtype {:?} (only F32 ships in V1)",
            output_dtype
        ));
    }

    let kernel_name = match in_t.dtype {
        matrix_ir::DType::F32 => "cast_f32_to_f32",
        matrix_ir::DType::U8 => "cast_u8_to_f32",
        matrix_ir::DType::I32 => "cast_i32_to_f32",
    };
    let pipeline = ctx
        .pipelines
        .get(kernel_name)
        .ok_or_else(|| format!("pipeline {} not in cache", kernel_name))?;

    let in_buf_ptr = ctx.buffers.get(in_t.residency.buffer)? as *const _;
    let out_buf_ptr = ctx.buffers.get(out_residency.buffer)? as *const _;
    // SAFETY: see `unary_dispatch` — same pattern, both buffers live
    // for the duration of the dispatch call via `&BufferStore`.
    let in_buf = unsafe { &*in_buf_ptr };
    let out_buf = unsafe { &*out_buf_ptr };

    let n_bytes = n.to_le_bytes();
    let tg = pipeline.preferred_threads_1d();

    ctx.queue.dispatch(|enc| {
        enc.set_pipeline(pipeline);
        enc.set_buffer(in_buf, 0);
        enc.set_buffer(out_buf, 1);
        enc.set_bytes(&n_bytes, 2);
        enc.dispatch_threads_1d(n, tg);
    });
    Ok(())
}

/// Reduction (sum/max/mean) over a single axis.
///
/// V1 only supports single-axis reductions because the kernels are
/// thread-per-output-element and the args struct hardcodes one
/// `reduce_axis` field.  Multi-axis reductions (`axes.len() > 1`)
/// return Err — the runtime can either:
///
///   - Surface the error to the caller, or
///   - Decompose the multi-axis reduce into a chain of single-axis
///     reductions before dispatching (V2 work).
///
/// The MSL kernel (`reduce_{sum,max,mean}_f32`) does the actual
/// arithmetic; this Rust function packs the args struct, validates
/// inputs, and launches the kernel.
fn reduce_dispatch(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    kernel_name: &str,
    input: TensorId,
    axes: &[u32],
    keep_dims: bool,
    output: TensorId,
) -> Result<(), String> {
    let in_t = graph
        .tensor(input)
        .ok_or_else(|| format!("reduce input tensor {} not found", input.0))?;
    let out_t = graph
        .tensor(output)
        .ok_or_else(|| format!("reduce output tensor {} not found", output.0))?;
    let out_residency = out_t.residency;
    let in_residency = in_t.residency;

    if axes.len() != 1 {
        return Err(format!(
            "matrix-metal {}: V1 supports single-axis reduce only, got {} axes",
            kernel_name,
            axes.len()
        ));
    }
    let reduce_axis = axes[0] as usize;
    let rank_in = in_t.shape.rank();
    if rank_in == 0 || rank_in > 4 {
        return Err(format!(
            "matrix-metal {}: input rank {} out of range (1..=4)",
            kernel_name, rank_in
        ));
    }
    if reduce_axis >= rank_in {
        return Err(format!(
            "matrix-metal {}: axis {} out of range for rank-{} input",
            kernel_name, reduce_axis, rank_in
        ));
    }
    let reduce_size = in_t.shape.dims[reduce_axis];

    let numel_out = out_t
        .shape
        .numel()
        .ok_or_else(|| "reduce output numel overflow".to_string())? as u32;
    if numel_out == 0 {
        return Ok(());
    }

    let pipeline = ctx
        .pipelines
        .get(kernel_name)
        .ok_or_else(|| format!("pipeline {} not in cache", kernel_name))?;

    let in_buf_ptr = ctx.buffers.get(in_residency.buffer)? as *const _;
    let out_buf_ptr = ctx.buffers.get(out_residency.buffer)? as *const _;
    // SAFETY: see `unary_dispatch`.
    let in_buf = unsafe { &*in_buf_ptr };
    let out_buf = unsafe { &*out_buf_ptr };

    // Encode ReduceArgs.  Field order matches the MSL struct:
    //   uint rank_in;
    //   uint reduce_axis;
    //   uint reduce_size;
    //   uint keep_dims;
    //   uint numel_out;
    //   uint in_dims[4];
    //   uint out_dims[4];
    // Total: 5 + 4 + 4 = 13 uints = 52 bytes.  Round up to 64 for
    // MSL alignment.
    let mut args = [0u8; 64];
    let mut off = 0usize;
    let put_u32 = |v: u32, args: &mut [u8; 64], off: &mut usize| {
        args[*off..*off + 4].copy_from_slice(&v.to_le_bytes());
        *off += 4;
    };
    put_u32(rank_in as u32, &mut args, &mut off);
    put_u32(reduce_axis as u32, &mut args, &mut off);
    put_u32(reduce_size, &mut args, &mut off);
    put_u32(if keep_dims { 1 } else { 0 }, &mut args, &mut off);
    put_u32(numel_out, &mut args, &mut off);
    for d in 0..4 {
        let v = if d < rank_in { in_t.shape.dims[d] } else { 0 };
        put_u32(v, &mut args, &mut off);
    }
    let rank_out = out_t.shape.rank();
    for d in 0..4 {
        let v = if d < rank_out {
            out_t.shape.dims[d]
        } else {
            0
        };
        put_u32(v, &mut args, &mut off);
    }
    let _ = off;

    let tg = pipeline.preferred_threads_1d();

    ctx.queue.dispatch(|enc| {
        enc.set_pipeline(pipeline);
        enc.set_buffer(in_buf, 0);
        enc.set_buffer(out_buf, 1);
        enc.set_bytes(&args, 2);
        enc.dispatch_threads_1d(numel_out, tg);
    });
    Ok(())
}

fn lookup_residency(graph: &ComputeGraph, id: TensorId) -> Result<Residency, String> {
    graph
        .tensor(id)
        .map(|t| t.residency)
        .ok_or_else(|| format!("tensor {} not in graph", id.0))
}

fn op_kind_name(op: &Op) -> &'static str {
    match op {
        Op::Neg { .. } => "Neg",
        Op::Abs { .. } => "Abs",
        Op::Sqrt { .. } => "Sqrt",
        Op::Exp { .. } => "Exp",
        Op::Log { .. } => "Log",
        Op::Tanh { .. } => "Tanh",
        Op::Recip { .. } => "Recip",
        Op::Add { .. } => "Add",
        Op::Sub { .. } => "Sub",
        Op::Mul { .. } => "Mul",
        Op::Div { .. } => "Div",
        Op::Max { .. } => "Max",
        Op::Min { .. } => "Min",
        Op::Pow { .. } => "Pow",
        Op::ReduceSum { .. } => "ReduceSum",
        Op::ReduceMax { .. } => "ReduceMax",
        Op::ReduceMean { .. } => "ReduceMean",
        Op::Reshape { .. } => "Reshape",
        Op::Transpose { .. } => "Transpose",
        Op::Broadcast { .. } => "Broadcast",
        Op::MatMul { .. } => "MatMul",
        Op::Equal { .. } => "Equal",
        Op::Less { .. } => "Less",
        Op::Greater { .. } => "Greater",
        Op::Where { .. } => "Where",
        Op::Cast { .. } => "Cast",
        Op::Const { .. } => "Const",
    }
}
