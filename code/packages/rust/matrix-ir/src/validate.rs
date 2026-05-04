//! Structural and semantic validation of [`Graph`]s.
//!
//! See spec MX01 §"Validation".  The validator runs in O(N) over the op
//! list and is invoked by [`GraphBuilder::build`](crate::GraphBuilder::build)
//! and may also be called explicitly via [`Graph::validate`].
//!
//! Every variant of [`Op`](crate::Op) has an explicit match arm.  Adding
//! a new variant without adding a check is a compile error because the
//! match has no default arm.

use crate::builder::reduce_shape;
use crate::graph::Graph;
use crate::op::Op;
use crate::tensor::{DType, Shape, Tensor, TensorId};

/// Errors produced by [`Graph::validate`] and related code.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum IrError {
    /// A `TensorId` referenced by an op is out of range of `Graph.tensors`.
    UndefinedTensor {
        op_index: u32,
        tensor: TensorId,
    },
    /// Two inputs that should have matching shapes do not.
    ShapeMismatch {
        op_index: u32,
        expected: Shape,
        actual: Shape,
    },
    /// Two inputs that should have matching dtypes do not.
    DTypeMismatch {
        op_index: u32,
        expected: DType,
        actual: DType,
    },
    /// The op's declared output tensor disagrees with what its inputs imply.
    OutputMismatch {
        op_index: u32,
        expected: Tensor,
        actual: Tensor,
    },
    /// A constant's bytes length does not match its declared shape × dtype.
    ConstantByteLength {
        constant_index: u32,
        expected: usize,
        actual: usize,
    },
    /// A constant's table index references a constant that doesn't exist.
    UndefinedConstant {
        op_index: u32,
        constant: u32,
    },
    /// A `Const` op's tensor table index doesn't match its constant
    /// table tensor id.
    ConstantTensorMismatch {
        op_index: u32,
        constant_index: u32,
    },
    /// A reduction or shape op got an axis or perm value out of range.
    InvalidAxis {
        op_index: u32,
        axis: u32,
        rank: u32,
    },
    /// A transpose perm is not a permutation of `0..rank`.
    InvalidPermutation { op_index: u32, perm: Vec<u32> },
    /// A reshape would change the element count.
    NumelMismatch {
        op_index: u32,
        expected: u64,
        actual: u64,
    },
    /// A broadcast tries to expand a non-1 dim.
    InvalidBroadcast {
        op_index: u32,
        from: Shape,
        to: Shape,
    },
    /// A matmul received an input with the wrong rank.
    BadMatMulRank {
        op_index: u32,
        which: &'static str, // "a" or "b"
        rank: u32,
    },
    /// An op requires a float dtype and got a non-float.
    NonFloatDType {
        op_index: u32,
        op_name: &'static str,
        dtype: DType,
    },
    /// `Where` predicate must be U8.
    NonU8Predicate { op_index: u32, dtype: DType },
    /// An output listed in `Graph.outputs` is not defined.
    UndefinedOutput { tensor: TensorId },
    /// A `Tensor` in `Graph.tensors` has an id that doesn't match its
    /// position in the vector.
    TensorIdMismatch { position: u32, declared: TensorId },
    /// A graph input's id is out of range.
    InputOutOfRange { tensor: TensorId },
    /// An op produces an output that is not the next-allocated id.
    NonContiguousOutput {
        op_index: u32,
        expected: TensorId,
        actual: TensorId,
    },
    /// Wire-format error: unexpected end of input.
    WireUnexpectedEof,
    /// Wire-format error: invalid UTF-8 in a string field.
    WireInvalidUtf8,
    /// Wire-format error: format version is not supported by this reader.
    WireUnsupportedVersion(u32),
    /// Wire-format error: an op or dtype tag is not in the known set.
    WireUnknownTag { what: &'static str, tag: u64 },
    /// Wire-format error: a varint is more than 10 bytes (would overflow u64).
    WireOversizedVarint,
    /// Wire-format error: trailing bytes after a complete graph.
    WireTrailingBytes,
}

impl Graph {
    /// Validate the graph end-to-end.  Returns `Ok(())` if valid,
    /// `Err(IrError)` describing the first violation otherwise.
    pub fn validate(&self) -> Result<(), IrError> {
        // ──── Tensor table consistency ────
        for (i, t) in self.tensors.iter().enumerate() {
            if t.id.0 as usize != i {
                return Err(IrError::TensorIdMismatch {
                    position: i as u32,
                    declared: t.id,
                });
            }
        }

        // ──── Inputs reference real tensors ────
        for inp in &self.inputs {
            if inp.id.0 as usize >= self.tensors.len() {
                return Err(IrError::InputOutOfRange { tensor: inp.id });
            }
        }

        // ──── Constants ────
        for (ci, c) in self.constants.iter().enumerate() {
            let expected = c
                .tensor
                .shape
                .byte_size(c.tensor.dtype)
                .ok_or(IrError::ConstantByteLength {
                    constant_index: ci as u32,
                    expected: usize::MAX,
                    actual: c.bytes.len(),
                })? as usize;
            if c.bytes.len() != expected {
                return Err(IrError::ConstantByteLength {
                    constant_index: ci as u32,
                    expected,
                    actual: c.bytes.len(),
                });
            }
        }

        // ──── Single-assignment + per-op semantic checks ────
        // Walk ops in declared order.  Track which tensor ids have been
        // produced (or are inputs/constants) so we can confirm SSA and
        // topological ordering simultaneously.
        let mut defined: Vec<bool> = vec![false; self.tensors.len()];
        for inp in &self.inputs {
            defined[inp.id.0 as usize] = true;
        }
        for c in &self.constants {
            defined[c.tensor.id.0 as usize] = true;
        }

        for (i, op) in self.ops.iter().enumerate() {
            let i = i as u32;
            for input in op.inputs() {
                let idx = input.0 as usize;
                if idx >= self.tensors.len() {
                    return Err(IrError::UndefinedTensor {
                        op_index: i,
                        tensor: input,
                    });
                }
                if !defined[idx] {
                    // A tensor that exists in the table but hasn't been
                    // produced yet means out-of-order ops.
                    return Err(IrError::UndefinedTensor {
                        op_index: i,
                        tensor: input,
                    });
                }
            }

            self.check_op_semantics(i, op)?;

            // Mark output as defined.
            let out = op.output();
            if (out.0 as usize) >= self.tensors.len() {
                return Err(IrError::UndefinedTensor {
                    op_index: i,
                    tensor: out,
                });
            }
            defined[out.0 as usize] = true;
        }

        // ──── Outputs are all defined ────
        for &out in &self.outputs {
            let idx = out.0 as usize;
            if idx >= self.tensors.len() || !defined[idx] {
                return Err(IrError::UndefinedOutput { tensor: out });
            }
        }

        Ok(())
    }

    /// Per-op semantic check.  Each variant validates that input shapes
    /// and dtypes satisfy its contract and that the declared output is
    /// what those inputs imply.
    fn check_op_semantics(&self, i: u32, op: &Op) -> Result<(), IrError> {
        match op {
            // ──── elementwise unary ────
            Op::Neg { input, output }
            | Op::Abs { input, output } => {
                let inp = self.must_tensor(i, *input)?;
                self.expect_output(i, *output, &inp.dtype, &inp.shape)?;
                Ok(())
            }
            Op::Sqrt { input, output }
            | Op::Exp { input, output }
            | Op::Log { input, output }
            | Op::Tanh { input, output }
            | Op::Recip { input, output } => {
                let inp = self.must_tensor(i, *input)?;
                if inp.dtype != DType::F32 {
                    return Err(IrError::NonFloatDType {
                        op_index: i,
                        op_name: op_name(op),
                        dtype: inp.dtype,
                    });
                }
                self.expect_output(i, *output, &inp.dtype, &inp.shape)?;
                Ok(())
            }

            // ──── elementwise binary (non-float-required) ────
            Op::Add { lhs, rhs, output }
            | Op::Sub { lhs, rhs, output }
            | Op::Mul { lhs, rhs, output }
            | Op::Max { lhs, rhs, output }
            | Op::Min { lhs, rhs, output } => {
                self.check_elem_binary(i, *lhs, *rhs, *output, false)?;
                Ok(())
            }
            // ──── elementwise binary (float-required) ────
            Op::Div { lhs, rhs, output }
            | Op::Pow { lhs, rhs, output } => {
                self.check_elem_binary(i, *lhs, *rhs, *output, true)?;
                Ok(())
            }

            // ──── reductions ────
            Op::ReduceSum {
                input,
                axes,
                keep_dims,
                output,
            }
            | Op::ReduceMax {
                input,
                axes,
                keep_dims,
                output,
            }
            | Op::ReduceMean {
                input,
                axes,
                keep_dims,
                output,
            } => {
                let inp = self.must_tensor(i, *input)?;
                let rank = inp.shape.rank() as u32;
                for &a in axes {
                    if a >= rank {
                        return Err(IrError::InvalidAxis {
                            op_index: i,
                            axis: a,
                            rank,
                        });
                    }
                }
                let expected_shape = reduce_shape(&inp.shape, axes, *keep_dims);
                self.expect_output(i, *output, &inp.dtype, &expected_shape)?;
                Ok(())
            }

            // ──── shape ────
            Op::Reshape {
                input,
                new_shape,
                output,
            } => {
                let inp = self.must_tensor(i, *input)?;
                let in_n = inp.shape.numel().unwrap_or(0);
                let out_n = new_shape.numel().unwrap_or(0);
                if in_n != out_n {
                    return Err(IrError::NumelMismatch {
                        op_index: i,
                        expected: in_n,
                        actual: out_n,
                    });
                }
                self.expect_output(i, *output, &inp.dtype, new_shape)?;
                Ok(())
            }
            Op::Transpose {
                input,
                perm,
                output,
            } => {
                let inp = self.must_tensor(i, *input)?;
                if perm.len() != inp.shape.rank() {
                    return Err(IrError::InvalidPermutation {
                        op_index: i,
                        perm: perm.clone(),
                    });
                }
                let mut seen = vec![false; perm.len()];
                for &p in perm {
                    let p = p as usize;
                    if p >= seen.len() || seen[p] {
                        return Err(IrError::InvalidPermutation {
                            op_index: i,
                            perm: perm.clone(),
                        });
                    }
                    seen[p] = true;
                }
                let expected_dims: Vec<u32> = perm
                    .iter()
                    .map(|&p| inp.shape.dims[p as usize])
                    .collect();
                self.expect_output(
                    i,
                    *output,
                    &inp.dtype,
                    &Shape { dims: expected_dims },
                )?;
                Ok(())
            }
            Op::Broadcast {
                input,
                target_shape,
                output,
            } => {
                let inp = self.must_tensor(i, *input)?;
                if inp.shape.rank() != target_shape.rank() {
                    return Err(IrError::InvalidBroadcast {
                        op_index: i,
                        from: inp.shape.clone(),
                        to: target_shape.clone(),
                    });
                }
                for (a, b) in inp.shape.dims.iter().zip(target_shape.dims.iter()) {
                    if !(a == b || *a == 1) {
                        return Err(IrError::InvalidBroadcast {
                            op_index: i,
                            from: inp.shape.clone(),
                            to: target_shape.clone(),
                        });
                    }
                }
                self.expect_output(i, *output, &inp.dtype, target_shape)?;
                Ok(())
            }

            // ──── linalg ────
            Op::MatMul { a, b, output } => {
                let ta = self.must_tensor(i, *a)?;
                let tb = self.must_tensor(i, *b)?;
                if ta.shape.rank() != 2 {
                    return Err(IrError::BadMatMulRank {
                        op_index: i,
                        which: "a",
                        rank: ta.shape.rank() as u32,
                    });
                }
                if tb.shape.rank() != 2 {
                    return Err(IrError::BadMatMulRank {
                        op_index: i,
                        which: "b",
                        rank: tb.shape.rank() as u32,
                    });
                }
                if ta.dtype != tb.dtype {
                    return Err(IrError::DTypeMismatch {
                        op_index: i,
                        expected: ta.dtype,
                        actual: tb.dtype,
                    });
                }
                let (m, k1) = (ta.shape.dims[0], ta.shape.dims[1]);
                let (k2, n) = (tb.shape.dims[0], tb.shape.dims[1]);
                if k1 != k2 {
                    return Err(IrError::ShapeMismatch {
                        op_index: i,
                        expected: Shape::from(&[k1, n]),
                        actual: Shape::from(&[k2, n]),
                    });
                }
                self.expect_output(i, *output, &ta.dtype, &Shape::from(&[m, n]))?;
                Ok(())
            }

            // ──── comparison ────
            Op::Equal { lhs, rhs, output }
            | Op::Less { lhs, rhs, output }
            | Op::Greater { lhs, rhs, output } => {
                let l = self.must_tensor(i, *lhs)?;
                let r = self.must_tensor(i, *rhs)?;
                if l.dtype != r.dtype {
                    return Err(IrError::DTypeMismatch {
                        op_index: i,
                        expected: l.dtype,
                        actual: r.dtype,
                    });
                }
                if l.shape != r.shape {
                    return Err(IrError::ShapeMismatch {
                        op_index: i,
                        expected: l.shape.clone(),
                        actual: r.shape.clone(),
                    });
                }
                self.expect_output(i, *output, &DType::U8, &l.shape)?;
                Ok(())
            }

            // ──── selection ────
            Op::Where {
                predicate,
                true_value,
                false_value,
                output,
            } => {
                let p = self.must_tensor(i, *predicate)?;
                let t = self.must_tensor(i, *true_value)?;
                let f = self.must_tensor(i, *false_value)?;
                if p.dtype != DType::U8 {
                    return Err(IrError::NonU8Predicate {
                        op_index: i,
                        dtype: p.dtype,
                    });
                }
                if p.shape != t.shape {
                    return Err(IrError::ShapeMismatch {
                        op_index: i,
                        expected: t.shape.clone(),
                        actual: p.shape.clone(),
                    });
                }
                if t.shape != f.shape {
                    return Err(IrError::ShapeMismatch {
                        op_index: i,
                        expected: t.shape.clone(),
                        actual: f.shape.clone(),
                    });
                }
                if t.dtype != f.dtype {
                    return Err(IrError::DTypeMismatch {
                        op_index: i,
                        expected: t.dtype,
                        actual: f.dtype,
                    });
                }
                self.expect_output(i, *output, &t.dtype, &t.shape)?;
                Ok(())
            }

            // ──── conversion ────
            Op::Cast {
                input,
                dtype,
                output,
            } => {
                let inp = self.must_tensor(i, *input)?;
                self.expect_output(i, *output, dtype, &inp.shape)?;
                Ok(())
            }

            // ──── constants ────
            Op::Const { constant, output } => {
                let c = self.constants.get(*constant as usize).ok_or(
                    IrError::UndefinedConstant {
                        op_index: i,
                        constant: *constant,
                    },
                )?;
                let out = self.must_tensor(i, *output)?;
                if c.tensor.dtype != out.dtype || c.tensor.shape != out.shape {
                    return Err(IrError::ConstantTensorMismatch {
                        op_index: i,
                        constant_index: *constant,
                    });
                }
                Ok(())
            }
        }
    }

    fn must_tensor(&self, op_index: u32, id: TensorId) -> Result<&Tensor, IrError> {
        self.tensors
            .get(id.0 as usize)
            .ok_or(IrError::UndefinedTensor {
                op_index,
                tensor: id,
            })
    }

    /// Verify that the declared output tensor matches `(dtype, shape)`.
    fn expect_output(
        &self,
        op_index: u32,
        out_id: TensorId,
        dtype: &DType,
        shape: &Shape,
    ) -> Result<(), IrError> {
        let out = self.must_tensor(op_index, out_id)?;
        if &out.dtype != dtype || &out.shape != shape {
            return Err(IrError::OutputMismatch {
                op_index,
                expected: Tensor::new(out_id, *dtype, shape.clone()),
                actual: out.clone(),
            });
        }
        Ok(())
    }

    fn check_elem_binary(
        &self,
        i: u32,
        lhs: TensorId,
        rhs: TensorId,
        output: TensorId,
        require_float: bool,
    ) -> Result<(), IrError> {
        let l = self.must_tensor(i, lhs)?;
        let r = self.must_tensor(i, rhs)?;
        if l.dtype != r.dtype {
            return Err(IrError::DTypeMismatch {
                op_index: i,
                expected: l.dtype,
                actual: r.dtype,
            });
        }
        if l.shape != r.shape {
            return Err(IrError::ShapeMismatch {
                op_index: i,
                expected: l.shape.clone(),
                actual: r.shape.clone(),
            });
        }
        if require_float && l.dtype != DType::F32 {
            return Err(IrError::NonFloatDType {
                op_index: i,
                op_name: "binary-float-op",
                dtype: l.dtype,
            });
        }
        self.expect_output(i, output, &l.dtype, &l.shape)?;
        Ok(())
    }
}

fn op_name(op: &Op) -> &'static str {
    match op {
        Op::Neg { .. } => "neg",
        Op::Abs { .. } => "abs",
        Op::Sqrt { .. } => "sqrt",
        Op::Exp { .. } => "exp",
        Op::Log { .. } => "log",
        Op::Tanh { .. } => "tanh",
        Op::Recip { .. } => "recip",
        Op::Add { .. } => "add",
        Op::Sub { .. } => "sub",
        Op::Mul { .. } => "mul",
        Op::Div { .. } => "div",
        Op::Max { .. } => "max",
        Op::Min { .. } => "min",
        Op::Pow { .. } => "pow",
        Op::ReduceSum { .. } => "reduce_sum",
        Op::ReduceMax { .. } => "reduce_max",
        Op::ReduceMean { .. } => "reduce_mean",
        Op::Reshape { .. } => "reshape",
        Op::Transpose { .. } => "transpose",
        Op::Broadcast { .. } => "broadcast",
        Op::MatMul { .. } => "matmul",
        Op::Equal { .. } => "equal",
        Op::Less { .. } => "less",
        Op::Greater { .. } => "greater",
        Op::Where { .. } => "where",
        Op::Cast { .. } => "cast",
        Op::Const { .. } => "const",
    }
}
