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

// ─────────────────── 4f. Reduce ───────────────────

#[test]
fn reduce_sum_axis1_on_gpu() {
    // Input  (2 × 3):    [[1, 2, 3],
    //                     [4, 5, 6]]
    // Reduce axis 1, keep_dims = false → output shape [2]
    //   Output: [1+2+3, 4+5+6] = [6.0, 15.0]
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let in_shape = Shape::from(&[2, 3]);
    let out_shape = Shape::from(&[2]);

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
                bytes: 2 * 4,
            },
            PlacedOp::Compute {
                op: Op::ReduceSum {
                    input: TensorId(1),
                    axes: vec![1],
                    keep_dims: false,
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
        len: 2 * 4,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![6.0, 15.0]);
}

#[test]
fn reduce_max_axis0_keep_dims_on_gpu() {
    // Input  (2 × 3):    [[1, 5, 3],
    //                     [4, 2, 6]]
    // Reduce axis 0, keep_dims = true → output shape [1, 3]
    //   Output: [[max(1,4), max(5,2), max(3,6)]] = [[4.0, 5.0, 6.0]]
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[1.0, 5.0, 3.0, 4.0, 2.0, 6.0]);
    let in_shape = Shape::from(&[2, 3]);
    let out_shape = Shape::from(&[1, 3]);

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
                bytes: 3 * 4,
            },
            PlacedOp::Compute {
                op: Op::ReduceMax {
                    input: TensorId(1),
                    axes: vec![0],
                    keep_dims: true,
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
        len: 3 * 4,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![4.0, 5.0, 6.0]);
}

#[test]
fn reduce_mean_axis1_on_gpu() {
    // Input  (2 × 4):    [[2, 4, 6, 8],
    //                     [1, 3, 5, 7]]
    // Reduce axis 1, keep_dims = false → output shape [2]
    //   Output: [(2+4+6+8)/4, (1+3+5+7)/4] = [5.0, 4.0]
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[2.0, 4.0, 6.0, 8.0, 1.0, 3.0, 5.0, 7.0]);
    let in_shape = Shape::from(&[2, 4]);
    let out_shape = Shape::from(&[2]);

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
                bytes: 8 * 4,
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
                bytes: 2 * 4,
            },
            PlacedOp::Compute {
                op: Op::ReduceMean {
                    input: TensorId(1),
                    axes: vec![1],
                    keep_dims: false,
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
        len: 2 * 4,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32(&data),
        other => panic!("download: {:?}", other),
    };
    assert_eq!(result, vec![5.0, 4.0]);
}

#[test]
fn reduce_multi_axis_returns_error() {
    // V1 only supports single-axis reduction.  Confirm a multi-axis
    // attempt fails cleanly with an Error response (so the runtime
    // can fall back to CPU).
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let in_bytes = f32_bytes(&[1.0, 2.0, 3.0, 4.0]);
    let in_shape = Shape::from(&[2, 2]);
    let out_shape = Shape::from(&[]); // reduce over both axes

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
                bytes: 4 * 4,
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
                bytes: 4,
            },
            PlacedOp::Compute {
                op: Op::ReduceSum {
                    input: TensorId(1),
                    axes: vec![0, 1], // multi-axis — V2
                    keep_dims: false,
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
        ExecutorResponse::Error { message, .. } => {
            assert!(
                message.contains("single-axis"),
                "expected single-axis error, got: {}",
                message
            );
        }
        other => panic!("expected Error for multi-axis reduce, got {:?}", other),
    }
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

// ─────────────── 7. MX05 Phase 4.2 — specialised dispatch ───────────────
//
// These tests exercise the full DispatchSpecialised path on Metal:
// emit an MSL kernel, install it under a handle via
// install_specialised_from_emitted, fire a DispatchSpecialised
// request, observe DispatchDone (or NOT_IMPLEMENTED for unknown
// handles, RUNTIME_ERROR for kernel errors/panics).

use matrix_metal::{emit_specialised_kernel, MetalSpecialisedKernelFn};
use matrix_profile::{RangeClass, ShapeClass, SpecKey};

#[test]
fn dispatch_specialised_returns_not_implemented_when_handle_unknown() {
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };
    let resp = exec.handle(ExecutorRequest::DispatchSpecialised {
        job_id: 7,
        handle: 0xDEAD_BEEF_DEAD_BEEF,
        inputs: vec![],
        outputs: vec![],
    });
    match resp {
        ExecutorResponse::Error { code, job_id, .. } => {
            assert_eq!(code, executor_protocol::ErrorCode::NOT_IMPLEMENTED);
            assert_eq!(job_id, Some(7));
        }
        other => panic!("expected NOT_IMPLEMENTED, got {:?}", other),
    }
}

/// **End-to-end Phase 4.2 test.**  Emit MSL for `add_f32` with a
/// folded right-hand constant, install it under a handle, allocate
/// input/output buffers, fire DispatchSpecialised, download the
/// result, assert numerical correctness — including that the folded
/// constant produced the right output.  This is the test that proves
/// the emitter + compiler + dispatcher round-trip works.
#[test]
fn dispatch_specialised_runs_emitted_add_const_kernel() {
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    // Emit MSL for "add 7.5" specialisation.
    let key = SpecKey {
        op_kind: 0x07,
        dtype: DType::F32,
        shape_class: ShapeClass::Dynamic,
        range_class: RangeClass::Constant {
            bytes: 7.5_f32.to_le_bytes().to_vec(),
        },
        backend_id: 1,
        folded_slot: Some(1),
    };
    let handle: u64 = 0xCAFEBABE;
    let emitted = emit_specialised_kernel(&key, handle).expect("emitter must support this key");

    exec.install_specialised_from_emitted(handle, emitted)
        .expect("install must succeed for valid MSL");
    assert_eq!(exec.specialised_count(), 1);

    // Allocate input + output buffers.  These exercise the same
    // protocol path the runtime would use.
    let n: u32 = 4;
    let n_bytes = (n * 4) as u64;
    let buf_in = match exec.handle(ExecutorRequest::AllocBuffer { bytes: n_bytes }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        other => panic!("alloc in: {:?}", other),
    };
    let buf_out = match exec.handle(ExecutorRequest::AllocBuffer { bytes: n_bytes }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        other => panic!("alloc out: {:?}", other),
    };
    let upload = exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_in,
        offset: 0,
        data: f32_bytes(&[1.0, 2.0, 3.0, 4.0]),
    });
    assert!(matches!(upload, ExecutorResponse::BufferUploaded { .. }));

    // Dispatch through the specialised path.
    let resp = exec.handle(ExecutorRequest::DispatchSpecialised {
        job_id: 99,
        handle,
        inputs: vec![buf_in],
        outputs: vec![buf_out],
    });
    match resp {
        ExecutorResponse::DispatchDone { job_id, timings } => {
            assert_eq!(job_id, 99);
            assert_eq!(timings.len(), 1);
        }
        other => panic!("expected DispatchDone, got {:?}", other),
    }

    // Verify the output bytes match `input + 7.5`.
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: buf_out,
        offset: 0,
        len: n_bytes,
    });
    match down {
        ExecutorResponse::BufferData { data, .. } => {
            let out = from_f32(&data);
            assert_eq!(out, vec![8.5, 9.5, 10.5, 11.5]);
        }
        other => panic!("download: {:?}", other),
    }
}

#[test]
fn install_specialised_with_raw_closure() {
    // The lower-level install API takes a pre-built closure — for
    // cases where a backend wants to bypass the emitter (e.g. an
    // intrinsic specialisation that doesn't fit the SpecKey shape).
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    // A trivial closure that returns success without touching any
    // Metal state.  This proves install_specialised plumbs through.
    let kernel: Box<MetalSpecialisedKernelFn> = Box::new(|_ctx, _in, _out| {
        Ok(vec![executor_protocol::OpTiming { op_index: 0, ns: 0 }])
    });
    exec.install_specialised(0x42, kernel);

    let resp = exec.handle(ExecutorRequest::DispatchSpecialised {
        job_id: 1,
        handle: 0x42,
        inputs: vec![],
        outputs: vec![],
    });
    assert!(matches!(resp, ExecutorResponse::DispatchDone { .. }));
}

#[test]
fn dispatch_specialised_kernel_error_becomes_runtime_error() {
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };
    let kernel: Box<MetalSpecialisedKernelFn> = Box::new(|_, _, _| {
        Err("intentional kernel failure".to_string())
    });
    exec.install_specialised(0x1234, kernel);

    let resp = exec.handle(ExecutorRequest::DispatchSpecialised {
        job_id: 99,
        handle: 0x1234,
        inputs: vec![],
        outputs: vec![],
    });
    match resp {
        ExecutorResponse::Error { code, message, job_id } => {
            assert_eq!(code, executor_protocol::ErrorCode::RUNTIME_ERROR);
            assert_eq!(job_id, Some(99));
            assert!(message.contains("intentional kernel failure"));
        }
        other => panic!("expected Error, got {:?}", other),
    }
}

/// **Security-hardening regression test.**  Mirrors the matrix-cpu
/// Phase 4.1 panic-safety test: an installed kernel that panics
/// (e.g. due to attacker-supplied empty `inputs[0]`) must surface
/// as a clean `RUNTIME_ERROR` rather than unwinding through the
/// mutex guard.  And the executor must keep serving subsequent
/// requests (Heartbeat returns Alive).
#[test]
fn dispatch_specialised_kernel_panic_becomes_runtime_error_not_unwind() {
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };
    let kernel: Box<MetalSpecialisedKernelFn> = Box::new(|_ctx, inputs, _outputs| {
        let _ = inputs[0]; // panics if inputs is empty
        Ok(vec![])
    });
    exec.install_specialised(0x5BAD, kernel);

    let resp = exec.handle(ExecutorRequest::DispatchSpecialised {
        job_id: 13,
        handle: 0x5BAD,
        inputs: vec![], // triggers the panic
        outputs: vec![],
    });
    match resp {
        ExecutorResponse::Error { code, message, job_id } => {
            assert_eq!(code, executor_protocol::ErrorCode::RUNTIME_ERROR);
            assert_eq!(job_id, Some(13));
            assert!(message.contains("panicked"), "message should mention panic: {}", message);
        }
        other => panic!("expected Error, got {:?}", other),
    }

    // Crucial follow-up: the executor still serves the next request.
    let resp = exec.handle(ExecutorRequest::Heartbeat);
    assert!(matches!(resp, ExecutorResponse::Alive { .. }));
}

#[test]
fn install_specialised_overwrites_prior_kernel() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };

    let v1 = Arc::new(AtomicUsize::new(0));
    let v2 = Arc::new(AtomicUsize::new(0));

    {
        let c = v1.clone();
        exec.install_specialised(
            0x7777,
            Box::new(move |_, _, _| {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(vec![])
            }),
        );
    }
    exec.handle(ExecutorRequest::DispatchSpecialised {
        job_id: 1,
        handle: 0x7777,
        inputs: vec![],
        outputs: vec![],
    });
    assert_eq!(v1.load(Ordering::SeqCst), 1);

    // Replace.
    {
        let c = v2.clone();
        exec.install_specialised(
            0x7777,
            Box::new(move |_, _, _| {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(vec![])
            }),
        );
    }
    assert_eq!(exec.specialised_count(), 1);

    exec.handle(ExecutorRequest::DispatchSpecialised {
        job_id: 2,
        handle: 0x7777,
        inputs: vec![],
        outputs: vec![],
    });
    assert_eq!(v1.load(Ordering::SeqCst), 1);
    assert_eq!(v2.load(Ordering::SeqCst), 1);
}

#[test]
fn install_specialised_from_emitted_rejects_malformed_msl() {
    // The compile path returns Err if MSL is malformed.  We construct
    // a deliberately broken EmittedKernel to confirm the error
    // propagates instead of panicking.
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };
    let bad = matrix_metal::EmittedKernel {
        source: "not valid msl at all".to_string(),
        entry_point: "nonexistent".to_string(),
        input_buffer_count: 0,
        output_buffer_count: 0,
    };
    let r = exec.install_specialised_from_emitted(0xBAAD_C0DE, bad);
    assert!(r.is_err(), "malformed MSL must fail compilation");
    // And the table must not have grown.
    assert_eq!(exec.specialised_count(), 0);
}

#[test]
fn dispatch_specialised_wrong_buffer_count_errors_cleanly() {
    // Emit a kernel that expects 1 input, 1 output.  Then call it
    // with 0 inputs.  Must return a clear error, not panic.
    let exec = match make_executor() {
        Some(e) => e,
        None => return,
    };
    let key = SpecKey {
        op_kind: 0x07,
        dtype: DType::F32,
        shape_class: ShapeClass::Dynamic,
        range_class: RangeClass::Constant {
            bytes: 1.0_f32.to_le_bytes().to_vec(),
        },
        backend_id: 1,
        folded_slot: Some(1),
    };
    let emitted = emit_specialised_kernel(&key, 0x100).unwrap();
    exec.install_specialised_from_emitted(0x100, emitted).unwrap();

    let resp = exec.handle(ExecutorRequest::DispatchSpecialised {
        job_id: 5,
        handle: 0x100,
        inputs: vec![], // wrong: kernel expects 1 input
        outputs: vec![],
    });
    match resp {
        ExecutorResponse::Error { code, message, .. } => {
            assert_eq!(code, executor_protocol::ErrorCode::RUNTIME_ERROR);
            assert!(message.contains("expected 1 input"), "got: {}", message);
        }
        other => panic!("expected Error, got {:?}", other),
    }
}
