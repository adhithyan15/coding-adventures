//! Dispatch a `ComputeGraph` on CUDA.
//!
//! MX06 Phase 5b.  Mirrors `matrix-metal::dispatch::run`: walks
//! `graph.ops` in order, handles `Alloc` / `Free` / `Transfer` /
//! `Compute`, and writes results to the planner-assigned buffer
//! IDs so downloads can find them.
//!
//! Compute ops route to CUDA C kernels via the cached [`Kernels`]
//! module.  Ops outside matrix-cuda's V1 set (anything other than
//! F32 elementwise + F32 matmul, plus `Const`) return `Err` — but
//! the planner's capability filter prevents routing them here once
//! `supported_ops_bitset()` is flipped on.
//!
//! Note: matrix-cuda's V1 scope is narrower than matrix-metal's
//! (no `Reshape`/`Transpose`/`Broadcast`/`Cast`/`Reduce*` yet).
//! Those ops live behind capability bits that matrix-cuda doesn't
//! advertise, so the planner sends them to CPU.  V2 work.

use crate::buffers::BufferStore;
use crate::kernels::Kernels;
use compute_ir::{ComputeGraph, PlacedOp, Residency, CPU_EXECUTOR};
use cuda_compute::CudaDevice;
use executor_protocol::OpTiming;
use matrix_ir::{Op, TensorId};
use std::collections::HashMap;

/// Context carried through the dispatch walk.  Holds references
/// borrowed from `CudaExecutor::State` for the duration of a single
/// `Dispatch` request.
pub struct DispatchCtx<'a> {
    pub device: &'a CudaDevice,
    pub buffers: &'a mut BufferStore,
    pub kernels: &'a Kernels,
    pub our_id: compute_ir::ExecutorId,
}

/// Walk a placed compute graph and execute every op on CUDA.
///
/// Returns per-op timings (currently zeros; Phase 7 will wire real
/// timing).  Errors surface as `String`s — same convention as
/// `matrix-metal::dispatch::run`.
pub fn run(ctx: &mut DispatchCtx<'_>, graph: &ComputeGraph) -> Result<Vec<OpTiming>, String> {
    // ──── Up-front validation pass (mirrors matrix-cpu / matrix-metal) ────
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
        ctx.buffers
            .write(ctx.device, c.residency.buffer, 0, &c.bytes)?;
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
                let data = ctx.buffers.read(ctx.device, src.buffer, 0, *bytes as usize)?;
                if !ctx.buffers.contains(dst.buffer) {
                    ctx.buffers
                        .alloc(ctx.device, dst.buffer, *bytes as usize)?;
                }
                ctx.buffers
                    .write(ctx.device, dst.buffer, 0, &data)?;
                residency.insert(*tensor, *dst);
            }
            PlacedOp::Compute {
                op,
                executor,
                timing: _,
            } => {
                if *executor == CPU_EXECUTOR {
                    return Err(format!(
                        "op {} routed to CPU but reached CudaExecutor",
                        op_idx
                    ));
                }
                // matrix-metal documents why the stricter "executor
                // == ctx.our_id" check is loosened to "anything but
                // CPU".  Same reasoning applies here: until the
                // runtime pushes a real id at registration time, our
                // own id stays at `u32::MAX` and the strict check
                // would fail for every dispatch.
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
            ctx.buffers
                .write(ctx.device, out_residency.buffer, 0, &c.bytes)?;
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

        // Anything else: planner shouldn't route it to us in V1,
        // but if it does we error explicitly so the issue is
        // visible.  V2 will add Reshape/Transpose/Broadcast/Cast/
        // reductions, matching matrix-metal's coverage.
        other => Err(format!(
            "matrix-cuda V1: op {:?} not supported; planner should route to CPU",
            std::mem::discriminant(other)
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
    let out_residency = lookup_residency(graph, output)?;
    let n = in_t
        .shape
        .numel()
        .ok_or_else(|| format!("input tensor {} numel overflow", input.0))?
        as u32;
    if n == 0 {
        return Ok(());
    }

    // Pull device pointers out of the buffer store *before* the
    // kernel launch.  CUdeviceptr is a u64 — copies are cheap and
    // dodge the "two refs into one HashMap" problem the &CudaBuffer
    // approach hits.
    let in_ptr = ctx.buffers.get(in_t.residency.buffer)?.device_ptr();
    let out_ptr = ctx.buffers.get(out_residency.buffer)?.device_ptr();

    ctx.kernels
        .launch_unary_by_ptr(ctx.device, kernel_name, in_ptr, out_ptr, n)
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

    let a_ptr = ctx.buffers.get(lhs_t.residency.buffer)?.device_ptr();
    let b_ptr = ctx.buffers.get(rhs_t.residency.buffer)?.device_ptr();
    let out_ptr = ctx.buffers.get(out_residency.buffer)?.device_ptr();

    ctx.kernels
        .launch_binary_by_ptr(ctx.device, kernel_name, a_ptr, b_ptr, out_ptr, n)
}

fn matmul_dispatch(
    ctx: &mut DispatchCtx<'_>,
    graph: &ComputeGraph,
    a: TensorId,
    b: TensorId,
    output: TensorId,
) -> Result<(), String> {
    let a_t = graph
        .tensor(a)
        .ok_or_else(|| format!("a tensor {} not found", a.0))?;
    let b_t = graph
        .tensor(b)
        .ok_or_else(|| format!("b tensor {} not found", b.0))?;
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

    let a_ptr = ctx.buffers.get(a_t.residency.buffer)?.device_ptr();
    let b_ptr = ctx.buffers.get(b_t.residency.buffer)?.device_ptr();
    let c_ptr = ctx.buffers.get(out_residency.buffer)?.device_ptr();

    ctx.kernels
        .launch_matmul_by_ptr(ctx.device, a_ptr, b_ptr, c_ptr, m, k, n)
}

/// Look up the residency of `id` in `graph.tensors`.  Returns an
/// error if the tensor isn't found.  Mirrors
/// `matrix-metal::dispatch::lookup_residency`.
fn lookup_residency(graph: &ComputeGraph, id: TensorId) -> Result<Residency, String> {
    graph
        .tensor(id)
        .map(|t| t.residency)
        .ok_or_else(|| format!("tensor {} not found in graph", id.0))
}
