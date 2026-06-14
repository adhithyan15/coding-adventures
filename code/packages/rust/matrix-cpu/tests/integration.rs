//! Integration tests for `matrix-cpu`.
//!
//! Covers:
//! 1. Direct dispatch via `CpuExecutor::handle()` for representative
//!    request/response pairs.
//! 2. End-to-end pipelines: build a matrix-ir Graph, plan with
//!    matrix-runtime + CPU only, run via the LocalTransport, get
//!    outputs back, assert numerical correctness.
//! 3. Per-op verification on each supported dtype.
//! 4. Edge cases: empty graphs, missing buffers, large tensors.

use compute_ir::{BufferId, ComputeGraph, ExecutorId, OpTiming as PlanOpTiming, PlacedConstant, PlacedOp, PlacedTensor, Residency, CPU_EXECUTOR};
use executor_protocol::{
    block_on, ExecutorRequest, ExecutorResponse, LocalTransport, Transport,
};
use matrix_cpu::{local_transport, CpuExecutor};
use matrix_ir::{DType, Op, Shape, TensorId};

fn cpu_buf(b: u64) -> Residency {
    Residency {
        executor: CPU_EXECUTOR,
        buffer: BufferId(b),
    }
}

fn placed(id: u32, dtype: DType, shape: Shape, residency: Residency) -> PlacedTensor {
    PlacedTensor {
        id: TensorId(id),
        dtype,
        shape,
        residency,
    }
}

fn f32_bytes(values: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for &v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn from_f32_bytes(bytes: &[u8]) -> Vec<f32> {
    let mut out = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks(4) {
        let arr: [u8; 4] = chunk.try_into().unwrap();
        out.push(f32::from_le_bytes(arr));
    }
    out
}

// ─────────────────── 1. Direct request/response ───────────────────

#[test]
fn alloc_upload_download_round_trip() {
    let exec = CpuExecutor::new();

    // Allocate a 16-byte buffer.
    let alloc_resp = exec.handle(ExecutorRequest::AllocBuffer { bytes: 16 });
    let buf = match alloc_resp {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        other => panic!("expected BufferAllocated, got {:?}", other),
    };

    // Upload some bytes.
    let payload = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let up_resp = exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf,
        offset: 0,
        data: payload.clone(),
    });
    assert!(matches!(up_resp, ExecutorResponse::BufferUploaded { .. }));

    // Download them back.
    let down_resp = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: buf,
        offset: 0,
        len: 16,
    });
    match down_resp {
        ExecutorResponse::BufferData { data, .. } => assert_eq!(data, payload),
        other => panic!("expected BufferData, got {:?}", other),
    }
}

#[test]
fn heartbeat_returns_alive_with_profile() {
    let exec = CpuExecutor::new();
    let resp = exec.handle(ExecutorRequest::Heartbeat);
    match resp {
        ExecutorResponse::Alive { profile } => assert_eq!(profile.kind, "cpu"),
        other => panic!("expected Alive, got {:?}", other),
    }
}

#[test]
fn shutdown_returns_shutting_down() {
    let exec = CpuExecutor::new();
    let resp = exec.handle(ExecutorRequest::Shutdown);
    assert!(matches!(resp, ExecutorResponse::ShuttingDown));
}

#[test]
fn cancel_returns_cancelled() {
    let exec = CpuExecutor::new();
    let resp = exec.handle(ExecutorRequest::CancelJob { job_id: 42 });
    match resp {
        ExecutorResponse::Cancelled { job_id } => assert_eq!(job_id, 42),
        other => panic!("got {:?}", other),
    }
}

// ─────────────────── 2. Dispatch — single op ───────────────────

/// Build a graph with one Add op over two pre-uploaded f32 vectors,
/// dispatch it, and assert the result.
#[test]
fn dispatch_add_f32() {
    let exec = CpuExecutor::new();

    // Allocate three buffers.
    let buf_a = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 12 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_b = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 12 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_out = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 12 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };

    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_a,
        offset: 0,
        data: f32_bytes(&[1.0, 2.0, 3.0]),
    });
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_b,
        offset: 0,
        data: f32_bytes(&[10.0, 20.0, 30.0]),
    });

    let graph = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![
            placed(0, DType::F32, Shape::from(&[3]), cpu_buf(buf_a.0)),
            placed(1, DType::F32, Shape::from(&[3]), cpu_buf(buf_b.0)),
        ],
        outputs: vec![placed(2, DType::F32, Shape::from(&[3]), cpu_buf(buf_out.0))],
        constants: vec![],
        ops: vec![PlacedOp::Compute {
            op: Op::Add {
                lhs: TensorId(0),
                rhs: TensorId(1),
                output: TensorId(2),
            },
            executor: CPU_EXECUTOR,
            timing: PlanOpTiming { estimated_ns: 0 },
        }],
        tensors: vec![
            placed(0, DType::F32, Shape::from(&[3]), cpu_buf(buf_a.0)),
            placed(1, DType::F32, Shape::from(&[3]), cpu_buf(buf_b.0)),
            placed(2, DType::F32, Shape::from(&[3]), cpu_buf(buf_out.0)),
        ],
    };

    let resp = exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph });
    assert!(matches!(resp, ExecutorResponse::DispatchDone { .. }));

    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: buf_out,
        offset: 0,
        len: 12,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32_bytes(&data),
        other => panic!("got {:?}", other),
    };
    assert_eq!(result, vec![11.0, 22.0, 33.0]);
}

#[test]
fn dispatch_matmul_f32() {
    let exec = CpuExecutor::new();
    // [[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]]
    let buf_a = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 16 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_b = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 16 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_c = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 16 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_a,
        offset: 0,
        data: f32_bytes(&[1.0, 2.0, 3.0, 4.0]),
    });
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_b,
        offset: 0,
        data: f32_bytes(&[5.0, 6.0, 7.0, 8.0]),
    });

    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![
            placed(0, DType::F32, Shape::from(&[2, 2]), cpu_buf(buf_a.0)),
            placed(1, DType::F32, Shape::from(&[2, 2]), cpu_buf(buf_b.0)),
        ],
        outputs: vec![placed(2, DType::F32, Shape::from(&[2, 2]), cpu_buf(buf_c.0))],
        constants: vec![],
        ops: vec![PlacedOp::Compute {
            op: Op::MatMul {
                a: TensorId(0),
                b: TensorId(1),
                output: TensorId(2),
            },
            executor: CPU_EXECUTOR,
            timing: PlanOpTiming { estimated_ns: 0 },
        }],
        tensors: vec![
            placed(0, DType::F32, Shape::from(&[2, 2]), cpu_buf(buf_a.0)),
            placed(1, DType::F32, Shape::from(&[2, 2]), cpu_buf(buf_b.0)),
            placed(2, DType::F32, Shape::from(&[2, 2]), cpu_buf(buf_c.0)),
        ],
    };
    exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g });
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: buf_c,
        offset: 0,
        len: 16,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32_bytes(&data),
        _ => panic!(),
    };
    assert_eq!(result, vec![19.0, 22.0, 43.0, 50.0]);
}

#[test]
fn dispatch_with_constant() {
    let exec = CpuExecutor::new();
    let buf_x = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 12 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_out = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 12 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_const = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 12 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_x,
        offset: 0,
        data: f32_bytes(&[1.0, 2.0, 3.0]),
    });

    let const_bytes = f32_bytes(&[10.0, 10.0, 10.0]);
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![placed(0, DType::F32, Shape::from(&[3]), cpu_buf(buf_x.0))],
        outputs: vec![placed(2, DType::F32, Shape::from(&[3]), cpu_buf(buf_out.0))],
        constants: vec![PlacedConstant {
            tensor: TensorId(1),
            bytes: const_bytes,
            residency: cpu_buf(buf_const.0),
        }],
        ops: vec![PlacedOp::Compute {
            op: Op::Add {
                lhs: TensorId(0),
                rhs: TensorId(1),
                output: TensorId(2),
            },
            executor: CPU_EXECUTOR,
            timing: PlanOpTiming { estimated_ns: 0 },
        }],
        tensors: vec![
            placed(0, DType::F32, Shape::from(&[3]), cpu_buf(buf_x.0)),
            placed(1, DType::F32, Shape::from(&[3]), cpu_buf(buf_const.0)),
            placed(2, DType::F32, Shape::from(&[3]), cpu_buf(buf_out.0)),
        ],
    };
    exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g });
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: buf_out,
        offset: 0,
        len: 12,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32_bytes(&data),
        _ => panic!(),
    };
    assert_eq!(result, vec![11.0, 12.0, 13.0]);
}

#[test]
fn dispatch_reduce_sum() {
    let exec = CpuExecutor::new();
    // Sum [[1,2],[3,4]] along axis=0 → [4, 6]
    let buf_x = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 16 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_out = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 8 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_x,
        offset: 0,
        data: f32_bytes(&[1.0, 2.0, 3.0, 4.0]),
    });
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![placed(0, DType::F32, Shape::from(&[2, 2]), cpu_buf(buf_x.0))],
        outputs: vec![placed(1, DType::F32, Shape::from(&[2]), cpu_buf(buf_out.0))],
        constants: vec![],
        ops: vec![PlacedOp::Compute {
            op: Op::ReduceSum {
                input: TensorId(0),
                axes: vec![0],
                keep_dims: false,
                output: TensorId(1),
            },
            executor: CPU_EXECUTOR,
            timing: PlanOpTiming { estimated_ns: 0 },
        }],
        tensors: vec![
            placed(0, DType::F32, Shape::from(&[2, 2]), cpu_buf(buf_x.0)),
            placed(1, DType::F32, Shape::from(&[2]), cpu_buf(buf_out.0)),
        ],
    };
    exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g });
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: buf_out,
        offset: 0,
        len: 8,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32_bytes(&data),
        _ => panic!(),
    };
    assert_eq!(result, vec![4.0, 6.0]);
}

#[test]
fn dispatch_where_chooses_per_predicate() {
    let exec = CpuExecutor::new();
    let buf_p = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 4 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_t = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 16 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_f = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 16 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_out = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 16 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_p,
        offset: 0,
        data: vec![1, 0, 1, 0],
    });
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_t,
        offset: 0,
        data: f32_bytes(&[10.0, 20.0, 30.0, 40.0]),
    });
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_f,
        offset: 0,
        data: f32_bytes(&[100.0, 200.0, 300.0, 400.0]),
    });
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![
            placed(0, DType::U8, Shape::from(&[4]), cpu_buf(buf_p.0)),
            placed(1, DType::F32, Shape::from(&[4]), cpu_buf(buf_t.0)),
            placed(2, DType::F32, Shape::from(&[4]), cpu_buf(buf_f.0)),
        ],
        outputs: vec![placed(3, DType::F32, Shape::from(&[4]), cpu_buf(buf_out.0))],
        constants: vec![],
        ops: vec![PlacedOp::Compute {
            op: Op::Where {
                predicate: TensorId(0),
                true_value: TensorId(1),
                false_value: TensorId(2),
                output: TensorId(3),
            },
            executor: CPU_EXECUTOR,
            timing: PlanOpTiming { estimated_ns: 0 },
        }],
        tensors: vec![
            placed(0, DType::U8, Shape::from(&[4]), cpu_buf(buf_p.0)),
            placed(1, DType::F32, Shape::from(&[4]), cpu_buf(buf_t.0)),
            placed(2, DType::F32, Shape::from(&[4]), cpu_buf(buf_f.0)),
            placed(3, DType::F32, Shape::from(&[4]), cpu_buf(buf_out.0)),
        ],
    };
    exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g });
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: buf_out,
        offset: 0,
        len: 16,
    });
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32_bytes(&data),
        _ => panic!(),
    };
    assert_eq!(result, vec![10.0, 200.0, 30.0, 400.0]);
}

#[test]
fn dispatch_comparison_yields_u8() {
    let exec = CpuExecutor::new();
    let buf_a = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 12 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_b = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 12 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_out = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 3 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_a,
        offset: 0,
        data: f32_bytes(&[1.0, 2.0, 3.0]),
    });
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: buf_b,
        offset: 0,
        data: f32_bytes(&[1.0, 5.0, 1.0]),
    });
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![
            placed(0, DType::F32, Shape::from(&[3]), cpu_buf(buf_a.0)),
            placed(1, DType::F32, Shape::from(&[3]), cpu_buf(buf_b.0)),
        ],
        outputs: vec![placed(2, DType::U8, Shape::from(&[3]), cpu_buf(buf_out.0))],
        constants: vec![],
        ops: vec![PlacedOp::Compute {
            op: Op::Less {
                lhs: TensorId(0),
                rhs: TensorId(1),
                output: TensorId(2),
            },
            executor: CPU_EXECUTOR,
            timing: PlanOpTiming { estimated_ns: 0 },
        }],
        tensors: vec![
            placed(0, DType::F32, Shape::from(&[3]), cpu_buf(buf_a.0)),
            placed(1, DType::F32, Shape::from(&[3]), cpu_buf(buf_b.0)),
            placed(2, DType::U8, Shape::from(&[3]), cpu_buf(buf_out.0)),
        ],
    };
    exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g });
    let down = exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: buf_out,
        offset: 0,
        len: 3,
    });
    match down {
        ExecutorResponse::BufferData { data, .. } => assert_eq!(data, vec![0, 1, 0]),
        _ => panic!(),
    }
}

// ─────────────────── 3. Per-dtype unary smoke tests ───────────────────

fn unary_test(input_bytes: Vec<u8>, output_bytes_len: u64, dtype: DType, op: Op) -> Vec<u8> {
    let exec = CpuExecutor::new();
    let in_buf = match exec.handle(ExecutorRequest::AllocBuffer {
        bytes: input_bytes.len() as u64,
    }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let out_buf = match exec.handle(ExecutorRequest::AllocBuffer {
        bytes: output_bytes_len,
    }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    exec.handle(ExecutorRequest::UploadBuffer {
        buffer: in_buf,
        offset: 0,
        data: input_bytes.clone(),
    });
    let n = match dtype {
        DType::F32 => input_bytes.len() / 4,
        DType::I32 => input_bytes.len() / 4,
        DType::U8 => input_bytes.len(),
    } as u32;
    let shape = Shape::from(&[n]);
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![placed(0, dtype, shape.clone(), cpu_buf(in_buf.0))],
        outputs: vec![placed(1, dtype, shape.clone(), cpu_buf(out_buf.0))],
        constants: vec![],
        ops: vec![PlacedOp::Compute {
            op,
            executor: CPU_EXECUTOR,
            timing: PlanOpTiming { estimated_ns: 0 },
        }],
        tensors: vec![
            placed(0, dtype, shape.clone(), cpu_buf(in_buf.0)),
            placed(1, dtype, shape, cpu_buf(out_buf.0)),
        ],
    };
    exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g });
    match exec.handle(ExecutorRequest::DownloadBuffer {
        buffer: out_buf,
        offset: 0,
        len: output_bytes_len,
    }) {
        ExecutorResponse::BufferData { data, .. } => data,
        _ => panic!(),
    }
}

#[test]
fn neg_f32() {
    let result = unary_test(
        f32_bytes(&[1.0, -2.0, 3.0]),
        12,
        DType::F32,
        Op::Neg {
            input: TensorId(0),
            output: TensorId(1),
        },
    );
    assert_eq!(from_f32_bytes(&result), vec![-1.0, 2.0, -3.0]);
}

#[test]
fn abs_i32() {
    // i32 values: -5, 7, -42 → 5, 7, 42
    let mut input = Vec::new();
    for v in [-5i32, 7, -42] {
        input.extend_from_slice(&v.to_le_bytes());
    }
    let result = unary_test(
        input,
        12,
        DType::I32,
        Op::Abs {
            input: TensorId(0),
            output: TensorId(1),
        },
    );
    let mut got = Vec::new();
    for chunk in result.chunks(4) {
        let arr: [u8; 4] = chunk.try_into().unwrap();
        got.push(i32::from_le_bytes(arr));
    }
    assert_eq!(got, vec![5, 7, 42]);
}

// ─────────────────── 4. Hardening: malicious graph rejection ───────────────────

#[test]
fn dispatch_rejects_oversized_tensor() {
    // A tensor with shape [2^20, 2^20, 4] f32 declares ~4 TiB.  The
    // dispatch validator should reject this without OOM.
    let exec = CpuExecutor::new();
    let buf = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 16 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let oversized = Shape::from(&[1 << 20, 1 << 20, 4]);
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![placed(0, DType::F32, oversized.clone(), cpu_buf(buf.0))],
        outputs: vec![],
        constants: vec![],
        ops: vec![],
        tensors: vec![placed(0, DType::F32, oversized, cpu_buf(buf.0))],
    };
    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::Error { message, .. } => {
            assert!(
                message.contains("exceeds") || message.contains("overflows"),
                "expected size-cap error, got: {}",
                message
            );
        }
        other => panic!("expected Error, got {:?}", other),
    }
}

#[test]
fn dispatch_rejects_buffer_smaller_than_shape() {
    // Declare a tensor of shape [10] f32 (40 bytes) but supply only a
    // 4-byte buffer.  Validator must reject.
    let exec = CpuExecutor::new();
    let buf = match exec.handle(ExecutorRequest::AllocBuffer { bytes: 4 }) {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![placed(0, DType::F32, Shape::from(&[10]), cpu_buf(buf.0))],
        outputs: vec![],
        constants: vec![],
        ops: vec![],
        tensors: vec![placed(0, DType::F32, Shape::from(&[10]), cpu_buf(buf.0))],
    };
    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::Error { message, .. } => {
            assert!(
                message.contains("declares") || message.contains("buffer"),
                "got: {}",
                message
            );
        }
        other => panic!("expected Error, got {:?}", other),
    }
}

#[test]
fn dispatch_rejects_constant_byte_length_mismatch() {
    let exec = CpuExecutor::new();
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![],
        outputs: vec![],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            // Tensor declares [3] f32 = 12 bytes, supply 5 — must reject.
            bytes: vec![0u8; 5],
            residency: cpu_buf(99),
        }],
        ops: vec![],
        tensors: vec![placed(0, DType::F32, Shape::from(&[3]), cpu_buf(99))],
    };
    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph: g }) {
        ExecutorResponse::Error { message, .. } => {
            assert!(
                message.contains("constant") || message.contains("bytes"),
                "got: {}",
                message
            );
        }
        other => panic!("expected Error, got {:?}", other),
    }
}

// ─────────────────── 5. Local transport ───────────────────

#[test]
fn local_transport_ferries_requests() {
    let t = local_transport();
    let resp = block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 16 })).unwrap();
    assert!(matches!(resp, ExecutorResponse::BufferAllocated { .. }));
}

#[test]
fn local_transport_full_pipeline() {
    let t = local_transport();
    // Allocate.
    let buf_a = match block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 12 })).unwrap() {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_b = match block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 12 })).unwrap() {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_out = match block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 12 })).unwrap() {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };

    // Upload.
    block_on(t.request(ExecutorRequest::UploadBuffer {
        buffer: buf_a,
        offset: 0,
        data: f32_bytes(&[1.0, 2.0, 3.0]),
    }))
    .unwrap();
    block_on(t.request(ExecutorRequest::UploadBuffer {
        buffer: buf_b,
        offset: 0,
        data: f32_bytes(&[4.0, 5.0, 6.0]),
    }))
    .unwrap();

    // Dispatch.
    let g = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: vec![
            placed(0, DType::F32, Shape::from(&[3]), cpu_buf(buf_a.0)),
            placed(1, DType::F32, Shape::from(&[3]), cpu_buf(buf_b.0)),
        ],
        outputs: vec![placed(2, DType::F32, Shape::from(&[3]), cpu_buf(buf_out.0))],
        constants: vec![],
        ops: vec![PlacedOp::Compute {
            op: Op::Mul {
                lhs: TensorId(0),
                rhs: TensorId(1),
                output: TensorId(2),
            },
            executor: CPU_EXECUTOR,
            timing: PlanOpTiming { estimated_ns: 0 },
        }],
        tensors: vec![
            placed(0, DType::F32, Shape::from(&[3]), cpu_buf(buf_a.0)),
            placed(1, DType::F32, Shape::from(&[3]), cpu_buf(buf_b.0)),
            placed(2, DType::F32, Shape::from(&[3]), cpu_buf(buf_out.0)),
        ],
    };
    block_on(t.request(ExecutorRequest::Dispatch {
        job_id: 1,
        graph: g,
    }))
    .unwrap();

    // Download.
    let down = block_on(t.request(ExecutorRequest::DownloadBuffer {
        buffer: buf_out,
        offset: 0,
        len: 12,
    }))
    .unwrap();
    let result = match down {
        ExecutorResponse::BufferData { data, .. } => from_f32_bytes(&data),
        _ => panic!(),
    };
    assert_eq!(result, vec![4.0, 10.0, 18.0]);
}

// ─────────────── 6. MX05 Phase 4.1 — specialised dispatch ───────────────
//
// These tests exercise the full DispatchSpecialised path end-to-end:
// install a closure via CpuExecutor::install_specialised, fire a
// DispatchSpecialised request, observe DispatchDone (or a NOT_IMPLEMENTED
// fall-through when the handle is unknown).

use matrix_cpu::SpecialisedKernelFn;
use std::sync::Arc;

/// Helper: get a `CpuExecutor` wrapped in `Arc` so we can both install
/// kernels and pass a clone into a `LocalTransport` for request routing.
fn arc_executor() -> Arc<CpuExecutor> {
    Arc::new(CpuExecutor::new())
}

fn transport_for(exec: Arc<CpuExecutor>) -> LocalTransport {
    LocalTransport::new(move |req| exec.handle(req))
}

#[test]
fn dispatch_specialised_returns_not_implemented_when_handle_unknown() {
    // The fall-through path: no closure installed, DispatchSpecialised
    // must still answer with a recognisable NOT_IMPLEMENTED so the
    // runtime can fall back to the generic Dispatch route.
    let exec = arc_executor();
    let t = transport_for(exec);
    let resp = block_on(t.request(ExecutorRequest::DispatchSpecialised {
        job_id: 7,
        handle: 0xDEAD_BEEF_DEAD_BEEF,
        inputs: vec![],
        outputs: vec![],
    }))
    .unwrap();
    match resp {
        ExecutorResponse::Error { code, job_id, .. } => {
            assert_eq!(code, executor_protocol::ErrorCode::NOT_IMPLEMENTED);
            assert_eq!(job_id, Some(7));
        }
        other => panic!("expected NOT_IMPLEMENTED error, got {:?}", other),
    }
}

#[test]
fn dispatch_specialised_returns_dispatch_done_after_install() {
    // The happy path: install a closure under handle H, fire
    // DispatchSpecialised with handle H, get DispatchDone back.
    let exec = arc_executor();

    // Identity-copy kernel: read input[0], write input[0]'s bytes to
    // output[0].  Proves the dispatch path can actually read and
    // write tensor bytes through the BufferStore.
    let kernel: Box<SpecialisedKernelFn> = Box::new(|bufs, inputs, outputs| {
        let src = bufs.read(inputs[0], 0, 4)?;
        bufs.write(outputs[0], 0, &src)?;
        Ok(vec![executor_protocol::OpTiming { op_index: 0, ns: 0 }])
    });
    exec.install_specialised(0xC0FF_EE00_C0FF_EE00, kernel);
    assert_eq!(exec.specialised_count(), 1);

    let t = transport_for(exec.clone());

    // Allocate two buffers and seed the input.
    let buf_in = match block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 4 })).unwrap() {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_out = match block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 4 })).unwrap() {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    block_on(t.request(ExecutorRequest::UploadBuffer {
        buffer: buf_in,
        offset: 0,
        data: vec![0xAA, 0xBB, 0xCC, 0xDD],
    }))
    .unwrap();

    let resp = block_on(t.request(ExecutorRequest::DispatchSpecialised {
        job_id: 42,
        handle: 0xC0FF_EE00_C0FF_EE00,
        inputs: vec![buf_in],
        outputs: vec![buf_out],
    }))
    .unwrap();

    match resp {
        ExecutorResponse::DispatchDone { job_id, timings } => {
            assert_eq!(job_id, 42);
            assert_eq!(timings.len(), 1);
        }
        other => panic!("expected DispatchDone, got {:?}", other),
    }

    // Verify the bytes actually landed in the output buffer.
    let down = block_on(t.request(ExecutorRequest::DownloadBuffer {
        buffer: buf_out,
        offset: 0,
        len: 4,
    }))
    .unwrap();
    match down {
        ExecutorResponse::BufferData { data, .. } => {
            assert_eq!(data, vec![0xAA, 0xBB, 0xCC, 0xDD]);
        }
        _ => panic!(),
    }
}

#[test]
fn dispatch_specialised_kernel_error_becomes_runtime_error() {
    // A kernel that returns Err must surface as ErrorCode::RUNTIME_ERROR
    // with the kernel's message embedded.  Same shape as generic
    // Dispatch failures.
    let exec = arc_executor();
    let kernel: Box<SpecialisedKernelFn> = Box::new(|_, _, _| {
        Err("intentional kernel failure".to_string())
    });
    exec.install_specialised(0x1234, kernel);
    let t = transport_for(exec);

    let resp = block_on(t.request(ExecutorRequest::DispatchSpecialised {
        job_id: 99,
        handle: 0x1234,
        inputs: vec![],
        outputs: vec![],
    }))
    .unwrap();
    match resp {
        ExecutorResponse::Error { code, message, job_id } => {
            assert_eq!(code, executor_protocol::ErrorCode::RUNTIME_ERROR);
            assert_eq!(job_id, Some(99));
            assert!(message.contains("intentional kernel failure"));
        }
        other => panic!("expected Error, got {:?}", other),
    }
}

#[test]
fn install_specialised_overwrites_prior_kernel() {
    // Re-installing the same handle replaces the closure — the path
    // Phase 5 deoptimisation will use when an observed assumption fails.
    use std::sync::atomic::{AtomicUsize, Ordering};

    let exec = arc_executor();
    let v1_calls = Arc::new(AtomicUsize::new(0));
    let v2_calls = Arc::new(AtomicUsize::new(0));

    {
        let c = v1_calls.clone();
        exec.install_specialised(
            7,
            Box::new(move |_, _, _| {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(vec![])
            }),
        );
    }

    let t = transport_for(exec.clone());
    block_on(t.request(ExecutorRequest::DispatchSpecialised {
        job_id: 1,
        handle: 7,
        inputs: vec![],
        outputs: vec![],
    }))
    .unwrap();
    assert_eq!(v1_calls.load(Ordering::SeqCst), 1);
    assert_eq!(v2_calls.load(Ordering::SeqCst), 0);

    // Overwrite.
    {
        let c = v2_calls.clone();
        exec.install_specialised(
            7,
            Box::new(move |_, _, _| {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(vec![])
            }),
        );
    }
    // specialised_count stays at 1 — install replaces, doesn't accumulate.
    assert_eq!(exec.specialised_count(), 1);

    block_on(t.request(ExecutorRequest::DispatchSpecialised {
        job_id: 2,
        handle: 7,
        inputs: vec![],
        outputs: vec![],
    }))
    .unwrap();
    assert_eq!(v1_calls.load(Ordering::SeqCst), 1);
    assert_eq!(v2_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn dispatch_specialised_kernel_can_call_real_eval() {
    // A specialised kernel can wrap any logic, including replaying a
    // real matrix-ir op.  Here we install a closure that adds two
    // length-3 f32 vectors via raw byte arithmetic — the same effect
    // as Op::Add, but routed through the specialised path.  This is
    // the shape Phase 4.2's metal emitter will take, just in MSL.
    let exec = arc_executor();
    let kernel: Box<SpecialisedKernelFn> = Box::new(|bufs, inputs, outputs| {
        let a_bytes = bufs.read(inputs[0], 0, 12)?;
        let b_bytes = bufs.read(inputs[1], 0, 12)?;
        let mut out_bytes = vec![0u8; 12];
        for i in 0..3 {
            let a = f32::from_le_bytes(a_bytes[i*4..i*4+4].try_into().unwrap());
            let b = f32::from_le_bytes(b_bytes[i*4..i*4+4].try_into().unwrap());
            let c = a + b;
            out_bytes[i*4..i*4+4].copy_from_slice(&c.to_le_bytes());
        }
        bufs.write(outputs[0], 0, &out_bytes)?;
        Ok(vec![executor_protocol::OpTiming { op_index: 0, ns: 0 }])
    });
    exec.install_specialised(0xADD3F32, kernel);

    let t = transport_for(exec);
    let buf_a = match block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 12 })).unwrap() {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_b = match block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 12 })).unwrap() {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    let buf_out = match block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 12 })).unwrap() {
        ExecutorResponse::BufferAllocated { buffer } => buffer,
        _ => panic!(),
    };
    block_on(t.request(ExecutorRequest::UploadBuffer {
        buffer: buf_a,
        offset: 0,
        data: f32_bytes(&[1.0, 2.0, 3.0]),
    }))
    .unwrap();
    block_on(t.request(ExecutorRequest::UploadBuffer {
        buffer: buf_b,
        offset: 0,
        data: f32_bytes(&[10.0, 20.0, 30.0]),
    }))
    .unwrap();

    block_on(t.request(ExecutorRequest::DispatchSpecialised {
        job_id: 0,
        handle: 0xADD3F32,
        inputs: vec![buf_a, buf_b],
        outputs: vec![buf_out],
    }))
    .unwrap();

    let down = block_on(t.request(ExecutorRequest::DownloadBuffer {
        buffer: buf_out,
        offset: 0,
        len: 12,
    }))
    .unwrap();
    match down {
        ExecutorResponse::BufferData { data, .. } => {
            assert_eq!(from_f32_bytes(&data), vec![11.0, 22.0, 33.0]);
        }
        _ => panic!(),
    }
}

#[test]
fn dispatch_specialised_kernel_panic_becomes_runtime_error_not_unwind() {
    // **Security-hardening regression test.**  The doc comment on
    // `CpuExecutor::handle()` promises "a single bad request cannot
    // DoS the executor for all subsequent clients".  An installed
    // kernel that panics (e.g. due to attacker-supplied empty inputs
    // triggering an out-of-bounds index) must surface as a clean
    // `RUNTIME_ERROR` rather than unwinding through the mutex guard
    // and out of `handle()`.
    let exec = arc_executor();
    let kernel: Box<SpecialisedKernelFn> = Box::new(|_bufs, inputs, _outputs| {
        // Deliberately out-of-bounds — panics if `inputs` is empty.
        let _ = inputs[0];
        Ok(vec![])
    });
    exec.install_specialised(0x5BAD, kernel);

    let t = transport_for(exec.clone());
    let resp = block_on(t.request(ExecutorRequest::DispatchSpecialised {
        job_id: 13,
        handle: 0x5BAD,
        inputs: vec![], // ← causes the panic
        outputs: vec![],
    }))
    .unwrap();

    match resp {
        ExecutorResponse::Error { code, job_id, message } => {
            assert_eq!(code, executor_protocol::ErrorCode::RUNTIME_ERROR);
            assert_eq!(job_id, Some(13));
            assert!(message.contains("panicked"), "message should mention panic: {}", message);
        }
        other => panic!("expected Error, got {:?}", other),
    }

    // **Crucial second assertion**: the executor still serves the
    // *next* request normally.  If the panic had unwound through
    // the mutex, the lock would be permanently poisoned (or worse,
    // the process aborted) and this Heartbeat would fail.
    let resp = block_on(t.request(ExecutorRequest::Heartbeat)).unwrap();
    assert!(matches!(resp, ExecutorResponse::Alive { .. }));
}
