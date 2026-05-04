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
                if *executor != ctx.our_id {
                    return Err(format!(
                        "op {} routed to executor {} but reached MetalExecutor (id {})",
                        op_idx, executor.0, ctx.our_id.0
                    ));
                }
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
