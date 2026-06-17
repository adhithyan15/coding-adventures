//! **MA-2: end-to-end execution** of a lowered array op on the CPU executor.
//!
//! MA-1 lowered each op to a `matrix-ir` graph and let `matrix-runtime`'s
//! planner *place* it on a backend, but produced values from the CPU reference
//! path in [`crate::ops`]. MA-2 closes the loop: it plans the graph **and runs
//! it** through `matrix-cpu`'s `CpuExecutor`, returning real numeric results
//! from the same pipeline a GPU would use. Register a GPU executor and a large
//! op flows to it with no change here — the dispatch decision and the execution
//! path are one and the same.
//!
//! ## Two boundary conversions
//!
//! 1. **Precision.** `array-runtime` computes in `f64`; `matrix-ir`/the executor
//!    work in `f32`. We convert to `f32` little-endian bytes on the way in and
//!    back to `f64` on the way out. (A future `f64` dtype in `matrix-ir` removes
//!    this; until then the reference path in [`crate::ops`] stays the exact-`f64`
//!    answer, and these results match it to `f32` precision.)
//!
//! 2. **Memory order.** `array-runtime` stores **column-major**; `matrix-cpu`'s
//!    kernels are **row-major**. Elementwise ops are positional, so order is
//!    irrelevant and the bytes pass straight through. `matmul` is *not*
//!    positional, so we transpose each operand into row-major on the way in and
//!    the result back into column-major on the way out (see [`execute`]).
//!
//! ## Scope
//!
//! [`execute`] runs **elementwise** (`add`/`sub`/`mul`/`div` on equal shapes) and
//! **`matmul`** end-to-end. `transpose` and the reductions stay on the reference
//! path for now (they need either trivial or axis-aware lowering); they are
//! tracked for a follow-up. Every executed result is cross-checked against the
//! reference path in the tests, so the two can never silently diverge.

use std::collections::HashMap;

use compute_ir::{BufferId, ComputeGraph, PlacedConstant, PlacedOp, PlacedTensor, Residency};
use executor_protocol::{ExecutorRequest, ExecutorResponse};
use matrix_cpu::CpuExecutor;
use matrix_ir::Graph;
use matrix_runtime::Runtime;

use crate::accel::{build_graph, Kernel};
use crate::ops::BinOp;
use crate::value::Array;

/// Hard cap on the total byte footprint of all placed tensors a single
/// execution may allocate. A crafted graph could declare giant shapes that pass
/// `matrix-ir`'s `u64`-overflow validation yet still funnel into a
/// `vec![0u8; bytes]` and abort the process; this rejects them first. 4 GiB is
/// generous for any single array op while staying well under host RAM. (Same
/// policy value the Rust/Python and Node bindings use.)
pub const MAX_TOTAL_BUFFER_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Execute `kernel` over `a` and `b` on the CPU executor, returning the result
/// as a column-major [`Array`]. This plans the lowered `matrix-ir` graph with
/// `matrix-runtime` and runs the placed graph through `matrix-cpu` — the real
/// hardware pipeline, not the reference path.
///
/// Supported: elementwise `add`/`sub`/`mul`/`div` on **equal-shaped** operands,
/// and `matmul` (`[m,k] · [k,n]`). Scalar broadcasting and `transpose`/reductions
/// are not yet lowered for execution — use [`crate::ops`] for those.
pub fn execute(kernel: Kernel, a: &Array, b: &Array) -> Result<Array, String> {
    match kernel {
        Kernel::Elementwise(op) => execute_elementwise(op, a, b),
        Kernel::MatMul => execute_matmul(a, b),
    }
}

/// Elementwise execution. Layout-agnostic (positional), so the column-major
/// bytes go straight to the row-major executor and back unchanged.
fn execute_elementwise(op: BinOp, a: &Array, b: &Array) -> Result<Array, String> {
    if a.shape() != b.shape() {
        return Err(format!(
            "execute: elementwise needs equal shapes, got {:?} vs {:?} \
             (scalar broadcasting is reference-path only)",
            a.shape(),
            b.shape()
        ));
    }
    let graph = build_graph(Kernel::Elementwise(op), a.shape(), b.shape())?;
    let out = run_graph_on_cpu(&graph, &[f32_bytes(a.data()), f32_bytes(b.data())])?;
    let data = f64_from_bytes(&out[0])?;
    Array::from_shape(data, a.shape().to_vec())
}

/// Matrix-product execution. Bridges the column-major ↔ row-major gap:
/// transpose each operand's data into row-major, run the row-major kernel, then
/// transpose the row-major result back into column-major storage.
fn execute_matmul(a: &Array, b: &Array) -> Result<Array, String> {
    let (m, k) = (a.nrows(), a.ncols());
    let (k2, n) = (b.nrows(), b.ncols());
    if k != k2 {
        return Err(format!(
            "execute: matmul inner dims disagree ({m}x{k} · {k2}x{n})"
        ));
    }
    let graph = build_graph(Kernel::MatMul, &[m, k], &[k, n])?;
    let a_row = col_to_row(a.data(), m, k);
    let b_row = col_to_row(b.data(), k, n);
    let out = run_graph_on_cpu(&graph, &[f32_bytes(&a_row), f32_bytes(&b_row)])?;
    let c_row = f64_from_bytes(&out[0])?; // row-major [m, n]
    let c_col = row_to_col(&c_row, m, n); // back to column-major
    Array::from_shape(c_col, vec![m, n])
}

// ── Boundary conversions ────────────────────────────────────────────────────

/// `f64` values → `f32` little-endian bytes (the executor's wire format).
fn f32_bytes(values: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for &v in values {
        out.extend_from_slice(&(v as f32).to_le_bytes());
    }
    out
}

/// `f32` little-endian bytes → `f64` values.
fn f64_from_bytes(bytes: &[u8]) -> Result<Vec<f64>, String> {
    let chunks = bytes.chunks_exact(4);
    if !chunks.remainder().is_empty() {
        return Err(format!(
            "output byte length {} is not a whole number of f32 values",
            bytes.len()
        ));
    }
    Ok(chunks
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f64)
        .collect())
}

/// Re-order a column-major `[rows, cols]` buffer into row-major.
/// Column-major `(r, c)` is at `c*rows + r`; row-major wants it at `r*cols + c`.
fn col_to_row(data: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let mut out = vec![0.0; data.len()];
    for c in 0..cols {
        for r in 0..rows {
            out[r * cols + c] = data[c * rows + r];
        }
    }
    out
}

/// Re-order a row-major `[rows, cols]` buffer into column-major (inverse of
/// [`col_to_row`]).
fn row_to_col(data: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let mut out = vec![0.0; data.len()];
    for r in 0..rows {
        for c in 0..cols {
            out[c * rows + r] = data[r * cols + c];
        }
    }
    out
}

// ── The plan-and-run orchestrator ───────────────────────────────────────────

/// Plan `graph` with `matrix-runtime` and execute the placed graph on a fresh
/// `matrix-cpu` `CpuExecutor`. `inputs[i]` is the little-endian `f32` byte
/// payload for `graph.inputs[i]`, in declaration order; returns one byte vector
/// per `graph.outputs`.
///
/// Adapted from `matrix-rust-python`'s `run_graph_on_cpu`: allocate one executor
/// buffer per planner buffer-id, rewrite every residency to the real ids, upload
/// constants then inputs, dispatch, download outputs. Every step is matched
/// explicitly — adversarial shapes return `Err`, never panic.
fn run_graph_on_cpu(graph: &Graph, inputs: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, String> {
    if inputs.len() != graph.inputs.len() {
        return Err(format!(
            "input count mismatch: graph declares {}, caller provided {}",
            graph.inputs.len(),
            inputs.len()
        ));
    }
    for (i, t) in graph.inputs.iter().enumerate() {
        let expected = t
            .shape
            .byte_size(t.dtype)
            .ok_or_else(|| format!("graph.inputs[{i}] tensor size overflows u64"))?;
        if inputs[i].len() as u64 != expected {
            return Err(format!(
                "graph.inputs[{i}] byte length mismatch: needs {expected}, got {}",
                inputs[i].len()
            ));
        }
    }

    let rt = Runtime::new(matrix_cpu::profile());
    let mut placed = rt
        .plan(graph)
        .map_err(|e| format!("matrix-runtime plan failed: {e:?}"))?;

    if placed.inputs.len() != graph.inputs.len() {
        return Err(format!(
            "planner invariant broken: {} graph inputs but {} placed inputs",
            graph.inputs.len(),
            placed.inputs.len()
        ));
    }

    // Resource cap: reject an over-large total footprint before allocating.
    let mut total: u64 = 0;
    for t in &placed.tensors {
        let bytes = t
            .shape
            .byte_size(t.dtype)
            .ok_or_else(|| format!("tensor {:?} byte size overflows u64", t.id))?;
        total = total
            .checked_add(bytes)
            .ok_or_else(|| "graph total byte size overflows u64".to_string())?;
        if total > MAX_TOTAL_BUFFER_BYTES {
            return Err(format!(
                "graph total buffer size {total} exceeds {MAX_TOTAL_BUFFER_BYTES} bytes; refusing"
            ));
        }
    }

    // Allocate one real CPU buffer per unique planner buffer-id.
    let exec = CpuExecutor::new();
    let mut id_map: HashMap<BufferId, BufferId> = HashMap::new();
    for t in &placed.tensors {
        if id_map.contains_key(&t.residency.buffer) {
            continue;
        }
        let bytes = t
            .shape
            .byte_size(t.dtype)
            .ok_or_else(|| format!("tensor {:?} byte size overflows u64", t.id))?;
        match exec.handle(ExecutorRequest::AllocBuffer { bytes }) {
            ExecutorResponse::BufferAllocated { buffer } => {
                id_map.insert(t.residency.buffer, buffer);
            }
            other => return Err(format!("AllocBuffer failed: {other:?}")),
        }
    }

    // Rewrite every residency to the executor's real buffer ids.
    let rewrite = |r: &mut Residency| {
        if let Some(real) = id_map.get(&r.buffer) {
            r.buffer = *real;
        }
    };
    let rewrite_t = |t: &mut PlacedTensor| rewrite(&mut t.residency);
    let rewrite_c = |c: &mut PlacedConstant| rewrite(&mut c.residency);
    placed.tensors.iter_mut().for_each(rewrite_t);
    placed.inputs.iter_mut().for_each(rewrite_t);
    placed.outputs.iter_mut().for_each(rewrite_t);
    placed.constants.iter_mut().for_each(rewrite_c);
    for op in placed.ops.iter_mut() {
        match op {
            PlacedOp::Compute { .. } => {}
            PlacedOp::Transfer { src, dst, .. } => {
                rewrite(src);
                rewrite(dst);
            }
            PlacedOp::Alloc { residency, .. } => rewrite(residency),
            PlacedOp::Free { residency } => rewrite(residency),
        }
    }

    // Upload constants, then caller inputs.
    for c in &placed.constants {
        match exec.handle(ExecutorRequest::UploadBuffer {
            buffer: c.residency.buffer,
            offset: 0,
            data: c.bytes.clone(),
        }) {
            ExecutorResponse::BufferUploaded { .. } => {}
            other => return Err(format!("UploadBuffer (constant) failed: {other:?}")),
        }
    }
    for (i, input) in inputs.iter().enumerate() {
        let buf = placed
            .inputs
            .get(i)
            .ok_or_else(|| format!("internal: placed.inputs[{i}] missing"))?
            .residency
            .buffer;
        match exec.handle(ExecutorRequest::UploadBuffer {
            buffer: buf,
            offset: 0,
            data: input.clone(),
        }) {
            ExecutorResponse::BufferUploaded { .. } => {}
            other => return Err(format!("UploadBuffer (input {i}) failed: {other:?}")),
        }
    }

    // Dispatch the whole graph, then download each output.
    match exec.handle(ExecutorRequest::Dispatch {
        job_id: 1,
        graph: ComputeGraph {
            format_version: placed.format_version,
            inputs: placed.inputs.clone(),
            outputs: placed.outputs.clone(),
            constants: placed.constants.clone(),
            ops: placed.ops.clone(),
            tensors: placed.tensors.clone(),
        },
    }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => return Err(format!("Dispatch failed: {other:?}")),
    }

    let mut outputs = Vec::with_capacity(placed.outputs.len());
    for out in &placed.outputs {
        let bytes = out
            .shape
            .byte_size(out.dtype)
            .ok_or_else(|| format!("output tensor {:?} byte size overflows u64", out.id))?;
        match exec.handle(ExecutorRequest::DownloadBuffer {
            buffer: out.residency.buffer,
            offset: 0,
            len: bytes,
        }) {
            ExecutorResponse::BufferData { data, .. } => outputs.push(data),
            other => return Err(format!("DownloadBuffer failed: {other:?}")),
        }
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    /// Executed result must match the CPU reference path (to f32 precision).
    fn assert_matches_reference(executed: &Array, reference: &Array) {
        assert_eq!(executed.shape(), reference.shape(), "shape mismatch");
        for (e, r) in executed.data().iter().zip(reference.data()) {
            assert!(
                (e - r).abs() <= 1e-5 * (1.0 + r.abs()),
                "executed {e} vs reference {r}"
            );
        }
    }

    #[test]
    fn elementwise_executes_and_matches_reference() {
        let a = Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        let b = Array::from_vec(vec![10.0, 20.0, 30.0, 40.0]);
        for op in [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div] {
            let executed = execute(Kernel::Elementwise(op), &a, &b).unwrap();
            let reference = ops::elementwise(op, &a, &b).unwrap();
            assert_matches_reference(&executed, &reference);
        }
    }

    #[test]
    fn elementwise_on_a_matrix_shape() {
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        let b = Array::from_rows(vec![vec![5.0, 6.0], vec![7.0, 8.0]]).unwrap();
        let executed = execute(Kernel::Elementwise(BinOp::Add), &a, &b).unwrap();
        let reference = ops::add(&a, &b).unwrap();
        assert_eq!(executed.shape(), &[2, 2]);
        assert_matches_reference(&executed, &reference);
    }

    #[test]
    fn elementwise_rejects_unequal_shapes() {
        let a = Array::from_vec(vec![1.0, 2.0]);
        let b = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert!(execute(Kernel::Elementwise(BinOp::Add), &a, &b).is_err());
    }

    #[test]
    fn matmul_executes_and_matches_reference() {
        // [[1,2],[3,4]] · [[5,6],[7,8]] = [[19,22],[43,50]].
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        let b = Array::from_rows(vec![vec![5.0, 6.0], vec![7.0, 8.0]]).unwrap();
        let executed = execute(Kernel::MatMul, &a, &b).unwrap();
        let reference = ops::matmul(&a, &b).unwrap();
        assert_eq!(executed.shape(), &[2, 2]);
        // Spot-check the actual values too, not just agreement with reference.
        assert_eq!(executed.get(0, 0), Some(19.0));
        assert_eq!(executed.get(0, 1), Some(22.0));
        assert_eq!(executed.get(1, 0), Some(43.0));
        assert_eq!(executed.get(1, 1), Some(50.0));
        assert_matches_reference(&executed, &reference);
    }

    #[test]
    fn matmul_nonsquare_executes() {
        // [2x3] · [3x1] -> [2x1].
        let a = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let x = Array::from_rows(vec![vec![1.0], vec![0.0], vec![-1.0]]).unwrap();
        let executed = execute(Kernel::MatMul, &a, &x).unwrap();
        let reference = ops::matmul(&a, &x).unwrap();
        assert_eq!(executed.shape(), &[2, 1]);
        assert_matches_reference(&executed, &reference);
    }

    #[test]
    fn matmul_identity_roundtrips_layout() {
        // a · I == a proves the column↔row conversions are inverses.
        let a = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let i = Array::eye(3);
        let executed = execute(Kernel::MatMul, &a, &i).unwrap();
        assert_matches_reference(&executed, &a);
    }

    #[test]
    fn matmul_inner_dim_mismatch_errors() {
        let a = Array::from_rows(vec![vec![1.0, 2.0]]).unwrap(); // 1x2
        let b = Array::from_rows(vec![vec![1.0, 2.0]]).unwrap(); // 1x2
        assert!(execute(Kernel::MatMul, &a, &b).is_err());
    }

    #[test]
    fn f64_from_bytes_rejects_partial_f32() {
        // A byte length that isn't a whole number of f32s is an error.
        assert!(f64_from_bytes(&[0u8; 6]).is_err());
        assert!(f64_from_bytes(&[0u8; 8]).is_ok());
    }

    #[test]
    fn run_graph_rejects_wrong_input_count() {
        let graph = build_graph(Kernel::Elementwise(BinOp::Add), &[2], &[2]).unwrap();
        // Graph declares two inputs; provide one.
        let err = run_graph_on_cpu(&graph, &[f32_bytes(&[1.0, 2.0])]);
        assert!(err.is_err());
    }

    #[test]
    fn run_graph_rejects_wrong_input_byte_length() {
        let graph = build_graph(Kernel::Elementwise(BinOp::Add), &[2], &[2]).unwrap();
        // Right count, wrong size: input 0 needs 2 f32s (8 bytes), give 1.
        let err = run_graph_on_cpu(&graph, &[f32_bytes(&[1.0]), f32_bytes(&[1.0, 2.0])]);
        assert!(err.is_err());
    }

    #[test]
    fn layout_conversions_are_inverse() {
        // Row-major [[1,2,3],[4,5,6]] = [1,2,3,4,5,6]; its column-major form is
        // [1,4,2,5,3,6]. col_to_row and row_to_col must undo each other.
        let col = vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0];
        let row = col_to_row(&col, 2, 3);
        assert_eq!(row, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(row_to_col(&row, 2, 3), col);
    }
}
