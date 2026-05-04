//! The placed graph aggregate.

use crate::placement::{PlacedConstant, PlacedOp, PlacedTensor};
use matrix_ir::TensorId;

/// A complete placed compute graph — what the planner produces and
/// what executors consume.
///
/// Carries:
/// - [`format_version`](Self::format_version): wire format version, currently 1.
/// - [`inputs`](Self::inputs): tensors the runtime supplies, with
///   their starting residency.
/// - [`outputs`](Self::outputs): tensors the runtime returns, with
///   the residency they end on.
/// - [`constants`](Self::constants): literal-data tensors with
///   residency assigned (may be replicated).
/// - [`ops`](Self::ops): topologically ordered placed ops.  Order is
///   honoured during execution.
/// - [`tensors`](Self::tensors): per-tensor metadata indexed by
///   [`TensorId`].  The residency in this table is the *most recent*
///   residency (after any transfers).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ComputeGraph {
    /// Wire format version.  Currently 1.  Any other value is rejected
    /// by the validator and decoder.
    pub format_version: u32,

    /// Inputs the runtime will receive at run time, with residency
    /// assigned (typically the host / executor 0).
    pub inputs: Vec<PlacedTensor>,

    /// Outputs the runtime will return, with the residency they end on.
    pub outputs: Vec<PlacedTensor>,

    /// Constants, with residency assigned.  May be replicated across
    /// executors when the planner expects multiple readers.
    pub constants: Vec<PlacedConstant>,

    /// Topologically ordered ops.  The order is honoured during
    /// execution; even when ops are independent, the executor runs them
    /// sequentially in V1 (concurrency is a V2 optimisation).
    pub ops: Vec<PlacedOp>,

    /// Per-tensor metadata, indexed by `TensorId`.  The residency
    /// reflects the tensor's *current* location after all transfers in
    /// the graph (i.e. its end-of-graph residency).
    pub tensors: Vec<PlacedTensor>,
}

impl ComputeGraph {
    /// Look up a tensor by id.  Returns `None` if out of range.
    pub fn tensor(&self, id: TensorId) -> Option<&PlacedTensor> {
        self.tensors.get(id.0 as usize)
    }

    /// Number of tensors.
    pub fn num_tensors(&self) -> usize {
        self.tensors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::{BufferId, ExecutorId, Residency};
    use matrix_ir::{DType, Shape};

    fn empty(version: u32) -> ComputeGraph {
        ComputeGraph {
            format_version: version,
            inputs: Vec::new(),
            outputs: Vec::new(),
            constants: Vec::new(),
            ops: Vec::new(),
            tensors: Vec::new(),
        }
    }

    #[test]
    fn lookup_out_of_range_returns_none() {
        let g = empty(1);
        assert!(g.tensor(TensorId(42)).is_none());
        assert_eq!(g.num_tensors(), 0);
    }

    #[test]
    fn lookup_in_range_returns_tensor() {
        let mut g = empty(1);
        let t = PlacedTensor {
            id: TensorId(0),
            dtype: DType::F32,
            shape: Shape::from(&[3]),
            residency: Residency {
                executor: ExecutorId(0),
                buffer: BufferId(1),
            },
        };
        g.tensors.push(t.clone());
        assert_eq!(g.tensor(TensorId(0)), Some(&t));
    }
}
