//! Integration tests for `compute-ir`.  Covers:
//!
//! 1. **Hand-built graphs** for representative shapes (Compute,
//!    Transfer, Alloc, Free).
//! 2. **Validator acceptance** of well-formed graphs.
//! 3. **Validator rejection** of malformed graphs (transfer source
//!    mismatch, non-resident input, double-alloc, free-without-alloc,
//!    constant byte length mismatch, etc.).
//! 4. **Wire round-trip** — every test graph is serialised,
//!    deserialised, and asserted equal to the original; encoding is
//!    asserted deterministic.
//! 5. **Wire decoder hardening** — amplification attack, truncation,
//!    fuzz.
//! 6. **`dump()` golden** — small graphs whose pretty-printed form is
//!    asserted byte-for-byte against an expected fixture.

use compute_ir::{
    BufferId, ComputeGraph, ComputeIrError, ExecutorId, OpTiming, PlacedConstant, PlacedOp,
    PlacedTensor, Residency, CPU_EXECUTOR,
};
use matrix_ir::{DType, Op, Shape, TensorId};

const VERSION: u32 = compute_ir::WIRE_FORMAT_VERSION;

fn cpu_buf(b: u64) -> Residency {
    Residency {
        executor: CPU_EXECUTOR,
        buffer: BufferId(b),
    }
}

fn gpu1_buf(b: u64) -> Residency {
    Residency {
        executor: ExecutorId(1),
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

/// Construct a minimal graph: one input, one elementwise op, one output, on CPU only.
fn cpu_only_neg() -> ComputeGraph {
    // t0 is born as input at cpu_buf(0); t1 is born by the compute at cpu_buf(1).
    let t0 = placed(0, DType::F32, Shape::from(&[3]), cpu_buf(0));
    let t1 = placed(1, DType::F32, Shape::from(&[3]), cpu_buf(1));
    ComputeGraph {
        format_version: VERSION,
        inputs: vec![t0.clone()],
        outputs: vec![t1.clone()],
        constants: vec![],
        ops: vec![
            // Allocate the output buffer before the compute writes to it.
            PlacedOp::Alloc {
                residency: cpu_buf(1),
                bytes: 12,
            },
            PlacedOp::Compute {
                op: Op::Neg {
                    input: TensorId(0),
                    output: TensorId(1),
                },
                executor: CPU_EXECUTOR,
                timing: OpTiming { estimated_ns: 100 },
            },
        ],
        tensors: vec![t0, t1],
    }
}

/// Cross-executor graph: input on CPU, transferred to GPU 1, neg there,
/// transferred back, output on CPU.
fn cpu_to_gpu_neg() -> ComputeGraph {
    // tensors[] holds birth residency.
    // - t0 is born at cpu_buf(0) (where it's first placed as an input).
    // - t1 is born at gpu1_buf(11) (where the compute on GPU 1 produces it).
    // Transfers later move them around; the table is not updated by transfers.
    // outputs[] reflects end-of-graph residency: t1 ends at cpu_buf(20)
    // after the transfer-back.
    let t0_birth = placed(0, DType::F32, Shape::from(&[3]), cpu_buf(0));
    let t1_birth = placed(1, DType::F32, Shape::from(&[3]), gpu1_buf(11));
    let t1_end = placed(1, DType::F32, Shape::from(&[3]), cpu_buf(20));

    ComputeGraph {
        format_version: VERSION,
        inputs: vec![t0_birth.clone()],
        outputs: vec![t1_end],
        constants: vec![],
        ops: vec![
            // Allocate destination buffer on GPU before transfer-in.
            PlacedOp::Alloc {
                residency: gpu1_buf(10),
                bytes: 12,
            },
            PlacedOp::Transfer {
                tensor: TensorId(0),
                src: cpu_buf(0),
                dst: gpu1_buf(10),
                bytes: 12,
                timing: OpTiming { estimated_ns: 5_000 },
            },
            // Allocate output buffer on GPU before the compute.
            PlacedOp::Alloc {
                residency: gpu1_buf(11),
                bytes: 12,
            },
            // Compute on GPU.  Output (t1) is born at gpu1_buf(11).
            PlacedOp::Compute {
                op: Op::Neg {
                    input: TensorId(0),
                    output: TensorId(1),
                },
                executor: ExecutorId(1),
                timing: OpTiming { estimated_ns: 500 },
            },
            // Allocate destination on CPU and transfer t1 back.
            PlacedOp::Alloc {
                residency: cpu_buf(20),
                bytes: 12,
            },
            PlacedOp::Transfer {
                tensor: TensorId(1),
                src: gpu1_buf(11),
                dst: cpu_buf(20),
                bytes: 12,
                timing: OpTiming { estimated_ns: 5_000 },
            },
            // Free the GPU buffers.
            PlacedOp::Free { residency: gpu1_buf(10) },
            PlacedOp::Free { residency: gpu1_buf(11) },
        ],
        tensors: vec![t0_birth, t1_birth],
    }
}

// ─────────────────── Validator acceptance ───────────────────

#[test]
fn validate_cpu_only_neg() {
    let g = cpu_only_neg();
    g.validate().expect("should validate");
}

#[test]
fn validate_cpu_to_gpu_neg() {
    let g = cpu_to_gpu_neg();
    g.validate().expect("should validate");
}

// ─────────────────── Validator rejection ───────────────────

#[test]
fn rejects_transfer_with_wrong_source() {
    let mut g = cpu_to_gpu_neg();
    // Mutate the first transfer's src to a wrong residency.
    if let PlacedOp::Transfer { src, .. } = &mut g.ops[1] {
        *src = gpu1_buf(99); // tensor is not actually here
    }
    assert!(matches!(
        g.validate(),
        Err(ComputeIrError::TransferSourceMismatch { .. })
    ));
}

#[test]
fn rejects_compute_with_input_on_wrong_executor() {
    // Input on CPU, compute on GPU, no transfer between.
    let g = ComputeGraph {
        format_version: VERSION,
        inputs: vec![placed(0, DType::F32, Shape::from(&[3]), cpu_buf(0))],
        outputs: vec![],
        constants: vec![],
        ops: vec![PlacedOp::Compute {
            op: Op::Neg {
                input: TensorId(0),
                output: TensorId(1),
            },
            executor: ExecutorId(1),
            timing: OpTiming { estimated_ns: 0 },
        }],
        tensors: vec![
            placed(0, DType::F32, Shape::from(&[3]), cpu_buf(0)),
            placed(1, DType::F32, Shape::from(&[3]), gpu1_buf(0)),
        ],
    };
    assert!(matches!(
        g.validate(),
        Err(ComputeIrError::InputNotResident { .. })
    ));
}

#[test]
fn rejects_double_alloc() {
    let g = ComputeGraph {
        format_version: VERSION,
        inputs: vec![],
        outputs: vec![],
        constants: vec![],
        ops: vec![
            PlacedOp::Alloc {
                residency: gpu1_buf(7),
                bytes: 32,
            },
            PlacedOp::Alloc {
                residency: gpu1_buf(7),
                bytes: 32,
            },
        ],
        tensors: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(ComputeIrError::AllocAlreadyAllocated { .. })
    ));
}

#[test]
fn rejects_free_without_alloc() {
    let g = ComputeGraph {
        format_version: VERSION,
        inputs: vec![],
        outputs: vec![],
        constants: vec![],
        ops: vec![PlacedOp::Free {
            residency: gpu1_buf(7),
        }],
        tensors: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(ComputeIrError::FreeUnallocated { .. })
    ));
}

#[test]
fn rejects_unsupported_format_version() {
    let g = ComputeGraph {
        format_version: 999,
        inputs: vec![],
        outputs: vec![],
        constants: vec![],
        ops: vec![],
        tensors: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(ComputeIrError::UnsupportedFormatVersion(999))
    ));
}

#[test]
fn rejects_tensor_id_mismatch() {
    let g = ComputeGraph {
        format_version: VERSION,
        inputs: vec![],
        outputs: vec![],
        constants: vec![],
        ops: vec![],
        tensors: vec![placed(7, DType::F32, Shape::from(&[1]), cpu_buf(0))],
    };
    assert!(matches!(
        g.validate(),
        Err(ComputeIrError::TensorIdMismatch { .. })
    ));
}

#[test]
fn rejects_constant_byte_length_mismatch() {
    let g = ComputeGraph {
        format_version: VERSION,
        inputs: vec![],
        outputs: vec![],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: vec![0u8; 5], // should be 8 (2 × f32)
            residency: cpu_buf(0),
        }],
        ops: vec![],
        tensors: vec![placed(0, DType::F32, Shape::from(&[2]), cpu_buf(0))],
    };
    assert!(matches!(
        g.validate(),
        Err(ComputeIrError::ConstantByteLength {
            expected: 8,
            actual: 5,
            ..
        })
    ));
}

// ─────────────────── Wire round-trip ───────────────────

fn round_trip(g: &ComputeGraph) {
    let bytes = g.to_bytes();
    let decoded = ComputeGraph::from_bytes(&bytes).expect("decode should succeed");
    assert_eq!(&decoded, g, "round-trip changed the graph");
    let bytes2 = decoded.to_bytes();
    assert_eq!(bytes, bytes2, "encoding is not deterministic");
}

#[test]
fn round_trip_cpu_only_neg() {
    round_trip(&cpu_only_neg());
}

#[test]
fn round_trip_cpu_to_gpu_neg() {
    round_trip(&cpu_to_gpu_neg());
}

#[test]
fn round_trip_with_constants() {
    let g = ComputeGraph {
        format_version: VERSION,
        inputs: vec![],
        outputs: vec![],
        constants: vec![PlacedConstant {
            tensor: TensorId(0),
            bytes: vec![1, 2, 3, 4, 5, 6, 7, 8],
            residency: cpu_buf(0),
        }],
        ops: vec![],
        tensors: vec![placed(0, DType::F32, Shape::from(&[2]), cpu_buf(0))],
    };
    round_trip(&g);
}

#[test]
fn round_trip_all_placed_op_variants() {
    let g = ComputeGraph {
        format_version: VERSION,
        inputs: vec![],
        outputs: vec![],
        constants: vec![],
        ops: vec![
            PlacedOp::Alloc {
                residency: cpu_buf(0),
                bytes: 16,
            },
            PlacedOp::Compute {
                op: Op::MatMul {
                    a: TensorId(0),
                    b: TensorId(1),
                    output: TensorId(2),
                },
                executor: CPU_EXECUTOR,
                timing: OpTiming { estimated_ns: 5_000 },
            },
            PlacedOp::Transfer {
                tensor: TensorId(0),
                src: cpu_buf(0),
                dst: gpu1_buf(0),
                bytes: 16,
                timing: OpTiming { estimated_ns: 1_000 },
            },
            PlacedOp::Free {
                residency: cpu_buf(0),
            },
        ],
        tensors: vec![],
    };
    round_trip(&g);
}

// ─────────────────── Wire decoder hardening ───────────────────

#[test]
fn decoder_amplification_attack_is_bounded() {
    // u32 version + 10-byte uv64 of u64::MAX, then nothing.
    let mut buf = Vec::new();
    buf.extend_from_slice(&VERSION.to_le_bytes());
    let mut v = u64::MAX;
    while v >= 0x80 {
        buf.push(((v as u8) & 0x7F) | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
    let result = ComputeGraph::from_bytes(&buf);
    assert!(matches!(result, Err(ComputeIrError::WireUnexpectedEof)));
}

#[test]
fn truncation_at_every_position_does_not_panic() {
    let g = cpu_to_gpu_neg();
    let bytes = g.to_bytes();
    for cut in 0..bytes.len() {
        let truncated = &bytes[..cut];
        let _ = ComputeGraph::from_bytes(truncated);
    }
}

#[test]
fn fuzz_random_bytes_do_not_panic() {
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
        let _ = ComputeGraph::from_bytes(&buf);
    }
}

// ─────────────────── dump() smoke test ───────────────────

#[test]
fn dump_is_human_readable() {
    let g = cpu_to_gpu_neg();
    let s = g.dump();
    // Spot-check a few expected fragments rather than full golden equality.
    // Full golden tests are deferred until the format stabilises.
    assert!(s.contains("ComputeGraph"));
    assert!(s.contains("format v"));
    assert!(s.contains("transfer"));
    assert!(s.contains("compute"));
    assert!(s.contains("alloc"));
    assert!(s.contains("free"));
    assert!(s.contains("≈"));
}

#[test]
fn dump_byte_units_visible() {
    let g = ComputeGraph {
        format_version: VERSION,
        inputs: vec![],
        outputs: vec![],
        constants: vec![],
        ops: vec![PlacedOp::Alloc {
            residency: cpu_buf(0),
            bytes: 4096,
        }],
        tensors: vec![],
    };
    let s = g.dump();
    // 4096 bytes = 4.0 KiB.
    assert!(s.contains("4.0 KiB"), "expected '4.0 KiB' in dump, got: {}", s);
}
