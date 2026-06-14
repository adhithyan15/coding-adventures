//! # `exec` — Pure-Rust glue: plan a `matrix_ir::Graph`, run it on
//! the CPU executor, return the output bytes.
//!
//! This module is the **Phase 2** core of MX09.  It is intentionally
//! Python-free so `cargo test` can exercise the real planning +
//! execution pipeline without a Python interpreter.  The Python C API
//! wrapper in `lib.rs` just marshals strings across the FFI boundary
//! and calls into here.
//!
//! Direct port of `matrix-rust-napi`'s `exec.rs` (MX07 Phase 2,
//! PR #3527 + PR #3539).  The pure-Rust shape is identical because
//! the underlying matrix-runtime / matrix-cpu APIs are language-agnostic
//! — only the binding edge differs (Python C API vs N-API).
//!
//! ## What the helper does
//!
//! ```text
//! matrix_ir::Graph    (caller-built; also reachable via matrix-ir-json::decode)
//!         │
//!         │ matrix_runtime::Runtime::plan()
//!         ▼
//! compute_ir::ComputeGraph    (with planner-assigned BufferIds)
//!         │
//!         │ allocate one CpuExecutor buffer per planner-BufferId,
//!         │ remember the planner→real BufferId map
//!         ▼
//! ComputeGraph with real BufferIds    (we rewrite every Residency in place)
//!         │
//!         │ upload constants → upload caller inputs → Dispatch → download outputs
//!         ▼
//! Vec<Vec<u8>>    (one byte vector per graph.outputs())
//! ```
//!
//! ## Why pre-allocate every buffer instead of relying on `PlacedOp::Alloc/Free`?
//!
//! The planner inserts `PlacedOp::Alloc` and `Free` lifetime
//! annotations as a memory optimisation (`compute-ir` spec says
//! executors that manage their own allocations may treat them as
//! no-ops — and CpuExecutor does).  We choose the simpler
//! "allocate everything up front, free nothing" path: it trades a
//! bit of peak memory for a much simpler glue layer that doesn't
//! need to thread the planner→real BufferId map into the executor.
//!
//! When profiling shows the memory cost matters for big graphs,
//! Phase 2b can switch to honouring lifetime ops by extending the
//! executor protocol with a server-controlled-id Alloc variant.

use std::collections::HashMap;

use compute_ir::{BufferId, ComputeGraph, PlacedConstant, PlacedOp, PlacedTensor, Residency};
use executor_protocol::{ExecutorRequest, ExecutorResponse};
use matrix_cpu::CpuExecutor;
use matrix_ir::Graph;
use matrix_runtime::Runtime;

/// Maximum total byte size of *all* placed tensors a single
/// `run_graph_on_cpu` call is permitted to allocate.  Hard-cap to
/// prevent a malicious envelope from declaring giant tensor shapes
/// (each `Shape::byte_size` only rejects on `u64` overflow — i.e. up
/// to ~18 EB).  Without this cap, a ~500-byte JSON envelope could
/// flow into `vec![0u8; bytes]` inside `matrix-cpu`'s `BufferStore`
/// and trigger a process abort via `handle_alloc_error`.
///
/// 4 GiB is "generous for a 2026-era inference workload, well below
/// the host RAM of every CI runner this project targets".  Bigger
/// graphs can override by setting an env var when the helper grows
/// env-var support (not in v0.2 — file an issue when it becomes a
/// real constraint).
///
/// Same value as `matrix-rust-napi::exec::MAX_TOTAL_BUFFER_BYTES`
/// — the two bindings share the policy so the CPU executor sees the
/// same effective DoS posture regardless of which FFI edge invoked
/// it.
pub const MAX_TOTAL_BUFFER_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Plan and execute `graph` on the CPU executor.  Each entry of
/// `inputs` provides the little-endian byte payload for the
/// corresponding `graph.inputs()` tensor in declaration order.
/// Returns one little-endian byte vector per `graph.outputs()` tensor.
///
/// All errors are stringified into a single `Err(String)`; this is a
/// binding-edge helper and the caller (the Python C API wrapper) will
/// turn the string into a `ValueError`.  No panics on adversarial
/// input — every fallible step is matched explicitly.  Panics across
/// the Python C API boundary are undefined behaviour (`extern "C"`
/// requires `extern "C-unwind"` for panic propagation, and we don't
/// use that here), so the "return Err, never panic" discipline is a
/// safety invariant, not a stylistic choice.
pub fn run_graph_on_cpu(graph: &Graph, inputs: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, String> {
    // ─── Argument validation ────────────────────────────────────
    if inputs.len() != graph.inputs.len() {
        return Err(format!(
            "input count mismatch: graph declares {} inputs, caller provided {}",
            graph.inputs.len(),
            inputs.len()
        ));
    }
    // Confirm each input's byte length matches the declared tensor
    // size.  Catches the most common caller bug — wrong dtype or
    // wrong shape on the Python side — with a precise error.
    for (i, t) in graph.inputs.iter().enumerate() {
        let expected = t
            .shape
            .byte_size(t.dtype)
            .ok_or_else(|| format!("graph.inputs[{}] tensor size overflows u64", i))?;
        if (inputs[i].len() as u64) != expected {
            return Err(format!(
                "graph.inputs[{}] byte length mismatch: tensor dtype/shape requires {} bytes, \
                 caller provided {}",
                i,
                expected,
                inputs[i].len()
            ));
        }
    }

    // ─── Plan ──────────────────────────────────────────────────
    let rt = Runtime::new(matrix_cpu::profile());
    let mut placed = rt
        .plan(graph)
        .map_err(|e| format!("matrix-runtime plan failed: {:?}", e))?;

    // ─── Defence-in-depth: planner-invariant check ────────────
    //
    // The upload loop below indexes `placed.inputs[i]` by the caller's
    // input index.  We've already pinned `inputs.len() == graph.inputs.len()`;
    // the trusted planner is expected to preserve
    // `placed.inputs.len() == graph.inputs.len()`.  Make that
    // expectation explicit so a planner regression surfaces as a
    // clean `Err` instead of an index-panic across the FFI boundary.
    if placed.inputs.len() != graph.inputs.len() {
        return Err(format!(
            "planner invariant broken: graph.inputs.len() = {} but placed.inputs.len() = {} \
             (matrix-runtime bug)",
            graph.inputs.len(),
            placed.inputs.len()
        ));
    }

    // ─── Resource cap: reject graphs whose total buffer footprint
    // would exceed MAX_TOTAL_BUFFER_BYTES.  This runs *before* any
    // AllocBuffer call, so we never start allocating a graph we'd
    // then have to fail mid-way through.
    //
    // Without this cap, a ~500-byte JSON envelope declaring a tensor
    // like `shape=[1_000_000_000, 1_000_000_000], dtype=F32` would
    // pass `Graph::validate()` (overflow check is u64-bounded only),
    // pass our pre-flight byte-length check (we never check
    // intermediate / output tensors against caller bytes — they
    // have no caller-supplied counterpart), then flow `bytes` into
    // `vec![0u8; bytes]` and abort the Python process.
    {
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
                    "graph total buffer size {} bytes exceeds limit of {} bytes \
                     (MAX_TOTAL_BUFFER_BYTES); refusing to allocate",
                    total, MAX_TOTAL_BUFFER_BYTES
                ));
            }
        }
    }

    // ─── Allocate one CPU buffer per unique planner-BufferId ───
    //
    // The planner assigns abstract BufferIds (a monotonic counter).
    // The executor uses its own BufferId space (also monotonic but
    // started at 1).  We need a mapping so we can rewrite every
    // Residency in the placed graph before dispatch.
    let exec = CpuExecutor::new();
    let mut id_map: HashMap<BufferId, BufferId> = HashMap::new();

    // Collect (planner_buf_id, bytes_needed) for every tensor we'll
    // touch.  Constants and compute outputs all live in placed.tensors
    // (per ComputeGraph docs §"the per-tensor metadata table"); inputs
    // and outputs are duplicate entries that re-state the same
    // tensors with their starting/ending residency.  Iterating
    // placed.tensors is sufficient to cover every buffer.
    for t in &placed.tensors {
        if id_map.contains_key(&t.residency.buffer) {
            continue; // already sized via another tensor (shouldn't happen
                      // in V1 since planner allocates 1 buffer/tensor, but
                      // belt-and-braces).
        }
        let bytes = t
            .shape
            .byte_size(t.dtype)
            .ok_or_else(|| format!("tensor {:?} byte size overflows u64", t.id))?;
        let resp = exec.handle(ExecutorRequest::AllocBuffer { bytes });
        let real = match resp {
            ExecutorResponse::BufferAllocated { buffer } => buffer,
            other => return Err(format!("AllocBuffer failed: {:?}", other)),
        };
        id_map.insert(t.residency.buffer, real);
    }

    // ─── Rewrite every Residency.buffer in the placed graph ────
    let rewrite_residency = |r: &mut Residency| {
        if let Some(real) = id_map.get(&r.buffer) {
            r.buffer = *real;
        }
    };
    let rewrite_placed_tensor = |t: &mut PlacedTensor| rewrite_residency(&mut t.residency);
    let rewrite_placed_constant = |c: &mut PlacedConstant| rewrite_residency(&mut c.residency);

    for t in placed.tensors.iter_mut() {
        rewrite_placed_tensor(t);
    }
    for t in placed.inputs.iter_mut() {
        rewrite_placed_tensor(t);
    }
    for t in placed.outputs.iter_mut() {
        rewrite_placed_tensor(t);
    }
    for c in placed.constants.iter_mut() {
        rewrite_placed_constant(c);
    }
    for op in placed.ops.iter_mut() {
        match op {
            PlacedOp::Compute { .. } => {} // no buffer refs inside
            PlacedOp::Transfer { src, dst, .. } => {
                rewrite_residency(src);
                rewrite_residency(dst);
            }
            PlacedOp::Alloc { residency, .. } => rewrite_residency(residency),
            PlacedOp::Free { residency } => rewrite_residency(residency),
        }
    }

    // ─── Upload constants to their buffers ──────────────────────
    for c in &placed.constants {
        let resp = exec.handle(ExecutorRequest::UploadBuffer {
            buffer: c.residency.buffer,
            offset: 0,
            data: c.bytes.clone(),
        });
        match resp {
            ExecutorResponse::BufferUploaded { .. } => {}
            other => return Err(format!("UploadBuffer (constant) failed: {:?}", other)),
        }
    }

    // ─── Upload caller-supplied inputs to input buffers ─────────
    //
    // Use `.get(i)` instead of indexing to make any planner-invariant
    // violation surface as an `Err` rather than a panic across the
    // FFI boundary (the planner-invariant check above should make
    // this unreachable, but defence in depth — panics across the
    // Python C API are UB).
    for (i, input) in inputs.iter().enumerate() {
        let buf = placed
            .inputs
            .get(i)
            .ok_or_else(|| format!("internal: placed.inputs[{}] missing after plan", i))?
            .residency
            .buffer;
        let resp = exec.handle(ExecutorRequest::UploadBuffer {
            buffer: buf,
            offset: 0,
            data: input.clone(),
        });
        match resp {
            ExecutorResponse::BufferUploaded { .. } => {}
            other => return Err(format!("UploadBuffer (input {}) failed: {:?}", i, other)),
        }
    }

    // ─── Dispatch ───────────────────────────────────────────────
    let resp = exec.handle(ExecutorRequest::Dispatch {
        job_id: 1,
        graph: ComputeGraph {
            format_version: placed.format_version,
            inputs: placed.inputs.clone(),
            outputs: placed.outputs.clone(),
            constants: placed.constants.clone(),
            ops: placed.ops.clone(),
            tensors: placed.tensors.clone(),
        },
    });
    match resp {
        ExecutorResponse::DispatchDone { .. } => {}
        other => return Err(format!("Dispatch failed: {:?}", other)),
    }

    // ─── Download outputs ───────────────────────────────────────
    let mut outputs = Vec::with_capacity(placed.outputs.len());
    for out in &placed.outputs {
        let bytes = out
            .shape
            .byte_size(out.dtype)
            .ok_or_else(|| format!("output tensor {:?} byte size overflows u64", out.id))?;
        let resp = exec.handle(ExecutorRequest::DownloadBuffer {
            buffer: out.residency.buffer,
            offset: 0,
            len: bytes,
        });
        match resp {
            ExecutorResponse::BufferData { data, .. } => outputs.push(data),
            other => return Err(format!("DownloadBuffer failed: {:?}", other)),
        }
    }

    Ok(outputs)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure-Rust, no Python interpreter needed
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_ir::{DType, GraphBuilder, Shape};

    /// Convert an `&[f32]` to little-endian bytes.
    fn f32_bytes(xs: &[f32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(xs.len() * 4);
        for x in xs {
            out.extend_from_slice(&x.to_le_bytes());
        }
        out
    }

    /// Read `&[u8]` (LE) as `Vec<f32>`.
    fn from_f32_bytes(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    /// The headline test: build an Add graph, run it on CPU through
    /// the full plan+alloc+upload+dispatch+download flow, assert the
    /// numerical result.  This is the property that proves the whole
    /// stack works end-to-end.
    #[test]
    fn add_two_vectors_executes_end_to_end() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[3]));
        let b = g.input(DType::F32, Shape::from(&[3]));
        let c = g.add(&a, &b);
        g.output(&c);
        let graph = g.build().expect("graph builds");

        let outputs = run_graph_on_cpu(
            &graph,
            &[f32_bytes(&[1.0, 2.0, 3.0]), f32_bytes(&[10.0, 20.0, 30.0])],
        )
        .expect("execution succeeds");

        assert_eq!(outputs.len(), 1);
        assert_eq!(from_f32_bytes(&outputs[0]), vec![11.0, 22.0, 33.0]);
    }

    /// MatMul is the workhorse op of every NN; if it works the rest
    /// is downstream.  2×2 × 2×2 = 2×2 (small enough to assert by hand).
    #[test]
    fn matmul_2x2_executes_end_to_end() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 2]));
        let b = g.input(DType::F32, Shape::from(&[2, 2]));
        let c = g.matmul(&a, &b);
        g.output(&c);
        let graph = g.build().expect("graph builds");

        let outputs = run_graph_on_cpu(
            &graph,
            &[
                f32_bytes(&[1.0, 2.0, 3.0, 4.0]), // [[1,2],[3,4]]
                f32_bytes(&[5.0, 6.0, 7.0, 8.0]), // [[5,6],[7,8]]
            ],
        )
        .expect("execution succeeds");

        // [[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]]
        assert_eq!(outputs.len(), 1);
        assert_eq!(from_f32_bytes(&outputs[0]), vec![19.0, 22.0, 43.0, 50.0]);
    }

    /// Wrong input count must fail cleanly.
    #[test]
    fn rejects_wrong_input_count() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2]));
        let b = g.input(DType::F32, Shape::from(&[2]));
        let c = g.add(&a, &b);
        g.output(&c);
        let graph = g.build().unwrap();

        let err = run_graph_on_cpu(&graph, &[f32_bytes(&[1.0, 2.0])])
            .expect_err("must reject 1 input when graph wants 2");
        assert!(err.contains("input count mismatch"), "got: {}", err);
    }

    /// Wrong input byte length must fail cleanly.
    #[test]
    fn rejects_wrong_input_byte_length() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[4])); // needs 16 bytes
        g.output(&a);
        let graph = g.build().unwrap();

        let err = run_graph_on_cpu(&graph, &[f32_bytes(&[1.0, 2.0])]) // 8 bytes
            .expect_err("must reject 8 bytes when 16 expected");
        assert!(err.contains("byte length mismatch"), "got: {}", err);
    }

    /// Adversarial graph with an *output* tensor whose byte size
    /// exceeds `MAX_TOTAL_BUFFER_BYTES` must be refused *before* any
    /// `AllocBuffer` call — otherwise a malicious envelope could
    /// crash the process via `handle_alloc_error`.
    ///
    /// We choose K such that the output tensor `[1, K]` of F32 just
    /// exceeds the 4 GiB cap, while the inputs are tiny enough to pass
    /// the pre-flight byte-length check.  The cap check is the only
    /// guard that fires.
    #[test]
    fn rejects_graph_with_oversized_output() {
        // MAX = 4 GiB; /4 = 1 GiB elements; K = 1 GiB + 1.
        let k: u32 = 1_073_741_825;
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[1, 1]));
        let b = g.input(DType::F32, Shape::from(&[1, k]));
        let c = g.matmul(&a, &b);
        g.output(&c);
        let graph = g.build().expect("graph builds");

        let err = run_graph_on_cpu(
            &graph,
            &[f32_bytes(&[1.0]), vec![0u8; (k as usize) * 4]],
        )
        .expect_err("oversized intermediate/output must be refused");
        assert!(err.contains("exceeds limit"), "got: {}", err);
    }

    /// A small ReLU layer: MatMul → Add → Max(0).  Exercises constants
    /// AND multiple ops, so it proves constant upload + op chaining.
    #[test]
    fn relu_layer_executes_end_to_end() {
        let mut g = GraphBuilder::new();
        // y = max(0, x @ W + b)
        let x = g.input(DType::F32, Shape::from(&[1, 2]));
        let w = g.constant(DType::F32, Shape::from(&[2, 2]), f32_bytes(&[1.0, 0.0, 0.0, 1.0])); // identity
        let b = g.constant(DType::F32, Shape::from(&[1, 2]), f32_bytes(&[-1.0, -5.0])); // bias
        let zero = g.constant(DType::F32, Shape::from(&[1, 2]), f32_bytes(&[0.0, 0.0]));
        let xw = g.matmul(&x, &w);
        let xwb = g.add(&xw, &b);
        let y = g.max(&xwb, &zero);
        g.output(&y);
        let graph = g.build().expect("graph builds");

        let outputs = run_graph_on_cpu(&graph, &[f32_bytes(&[10.0, 3.0])])
            .expect("execution succeeds");

        // x @ W + b = [10, 3] + [-1, -5] = [9, -2]
        // max(0, [9, -2]) = [9, 0]
        assert_eq!(outputs.len(), 1);
        assert_eq!(from_f32_bytes(&outputs[0]), vec![9.0, 0.0]);
    }
}
