//! The planner.  Lowers `matrix_ir::Graph` to `compute_ir::ComputeGraph`
//! using the registry's `BackendProfile`s.
//!
//! Algorithm (per spec MX04 §"The planner algorithm"):
//!
//! 1. **Capability filter** — for each op, set of executors that can run it.
//! 2. **Greedy cost minimisation** — for each op in topological order,
//!    pick the executor that minimises (compute + transfer-in) cost.
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
}
