//! Placement primitives — [`ExecutorId`], [`BufferId`], [`KernelId`],
//! [`Residency`], [`PlacedTensor`], [`PlacedConstant`], [`PlacedOp`],
//! [`OpTiming`].  See spec MX02 §"Core types".

use core::fmt;
use matrix_ir::{DType, Op, Shape, TensorId};

/// A unique identifier for an executor in the runtime registry.
///
/// `ExecutorId(0)` is by convention the CPU executor and is always
/// present (see [`CPU_EXECUTOR`]).  Other ids are assigned by the
/// runtime as backends register themselves.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ExecutorId(pub u32);

/// The CPU fallback executor's id.  Always available; supports every
/// op on every dtype so the planner can route to it whenever a
/// specialised backend can't take an op.
pub const CPU_EXECUTOR: ExecutorId = ExecutorId(0);

/// A buffer identifier within an executor.  Unique per executor; the
/// same numeric value on two different executors is unrelated.
///
/// 64-bit even though 32 bits is plausibly enough — widening the wire
/// format later would be painful.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BufferId(pub u64);

/// A compiled-kernel identifier within an executor.  Same uniqueness
/// rules as [`BufferId`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct KernelId(pub u64);

/// Where a tensor currently lives.  The fundamental unit of "memory
/// location" in the placed graph.  Tensors can change residency over
/// the course of a graph as transfers move them.
///
/// Two `Residency` values are equal iff both their executor and buffer
/// match — which means the same `BufferId` on different executors
/// represents different physical memory.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Residency {
    /// The executor whose memory holds the tensor.
    pub executor: ExecutorId,
    /// The buffer id within that executor.
    pub buffer: BufferId,
}

impl fmt::Display for Residency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exec {} buf {}", self.executor.0, self.buffer.0)
    }
}

/// A tensor in the placed graph.  Carries dtype, shape, and a
/// residency annotation.  Same `id` space as [`matrix_ir::Tensor`] —
/// the placed graph uses the same `TensorId`s the upstream
/// [`Op`]s reference.
///
/// The meaning of `residency` depends on the context:
///
/// - In [`ComputeGraph::tensors`] (the per-tensor metadata table):
///   the tensor's **birth residency** — where the tensor is first
///   created.  Inputs are born at the residency the runtime placed
///   them at; constants are born wherever they're uploaded; compute
///   outputs are born at the executor that ran the compute, in the
///   buffer the planner allocated for them.  Transfers do **not**
///   update this field; they update the runtime-side `current
///   residency` tracking.
/// - In [`ComputeGraph::inputs`]: the **starting residency** the
///   runtime places the input at when execution begins.
/// - In [`ComputeGraph::outputs`]: the **end-of-graph residency** the
///   tensor lives at when the runtime returns.  Often this is back on
///   the host (`CPU_EXECUTOR`).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PlacedTensor {
    /// Tensor id (shared with the matrix-ir graph this was lowered from).
    pub id: TensorId,
    /// Element type.
    pub dtype: DType,
    /// Static shape.
    pub shape: Shape,
    /// Residency annotation.  Meaning varies by where this struct
    /// appears in the graph; see the type-level docs.
    pub residency: Residency,
}

/// A literal-data constant placed on a specific executor.  Multiple
/// `PlacedConstant`s may share a [`TensorId`] when the planner has
/// chosen to replicate a constant across multiple executors that read
/// it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlacedConstant {
    /// The tensor id this constant materialises.
    pub tensor: TensorId,
    /// Dtype-encoded little-endian bytes.  Length must equal
    /// `shape.numel() * dtype.size_bytes()`.
    pub bytes: Vec<u8>,
    /// Where this copy of the constant must be uploaded.
    pub residency: Residency,
}

/// A telemetry annotation: how long the planner *estimated* an op
/// would take on its assigned executor.  Kept on the placed op so
/// inspection (`dump()`) can show the cost numbers that drove the
/// placement decision.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct OpTiming {
    /// Estimated nanoseconds.  Zero is a valid value for ops below the
    /// planner's resolution threshold.
    pub estimated_ns: u64,
}

/// One node in the placed graph.  See spec MX02 §"Core types" for
/// the rationale behind the four variants.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PlacedOp {
    /// A normal compute op, lowered as-is from `matrix-ir`.  All
    /// inputs of `op` must be resident on `executor` at the time this
    /// op runs (transfers preceding it ensure that).
    Compute {
        /// The underlying matrix-ir op.
        op: Op,
        /// Which executor runs this op.
        executor: ExecutorId,
        /// Cost annotation (estimate, not authoritative).
        timing: OpTiming,
    },

    /// Move a tensor between two residencies.  The runtime executes
    /// the transfer (potentially routing through host memory) — see
    /// MX02 §"The `Transfer` op in detail".
    Transfer {
        /// The tensor being moved.  Identifies the same logical value
        /// before and after the transfer.
        tensor: TensorId,
        /// Source residency.  Must equal the tensor's current residency
        /// at the point this op runs.
        src: Residency,
        /// Destination residency.  Becomes the tensor's new residency
        /// after this op.
        dst: Residency,
        /// How many bytes the transfer moves.  Equal to
        /// `shape.numel() * dtype.size_bytes()`.
        bytes: u64,
        /// Cost annotation.
        timing: OpTiming,
    },

    /// Allocate the buffer for a tensor.  Issued before the first op
    /// that reads or writes it.  Executors that manage their own
    /// allocations may treat `Alloc` as a no-op without affecting
    /// correctness.
    Alloc {
        /// What to allocate.
        residency: Residency,
        /// Size in bytes.
        bytes: u64,
    },

    /// Release a buffer.  Issued after the tensor's last use.
    /// Executors that ignore `Free` still produce correct results,
    /// just with more memory pressure.
    Free {
        /// Which buffer to release.
        residency: Residency,
    },
}

impl PlacedOp {
    /// Wire-format tag for this variant.  Stable across versions.
    pub const fn wire_tag(&self) -> u8 {
        match self {
            PlacedOp::Compute { .. } => 0x00,
            PlacedOp::Transfer { .. } => 0x01,
            PlacedOp::Alloc { .. } => 0x02,
            PlacedOp::Free { .. } => 0x03,
        }
    }

    /// True iff this op is a compute step (not a memory-management op).
    pub const fn is_compute(&self) -> bool {
        matches!(self, PlacedOp::Compute { .. })
    }

    /// True iff this op moves bytes between executors.
    pub const fn is_transfer(&self) -> bool {
        matches!(self, PlacedOp::Transfer { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_ir::TensorId;

    #[test]
    fn cpu_executor_is_zero() {
        assert_eq!(CPU_EXECUTOR, ExecutorId(0));
    }

    #[test]
    fn residency_equality() {
        let a = Residency {
            executor: ExecutorId(1),
            buffer: BufferId(7),
        };
        let b = Residency {
            executor: ExecutorId(1),
            buffer: BufferId(7),
        };
        assert_eq!(a, b);
        // Different executor with same buffer id must compare unequal —
        // BufferIds are scoped to their executor.
        let c = Residency {
            executor: ExecutorId(2),
            buffer: BufferId(7),
        };
        assert_ne!(a, c);
    }

    #[test]
    fn placed_op_classifiers() {
        let compute = PlacedOp::Compute {
            op: Op::Neg {
                input: TensorId(0),
                output: TensorId(1),
            },
            executor: ExecutorId(1),
            timing: OpTiming { estimated_ns: 10 },
        };
        let transfer = PlacedOp::Transfer {
            tensor: TensorId(0),
            src: Residency {
                executor: ExecutorId(0),
                buffer: BufferId(1),
            },
            dst: Residency {
                executor: ExecutorId(1),
                buffer: BufferId(2),
            },
            bytes: 16,
            timing: OpTiming { estimated_ns: 100 },
        };
        let alloc = PlacedOp::Alloc {
            residency: Residency {
                executor: ExecutorId(1),
                buffer: BufferId(3),
            },
            bytes: 16,
        };
        let free = PlacedOp::Free {
            residency: Residency {
                executor: ExecutorId(1),
                buffer: BufferId(3),
            },
        };
        assert!(compute.is_compute());
        assert!(!compute.is_transfer());
        assert!(transfer.is_transfer());
        assert!(!alloc.is_compute());
        assert!(!free.is_transfer());
    }

    #[test]
    fn placed_op_wire_tags_unique() {
        let ops = [
            PlacedOp::Compute {
                op: Op::Neg {
                    input: TensorId(0),
                    output: TensorId(1),
                },
                executor: ExecutorId(0),
                timing: OpTiming { estimated_ns: 0 },
            },
            PlacedOp::Transfer {
                tensor: TensorId(0),
                src: Residency {
                    executor: ExecutorId(0),
                    buffer: BufferId(0),
                },
                dst: Residency {
                    executor: ExecutorId(1),
                    buffer: BufferId(0),
                },
                bytes: 0,
                timing: OpTiming { estimated_ns: 0 },
            },
            PlacedOp::Alloc {
                residency: Residency {
                    executor: ExecutorId(0),
                    buffer: BufferId(0),
                },
                bytes: 0,
            },
            PlacedOp::Free {
                residency: Residency {
                    executor: ExecutorId(0),
                    buffer: BufferId(0),
                },
            },
        ];
        let mut tags: Vec<u8> = ops.iter().map(|o| o.wire_tag()).collect();
        tags.sort();
        let len = tags.len();
        tags.dedup();
        assert_eq!(tags.len(), len);
    }
}
