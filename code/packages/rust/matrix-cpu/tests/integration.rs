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

// ─────────────────── 4. Local transport ───────────────────

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
