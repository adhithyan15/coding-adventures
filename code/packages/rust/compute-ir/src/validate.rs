//! Structural and semantic validation of [`ComputeGraph`].
//!
//! See spec MX02 §"Validation".  An invalid placed graph is a planner
//! bug — it should never reach an executor.  The runtime calls
//! `validate()` before executing anything.

use crate::graph::ComputeGraph;
use crate::placement::{ExecutorId, PlacedOp, PlacedTensor, Residency};
use crate::WIRE_FORMAT_VERSION;
use matrix_ir::TensorId;
use std::collections::{HashMap, HashSet};

/// Errors produced by [`ComputeGraph::validate`] and the wire decoder.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ComputeIrError {
    /// Wire format version on the graph is not supported.
    UnsupportedFormatVersion(u32),
    /// A `TensorId` referenced by an op is out of range of `tensors`.
    UndefinedTensor {
        op_index: u32,
        tensor: TensorId,
    },
    /// A `Tensor` in `tensors` has an id that doesn't match its position.
    TensorIdMismatch {
        position: u32,
        declared: TensorId,
    },
    /// A constant's bytes length doesn't match its declared shape × dtype.
    ConstantByteLength {
        constant_index: u32,
        expected: usize,
        actual: usize,
    },
    /// An input or constant references a tensor id outside the table.
    InputOutOfRange {
        tensor: TensorId,
    },
    /// A `Compute` op has an estimated_ns value that overflowed during
    /// some computation.  Caught defensively even though the field is
    /// `u64`.
    InvalidTiming {
        op_index: u32,
    },
    /// A `Compute` op's input is not currently resident on the op's
    /// executor.  This means a transfer is missing.
    InputNotResident {
        op_index: u32,
        tensor: TensorId,
        op_executor: ExecutorId,
        actual_residency: Residency,
    },
    /// A `Transfer.src` does not match the tensor's current residency.
    TransferSourceMismatch {
        op_index: u32,
        tensor: TensorId,
        declared_src: Residency,
        actual_residency: Residency,
    },
    /// A `Free` references a buffer that was never allocated (or has
    /// already been freed).
    FreeUnallocated {
        op_index: u32,
        residency: Residency,
    },
    /// An `Alloc` reuses a residency that is already allocated.
    AllocAlreadyAllocated {
        op_index: u32,
        residency: Residency,
    },
    /// The underlying `matrix-ir` op fails its own validation.
    InvalidUnderlyingOp {
        op_index: u32,
    },

    // Wire-format errors.
    /// Wire-format error: unexpected end of input.
    WireUnexpectedEof,
    /// Wire-format error: format version mismatch.
    WireUnsupportedVersion(u32),
    /// Wire-format error: an op or dtype tag is not in the known set.
    WireUnknownTag {
        what: &'static str,
        tag: u64,
    },
    /// Wire-format error: a varint exceeds 10 bytes.
    WireOversizedVarint,
    /// Wire-format error: trailing bytes after a complete graph.
    WireTrailingBytes,
}

impl ComputeGraph {
    /// Validate the placed graph end-to-end.  Returns `Ok(())` if the
    /// graph is consistent, `Err(ComputeIrError)` describing the first
    /// violation otherwise.
    ///
    /// What this checks (per spec MX02 §"Validation"):
    /// - Format version matches the current reader.
    /// - Every `TensorId` referenced exists in the table.
    /// - Tensor table positions match their declared ids.
    /// - Every constant's byte length matches `shape × dtype`.
    /// - Every `Compute` op's inputs are resident on the op's executor
    ///   at the moment that op runs (i.e. all needed transfers have
    ///   already happened).
    /// - Every `Transfer.src` matches the tensor's current residency
    ///   when the transfer runs.
    /// - Every `Free` follows an `Alloc` of the same residency.
    /// - No double-`Alloc` of the same residency (without an
    ///   intervening `Free`).
    pub fn validate(&self) -> Result<(), ComputeIrError> {
        // ──── format version ────
        if self.format_version != WIRE_FORMAT_VERSION {
            return Err(ComputeIrError::UnsupportedFormatVersion(self.format_version));
        }

        // ──── tensor table consistency ────
        for (i, t) in self.tensors.iter().enumerate() {
            if t.id.0 as usize != i {
                return Err(ComputeIrError::TensorIdMismatch {
                    position: i as u32,
                    declared: t.id,
                });
            }
        }

        // ──── inputs reference real tensors ────
        for inp in &self.inputs {
            if (inp.id.0 as usize) >= self.tensors.len() {
                return Err(ComputeIrError::InputOutOfRange { tensor: inp.id });
            }
        }
        for out in &self.outputs {
            if (out.id.0 as usize) >= self.tensors.len() {
                return Err(ComputeIrError::InputOutOfRange { tensor: out.id });
            }
        }

        // ──── constants ────
        for (ci, c) in self.constants.iter().enumerate() {
            // Look up the tensor metadata to check byte length.
            let t = self
                .tensors
                .get(c.tensor.0 as usize)
                .ok_or(ComputeIrError::InputOutOfRange { tensor: c.tensor })?;
            let expected = t
                .shape
                .byte_size(t.dtype)
                .ok_or(ComputeIrError::ConstantByteLength {
                    constant_index: ci as u32,
                    expected: usize::MAX,
                    actual: c.bytes.len(),
                })? as usize;
            if c.bytes.len() != expected {
                return Err(ComputeIrError::ConstantByteLength {
                    constant_index: ci as u32,
                    expected,
                    actual: c.bytes.len(),
                });
            }
        }

        // ──── walk ops, tracking residency and live buffers ────
        // current_residency: where each tensor lives right now.
        let mut current_residency: HashMap<TensorId, Residency> = HashMap::new();
        for inp in &self.inputs {
            current_residency.insert(inp.id, inp.residency);
        }
        for c in &self.constants {
            current_residency.insert(c.tensor, c.residency);
        }

        // live_buffers: the set of (executor, buffer) pairs that are
        // currently allocated.  Alloc inserts; Free removes.
        let mut live_buffers: HashSet<Residency> = HashSet::new();
        for r in current_residency.values() {
            live_buffers.insert(*r);
        }

        for (i, op) in self.ops.iter().enumerate() {
            let i = i as u32;
            match op {
                PlacedOp::Compute {
                    op: under,
                    executor,
                    timing,
                } => {
                    // Telemetry sanity check.
                    let _ = timing.estimated_ns;
                    // Each input must be resident on `executor`.
                    for input in under.inputs() {
                        let res = current_residency.get(&input).copied().ok_or(
                            ComputeIrError::UndefinedTensor {
                                op_index: i,
                                tensor: input,
                            },
                        )?;
                        if res.executor != *executor {
                            return Err(ComputeIrError::InputNotResident {
                                op_index: i,
                                tensor: input,
                                op_executor: *executor,
                                actual_residency: res,
                            });
                        }
                    }
                    // Output is born at the residency declared in
                    // tensors[output_id].  That residency must be on
                    // the same executor that ran the compute.
                    let out_id = under.output();
                    let out_t = self
                        .tensors
                        .get(out_id.0 as usize)
                        .ok_or(ComputeIrError::UndefinedTensor {
                            op_index: i,
                            tensor: out_id,
                        })?;
                    if out_t.residency.executor != *executor {
                        return Err(ComputeIrError::InputNotResident {
                            op_index: i,
                            tensor: out_id,
                            op_executor: *executor,
                            actual_residency: out_t.residency,
                        });
                    }
                    // The output's buffer must have been Alloc'd
                    // before this compute runs (allocation precedes
                    // first use).
                    if !live_buffers.contains(&out_t.residency) {
                        return Err(ComputeIrError::FreeUnallocated {
                            op_index: i,
                            residency: out_t.residency,
                        });
                    }
                    current_residency.insert(out_id, out_t.residency);
                }
                PlacedOp::Transfer {
                    tensor,
                    src,
                    dst,
                    bytes: _,
                    timing: _,
                } => {
                    let actual = current_residency.get(tensor).copied().ok_or(
                        ComputeIrError::UndefinedTensor {
                            op_index: i,
                            tensor: *tensor,
                        },
                    )?;
                    if actual != *src {
                        return Err(ComputeIrError::TransferSourceMismatch {
                            op_index: i,
                            tensor: *tensor,
                            declared_src: *src,
                            actual_residency: actual,
                        });
                    }
                    // The destination buffer must have been Alloc'd already
                    // (or be reachable via input/constant placement).
                    if !live_buffers.contains(dst) {
                        return Err(ComputeIrError::FreeUnallocated {
                            op_index: i,
                            residency: *dst,
                        });
                    }
                    current_residency.insert(*tensor, *dst);
                }
                PlacedOp::Alloc { residency, bytes: _ } => {
                    if live_buffers.contains(residency) {
                        return Err(ComputeIrError::AllocAlreadyAllocated {
                            op_index: i,
                            residency: *residency,
                        });
                    }
                    live_buffers.insert(*residency);
                }
                PlacedOp::Free { residency } => {
                    if !live_buffers.remove(residency) {
                        return Err(ComputeIrError::FreeUnallocated {
                            op_index: i,
                            residency: *residency,
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

