//! The `Graph` aggregate — a complete tensor-algebra computation.
//!
//! See spec MX01 §"Core types" for the definition.

use crate::op::Op;
use crate::tensor::{Tensor, TensorId};

/// A literal-data tensor baked into a graph.  Materialised at run time
/// via [`Op::Const`].
///
/// Bytes are stored in dtype-encoded little-endian.  For example, a
/// scalar f32 constant `3.14` is stored as `[0xC3, 0xF5, 0x48, 0x40]`.
///
/// The validator checks that `bytes.len() == tensor.shape.numel() *
/// tensor.dtype.size_bytes()`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Constant {
    /// The tensor the constant produces.  Its [`TensorId`] is the
    /// output id of the [`Op::Const`] that materialises it.
    pub tensor: Tensor,
    /// Dtype-encoded little-endian bytes.
    pub bytes: Vec<u8>,
}

/// A complete computation in `matrix-ir` form.
///
/// A graph carries:
/// - [`inputs`](Self::inputs) — tensors the caller supplies at run time.
/// - [`outputs`](Self::outputs) — tensors the caller receives back.
/// - [`ops`](Self::ops) — topologically-ordered operations.
/// - [`tensors`](Self::tensors) — every tensor referenced anywhere,
///   indexed by [`TensorId`].
/// - [`constants`](Self::constants) — literal-data tensors.
///
/// Graphs are typically constructed via [`GraphBuilder`](crate::GraphBuilder)
/// rather than by hand.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Graph {
    /// Tensors supplied by the caller at run time.  Their `TensorId`s
    /// must be the lowest contiguous block (0..N) within the graph.
    pub inputs: Vec<Tensor>,

    /// Tensors the caller wants back.  Each must be produced by an op
    /// or be an input or constant.
    pub outputs: Vec<TensorId>,

    /// Operations in topological order.  Op `i` may reference any
    /// tensor produced by op `j < i`, plus inputs and constants.
    pub ops: Vec<Op>,

    /// Every tensor referenced anywhere in the graph.  The vec is
    /// indexed by `TensorId` (i.e. `tensors[t.0 as usize]` is the
    /// tensor with id `t`).
    pub tensors: Vec<Tensor>,

    /// Literal-data tensors.  Indexed by [`Op::Const`]'s `constant`
    /// field.
    pub constants: Vec<Constant>,
}

impl Graph {
    /// Look up a tensor by id.  Returns `None` if the id is out of range.
    pub fn tensor(&self, id: TensorId) -> Option<&Tensor> {
        self.tensors.get(id.0 as usize)
    }

    /// Number of tensors in the graph (counting every defined value:
    /// inputs, constants, op outputs).
    pub fn num_tensors(&self) -> usize {
        self.tensors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::{DType, Shape};

    fn empty() -> Graph {
        Graph {
            inputs: Vec::new(),
            outputs: Vec::new(),
            ops: Vec::new(),
            tensors: Vec::new(),
            constants: Vec::new(),
        }
    }

    #[test]
    fn lookup_out_of_range_returns_none() {
        let g = empty();
        assert!(g.tensor(TensorId(42)).is_none());
    }

    #[test]
    fn lookup_in_range_returns_tensor() {
        let mut g = empty();
        let t = Tensor::new(TensorId(0), DType::F32, Shape::from(&[3]));
        g.tensors.push(t.clone());
        assert_eq!(g.tensor(TensorId(0)), Some(&t));
        assert_eq!(g.num_tensors(), 1);
    }
}
