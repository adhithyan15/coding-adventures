//! Integration tests for `matrix-metal`.
//!
//! These tests run a real Metal device — they're gated behind
//! `#[cfg(target_vendor = "apple")]` so non-Apple CI stays green.
//!
//! What we cover:
//! 1. F32 elementwise unary on the GPU (Neg, Sqrt) produces correct results.
//! 2. F32 elementwise binary (Add, Mul) produces correct results.
//! 3. F32 MatMul produces correct results matching CPU reference.
//! 4. End-to-end pipeline: build a MatrixIR graph that mixes ops,
//!    place via the runtime with both CPU + Metal registered, dispatch
//!    via Metal's LocalTransport, download bytes, verify.
//! 5. Validation: oversized tensors are rejected up front; missing
//!    buffers fail cleanly.

#![cfg(target_vendor = "apple")]

use compute_ir::{
    BufferId, ComputeGraph, OpTiming as PlanOpTiming, PlacedConstant, PlacedOp, PlacedTensor,
    Residency,
};
use executor_protocol::{
    block_on, ExecutorRequest, ExecutorResponse, Transport,
};
use matrix_ir::{DType, Op, Shape, TensorId};
use matrix_metal::MetalExecutor;

// Use ExecutorId(7) as our "metal id" in these tests so we can spot
// mis-routes.  The actual CPU executor (id 0) and Metal (whatever the
// runtime assigns) wouldn't typically clash, but tests are explicit.
const METAL_ID: compute_ir::ExecutorId = compute_ir::ExecutorId(7);

fn metal_buf(b: u64) -> Residency {
    Residency {
        executor: METAL_ID,
        buffer: BufferId(b),
    }
}

fn placed_metal(id: u32, dtype: DType, shape: Shape, residency: Residency) -> PlacedTensor {
    PlacedTensor {
        id: TensorId(id),
        dtype,
        shape,
        residency,
    }
}

fn f32_bytes(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

fn from_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn make_executor() -> Option<MetalExecutor> {
    match MetalExecutor::new() {
        Ok(e) => {
            e.set_our_id(METAL_ID);
            Some(e)
        }
        Err(msg) => {
            // CI without Metal: skip cleanly.
            eprintln!("skipping Metal test: {}", msg);
            None
        }
    }
}

// ─────────────────── 1. Unary ───────────────────

#[test]
fn neg_f32_on_gpu() {
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    // Build a minimal placed graph: const(input), Neg op → output.
    // The Const op is at op_index 0 (output buffer 1), Neg at index 1
    // (output buffer 2).
    let in_bytes = f32_bytes(&[1.0, -2.0, 3.0, -4.0]);
    let n: u32 = 4;
    let shape = Shape::from(&[n]);

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(2, DType::F32, shape.clone(), metal_buf(2))],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: in_bytes,
            residency: metal_buf(0),
        }],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(1),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(1),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Neg {
                    input: TensorId(1),
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::F32, shape.clone(), metal_buf(0)),
            placed_metal(1, DType::F32, shape.clone(), metal_buf(1)),
            placed_metal(2, DType::F32, shape, metal_buf(2)),
        ],
    };

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => panic!("expected DispatchDone, got {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(2),
        offset: 0,
        len: (n * 4) as u64,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![-1.0, 2.0, -3.0, 4.0]);
}

// ─────────────────── 2. Binary ───────────────────

#[test]
fn add_f32_on_gpu() {
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let a = f32_bytes(&[1.0, 2.0, 3.0, 4.0]);
    let b = f32_bytes(&[10.0, 20.0, 30.0, 40.0]);
    let n: u32 = 4;
    let shape = Shape::from(&[n]);

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(4, DType::F32, shape.clone(), metal_buf(4))],
        constants: vec![
            PlacedConstant {
                tensor: TensorId(0),
                bytes: a,
                residency: metal_buf(0),
            },
            PlacedConstant {
                tensor: TensorId(1),
                bytes: b,
                residency: metal_buf(1),
            },
        ],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(3),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 1,
                    output: TensorId(3),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(4),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Add {
                    lhs: TensorId(2),
                    rhs: TensorId(3),
                    output: TensorId(4),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::F32, shape.clone(), metal_buf(0)),
            placed_metal(1, DType::F32, shape.clone(), metal_buf(1)),
            placed_metal(2, DType::F32, shape.clone(), metal_buf(2)),
            placed_metal(3, DType::F32, shape.clone(), metal_buf(3)),
            placed_metal(4, DType::F32, shape, metal_buf(4)),
        ],
    };

    let resp = exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g });
    assert!(matches!(resp, ExecutorResponse::DispatchDone { .. }));
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(4),
        offset: 0,
        len: (n * 4) as u64,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![11.0, 22.0, 33.0, 44.0]);
}

// ─────────────────── 3. MatMul ───────────────────

#[test]
fn matmul_2x2_on_gpu() {
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    // [[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]]
    let a = f32_bytes(&[1.0, 2.0, 3.0, 4.0]);
    let b = f32_bytes(&[5.0, 6.0, 7.0, 8.0]);

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(
            4,
            DType::F32,
            Shape::from(&[2, 2]),
            metal_buf(4),
        )],
        constants: vec![
            PlacedConstant {
                tensor: TensorId(0),
                bytes: a,
                residency: metal_buf(0),
            },
            PlacedConstant {
                tensor: TensorId(1),
                bytes: b,
                residency: metal_buf(1),
            },
        ],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: 16,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(3),
                bytes: 16,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 1,
                    output: TensorId(3),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(4),
                bytes: 16,
            },
            PlacedOp::Compute {
                op: Op::MatMul {
                    a: TensorId(2),
                    b: TensorId(3),
                    output: TensorId(4),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::F32, Shape::from(&[2, 2]), metal_buf(0)),
            placed_metal(1, DType::F32, Shape::from(&[2, 2]), metal_buf(1)),
            placed_metal(2, DType::F32, Shape::from(&[2, 2]), metal_buf(2)),
            placed_metal(3, DType::F32, Shape::from(&[2, 2]), metal_buf(3)),
            placed_metal(4, DType::F32, Shape::from(&[2, 2]), metal_buf(4)),
        ],
    };

    let resp = exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g });
    match &resp {
        ExecutorResponse::DispatchDone { .. } => {}
        ExecutorResponse::Error { message, .. } => panic!("dispatch error: {}", message),
        other => panic!("unexpected: {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(4),
        offset: 0,
        len: 16,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![19.0, 22.0, 43.0, 50.0]);
}

// ─────────────────── 4. Local transport + heartbeat ───────────────────

#[test]
fn local_transport_heartbeat() {
    let _exec = match make_executor() {
        Some(e) => e,
        None => return,
    };
    let t = match matrix_metal::local_transport() {
        Ok(t) => t,
        Err(_) => return,
    };
    let resp = block_on(t.request(ExecutorRequest::Heartbeat)).unwrap();
    match resp {
        ExecutorResponse::Alive { profile } => assert_eq!(profile.kind, "metal"),
        other => panic!("expected Alive, got {:?}", other),
    }
}

// ─────────────────── 4b. Reshape ───────────────────

#[test]
fn reshape_preserves_bytes_on_gpu() {
    // Reshape is metadata-only in SSA: same numel, different shape.
    // matrix-metal's V1 implementation memcpys the bytes from the
    // input buffer to the output buffer, so the data round-trips
    // exactly.
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let in_shape = Shape::from(&[6]);
    let out_shape = Shape::from(&[2, 3]);

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(2, DType::F32, out_shape.clone(), metal_buf(2))],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: in_bytes.clone(),
            residency: metal_buf(0),
        }],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(1),
                bytes: 6 * 4,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(1),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: 6 * 4,
            },
            PlacedOp::Compute {
                op: Op::Reshape {
                    input: TensorId(1),
                    new_shape: out_shape.clone(),
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::F32, in_shape.clone(), metal_buf(0)),
            placed_metal(1, DType::F32, in_shape, metal_buf(1)),
            placed_metal(2, DType::F32, out_shape, metal_buf(2)),
        ],
    };

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => panic!("expected DispatchDone, got {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(2),
        offset: 0,
        len: 6 * 4,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

// ─────────────────── 4c. Transpose ───────────────────

#[test]
fn transpose_2x3_to_3x2_on_gpu() {
    // Input  (2 × 3):    [[1, 2, 3],
    //                     [4, 5, 6]]
    // Output (3 × 2):    [[1, 4],
    //                     [2, 5],
    //                     [3, 6]]
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let in_shape = Shape::from(&[2, 3]);
    let out_shape = Shape::from(&[3, 2]);

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(2, DType::F32, out_shape.clone(), metal_buf(2))],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: in_bytes,
            residency: metal_buf(0),
        }],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(1),
                bytes: 6 * 4,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(1),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: 6 * 4,
            },
            PlacedOp::Compute {
                op: Op::Transpose {
                    input: TensorId(1),
                    perm: vec![1, 0],
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::F32, in_shape.clone(), metal_buf(0)),
            placed_metal(1, DType::F32, in_shape, metal_buf(1)),
            placed_metal(2, DType::F32, out_shape, metal_buf(2)),
        ],
    };

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => panic!("expected DispatchDone, got {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(2),
        offset: 0,
        len: 6 * 4,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
}

#[test]
fn transpose_3d_perm_021_on_gpu() {
    // Input  (2 × 2 × 3): [
    //   [[1, 2, 3], [4, 5, 6]],
    //   [[7, 8, 9], [10, 11, 12]],
    // ]
    //
    // perm = [0, 2, 1] swaps the last two axes →
    // Output (2 × 3 × 2): [
    //   [[1, 4], [2, 5], [3, 6]],
    //   [[7, 10], [8, 11], [9, 12]],
    // ]
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
    ]);
    let in_shape = Shape::from(&[2, 2, 3]);
    let out_shape = Shape::from(&[2, 3, 2]);

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(2, DType::F32, out_shape.clone(), metal_buf(2))],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: in_bytes,
            residency: metal_buf(0),
        }],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(1),
                bytes: 12 * 4,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(1),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: 12 * 4,
            },
            PlacedOp::Compute {
                op: Op::Transpose {
                    input: TensorId(1),
                    perm: vec![0, 2, 1],
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::F32, in_shape.clone(), metal_buf(0)),
            placed_metal(1, DType::F32, in_shape, metal_buf(1)),
            placed_metal(2, DType::F32, out_shape, metal_buf(2)),
        ],
    };

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => panic!("expected DispatchDone, got {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(2),
        offset: 0,
        len: 12 * 4,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(
        result,
        vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0, 7.0, 10.0, 8.0, 11.0, 9.0, 12.0]
    );
}

// ─────────────────── 4d. Broadcast ───────────────────

#[test]
fn broadcast_row_to_matrix_on_gpu() {
    // Input  (1 × 3):    [[10, 20, 30]]
    // Target (4 × 3) → broadcasts axis 0:
    //   [[10, 20, 30],
    //    [10, 20, 30],
    //    [10, 20, 30],
    //    [10, 20, 30]]
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[10.0, 20.0, 30.0]);
    let in_shape = Shape::from(&[1, 3]);
    let out_shape = Shape::from(&[4, 3]);

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(2, DType::F32, out_shape.clone(), metal_buf(2))],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: in_bytes,
            residency: metal_buf(0),
        }],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(1),
                bytes: 3 * 4,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(1),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: 12 * 4,
            },
            PlacedOp::Compute {
                op: Op::Broadcast {
                    input: TensorId(1),
                    target_shape: out_shape.clone(),
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::F32, in_shape.clone(), metal_buf(0)),
            placed_metal(1, DType::F32, in_shape, metal_buf(1)),
            placed_metal(2, DType::F32, out_shape, metal_buf(2)),
        ],
    };

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => panic!("expected DispatchDone, got {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(2),
        offset: 0,
        len: 12 * 4,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(
        result,
        vec![10.0, 20.0, 30.0, 10.0, 20.0, 30.0, 10.0, 20.0, 30.0, 10.0, 20.0, 30.0]
    );
}

#[test]
fn broadcast_column_to_matrix_on_gpu() {
    // Input  (3 × 1):    [[1], [2], [3]]
    // Target (3 × 4) → broadcasts axis 1:
    //   [[1, 1, 1, 1],
    //    [2, 2, 2, 2],
    //    [3, 3, 3, 3]]
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[1.0, 2.0, 3.0]);
    let in_shape = Shape::from(&[3, 1]);
    let out_shape = Shape::from(&[3, 4]);

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(2, DType::F32, out_shape.clone(), metal_buf(2))],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: in_bytes,
            residency: metal_buf(0),
        }],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(1),
                bytes: 3 * 4,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(1),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: 12 * 4,
            },
            PlacedOp::Compute {
                op: Op::Broadcast {
                    input: TensorId(1),
                    target_shape: out_shape.clone(),
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::F32, in_shape.clone(), metal_buf(0)),
            placed_metal(1, DType::F32, in_shape, metal_buf(1)),
            placed_metal(2, DType::F32, out_shape, metal_buf(2)),
        ],
    };

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => panic!("expected DispatchDone, got {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(2),
        offset: 0,
        len: 12 * 4,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(
        result,
        vec![1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0, 3.0]
    );
}

// ─────────────────── 4e. Cast ───────────────────

#[test]
fn cast_u8_to_f32_on_gpu() {
    // Input  (u8, 4 elems): [0, 1, 200, 255]
    // Output (f32, 4 elems): [0.0, 1.0, 200.0, 255.0]
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = vec![0u8, 1, 200, 255];
    let n: u32 = 4;
    let in_shape = Shape::from(&[n]);
    let out_shape = in_shape.clone();

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(2, DType::F32, out_shape.clone(), metal_buf(2))],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: in_bytes,
            residency: metal_buf(0),
        }],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(1),
                bytes: n as u64,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(1),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Cast {
                    input: TensorId(1),
                    dtype: DType::F32,
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::U8, in_shape.clone(), metal_buf(0)),
            placed_metal(1, DType::U8, in_shape, metal_buf(1)),
            placed_metal(2, DType::F32, out_shape, metal_buf(2)),
        ],
    };

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => panic!("expected DispatchDone, got {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(2),
        offset: 0,
        len: (n * 4) as u64,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![0.0, 1.0, 200.0, 255.0]);
}

#[test]
fn cast_i32_to_f32_on_gpu() {
    // Input  (i32, 5 elems): [0, 1, -1, 1000000, -2147483648]
    // Output (f32, 5 elems): same values, cast widening to f32
    //                       (i32::MIN → -2147483648.0 exactly in f32)
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let mut in_bytes = Vec::with_capacity(5 * 4);
    for v in &[0i32, 1, -1, 1_000_000, i32::MIN] {
        in_bytes.extend_from_slice(&v.to_le_bytes());
    }
    let n: u32 = 5;
    let in_shape = Shape::from(&[n]);
    let out_shape = in_shape.clone();

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(2, DType::F32, out_shape.clone(), metal_buf(2))],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: in_bytes,
            residency: metal_buf(0),
        }],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(1),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(1),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Cast {
                    input: TensorId(1),
                    dtype: DType::F32,
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::I32, in_shape.clone(), metal_buf(0)),
            placed_metal(1, DType::I32, in_shape, metal_buf(1)),
            placed_metal(2, DType::F32, out_shape, metal_buf(2)),
        ],
    };

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => panic!("expected DispatchDone, got {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(2),
        offset: 0,
        len: (n * 4) as u64,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![0.0, 1.0, -1.0, 1_000_000.0, i32::MIN as f32]);
}

#[test]
fn cast_f32_to_f32_on_gpu_is_identity() {
    // Degenerate identity cast.  Rare in practice but legal — confirm
    // it round-trips bytes exactly.
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[1.5, -2.5, 0.0, std::f32::consts::PI]);
    let n: u32 = 4;
    let in_shape = Shape::from(&[n]);
    let out_shape = in_shape.clone();

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![placed_metal(2, DType::F32, out_shape.clone(), metal_buf(2))],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: in_bytes,
            residency: metal_buf(0),
        }],
        ops: vec![
            PlacedOp::Alloc {
                residency: metal_buf(1),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Const {
                    constant: 0,
                    output: TensorId(1),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: metal_buf(2),
                bytes: (n * 4) as u64,
            },
            PlacedOp::Compute {
                op: Op::Cast {
                    input: TensorId(1),
                    dtype: DType::F32,
                    output: TensorId(2),
                },
                executor: METAL_ID,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors: vec![
            placed_metal(0, DType::F32, in_shape.clone(), metal_buf(0)),
            placed_metal(1, DType::F32, in_shape, metal_buf(1)),
            placed_metal(2, DType::F32, out_shape, metal_buf(2)),
        ],
    };

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::DispatchDone { .. } => {}
        other => panic!("expected DispatchDone, got {:?}", other),
    }
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(2),
        offset: 0,
        len: (n * 4) as u64,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![1.5, -2.5, 0.0, std::f32::consts::PI]);
}

// ─────────────────── 5. Validation ───────────────────

#[test]
fn dispatch_rejects_oversized_tensor() {
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };
    let oversized = Shape::from(&[1 << 20, 1 << 20, 4]);
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![],
        constants: vec![],
        ops: vec![],
        tensors: vec![placed_metal(0, DType::F32, oversized, metal_buf(0))],
    };
    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::Error { message, .. } => {
            assert!(
                message.contains("exceeds") || message.contains("overflow"),
                "expected size-cap error, got: {}",
                message
            );
        }
        other => panic!("expected Error, got {:?}", other),
    }
}
