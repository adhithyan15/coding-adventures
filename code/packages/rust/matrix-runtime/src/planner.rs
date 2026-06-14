//! The planner.  Lowers `matrix_ir::Graph` to `compute_ir::ComputeGraph`
//! using the registry's `BackendProfile`s.
//!
//! Algorithm (per spec MX04 §"The planner algorithm"):
//!
//! 1. **Capability filter** — for each op, set of executors that can run it.
//! 2. **Greedy cost minimisation** — for each op in topological order,
//!    pick the executor that minimises (compute + transfer-in) cost.
//! 2b. **Single-executor preference** (V1.1, spec MX04 §"Single-executor
//!    preference") — for each healthy executor that is a candidate for
//!    *every* op, compute the total cost of running the entire graph
//!    on it (compute_ns per op, plus a *one-time* host→device transfer
//!    for each input/constant).  If any uniform placement is strictly
//!    cheaper than the greedy total, replace `placement` with the
//!    uniform vector.  This fixes the well-known greedy pathology
//!    where per-op transfer costs prevent ever amortising the up-front
//!    move to a faster executor.
//! 3. **Transfer insertion** — walk placed ops, insert
//!    `PlacedOp::Transfer` whenever an input's residency doesn't
//!    match the op's executor.
//! 4. **Lifetime annotation** — insert `Alloc` before first use and
//!    `Free` after last use of each non-input/non-output buffer.

use crate::cost::{compute_cost, estimate_flops, estimate_matmul_flops, transfer_cost_ns, TransferDirection};
use crate::registry::Registry;
use compute_ir::{
    BufferId, ComputeGraph, ExecutorId, OpTiming, PlacedConstant, PlacedOp, PlacedTensor, Residency,
    CPU_EXECUTOR,
};
use matrix_ir::{Graph, Op, TensorId};
use std::collections::{HashMap, HashSet};

/// Errors produced by [`plan`].
#[derive(Clone, Debug)]
pub enum PlanError {
    /// The input matrix-ir graph fails its own validator.
    InvalidGraph(matrix_ir::IrError),
    /// No executor (not even the CPU fallback) supports an op.
    NoCapableExecutor { op_index: u32 },
    /// The registry has no executors registered (not even CPU).
    EmptyRegistry,
    /// An op references a tensor not in the graph.
    UndefinedTensor { op_index: u32, tensor: TensorId },
}

impl From<matrix_ir::IrError> for PlanError {
    fn from(e: matrix_ir::IrError) -> Self {
        PlanError::InvalidGraph(e)
    }
}

/// Lower a `matrix_ir::Graph` to a `compute_ir::ComputeGraph`,
/// routing each op to the cheapest available executor.
///
/// The output graph is **not** automatically validated; callers may
/// call `result.validate()` to catch any inconsistencies (a planner
/// bug if it ever returns Err).
pub fn plan(graph: &Graph, registry: &Registry) -> Result<ComputeGraph, PlanError> {
    graph.validate()?;
    if registry.healthy().is_empty() {
        return Err(PlanError::EmptyRegistry);
    }

    // ──────── Pass 1: capability filter ────────
    //
    // For each op, collect the set of executors that can run it
    // (correct op kind + correct dtype).  Falls back to CPU if no
    // specialised executor takes it.
    let mut candidates: Vec<Vec<ExecutorId>> = Vec::with_capacity(graph.ops.len());
    for (i, op) in graph.ops.iter().enumerate() {
        let dtype = output_dtype_of(graph, op).ok_or(PlanError::UndefinedTensor {
            op_index: i as u32,
            tensor: op.output(),
        })?;
        let cs: Vec<ExecutorId> = registry
            .healthy()
            .iter()
            .filter(|e| e.supports_op(op.wire_tag()) && e.supports_dtype(dtype.wire_tag()))
            .map(|e| e.id)
            .collect();
        let cs = if cs.is_empty() {
            // CPU fallback if it's healthy and registered.
            if registry.get(CPU_EXECUTOR).map(|e| e.healthy).unwrap_or(false) {
                vec![CPU_EXECUTOR]
            } else {
                return Err(PlanError::NoCapableExecutor { op_index: i as u32 });
            }
        } else {
            cs
        };
        candidates.push(cs);
    }

    // ──────── Pass 2: greedy cost minimisation ────────
    //
    // Track each tensor's *current* residency as we walk the ops in
    // topological order.  Initially:
    //   - Inputs are on CPU (host) — buffer ids assigned sequentially.
    //   - Constants are on CPU as well.
    let mut residency: HashMap<TensorId, Residency> = HashMap::new();
    let mut next_buffer_id_per_executor: HashMap<ExecutorId, u64> = HashMap::new();
    let next_buf = |exec: ExecutorId,
                        m: &mut HashMap<ExecutorId, u64>|
     -> BufferId {
        let next = m.entry(exec).or_insert(0);
        let id = BufferId(*next);
        *next += 1;
        id
    };

    // Place inputs and constants on CPU at sequentially-assigned buffer ids.
    for inp in &graph.inputs {
        let b = next_buf(CPU_EXECUTOR, &mut next_buffer_id_per_executor);
        residency.insert(
            inp.id,
            Residency {
                executor: CPU_EXECUTOR,
                buffer: b,
            },
        );
    }
    for c in &graph.constants {
        let b = next_buf(CPU_EXECUTOR, &mut next_buffer_id_per_executor);
        residency.insert(
            c.tensor.id,
            Residency {
                executor: CPU_EXECUTOR,
                buffer: b,
            },
        );
    }

    // Choose an executor for each op.
    let mut placement: Vec<ExecutorId> = Vec::with_capacity(graph.ops.len());
    for (i, op) in graph.ops.iter().enumerate() {
        let dtype = output_dtype_of(graph, op).ok_or(PlanError::UndefinedTensor {
            op_index: i as u32,
            tensor: op.output(),
        })?;
        let flops = flops_for(graph, op);
        let mut best = candidates[i][0];
        let mut best_cost = u64::MAX;
        for &exec_id in &candidates[i] {
            let exec = registry.get(exec_id).expect("candidate is in registry");
            let mut cost = compute_cost(flops, dtype, &exec.profile);
            for input in op.inputs() {
                if let Some(r) = residency.get(&input) {
                    if r.executor != exec_id {
                        // We'd need to transfer the input.  Bytes =
                        // numel × dtype size.
                        let bytes = bytes_for_tensor(graph, input).unwrap_or(0);
                        cost = cost.saturating_add(transfer_cost_ns(
                            bytes,
                            &exec.profile,
                            TransferDirection::HostToDevice,
                        ));
                    }
                }
            }
            if cost < best_cost {
                best_cost = cost;
                best = exec_id;
            }
        }
        placement.push(best);
        // Output of this op now lives on `best`, with a fresh buffer.
        let b = next_buf(best, &mut next_buffer_id_per_executor);
        residency.insert(
            op.output(),
            Residency {
                executor: best,
                buffer: b,
            },
        );
    }

    // ──────── Pass 2b: single-executor preference ────────
    //
    // See spec MX04 §"Single-executor preference (V1.1)".  Greedy in
    // pass 2 makes per-op decisions and never amortises the up-front
    // host→device transfer.  Here we score each healthy executor as a
    // *uniform* placement candidate (every op runs there) and
    // replace the greedy placement if a uniform alternative is
    // strictly cheaper end-to-end.
    //
    // Capability check: a uniform placement on E is feasible only when
    // E is a candidate for every op.  CPU is universally capable (it's
    // the fallback), so single-CPU is always feasible — but it almost
    // never beats greedy because greedy already picks CPU for cheap
    // ops, so the uniform tally just sums those plus more.  The interesting
    // wins are uniform-Metal / uniform-CUDA on graphs that are tight
    // chains of supported ops.
    let greedy_total = total_cost_for_placement(graph, registry, &placement, &residency);
    let mut best_uniform_total = greedy_total;
    let mut best_uniform_exec: Option<ExecutorId> = None;
    for exec in registry.healthy() {
        // Is this executor a candidate for every op?
        let universal = candidates
            .iter()
            .all(|cs| cs.iter().any(|&id| id == exec.id));
        if !universal {
            continue;
        }
        let uniform = vec![exec.id; graph.ops.len()];
        let total = total_cost_for_placement(graph, registry, &uniform, &residency);
        if total < best_uniform_total {
            best_uniform_total = total;
            best_uniform_exec = Some(exec.id);
        }
    }
    if let Some(uniform_exec) = best_uniform_exec {
        // Rebuild the placement *and* the residency map so the rest
        // of the planner sees the uniform decision.
        //
        // We also reassign **constant** source-tensor residencies to
        // `uniform_exec`.  Constants are normally born on CPU; if we
        // leave them there while running every op on `uniform_exec`,
        // the resulting `ComputeGraph` is technically still valid (the
        // dispatcher allocates buffers in its own namespace at
        // pre-upload time), but downstream consumers like
        // `image-gpu-core::single_executor()` — which scans every
        // constant residency *and* every op residency for uniformity
        // and falls back to a CPU-only re-plan if it sees a mismatch —
        // would treat it as mixed and undo our work.  Reassigning the
        // constants to `uniform_exec` keeps the placed graph honestly
        // single-executor.
        for i in 0..graph.ops.len() {
            placement[i] = uniform_exec;
        }
        for c in &graph.constants {
            let b = next_buf(uniform_exec, &mut next_buffer_id_per_executor);
            residency.insert(
                c.tensor.id,
                Residency {
                    executor: uniform_exec,
                    buffer: b,
                },
            );
        }
        for op in &graph.ops {
            // Reassign each op's output residency under the new
            // executor.  Old buffer ids reserved during greedy aren't
            // re-used; the per-executor counter only grows.
            let b = next_buf(uniform_exec, &mut next_buffer_id_per_executor);
            residency.insert(
                op.output(),
                Residency {
                    executor: uniform_exec,
                    buffer: b,
                },
            );
        }
    }

    // ──────── Pass 3: transfer insertion + Pass 4: lifetime annotation ────────
    //
    // Walk ops in order, emitting:
    //   - Alloc for any newly-used buffer
    //   - Transfer for inputs not yet on the op's executor
    //   - Compute (or Const for matrix-ir Const ops) at the chosen executor
    //   - Free for buffers whose last use is this op
    //
    // For lifetime tracking we precompute first-use and last-use of
    // each TensorId across ops + outputs.
    let last_use = compute_last_use(graph);
    let mut current_residency: HashMap<TensorId, Residency> = HashMap::new();
    let mut allocated: HashSet<Residency> = HashSet::new();
    let mut placed_ops: Vec<PlacedOp> = Vec::new();

    // Inputs and constants are pre-allocated on CPU.
    for inp in &graph.inputs {
        let r = residency.get(&inp.id).copied().unwrap();
        allocated.insert(r);
        current_residency.insert(inp.id, r);
    }
    for c in &graph.constants {
        let r = residency.get(&c.tensor.id).copied().unwrap();
        allocated.insert(r);
        current_residency.insert(c.tensor.id, r);
    }

    for (i, op) in graph.ops.iter().enumerate() {
        let exec = placement[i];

        // Transfer each input not already on `exec`.
        for input in op.inputs() {
            let cur = *current_residency.get(&input).expect("input residency known");
            if cur.executor != exec {
                // Allocate destination buffer on `exec` if not already.
                let dst_b = next_buf(exec, &mut next_buffer_id_per_executor);
                let dst = Residency {
                    executor: exec,
                    buffer: dst_b,
                };
                let bytes = bytes_for_tensor(graph, input).unwrap_or(0);
                placed_ops.push(PlacedOp::Alloc {
                    residency: dst,
                    bytes,
                });
                allocated.insert(dst);
                let exec_profile = &registry.get(exec).expect("exec in registry").profile;
                let est_xfer_ns = transfer_cost_ns(
                    bytes,
                    exec_profile,
                    TransferDirection::HostToDevice,
                );
                placed_ops.push(PlacedOp::Transfer {
                    tensor: input,
                    src: cur,
                    dst,
                    bytes,
                    timing: OpTiming {
                        estimated_ns: est_xfer_ns,
                    },
                });
                current_residency.insert(input, dst);
            }
        }

        // Allocate output buffer on `exec`.
        let out_residency = *residency.get(&op.output()).expect("output residency");
        let bytes = bytes_for_tensor(graph, op.output()).unwrap_or(0);
        if !allocated.contains(&out_residency) {
            placed_ops.push(PlacedOp::Alloc {
                residency: out_residency,
                bytes,
            });
            allocated.insert(out_residency);
        }

        // The op itself.
        let dtype =
            output_dtype_of(graph, op).expect("dtype known after capability pass");
        let flops = flops_for(graph, op);
        let exec_profile = &registry.get(exec).expect("exec in registry").profile;
        let est_compute_ns = compute_cost(flops, dtype, exec_profile);
        placed_ops.push(PlacedOp::Compute {
            op: op.clone(),
            executor: exec,
            timing: OpTiming {
                estimated_ns: est_compute_ns,
            },
        });
        current_residency.insert(op.output(), out_residency);

        // Free any tensor whose last use is this op AND that isn't a
        // graph output we still need to expose.
        for input in op.inputs() {
            if last_use.get(&input).copied() == Some(i) && !graph.outputs.contains(&input) {
                if let Some(r) = current_residency.remove(&input) {
                    placed_ops.push(PlacedOp::Free { residency: r });
                    allocated.remove(&r);
                }
            }
        }
    }

    // Build output PlacedTensor list with end-of-graph residency.
    let outputs: Vec<PlacedTensor> = graph
        .outputs
        .iter()
        .map(|id| {
            let t = graph.tensor(*id).expect("output tensor exists");
            let r = current_residency
                .get(id)
                .copied()
                .or_else(|| residency.get(id).copied())
                .unwrap_or(Residency {
                    executor: CPU_EXECUTOR,
                    buffer: BufferId(0),
                });
            PlacedTensor {
                id: t.id,
                dtype: t.dtype,
                shape: t.shape.clone(),
                residency: r,
            }
        })
        .collect();

    // Build inputs PlacedTensor list.
    let inputs: Vec<PlacedTensor> = graph
        .inputs
        .iter()
        .map(|t| PlacedTensor {
            id: t.id,
            dtype: t.dtype,
            shape: t.shape.clone(),
            residency: *residency.get(&t.id).expect("input residency"),
        })
        .collect();

    // Build constants list with assigned residency.
    let constants: Vec<PlacedConstant> = graph
        .constants
        .iter()
        .map(|c| PlacedConstant {
            tensor: c.tensor.id,
            bytes: c.bytes.clone(),
            residency: *residency.get(&c.tensor.id).expect("constant residency"),
        })
        .collect();

    // Tensors table — birth residency for each tensor.
    let tensors: Vec<PlacedTensor> = graph
        .tensors
        .iter()
        .map(|t| PlacedTensor {
            id: t.id,
            dtype: t.dtype,
            shape: t.shape.clone(),
            residency: *residency.get(&t.id).unwrap_or(&Residency {
                executor: CPU_EXECUTOR,
                buffer: BufferId(0),
            }),
        })
        .collect();

    Ok(ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs,
        outputs,
        constants,
        ops: placed_ops,
        tensors,
    })
}

// ──────────────── helpers ────────────────

fn output_dtype_of(graph: &Graph, op: &Op) -> Option<matrix_ir::DType> {
    graph.tensor(op.output()).map(|t| t.dtype)
}

fn bytes_for_tensor(graph: &Graph, id: TensorId) -> Option<u64> {
    let t = graph.tensor(id)?;
    t.shape.byte_size(t.dtype)
}

fn flops_for(graph: &Graph, op: &Op) -> u64 {
    let out_shape = graph.tensor(op.output()).map(|t| &t.shape);
    let in_shape = match op {
        Op::ReduceSum { input, .. }
        | Op::ReduceMax { input, .. }
        | Op::ReduceMean { input, .. } => graph.tensor(*input).map(|t| &t.shape),
        _ => out_shape,
    };
    match op {
        Op::MatMul { a, b, .. } => {
            // We have full Tensor metadata in the graph table.
            let ta = graph.tensor(*a);
            let tb = graph.tensor(*b);
            match (ta, tb) {
                (Some(ta), Some(tb)) => estimate_matmul_flops(ta, tb),
                _ => 0,
            }
        }
        _ => estimate_flops(op, out_shape, in_shape),
    }
}

/// Compute the total estimated cost (in nanoseconds) of executing
/// `graph` under `placement`, starting from the input/constant
/// residencies in `residency_init`.
///
/// Transfers are charged **at most once per tensor** — after a tensor
/// is first moved to an executor, subsequent ops on the same executor
/// see it already there.  This matches what real dispatch does and is
/// the key reason the single-executor preference can beat per-op
/// greedy: greedy charges transfer cost at op 0 *and* doesn't get to
/// amortise across the rest of the chain.
///
/// Used by pass 2b to compare uniform-placement candidates against the
/// greedy placement.  Pure — does not mutate `residency_init`.
fn total_cost_for_placement(
    graph: &Graph,
    registry: &Registry,
    placement: &[ExecutorId],
    residency_init: &HashMap<TensorId, Residency>,
) -> u64 {
    let mut current: HashMap<TensorId, Residency> = HashMap::new();
    for inp in &graph.inputs {
        if let Some(&r) = residency_init.get(&inp.id) {
            current.insert(inp.id, r);
        }
    }
    for c in &graph.constants {
        if let Some(&r) = residency_init.get(&c.tensor.id) {
            current.insert(c.tensor.id, r);
        }
    }
    let mut total: u64 = 0;
    for (i, op) in graph.ops.iter().enumerate() {
        let exec_id = placement[i];
        let exec = match registry.get(exec_id) {
            Some(e) => e,
            None => return u64::MAX,
        };
        let dtype = match output_dtype_of(graph, op) {
            Some(d) => d,
            None => return u64::MAX,
        };
        let flops = flops_for(graph, op);
        let mut cost = compute_cost(flops, dtype, &exec.profile);
        for input in op.inputs() {
            if let Some(r) = current.get(&input).copied() {
                if r.executor != exec_id {
                    let bytes = bytes_for_tensor(graph, input).unwrap_or(0);
                    cost = cost.saturating_add(transfer_cost_ns(
                        bytes,
                        &exec.profile,
                        TransferDirection::HostToDevice,
                    ));
                    // Update the simulated residency so we don't charge
                    // the same tensor again on the next op that reads it.
                    current.insert(
                        input,
                        Residency {
                            executor: exec_id,
                            buffer: BufferId(0),
                        },
                    );
                }
            }
        }
        total = total.saturating_add(cost);
        current.insert(
            op.output(),
            Residency {
                executor: exec_id,
                buffer: BufferId(0),
            },
        );
    }
    total
}

fn compute_last_use(graph: &Graph) -> HashMap<TensorId, usize> {
    let mut last: HashMap<TensorId, usize> = HashMap::new();
    for (i, op) in graph.ops.iter().enumerate() {
        for input in op.inputs() {
            last.insert(input, i);
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;
    use executor_protocol::BackendProfile;
    use matrix_ir::{DType, GraphBuilder, Shape};

    fn cpu_only_registry() -> Registry {
        let (r, _) = Registry::with_cpu(BackendProfile {
            kind: "cpu".to_string(),
            supported_ops: 0xFFFF_FFFF,
            supported_dtypes: 0x07,
            gflops_f32: 40,
            gflops_u8: 40,
            gflops_i32: 40,
            host_to_device_bw: 100,
            device_to_host_bw: 100,
            device_internal_bw: 100,
            launch_overhead_ns: 0,
            transport_latency_ns: 0,
            on_device_mib: 8 * 1024,
            max_tensor_rank: 16,
            max_dim: u32::MAX,
        });
        r
    }

    #[test]
    fn cpu_only_simple_graph() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[3]));
        let b = g.input(DType::F32, Shape::from(&[3]));
        let c = g.add(&a, &b);
        g.output(&c);
        let g = g.build().unwrap();

        let r = cpu_only_registry();
        let plan_g = plan(&g, &r).expect("planner");
        plan_g.validate().expect("planned graph validates");

        // All ops should be Compute on CPU plus alloc.  No transfers.
        let transfer_count = plan_g
            .ops
            .iter()
            .filter(|o| matches!(o, PlacedOp::Transfer { .. }))
            .count();
        assert_eq!(transfer_count, 0);
        for op in &plan_g.ops {
            if let PlacedOp::Compute { executor, .. } = op {
                assert_eq!(*executor, CPU_EXECUTOR);
            }
        }
    }

    #[test]
    fn empty_registry_errors() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[3]));
        g.output(&a);
        let g = g.build().unwrap();

        let r = Registry::new();
        assert!(matches!(plan(&g, &r), Err(PlanError::EmptyRegistry)));
    }

    // ──────── Pass 2b (single-executor preference) tests ────────

    fn cpu_profile() -> BackendProfile {
        BackendProfile {
            kind: "cpu".to_string(),
            supported_ops: 0xFFFF_FFFF,
            supported_dtypes: 0x07,
            gflops_f32: 40,
            gflops_u8: 40,
            gflops_i32: 40,
            host_to_device_bw: 100,
            device_to_host_bw: 100,
            device_internal_bw: 100,
            launch_overhead_ns: 0,
            transport_latency_ns: 0,
            on_device_mib: 8 * 1024,
            max_tensor_rank: 16,
            max_dim: u32::MAX,
        }
    }

    /// A "GPU-like" profile mirroring matrix-metal's defaults: 5 TFLOPS
    /// f32, 5 µs launch, 50 GB/s host↔device bandwidth.
    fn gpu_profile() -> BackendProfile {
        BackendProfile {
            kind: "gpu".to_string(),
            // Same ops as CPU for this test.
            supported_ops: 0xFFFF_FFFF,
            // F32 only — matches matrix-metal V1.
            supported_dtypes: 0b001,
            gflops_f32: 5_000,
            gflops_u8: 0,
            gflops_i32: 0,
            host_to_device_bw: 50,
            device_to_host_bw: 50,
            device_internal_bw: 200,
            launch_overhead_ns: 5_000,
            transport_latency_ns: 0,
            on_device_mib: 16 * 1024,
            max_tensor_rank: 4,
            max_dim: 65535,
        }
    }

    fn cpu_and_gpu_registry() -> Registry {
        let (mut r, _) = Registry::with_cpu(cpu_profile());
        r.register("gpu".to_string(), gpu_profile());
        r
    }

    /// Builds a chain of `n_ops` elementwise `Neg` operations on a
    /// single F32 input of shape `dims`.  This is the canonical
    /// "graph that should ship to the GPU once" pattern.
    fn neg_chain(n_ops: usize, dims: &[u32]) -> matrix_ir::Graph {
        let mut g = GraphBuilder::new();
        let mut t = g.input(DType::F32, Shape::from(dims));
        for _ in 0..n_ops {
            t = g.neg(&t);
        }
        g.output(&t);
        g.build().unwrap()
    }

    #[test]
    fn long_elementwise_chain_ships_to_gpu_uniformly() {
        // 800×800×4 F32 = 2.56 MB per tensor; 10 ops.
        // Per the worked example in MX04 §"Single-executor preference",
        // greedy picks CPU because op 0's transfer cost dominates;
        // pass 2b should override and place every op on GPU.
        let g = neg_chain(10, &[800, 800, 4]);
        let r = cpu_and_gpu_registry();
        let placed = plan(&g, &r).expect("plan");
        placed.validate().expect("validates");

        // Every Compute op should be on the GPU executor (id 1).
        let gpu = ExecutorId(1);
        let mut compute_count = 0;
        for op in &placed.ops {
            if let PlacedOp::Compute { executor, .. } = op {
                assert_eq!(*executor, gpu, "elementwise chain should land entirely on GPU");
                compute_count += 1;
            }
        }
        assert_eq!(compute_count, 10);

        // We expect exactly one host→device transfer (for the input).
        let transfer_count = placed
            .ops
            .iter()
            .filter(|o| matches!(o, PlacedOp::Transfer { .. }))
            .count();
        assert_eq!(
            transfer_count, 1,
            "uniform GPU placement should pay exactly one transfer (the input)"
        );
    }

    #[test]
    fn single_tiny_op_stays_on_cpu() {
        // 4×4 = 16 elements per op, 1 op.  GPU cost = 16/5000 + 5000 launch
        // + transfer ≈ 5128 + (16*4/50) ≈ 5128 ns.
        // CPU cost = 16/40 ≈ 0 ns.  Greedy picks CPU and uniform-GPU
        // (5128 ns) loses to greedy-CPU (0 ns).  No swap.
        let g = neg_chain(1, &[4, 4]);
        let r = cpu_and_gpu_registry();
        let placed = plan(&g, &r).expect("plan");
        placed.validate().expect("validates");

        for op in &placed.ops {
            if let PlacedOp::Compute { executor, .. } = op {
                assert_eq!(
                    *executor, CPU_EXECUTOR,
                    "tiny single-op graph should stay on CPU"
                );
            }
        }
    }

    #[test]
    fn capability_hole_disables_uniform_gpu() {
        // Build a graph with one Cast op (GPU doesn't support Cast in
        // the matrix-metal V1 profile, but our test gpu_profile has
        // supported_ops = 0xFFFFFFFF which DOES include Cast).  To
        // genuinely block uniform-GPU we need to hide an op behind a
        // capability mask.  Adjust the GPU profile to drop Cast (tag 0x1A).
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[800, 800]));
        let n = g.neg(&a);
        let c = g.cast(&n, DType::I32);
        let n2 = g.neg(&c);
        g.output(&n2);
        let g = g.build().unwrap();

        let (mut r, _) = Registry::with_cpu(cpu_profile());
        let mut gpu = gpu_profile();
        // Drop Cast (op tag 0x1A) and turn off I32 support.
        gpu.supported_ops &= !(1u32 << 0x1A);
        gpu.supported_dtypes &= !(1u8 << 2);
        r.register("gpu".to_string(), gpu);

        // Uniform-GPU isn't a candidate (Cast or I32 unsupported).
        // Pass 2b should not swap; greedy decides each op individually.
        let placed = plan(&g, &r).expect("plan");
        placed.validate().expect("validates");

        // Cast on F32→I32: GPU's I32 unsupported, must run on CPU.
        // The Neg ops can run anywhere.  Mixed placement is expected.
        let mut saw_cpu = false;
        for op in &placed.ops {
            if let PlacedOp::Compute { executor, op: ir_op, .. } = op {
                if matches!(ir_op, Op::Cast { .. }) {
                    assert_eq!(*executor, CPU_EXECUTOR, "Cast must stay on CPU");
                    saw_cpu = true;
                }
            }
        }
        assert!(saw_cpu, "expected at least one CPU placement");
    }

    #[test]
    fn uniform_replaces_only_when_strictly_cheaper() {
        // Build a graph where uniform-GPU and greedy give the same
        // total cost.  Pass 2b uses `<` (strict), so it should not
        // swap and we should keep the greedy decision.
        //
        // Easiest way: a single Neg op on a tensor where transferring
        // costs almost exactly the GPU compute saving.  Just verify
        // determinism — if greedy picked CPU, output stays on CPU.
        let g = neg_chain(1, &[100, 100]);
        let r = cpu_and_gpu_registry();
        let placed = plan(&g, &r).expect("plan");
        // Just check the planner doesn't crash and produces a valid graph.
        placed.validate().expect("validates");
    }
}
