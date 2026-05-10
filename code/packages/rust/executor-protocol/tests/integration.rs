//! Integration tests for `executor-protocol`.  Covers:
//!
//! 1. **Wire round-trip** — every variant of every message enum is
//!    encoded into a frame, decoded, and asserted equal to the original.
//! 2. **Truncation** — every wire form fails cleanly (no panic) when
//!    truncated at any byte position.
//! 3. **Forward-compat** — frames with a future format_version are
//!    rejected with `UnsupportedFrameVersion`.
//! 4. **Frame fuzzing** — 1024 deterministic random byte strings are
//!    fed to the decoder; none cause panics.
//! 5. **LocalTransport** — round-trips representative requests through
//!    a trivial echo executor.
//! 6. **Kernel cache** — the cache key is content-based and stable.

use compute_ir::{BufferId, ComputeGraph, ExecutorId, KernelId};
use executor_protocol::{
    block_on, BackendProfile, ErrorCode, ExecutorEvent, ExecutorRequest, ExecutorResponse,
    KernelCacheKey, KernelSource, LocalTransport, MessageFrame, MessageKind, OpTiming, Transport,
    WireError,
};

fn stub_profile() -> BackendProfile {
    BackendProfile {
        kind: "test".to_string(),
        supported_ops: 0xFFFF_FFFF,
        supported_dtypes: 0x07,
        gflops_f32: 100,
        gflops_u8: 50,
        gflops_i32: 75,
        host_to_device_bw: 12,
        device_to_host_bw: 11,
        device_internal_bw: 100,
        launch_overhead_ns: 1_500,
        transport_latency_ns: 0,
        on_device_mib: 8 * 1024,
        max_tensor_rank: 6,
        max_dim: 65535,
    }
}

fn stub_graph() -> ComputeGraph {
    ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: Vec::new(),
        outputs: Vec::new(),
        constants: Vec::new(),
        ops: Vec::new(),
        tensors: Vec::new(),
    }
}

// ─────────────────── 1. Wire round-trip ───────────────────

fn rt_request(req: ExecutorRequest) {
    let frame = MessageFrame::request(42, &req);
    let bytes = frame.to_bytes();
    let decoded = MessageFrame::from_bytes(&bytes).expect("frame decode");
    assert_eq!(decoded.kind, MessageKind::Request);
    assert_eq!(decoded.correlation_id, 42);
    let decoded_req = decoded.as_request().expect("request decode");
    assert_eq!(decoded_req, req);
    // Determinism.
    let bytes2 = MessageFrame::request(42, &decoded_req).to_bytes();
    assert_eq!(bytes, bytes2);
}

fn rt_response(resp: ExecutorResponse) {
    let frame = MessageFrame::response(7, &resp);
    let bytes = frame.to_bytes();
    let decoded = MessageFrame::from_bytes(&bytes).expect("frame decode");
    assert_eq!(decoded.kind, MessageKind::Response);
    let decoded_resp = decoded.as_response().expect("response decode");
    assert_eq!(decoded_resp, resp);
}

fn rt_event(ev: ExecutorEvent) {
    let frame = MessageFrame::event(&ev);
    let bytes = frame.to_bytes();
    let decoded = MessageFrame::from_bytes(&bytes).expect("frame decode");
    assert_eq!(decoded.kind, MessageKind::Event);
    assert_eq!(decoded.correlation_id, 0);
    let decoded_ev = decoded.as_event().expect("event decode");
    assert_eq!(decoded_ev, ev);
}

#[test]
fn round_trip_register() {
    rt_request(ExecutorRequest::Register {
        protocol_version: 1,
        executor_kind: "metal".to_string(),
        profile: stub_profile(),
    });
}

#[test]
fn round_trip_prepare_kernel_each_source() {
    for source in [
        KernelSource::Msl {
            code: "void k() {}".to_string(),
            entry: "k".to_string(),
        },
        KernelSource::CudaC {
            code: "extern \"C\" __global__ void k() {}".to_string(),
            entry: "k".to_string(),
        },
        KernelSource::Glsl {
            code: "void main() {}".to_string(),
            entry: "main".to_string(),
        },
        KernelSource::SpirV {
            bytes: vec![0, 1, 2, 3, 4, 5],
            entry: "main".to_string(),
        },
        KernelSource::Wgsl {
            code: "@compute fn k() {}".to_string(),
            entry: "k".to_string(),
        },
        KernelSource::OpenClC {
            code: "kernel void k() {}".to_string(),
            entry: "k".to_string(),
        },
        KernelSource::Native {
            backend: "asic-v1".to_string(),
            blob: vec![0xDE, 0xAD, 0xBE, 0xEF],
        },
    ] {
        rt_request(ExecutorRequest::PrepareKernel {
            kernel_id: KernelId(99),
            source,
        });
    }
}

#[test]
fn round_trip_alloc_buffer() {
    rt_request(ExecutorRequest::AllocBuffer { bytes: 4096 });
}

#[test]
fn round_trip_upload_buffer() {
    rt_request(ExecutorRequest::UploadBuffer {
        buffer: BufferId(7),
        offset: 0,
        data: vec![1, 2, 3, 4, 5, 6, 7, 8],
    });
}

#[test]
fn round_trip_dispatch() {
    rt_request(ExecutorRequest::Dispatch {
        job_id: 17,
        graph: stub_graph(),
    });
}

#[test]
fn round_trip_download_buffer() {
    rt_request(ExecutorRequest::DownloadBuffer {
        buffer: BufferId(7),
        offset: 64,
        len: 1024,
    });
}

#[test]
fn round_trip_free_buffer() {
    rt_request(ExecutorRequest::FreeBuffer {
        buffer: BufferId(99),
    });
}

#[test]
fn round_trip_cancel_job() {
    rt_request(ExecutorRequest::CancelJob { job_id: 42 });
}

#[test]
fn round_trip_heartbeat() {
    rt_request(ExecutorRequest::Heartbeat);
}

#[test]
fn round_trip_shutdown() {
    rt_request(ExecutorRequest::Shutdown);
}

#[test]
fn round_trip_responses_each() {
    rt_response(ExecutorResponse::Registered {
        executor_id: ExecutorId(2),
    });
    rt_response(ExecutorResponse::KernelReady {
        kernel_id: KernelId(3),
    });
    rt_response(ExecutorResponse::BufferAllocated {
        buffer: BufferId(7),
    });
    rt_response(ExecutorResponse::BufferUploaded {
        buffer: BufferId(7),
    });
    rt_response(ExecutorResponse::DispatchDone {
        job_id: 5,
        timings: vec![
            OpTiming { op_index: 0, ns: 1234 },
            OpTiming { op_index: 1, ns: 5678 },
        ],
    });
    rt_response(ExecutorResponse::BufferData {
        buffer: BufferId(9),
        data: vec![0xAA; 64],
    });
    rt_response(ExecutorResponse::BufferFreed);
    rt_response(ExecutorResponse::Cancelled { job_id: 7 });
    rt_response(ExecutorResponse::Alive {
        profile: stub_profile(),
    });
    rt_response(ExecutorResponse::ShuttingDown);
    rt_response(ExecutorResponse::Error {
        code: ErrorCode::OUT_OF_MEMORY,
        message: "oom".to_string(),
        job_id: Some(99),
    });
    rt_response(ExecutorResponse::Error {
        code: ErrorCode::COMPILATION_FAILED,
        message: "bad shader".to_string(),
        job_id: None,
    });
}

#[test]
fn round_trip_events_each() {
    rt_event(ExecutorEvent::BufferLost {
        buffer: BufferId(7),
        reason: "device reset".to_string(),
    });
    rt_event(ExecutorEvent::ProfileUpdated {
        profile: stub_profile(),
    });
    rt_event(ExecutorEvent::ShuttingDown);
}

// ─────────────────── 2. Truncation ───────────────────

#[test]
fn truncation_at_every_position_does_not_panic() {
    let frame = MessageFrame::request(
        42,
        &ExecutorRequest::Register {
            protocol_version: 1,
            executor_kind: "metal".to_string(),
            profile: stub_profile(),
        },
    );
    let bytes = frame.to_bytes();
    for cut in 0..bytes.len() {
        // Must return Err, never panic.
        let _ = MessageFrame::from_bytes(&bytes[..cut]);
    }
}

// ─────────────────── 3. Forward-compat ───────────────────

#[test]
fn unsupported_frame_version_rejected() {
    // Hand-roll a fake frame with version 99.
    let mut buf = Vec::new();
    buf.push(99); // format_version
    buf.push(0); // kind = Request
    buf.extend_from_slice(&0u64.to_le_bytes()); // correlation_id
    buf.push(0); // payload_length varint = 0
    let result = MessageFrame::from_bytes(&buf);
    assert!(matches!(
        result,
        Err(WireError::UnsupportedFrameVersion(99))
    ));
}

// ─────────────────── 4. Frame fuzzing ───────────────────

#[test]
fn fuzz_random_bytes_do_not_panic() {
    // Deterministic LCG for reproducibility; no external crate.
    let mut state: u64 = 0xCAFEBABE_DEADBEEF;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };
    for _ in 0..1024 {
        let len = (next() & 0xFF) as usize;
        let buf: Vec<u8> = (0..len).map(|_| (next() & 0xFF) as u8).collect();
        let _ = MessageFrame::from_bytes(&buf);
    }
}

// ─────────────────── 5. LocalTransport ───────────────────

#[test]
fn local_transport_alloc_round_trip() {
    let t = LocalTransport::new(|req| match req {
        ExecutorRequest::AllocBuffer { bytes } => ExecutorResponse::BufferAllocated {
            buffer: BufferId(bytes),
        },
        _ => ExecutorResponse::ShuttingDown,
    });
    let resp = block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 1024 })).unwrap();
    match resp {
        ExecutorResponse::BufferAllocated { buffer } => assert_eq!(buffer.0, 1024),
        other => panic!("unexpected: {:?}", other),
    }
}

#[test]
fn local_transport_dispatch_round_trip() {
    // Echo handler that responds to Dispatch with DispatchDone.
    let t = LocalTransport::new(|req| match req {
        ExecutorRequest::Dispatch { job_id, .. } => ExecutorResponse::DispatchDone {
            job_id,
            timings: vec![],
        },
        _ => ExecutorResponse::ShuttingDown,
    });
    let resp = block_on(t.request(ExecutorRequest::Dispatch {
        job_id: 88,
        graph: stub_graph(),
    }))
    .unwrap();
    match resp {
        ExecutorResponse::DispatchDone { job_id, .. } => assert_eq!(job_id, 88),
        other => panic!("unexpected: {:?}", other),
    }
}

#[test]
fn local_transport_heartbeat() {
    let t = LocalTransport::new(|_| ExecutorResponse::Alive {
        profile: stub_profile(),
    });
    let resp = block_on(t.request(ExecutorRequest::Heartbeat)).unwrap();
    assert!(matches!(resp, ExecutorResponse::Alive { .. }));
}

// ─────────────────── 6. Kernel cache ───────────────────

#[test]
fn cache_key_is_stable_across_calls() {
    let s = KernelSource::Msl {
        code: "void k() {}".to_string(),
        entry: "k".to_string(),
    };
    let k1 = KernelCacheKey::of(&s);
    let k2 = KernelCacheKey::of(&s);
    assert_eq!(k1, k2);
}

#[test]
fn cache_key_distinguishes_languages() {
    let msl = KernelSource::Msl {
        code: "void main() {}".to_string(),
        entry: "main".to_string(),
    };
    let cuda = KernelSource::CudaC {
        code: "void main() {}".to_string(),
        entry: "main".to_string(),
    };
    assert_ne!(KernelCacheKey::of(&msl), KernelCacheKey::of(&cuda));
}
