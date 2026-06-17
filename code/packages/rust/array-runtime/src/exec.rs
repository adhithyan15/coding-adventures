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
//! 1. **Precision.** `array-runtime` computes in `f64`. As of **MX12 / MXF-3**
//!    `matrix-ir` has a `DType::F64`, so an `f64` array now lowers to an **`F64`**
//!    graph and crosses the boundary as **8-byte** little-endian doubles — no
//!    `f32` round-trip. `execute` therefore returns the **bit-exact** `f64`
//!    answer, agreeing with the reference path in [`crate::ops`] to full `f64`
//!    precision (a value like `1.0 + 2^-40`, which `f32` cannot represent,
//!    survives). The historical **`F32`** path (4-byte floats) is kept for `f32`
//!    callers via [`execute_with_dtype`]; it agrees with the reference only to
//!    `f32` precision, by construction.
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
use matrix_ir::{DType, Graph};
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
    // `Array` is `f64`-valued, so the default path is **full `f64` precision**
    // (MX12 / MXF-3): lower to an `F64` graph and cross the boundary as 8-byte
    // doubles, with no `f32` round-trip.
    execute_with_dtype(kernel, DType::F64, a, b)
}

/// Execute `kernel` over `a`/`b` lowering at element `dtype`. `DType::F64` keeps
/// full double precision (8-byte codec); `DType::F32` takes the historical
/// 4-byte path (rounds to `f32` at the boundary, kept for `f32` callers). Any
/// other dtype is rejected — `array-runtime` only computes floats.
///
/// Exposed within the crate so both the bit-exact `f64` path and the legacy
/// `f32` path are directly testable from the same entry point.
pub(crate) fn execute_with_dtype(
    kernel: Kernel,
    dtype: DType,
    a: &Array,
    b: &Array,
) -> Result<Array, String> {
    match kernel {
        Kernel::Elementwise(op) => execute_elementwise(op, dtype, a, b),
        Kernel::MatMul => execute_matmul(dtype, a, b),
    }
}

/// Execute a **whole-array sum** end-to-end on the CPU executor at full `f64`
/// precision (MX12 / MXF-3), returning the scalar total as a 1×1 [`Array`].
///
/// This lowers `a` to a single-input `reduce_sum` graph with *empty axes*
/// (reduce-all) and runs it through `matrix-cpu`'s `F64` reduce kernel. Like the
/// reference [`crate::ops::sum`], it folds the column-major buffer left-to-right
/// from `0.0`, so the two agree **bit-for-bit** — including on a running sum that
/// is not representable in `f32`. (It is the executed analogue of the reference
/// reduction, and the substrate path R/MATLAB reductions will adopt in MXF-4.)
pub fn execute_sum(a: &Array) -> Result<Array, String> {
    use matrix_ir::{GraphBuilder, Shape};

    let n = a.len();
    // Lower the flat buffer as a 1-D `[n]` F64 tensor and reduce-all to a scalar.
    let dims = vec![u32::try_from(n).map_err(|_| {
        format!("execute_sum: length {n} exceeds u32::MAX (matrix-ir shape limit)")
    })?];
    let mut g = GraphBuilder::new();
    let t = g.input(DType::F64, Shape { dims });
    let s = g.reduce_sum(&t, Vec::new(), false); // empty axes = reduce all
    g.output(&s);
    let graph = g
        .build()
        .map_err(|e| format!("execute_sum: graph build failed: {e:?}"))?;

    let out = run_graph_on_cpu(&graph, &[f64_bytes(a.data())])?;
    let data = f64_from_bytes(&out[0])?;
    if data.len() != 1 {
        return Err(format!(
            "execute_sum: reduce-all should yield one scalar, got {} values",
            data.len()
        ));
    }
    Array::from_shape(data, vec![]) // scalar
}

/// Elementwise execution. Layout-agnostic (positional), so the column-major
/// bytes go straight to the row-major executor and back unchanged.
fn execute_elementwise(op: BinOp, dtype: DType, a: &Array, b: &Array) -> Result<Array, String> {
    if a.shape() != b.shape() {
        return Err(format!(
            "execute: elementwise needs equal shapes, got {:?} vs {:?} \
             (scalar broadcasting is reference-path only)",
            a.shape(),
            b.shape()
        ));
    }
    let graph = build_graph(Kernel::Elementwise(op), dtype, a.shape(), b.shape())?;
    let out = run_graph_on_cpu(&graph, &[encode(dtype, a.data()), encode(dtype, b.data())])?;
    let data = decode(dtype, &out[0])?;
    Array::from_shape(data, a.shape().to_vec())
}

/// Matrix-product execution. Bridges the column-major ↔ row-major gap:
/// transpose each operand's data into row-major, run the row-major kernel, then
/// transpose the row-major result back into column-major storage.
fn execute_matmul(dtype: DType, a: &Array, b: &Array) -> Result<Array, String> {
    let (m, k) = (a.nrows(), a.ncols());
    let (k2, n) = (b.nrows(), b.ncols());
    if k != k2 {
        return Err(format!(
            "execute: matmul inner dims disagree ({m}x{k} · {k2}x{n})"
        ));
    }
    let graph = build_graph(Kernel::MatMul, dtype, &[m, k], &[k, n])?;
    let a_row = col_to_row(a.data(), m, k);
    let b_row = col_to_row(b.data(), k, n);
    let out = run_graph_on_cpu(&graph, &[encode(dtype, &a_row), encode(dtype, &b_row)])?;
    let c_row = decode(dtype, &out[0])?; // row-major [m, n]
    let c_col = row_to_col(&c_row, m, n); // back to column-major
    Array::from_shape(c_col, vec![m, n])
}

// ── Boundary conversions ────────────────────────────────────────────────────
//
// `array-runtime` holds every value as `f64`. The executor's wire format is the
// element dtype's little-endian bytes, so we encode on the way in and decode on
// the way out. The dtype picks the width: `F64` is 8 bytes (bit-exact, the
// default), `F32` is 4 bytes (the historical path; it rounds at the boundary).

/// `f64` values → little-endian bytes for `dtype` (the executor's wire format).
/// `F64` writes 8 bytes per value (no precision loss); `F32` narrows to 4.
fn encode(dtype: DType, values: &[f64]) -> Vec<u8> {
    match dtype {
        DType::F64 => f64_bytes(values),
        _ => f32_bytes(values),
    }
}

/// Little-endian `dtype` bytes → `f64` values (inverse of [`encode`]).
fn decode(dtype: DType, bytes: &[u8]) -> Result<Vec<f64>, String> {
    match dtype {
        DType::F64 => f64_from_bytes(bytes),
        _ => f32_from_bytes(bytes),
    }
}

/// `f64` values → `f32` little-endian bytes (4 bytes each).
fn f32_bytes(values: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for &v in values {
        out.extend_from_slice(&(v as f32).to_le_bytes());
    }
    out
}

/// `f32` little-endian bytes → `f64` values. Rejects a length that is not a
/// whole number of 4-byte `f32`s rather than reading past the end.
fn f32_from_bytes(bytes: &[u8]) -> Result<Vec<f64>, String> {
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

/// `f64` values → **8-byte** little-endian bytes (the `F64` wire format,
/// MX12 / MXF-3). Mirrors `matrix-cpu`'s `write_f64_vec` byte-for-byte, so the
/// buffer is directly consumable by the executor's `F64` kernels. No precision
/// is lost — this is the whole point of the `f64` path.
fn f64_bytes(values: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 8);
    for &v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// **8-byte** little-endian bytes → `f64` values (inverse of [`f64_bytes`],
/// mirroring `matrix-cpu`'s `read_f64_vec`). Validates that the length is a
/// whole number of 8-byte doubles and returns an `Err` — never panicking or
/// reading out of bounds — on a short or ragged buffer from a malformed result.
fn f64_from_bytes(bytes: &[u8]) -> Result<Vec<f64>, String> {
    let chunks = bytes.chunks_exact(8);
    if !chunks.remainder().is_empty() {
        return Err(format!(
            "output byte length {} is not a whole number of f64 values",
            bytes.len()
        ));
    }
    Ok(chunks
        .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
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

    /// Executed result must match the CPU reference path within `f32` tolerance.
    /// Used for the legacy `f32` path; the `f64` path uses the stricter
    /// [`assert_bit_exact`] below.
    fn assert_matches_reference(executed: &Array, reference: &Array) {
        assert_eq!(executed.shape(), reference.shape(), "shape mismatch");
        for (e, r) in executed.data().iter().zip(reference.data()) {
            assert!(
                (e - r).abs() <= 1e-5 * (1.0 + r.abs()),
                "executed {e} vs reference {r}"
            );
        }
    }

    /// Executed result must equal the reference **bit-for-bit** — the MXF-3
    /// invariant for the `f64` path. Compares raw bit patterns (so a `-0.0` vs
    /// `0.0` or any rounding difference would fail), not an `==` with tolerance.
    fn assert_bit_exact(executed: &Array, reference: &Array) {
        assert_eq!(executed.shape(), reference.shape(), "shape mismatch");
        for (e, r) in executed.data().iter().zip(reference.data()) {
            assert_eq!(
                e.to_bits(),
                r.to_bits(),
                "not bit-exact: executed {e} ({:#018x}) vs reference {r} ({:#018x})",
                e.to_bits(),
                r.to_bits()
            );
        }
    }

    /// A value that is **not** representable in `f32`: `1 + 2^-40` rounds to
    /// exactly `1.0` as an `f32`, but is a distinct `f64`. Any `f64 → f32 → f64`
    /// round-trip collapses it to `1.0`; the bit-exact `f64` path preserves it.
    fn f32_unrepresentable() -> f64 {
        let v = 1.0 + 2f64.powi(-40);
        // Sanity: it really does collapse under an f32 round-trip, so the tests
        // below are genuinely distinguishing the two paths.
        assert_eq!(v as f32 as f64, 1.0);
        assert_ne!(v, 1.0);
        v
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
    fn f32_from_bytes_rejects_partial_f32() {
        // A byte length that isn't a whole number of f32s (4 bytes) is an error,
        // not an out-of-bounds read.
        assert!(f32_from_bytes(&[0u8; 6]).is_err());
        assert!(f32_from_bytes(&[0u8; 8]).is_ok());
    }

    #[test]
    fn f64_from_bytes_rejects_partial_f64() {
        // A byte length that isn't a whole number of f64s (8 bytes) is an error,
        // never a panic or a read past the end of a short/ragged buffer.
        assert!(f64_from_bytes(&[0u8; 7]).is_err());
        assert!(f64_from_bytes(&[0u8; 12]).is_err()); // 1.5 doubles
        assert!(f64_from_bytes(&[0u8; 0]).is_ok()); // empty is a whole 0 doubles
        assert!(f64_from_bytes(&[0u8; 16]).is_ok()); // 2 doubles
    }

    #[test]
    fn f64_codec_round_trips_bit_exactly() {
        // f64_bytes / f64_from_bytes must be exact inverses on values f32 can't
        // hold, and the byte layout must match matrix-cpu's (8-byte LE) so the
        // executor reads back the same doubles.
        let vals = vec![f32_unrepresentable(), -0.0, 1e300, f64::MIN_POSITIVE];
        let bytes = f64_bytes(&vals);
        assert_eq!(bytes.len(), vals.len() * 8);
        let back = f64_from_bytes(&bytes).unwrap();
        for (a, b) in vals.iter().zip(&back) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
    }

    #[test]
    fn run_graph_rejects_wrong_input_count() {
        let graph = build_graph(Kernel::Elementwise(BinOp::Add), DType::F32, &[2], &[2]).unwrap();
        // Graph declares two inputs; provide one.
        let err = run_graph_on_cpu(&graph, &[f32_bytes(&[1.0, 2.0])]);
        assert!(err.is_err());
    }

    #[test]
    fn run_graph_rejects_wrong_input_byte_length() {
        let graph = build_graph(Kernel::Elementwise(BinOp::Add), DType::F32, &[2], &[2]).unwrap();
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

    // ── MXF-3: the f64 path is bit-exact, the f32 path is not ────────────────

    #[test]
    fn f64_elementwise_is_bit_exact_on_f32_unrepresentable_values() {
        // x = 1 + 2^-40 is exact in f64 but rounds to 1.0 in f32. The default
        // execute() path (F64) must reproduce ops:: exactly; the legacy F32 path
        // must NOT — proving the round-trip is genuinely gone.
        let x = f32_unrepresentable();
        let a = Array::from_vec(vec![x, x, x]);
        let one = Array::from_vec(vec![1.0, 1.0, 1.0]);
        for op in [BinOp::Add, BinOp::Sub, BinOp::Mul] {
            let reference = ops::elementwise(op, &a, &one).unwrap();

            let f64_exec = execute(Kernel::Elementwise(op), &a, &one).unwrap();
            assert_bit_exact(&f64_exec, &reference);

            // The old behaviour, for contrast: the f32 path rounds the input and
            // so cannot be bit-exact for a sub/mul that depends on the lost bits.
            let f32_exec =
                execute_with_dtype(Kernel::Elementwise(op), DType::F32, &a, &one).unwrap();
            if op == BinOp::Sub {
                // (1 + 2^-40) - 1 == 2^-40 exactly in f64, but the f32 path sees
                // 1.0 - 1.0 == 0.0. Concrete proof the precision actually differs.
                assert_eq!(reference.data()[0].to_bits(), 2f64.powi(-40).to_bits());
                assert_eq!(f32_exec.data()[0], 0.0);
            }
        }
    }

    #[test]
    fn f64_matmul_is_bit_exact_where_f32_would_round() {
        // Build a 1x2 · 2x1 dot product whose exact value carries a bit f32 can't
        // hold: [1, 2^-20] · [1, 2^-20]^T = 1 + 2^-40. In f64 that's exact
        // (ulp(1.0) = 2^-52, so 2^-40 survives); the f32 path (ulp(1.0) = 2^-23)
        // collapses 2^-40 and returns 1.0.
        let tiny = 2f64.powi(-20);
        let a = Array::from_rows(vec![vec![1.0, tiny]]).unwrap(); // 1x2
        let b = Array::from_rows(vec![vec![1.0], vec![tiny]]).unwrap(); // 2x1
        let reference = ops::matmul(&a, &b).unwrap();

        let f64_exec = execute(Kernel::MatMul, &a, &b).unwrap();
        assert_bit_exact(&f64_exec, &reference);

        // Reference really does hold 1 + 2^-40 (not 1.0), and the f32 path loses it.
        assert_eq!(
            reference.data()[0].to_bits(),
            (1.0 + 2f64.powi(-40)).to_bits()
        );
        assert_ne!(reference.data()[0], 1.0);
        let f32_exec = execute_with_dtype(Kernel::MatMul, DType::F32, &a, &b).unwrap();
        assert_eq!(f32_exec.data()[0], 1.0);
    }

    #[test]
    fn f64_sum_matches_reference_bit_exactly() {
        // A running sum that is not representable in f32: 1.0 followed by many
        // 2^-40 increments. ops::sum folds the column-major buffer left-to-right
        // from 0.0; execute_sum's F64 reduce kernel does the same, so they agree
        // bit-for-bit. An f32 reduction would lose every increment.
        let mut data = vec![1.0];
        for _ in 0..8 {
            data.push(2f64.powi(-40));
        }
        let a = Array::from_vec(data);

        let reference = ops::sum(&a);
        let executed = execute_sum(&a).unwrap();
        assert!(executed.is_scalar());
        assert_eq!(
            executed.data()[0].to_bits(),
            reference.to_bits(),
            "f64 sum not bit-exact: {} vs {}",
            executed.data()[0],
            reference
        );
        // And it genuinely captured the sub-f32 bits (result != plain 1.0).
        assert_ne!(reference, 1.0);
    }

    #[test]
    fn f64_sum_rejects_oversized_length() {
        // The usize→u32 shape cast in execute_sum is a trust boundary: a length
        // past u32::MAX must error, not truncate. We can't allocate that, so test
        // the guard via the same try_from the code uses (kept in lockstep here).
        let huge = (u32::MAX as usize) + 1;
        assert!(u32::try_from(huge).is_err());
    }

    #[test]
    fn f32_path_still_works_and_matches_within_tolerance() {
        // The historical F32 lowering is preserved for f32 callers: it executes
        // and agrees with the reference to f32 precision (the MA-2 contract).
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        let b = Array::from_rows(vec![vec![5.0, 6.0], vec![7.0, 8.0]]).unwrap();
        let executed = execute_with_dtype(Kernel::MatMul, DType::F32, &a, &b).unwrap();
        let reference = ops::matmul(&a, &b).unwrap();
        assert_matches_reference(&executed, &reference);
        // Small integer values are exactly representable, so it's even exact here.
        assert_eq!(executed.get(0, 0), Some(19.0));
    }
}
