//! The 27-variant op enum.
//!
//! Each op carries `output: TensorId` even though it could be inferred
//! positionally.  This serves two purposes:
//!
//! 1. **Round-trip stability**: serialising and deserialising preserves
//!    the exact `TensorId` numbering, which tests assert against.
//! 2. **Validation locality**: a validator can check op-by-op without
//!    walking back through the graph to recompute output ids.
//!
//! The wire tag for each variant is fixed by spec MX03 §"Encoding
//! `matrix-ir` types" and never changes.  V2 ops get tags `0x1C` and
//! beyond.

use crate::tensor::{DType, Shape, TensorId};

/// One operation node in a tensor algebra graph.
///
/// Variants are grouped:
/// - **Elementwise unary** (7): Neg, Abs, Sqrt, Exp, Log, Tanh, Recip
/// - **Elementwise binary** (7): Add, Sub, Mul, Div, Max, Min, Pow
/// - **Reductions** (3): ReduceSum, ReduceMax, ReduceMean
/// - **Shape** (3): Reshape, Transpose, Broadcast
/// - **Linear algebra** (1): MatMul
/// - **Comparison** (3): Equal, Less, Greater
/// - **Selection** (1): Where
/// - **Conversion** (1): Cast
/// - **Constants** (1): Const
///
/// Total: 27 ops.  See spec MX01 §"V1 op set" for the contract on each.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Op {
    // ──────────── elementwise unary ────────────
    /// `-x`, elementwise.  Output shape and dtype match input.
    Neg { input: TensorId, output: TensorId },
    /// `|x|`, elementwise.  Output shape and dtype match input.
    Abs { input: TensorId, output: TensorId },
    /// `sqrt(x)`, elementwise.  Output dtype must be a float (F32 in V1).
    Sqrt { input: TensorId, output: TensorId },
    /// `e^x`, elementwise.  Float-only.
    Exp { input: TensorId, output: TensorId },
    /// `ln(x)`, elementwise.  Float-only.
    Log { input: TensorId, output: TensorId },
    /// `tanh(x)`, elementwise.  Float-only.
    Tanh { input: TensorId, output: TensorId },
    /// `1/x`, elementwise.  Float-only.
    Recip { input: TensorId, output: TensorId },

    // ──────────── elementwise binary ────────────
    /// `lhs + rhs`, elementwise.  Inputs must have identical shape and dtype.
    Add { lhs: TensorId, rhs: TensorId, output: TensorId },
    /// `lhs - rhs`.
    Sub { lhs: TensorId, rhs: TensorId, output: TensorId },
    /// `lhs * rhs`.
    Mul { lhs: TensorId, rhs: TensorId, output: TensorId },
    /// `lhs / rhs`.  Float-only in V1; integer division reserved for V2.
    Div { lhs: TensorId, rhs: TensorId, output: TensorId },
    /// `max(lhs, rhs)`.
    Max { lhs: TensorId, rhs: TensorId, output: TensorId },
    /// `min(lhs, rhs)`.
    Min { lhs: TensorId, rhs: TensorId, output: TensorId },
    /// `lhs ^ rhs`.  Float-only.
    Pow { lhs: TensorId, rhs: TensorId, output: TensorId },

    // ──────────── reductions ────────────
    /// Sum over the given axes.  If `keep_dims` is false, reduced axes
    /// are removed; if true, they remain as size-1.  Empty `axes` means
    /// reduce-all.
    ReduceSum {
        input: TensorId,
        axes: Vec<u32>,
        keep_dims: bool,
        output: TensorId,
    },
    /// Max over axes.  Same axis semantics as ReduceSum.
    ReduceMax {
        input: TensorId,
        axes: Vec<u32>,
        keep_dims: bool,
        output: TensorId,
    },
    /// Mean over axes.  For integer dtypes this uses integer division
    /// (truncation toward zero).
    ReduceMean {
        input: TensorId,
        axes: Vec<u32>,
        keep_dims: bool,
        output: TensorId,
    },

    // ──────────── shape ────────────
    /// Reshape to `new_shape`.  Total element count must match.
    Reshape {
        input: TensorId,
        new_shape: Shape,
        output: TensorId,
    },
    /// Transpose by the given permutation.  `perm` must be a permutation
    /// of `0..rank`.
    Transpose {
        input: TensorId,
        perm: Vec<u32>,
        output: TensorId,
    },
    /// Broadcast to `target_shape`.  Each dim of input must equal the
    /// corresponding target dim or be 1.
    Broadcast {
        input: TensorId,
        target_shape: Shape,
        output: TensorId,
    },

    // ──────────── linear algebra ────────────
    /// 2D matrix multiplication.  `a` is `[m, k]`, `b` is `[k, n]`,
    /// output is `[m, n]`.  Both inputs share dtype.
    MatMul {
        a: TensorId,
        b: TensorId,
        output: TensorId,
    },

    // ──────────── comparison ────────────
    /// Elementwise `lhs == rhs`.  Output dtype is U8 (0 or 1).
    Equal { lhs: TensorId, rhs: TensorId, output: TensorId },
    /// Elementwise `lhs < rhs`.  Output dtype is U8.
    Less { lhs: TensorId, rhs: TensorId, output: TensorId },
    /// Elementwise `lhs > rhs`.  Output dtype is U8.
    Greater { lhs: TensorId, rhs: TensorId, output: TensorId },

    // ──────────── selection ────────────
    /// Elementwise `predicate ? true_value : false_value`.
    /// `predicate` must be U8.  All shapes must match.  Output dtype
    /// matches `true_value`/`false_value`.
    Where {
        predicate: TensorId,
        true_value: TensorId,
        false_value: TensorId,
        output: TensorId,
    },

    // ──────────── conversion ────────────
    /// Numerical conversion to the target dtype.  Float-to-int
    /// truncates toward zero.
    Cast {
        input: TensorId,
        dtype: DType,
        output: TensorId,
    },

    // ──────────── constants ────────────
    /// Materialise a constant from the graph's constant table.  The
    /// `constant` field is the index into `Graph.constants`.
    Const { constant: u32, output: TensorId },
}

impl Op {
    /// The wire-format tag for this op variant.  Stable across versions
    /// per spec MX03.
    pub const fn wire_tag(&self) -> u8 {
        match self {
            Op::Neg { .. } => 0x00,
            Op::Abs { .. } => 0x01,
            Op::Sqrt { .. } => 0x02,
            Op::Exp { .. } => 0x03,
            Op::Log { .. } => 0x04,
            Op::Tanh { .. } => 0x05,
            Op::Recip { .. } => 0x06,
            Op::Add { .. } => 0x07,
            Op::Sub { .. } => 0x08,
            Op::Mul { .. } => 0x09,
            Op::Div { .. } => 0x0A,
            Op::Max { .. } => 0x0B,
            Op::Min { .. } => 0x0C,
            Op::Pow { .. } => 0x0D,
            Op::ReduceSum { .. } => 0x0E,
            Op::ReduceMax { .. } => 0x0F,
            Op::ReduceMean { .. } => 0x10,
            Op::Reshape { .. } => 0x11,
            Op::Transpose { .. } => 0x12,
            Op::Broadcast { .. } => 0x13,
            Op::MatMul { .. } => 0x15,
            Op::Equal { .. } => 0x16,
            Op::Less { .. } => 0x17,
            Op::Greater { .. } => 0x18,
            Op::Where { .. } => 0x19,
            Op::Cast { .. } => 0x1A,
            Op::Const { .. } => 0x1B,
        }
    }

    /// The output tensor id this op produces.
    pub const fn output(&self) -> TensorId {
        match *self {
            Op::Neg { output, .. } => output,
            Op::Abs { output, .. } => output,
            Op::Sqrt { output, .. } => output,
            Op::Exp { output, .. } => output,
            Op::Log { output, .. } => output,
            Op::Tanh { output, .. } => output,
            Op::Recip { output, .. } => output,
            Op::Add { output, .. } => output,
            Op::Sub { output, .. } => output,
            Op::Mul { output, .. } => output,
            Op::Div { output, .. } => output,
            Op::Max { output, .. } => output,
            Op::Min { output, .. } => output,
            Op::Pow { output, .. } => output,
            Op::ReduceSum { output, .. } => output,
            Op::ReduceMax { output, .. } => output,
            Op::ReduceMean { output, .. } => output,
            Op::Reshape { output, .. } => output,
            Op::Transpose { output, .. } => output,
            Op::Broadcast { output, .. } => output,
            Op::MatMul { output, .. } => output,
            Op::Equal { output, .. } => output,
            Op::Less { output, .. } => output,
            Op::Greater { output, .. } => output,
            Op::Where { output, .. } => output,
            Op::Cast { output, .. } => output,
            Op::Const { output, .. } => output,
        }
    }

    /// Iterate over the input tensor ids referenced by this op (excluding
    /// constants, which are looked up via index).  Order is variant-defined.
    ///
    /// Returns a small Vec to keep the API simple; an iterator could be
    /// added if profiling shows allocation pressure.
    pub fn inputs(&self) -> Vec<TensorId> {
        match self {
            Op::Neg { input, .. }
            | Op::Abs { input, .. }
            | Op::Sqrt { input, .. }
            | Op::Exp { input, .. }
            | Op::Log { input, .. }
            | Op::Tanh { input, .. }
            | Op::Recip { input, .. } => vec![*input],

            Op::Add { lhs, rhs, .. }
            | Op::Sub { lhs, rhs, .. }
            | Op::Mul { lhs, rhs, .. }
            | Op::Div { lhs, rhs, .. }
            | Op::Max { lhs, rhs, .. }
            | Op::Min { lhs, rhs, .. }
            | Op::Pow { lhs, rhs, .. }
            | Op::Equal { lhs, rhs, .. }
            | Op::Less { lhs, rhs, .. }
            | Op::Greater { lhs, rhs, .. } => vec![*lhs, *rhs],

            Op::ReduceSum { input, .. }
            | Op::ReduceMax { input, .. }
            | Op::ReduceMean { input, .. }
            | Op::Reshape { input, .. }
            | Op::Transpose { input, .. }
            | Op::Broadcast { input, .. }
            | Op::Cast { input, .. } => vec![*input],

            Op::MatMul { a, b, .. } => vec![*a, *b],

            Op::Where {
                predicate,
                true_value,
                false_value,
                ..
            } => vec![*predicate, *true_value, *false_value],

            Op::Const { .. } => Vec::new(),
        }
    }

    /// True iff this op is one of the seven elementwise-unary variants.
    /// Useful for downstream optimisers (fusion, etc.).
    pub const fn is_elementwise_unary(&self) -> bool {
        matches!(
            self,
            Op::Neg { .. }
                | Op::Abs { .. }
                | Op::Sqrt { .. }
                | Op::Exp { .. }
                | Op::Log { .. }
                | Op::Tanh { .. }
                | Op::Recip { .. }
        )
    }

    /// True iff this op is one of the seven elementwise-binary variants.
    pub const fn is_elementwise_binary(&self) -> bool {
        matches!(
            self,
            Op::Add { .. }
                | Op::Sub { .. }
                | Op::Mul { .. }
                | Op::Div { .. }
                | Op::Max { .. }
                | Op::Min { .. }
                | Op::Pow { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All 27 op tags must be unique.  This guards against typos in the
    /// `wire_tag` match arms.
    #[test]
    fn wire_tags_are_unique() {
        let ops = sample_one_per_variant();
        let mut tags: Vec<u8> = ops.iter().map(|o| o.wire_tag()).collect();
        tags.sort();
        let len_before = tags.len();
        tags.dedup();
        assert_eq!(
            tags.len(),
            len_before,
            "duplicate wire tag in Op enum: {:?}",
            tags
        );
        // 27 variants in V1.
        assert_eq!(len_before, 27);
    }

    #[test]
    fn output_method_returns_declared_output() {
        let op = Op::Add {
            lhs: TensorId(1),
            rhs: TensorId(2),
            output: TensorId(7),
        };
        assert_eq!(op.output(), TensorId(7));
    }

    #[test]
    fn inputs_method_returns_referenced_tensors() {
        let op = Op::MatMul {
            a: TensorId(1),
            b: TensorId(2),
            output: TensorId(3),
        };
        assert_eq!(op.inputs(), vec![TensorId(1), TensorId(2)]);

        let op = Op::Where {
            predicate: TensorId(1),
            true_value: TensorId(2),
            false_value: TensorId(3),
            output: TensorId(4),
        };
        assert_eq!(
            op.inputs(),
            vec![TensorId(1), TensorId(2), TensorId(3)]
        );

        let op = Op::Const {
            constant: 0,
            output: TensorId(0),
        };
        assert_eq!(op.inputs(), Vec::<TensorId>::new());
    }

    #[test]
    fn elementwise_classifiers() {
        let neg = Op::Neg {
            input: TensorId(0),
            output: TensorId(1),
        };
        let add = Op::Add {
            lhs: TensorId(0),
            rhs: TensorId(1),
            output: TensorId(2),
        };
        let mm = Op::MatMul {
            a: TensorId(0),
            b: TensorId(1),
            output: TensorId(2),
        };
        assert!(neg.is_elementwise_unary());
        assert!(!neg.is_elementwise_binary());
        assert!(add.is_elementwise_binary());
        assert!(!add.is_elementwise_unary());
        assert!(!mm.is_elementwise_unary());
        assert!(!mm.is_elementwise_binary());
    }

    /// Helper: produce one op of each variant for exhaustive coverage tests.
    pub(crate) fn sample_one_per_variant() -> Vec<Op> {
        let t = |n: u32| TensorId(n);
        vec![
            Op::Neg { input: t(0), output: t(1) },
            Op::Abs { input: t(0), output: t(1) },
            Op::Sqrt { input: t(0), output: t(1) },
            Op::Exp { input: t(0), output: t(1) },
            Op::Log { input: t(0), output: t(1) },
            Op::Tanh { input: t(0), output: t(1) },
            Op::Recip { input: t(0), output: t(1) },
            Op::Add { lhs: t(0), rhs: t(1), output: t(2) },
            Op::Sub { lhs: t(0), rhs: t(1), output: t(2) },
            Op::Mul { lhs: t(0), rhs: t(1), output: t(2) },
            Op::Div { lhs: t(0), rhs: t(1), output: t(2) },
            Op::Max { lhs: t(0), rhs: t(1), output: t(2) },
            Op::Min { lhs: t(0), rhs: t(1), output: t(2) },
            Op::Pow { lhs: t(0), rhs: t(1), output: t(2) },
            Op::ReduceSum {
                input: t(0),
                axes: vec![1],
                keep_dims: false,
                output: t(1),
            },
            Op::ReduceMax {
                input: t(0),
                axes: vec![1],
                keep_dims: false,
                output: t(1),
            },
            Op::ReduceMean {
                input: t(0),
                axes: vec![1],
                keep_dims: false,
                output: t(1),
            },
            Op::Reshape {
                input: t(0),
                new_shape: Shape::from(&[6]),
                output: t(1),
            },
            Op::Transpose {
                input: t(0),
                perm: vec![1, 0],
                output: t(1),
            },
            Op::Broadcast {
                input: t(0),
                target_shape: Shape::from(&[2, 3]),
                output: t(1),
            },
            Op::MatMul { a: t(0), b: t(1), output: t(2) },
            Op::Equal { lhs: t(0), rhs: t(1), output: t(2) },
            Op::Less { lhs: t(0), rhs: t(1), output: t(2) },
            Op::Greater { lhs: t(0), rhs: t(1), output: t(2) },
            Op::Where {
                predicate: t(0),
                true_value: t(1),
                false_value: t(2),
                output: t(3),
            },
            Op::Cast {
                input: t(0),
                dtype: DType::I32,
                output: t(1),
            },
            Op::Const {
                constant: 0,
                output: t(0),
            },
        ]
    }
}
