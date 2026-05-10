//! Ergonomic graph construction.
//!
//! `GraphBuilder` allocates [`TensorId`]s densely, infers output shapes
//! from inputs, performs eager shape-and-dtype checks (panicking with a
//! clear message on misuse), and produces a [`Graph`] when [`build`] is
//! called.
//!
//! The eager-panic behaviour is intentional: in this builder a
//! mismatched shape is a programmer error, not a runtime input — the
//! sooner it's surfaced the better.  Production code that wants
//! per-call results can use the lower-level constructors directly.
//!
//! [`build`]: GraphBuilder::build
//! [`TensorId`]: crate::TensorId

use crate::graph::{Constant, Graph};
use crate::op::Op;
use crate::tensor::{DType, Shape, Tensor, TensorId};
use crate::validate::IrError;

/// Ergonomic builder for [`Graph`].  See module-level docs.
pub struct GraphBuilder {
    inputs: Vec<Tensor>,
    outputs: Vec<TensorId>,
    ops: Vec<Op>,
    tensors: Vec<Tensor>,
    constants: Vec<Constant>,
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBuilder {
    /// Construct an empty builder.
    pub fn new() -> Self {
        GraphBuilder {
            inputs: Vec::new(),
            outputs: Vec::new(),
            ops: Vec::new(),
            tensors: Vec::new(),
            constants: Vec::new(),
        }
    }

    /// Allocate the next [`TensorId`] and register the tensor.
    fn alloc(&mut self, dtype: DType, shape: Shape) -> Tensor {
        let id = TensorId(self.tensors.len() as u32);
        let t = Tensor::new(id, dtype, shape);
        self.tensors.push(t.clone());
        t
    }

    /// Declare a new graph input.  The returned [`Tensor`] has a fresh
    /// id; the input is also recorded in [`Graph::inputs`] in the order
    /// declared.
    pub fn input(&mut self, dtype: DType, shape: Shape) -> Tensor {
        let t = self.alloc(dtype, shape);
        self.inputs.push(t.clone());
        t
    }

    /// Declare a constant.  `bytes` must be `shape.numel() *
    /// dtype.size_bytes()` long, dtype-encoded little-endian.  Panics
    /// on mismatch.
    pub fn constant(&mut self, dtype: DType, shape: Shape, bytes: Vec<u8>) -> Tensor {
        let expected = shape
            .byte_size(dtype)
            .expect("constant byte_size overflows u64") as usize;
        assert_eq!(
            bytes.len(),
            expected,
            "constant bytes length {} != expected {} ({} elems × {} bytes)",
            bytes.len(),
            expected,
            shape.numel().unwrap_or(0),
            dtype.size_bytes()
        );
        let t = self.alloc(dtype, shape);
        let c_idx = self.constants.len() as u32;
        self.constants.push(Constant {
            tensor: t.clone(),
            bytes,
        });
        let out = self.alloc(t.dtype, t.shape.clone());
        self.ops.push(Op::Const {
            constant: c_idx,
            output: out.id,
        });
        out
    }

    /// Mark a tensor as a graph output.  The same tensor may be marked
    /// multiple times, though the validator does not require it.
    pub fn output(&mut self, t: &Tensor) {
        self.outputs.push(t.id);
    }

    // ──────────── elementwise unary helpers ────────────

    fn unary(&mut self, input: &Tensor, op_ctor: fn(TensorId, TensorId) -> Op) -> Tensor {
        let out = self.alloc(input.dtype, input.shape.clone());
        self.ops.push(op_ctor(input.id, out.id));
        out
    }

    /// `-x`.
    pub fn neg(&mut self, t: &Tensor) -> Tensor {
        self.unary(t, |i, o| Op::Neg {
            input: i,
            output: o,
        })
    }
    /// `|x|`.
    pub fn abs(&mut self, t: &Tensor) -> Tensor {
        self.unary(t, |i, o| Op::Abs {
            input: i,
            output: o,
        })
    }
    /// `sqrt(x)`.  Float-only.
    pub fn sqrt(&mut self, t: &Tensor) -> Tensor {
        assert_float(t.dtype, "sqrt");
        self.unary(t, |i, o| Op::Sqrt {
            input: i,
            output: o,
        })
    }
    /// `e^x`.  Float-only.
    pub fn exp(&mut self, t: &Tensor) -> Tensor {
        assert_float(t.dtype, "exp");
        self.unary(t, |i, o| Op::Exp {
            input: i,
            output: o,
        })
    }
    /// `ln(x)`.  Float-only.
    pub fn log(&mut self, t: &Tensor) -> Tensor {
        assert_float(t.dtype, "log");
        self.unary(t, |i, o| Op::Log {
            input: i,
            output: o,
        })
    }
    /// `tanh(x)`.  Float-only.
    pub fn tanh(&mut self, t: &Tensor) -> Tensor {
        assert_float(t.dtype, "tanh");
        self.unary(t, |i, o| Op::Tanh {
            input: i,
            output: o,
        })
    }
    /// `1/x`.  Float-only.
    pub fn recip(&mut self, t: &Tensor) -> Tensor {
        assert_float(t.dtype, "recip");
        self.unary(t, |i, o| Op::Recip {
            input: i,
            output: o,
        })
    }

    // ──────────── elementwise binary helpers ────────────

    fn binary(
        &mut self,
        lhs: &Tensor,
        rhs: &Tensor,
        op_ctor: fn(TensorId, TensorId, TensorId) -> Op,
        op_name: &'static str,
    ) -> Tensor {
        assert_eq!(
            lhs.dtype, rhs.dtype,
            "{}: dtype mismatch ({} vs {})",
            op_name, lhs.dtype, rhs.dtype
        );
        assert_eq!(
            lhs.shape, rhs.shape,
            "{}: shape mismatch ({} vs {})",
            op_name, lhs.shape, rhs.shape
        );
        let out = self.alloc(lhs.dtype, lhs.shape.clone());
        self.ops.push(op_ctor(lhs.id, rhs.id, out.id));
        out
    }

    /// Elementwise `lhs + rhs`.
    pub fn add(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        self.binary(lhs, rhs, |l, r, o| Op::Add { lhs: l, rhs: r, output: o }, "add")
    }
    /// Elementwise `lhs - rhs`.
    pub fn sub(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        self.binary(lhs, rhs, |l, r, o| Op::Sub { lhs: l, rhs: r, output: o }, "sub")
    }
    /// Elementwise `lhs * rhs`.
    pub fn mul(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        self.binary(lhs, rhs, |l, r, o| Op::Mul { lhs: l, rhs: r, output: o }, "mul")
    }
    /// Elementwise `lhs / rhs`.  Float-only.
    pub fn div(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        assert_float(lhs.dtype, "div");
        self.binary(lhs, rhs, |l, r, o| Op::Div { lhs: l, rhs: r, output: o }, "div")
    }
    /// Elementwise `max(lhs, rhs)`.
    pub fn max(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        self.binary(lhs, rhs, |l, r, o| Op::Max { lhs: l, rhs: r, output: o }, "max")
    }
    /// Elementwise `min(lhs, rhs)`.
    pub fn min(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        self.binary(lhs, rhs, |l, r, o| Op::Min { lhs: l, rhs: r, output: o }, "min")
    }
    /// Elementwise `lhs ^ rhs`.  Float-only.
    pub fn pow(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        assert_float(lhs.dtype, "pow");
        self.binary(lhs, rhs, |l, r, o| Op::Pow { lhs: l, rhs: r, output: o }, "pow")
    }

    // ──────────── reductions ────────────

    fn reduce(
        &mut self,
        input: &Tensor,
        axes: Vec<u32>,
        keep_dims: bool,
        ctor: fn(TensorId, Vec<u32>, bool, TensorId) -> Op,
        op_name: &'static str,
    ) -> Tensor {
        let rank = input.shape.rank() as u32;
        for &a in &axes {
            assert!(a < rank, "{}: axis {} out of range (rank {})", op_name, a, rank);
        }
        let out_shape = reduce_shape(&input.shape, &axes, keep_dims);
        let out = self.alloc(input.dtype, out_shape);
        self.ops.push(ctor(input.id, axes, keep_dims, out.id));
        out
    }

    /// Sum reduction over `axes`.  Empty axes = reduce all.
    pub fn reduce_sum(&mut self, input: &Tensor, axes: Vec<u32>, keep_dims: bool) -> Tensor {
        self.reduce(
            input,
            axes,
            keep_dims,
            |i, ax, kd, o| Op::ReduceSum {
                input: i,
                axes: ax,
                keep_dims: kd,
                output: o,
            },
            "reduce_sum",
        )
    }
    /// Max reduction over `axes`.
    pub fn reduce_max(&mut self, input: &Tensor, axes: Vec<u32>, keep_dims: bool) -> Tensor {
        self.reduce(
            input,
            axes,
            keep_dims,
            |i, ax, kd, o| Op::ReduceMax {
                input: i,
                axes: ax,
                keep_dims: kd,
                output: o,
            },
            "reduce_max",
        )
    }
    /// Mean reduction over `axes`.
    pub fn reduce_mean(&mut self, input: &Tensor, axes: Vec<u32>, keep_dims: bool) -> Tensor {
        self.reduce(
            input,
            axes,
            keep_dims,
            |i, ax, kd, o| Op::ReduceMean {
                input: i,
                axes: ax,
                keep_dims: kd,
                output: o,
            },
            "reduce_mean",
        )
    }

    // ──────────── shape ────────────

    /// Reshape to `new_shape`.  Total element count must match.
    pub fn reshape(&mut self, input: &Tensor, new_shape: Shape) -> Tensor {
        assert_eq!(
            input.shape.numel(),
            new_shape.numel(),
            "reshape: numel mismatch (input {} new {})",
            input.shape,
            new_shape
        );
        let out = self.alloc(input.dtype, new_shape.clone());
        self.ops.push(Op::Reshape {
            input: input.id,
            new_shape,
            output: out.id,
        });
        out
    }

    /// Transpose by permutation.  `perm` must be a permutation of `0..rank`.
    pub fn transpose(&mut self, input: &Tensor, perm: Vec<u32>) -> Tensor {
        assert_eq!(
            perm.len(),
            input.shape.rank(),
            "transpose: perm length {} != rank {}",
            perm.len(),
            input.shape.rank()
        );
        let mut seen = vec![false; perm.len()];
        for &p in &perm {
            let p = p as usize;
            assert!(p < seen.len(), "transpose: perm value {} out of range", p);
            assert!(!seen[p], "transpose: perm contains duplicate {}", p);
            seen[p] = true;
        }
        let new_dims: Vec<u32> = perm
            .iter()
            .map(|&p| input.shape.dims[p as usize])
            .collect();
        let out = self.alloc(input.dtype, Shape { dims: new_dims });
        self.ops.push(Op::Transpose {
            input: input.id,
            perm,
            output: out.id,
        });
        out
    }

    /// Broadcast to `target_shape`.  Each dim of input must equal the
    /// corresponding target dim or be 1.
    pub fn broadcast(&mut self, input: &Tensor, target_shape: Shape) -> Tensor {
        assert_eq!(
            input.shape.rank(),
            target_shape.rank(),
            "broadcast: rank mismatch ({} vs {})",
            input.shape,
            target_shape
        );
        for (i, (&inp, &tgt)) in input
            .shape
            .dims
            .iter()
            .zip(target_shape.dims.iter())
            .enumerate()
        {
            assert!(
                inp == tgt || inp == 1,
                "broadcast: dim {} of input ({}) is not 1 and does not match target ({})",
                i,
                inp,
                tgt
            );
        }
        let out = self.alloc(input.dtype, target_shape.clone());
        self.ops.push(Op::Broadcast {
            input: input.id,
            target_shape,
            output: out.id,
        });
        out
    }

    // ──────────── linear algebra ────────────

    /// 2D matrix multiply.  `a: [m,k]` × `b: [k,n]` → `[m,n]`.
    pub fn matmul(&mut self, a: &Tensor, b: &Tensor) -> Tensor {
        assert_eq!(a.dtype, b.dtype, "matmul: dtype mismatch");
        assert_eq!(a.shape.rank(), 2, "matmul: lhs rank must be 2");
        assert_eq!(b.shape.rank(), 2, "matmul: rhs rank must be 2");
        let (m, k1) = (a.shape.dims[0], a.shape.dims[1]);
        let (k2, n) = (b.shape.dims[0], b.shape.dims[1]);
        assert_eq!(
            k1, k2,
            "matmul: inner dims mismatch ({} vs {})",
            k1, k2
        );
        let out = self.alloc(a.dtype, Shape::from(&[m, n]));
        self.ops.push(Op::MatMul {
            a: a.id,
            b: b.id,
            output: out.id,
        });
        out
    }

    // ──────────── comparison ────────────

    fn compare(
        &mut self,
        lhs: &Tensor,
        rhs: &Tensor,
        ctor: fn(TensorId, TensorId, TensorId) -> Op,
        op_name: &'static str,
    ) -> Tensor {
        assert_eq!(
            lhs.dtype, rhs.dtype,
            "{}: dtype mismatch",
            op_name
        );
        assert_eq!(
            lhs.shape, rhs.shape,
            "{}: shape mismatch",
            op_name
        );
        let out = self.alloc(DType::U8, lhs.shape.clone());
        self.ops.push(ctor(lhs.id, rhs.id, out.id));
        out
    }

    /// Elementwise `lhs == rhs`.  Output is U8 (0 or 1).
    pub fn equal(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        self.compare(
            lhs,
            rhs,
            |l, r, o| Op::Equal { lhs: l, rhs: r, output: o },
            "equal",
        )
    }
    /// Elementwise `lhs < rhs`.
    pub fn less(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        self.compare(
            lhs,
            rhs,
            |l, r, o| Op::Less { lhs: l, rhs: r, output: o },
            "less",
        )
    }
    /// Elementwise `lhs > rhs`.
    pub fn greater(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor {
        self.compare(
            lhs,
            rhs,
            |l, r, o| Op::Greater { lhs: l, rhs: r, output: o },
            "greater",
        )
    }

    // ──────────── selection ────────────

    /// Per-element `predicate ? true_value : false_value`.
    pub fn where_(&mut self, predicate: &Tensor, t: &Tensor, f: &Tensor) -> Tensor {
        assert_eq!(predicate.dtype, DType::U8, "where: predicate must be u8");
        assert_eq!(predicate.shape, t.shape, "where: predicate/true_value shape mismatch");
        assert_eq!(t.shape, f.shape, "where: true_value/false_value shape mismatch");
        assert_eq!(t.dtype, f.dtype, "where: true_value/false_value dtype mismatch");
        let out = self.alloc(t.dtype, t.shape.clone());
        self.ops.push(Op::Where {
            predicate: predicate.id,
            true_value: t.id,
            false_value: f.id,
            output: out.id,
        });
        out
    }

    // ──────────── conversion ────────────

    /// Numerical conversion to `dtype`.
    pub fn cast(&mut self, input: &Tensor, dtype: DType) -> Tensor {
        let out = self.alloc(dtype, input.shape.clone());
        self.ops.push(Op::Cast {
            input: input.id,
            dtype,
            output: out.id,
        });
        out
    }

    /// Finalise the builder, returning a [`Graph`].  Runs structural
    /// validation; semantic validation (per-op shape/dtype checks)
    /// happens lazily in [`Graph::validate`](crate::Graph::validate).
    pub fn build(self) -> Result<Graph, IrError> {
        let g = Graph {
            inputs: self.inputs,
            outputs: self.outputs,
            ops: self.ops,
            tensors: self.tensors,
            constants: self.constants,
        };
        // Always do a structural-and-semantic validate so that build()
        // returning Ok is a strong guarantee.
        g.validate()?;
        Ok(g)
    }
}

fn assert_float(dt: DType, op_name: &'static str) {
    assert!(
        matches!(dt, DType::F32),
        "{}: dtype must be a float (got {})",
        op_name,
        dt
    );
}

/// Compute the output shape of a reduction.
///
/// - If `keep_dims` is true: reduced axes become size 1.
/// - If `keep_dims` is false: reduced axes are removed.
/// - Empty `axes` means reduce-all (scalar output unless `keep_dims`,
///   in which case all dims become 1).
pub(crate) fn reduce_shape(input: &Shape, axes: &[u32], keep_dims: bool) -> Shape {
    if axes.is_empty() {
        // Reduce all.
        if keep_dims {
            Shape {
                dims: vec![1; input.rank()],
            }
        } else {
            Shape::scalar()
        }
    } else {
        let mut out = Vec::with_capacity(input.rank());
        for (i, &d) in input.dims.iter().enumerate() {
            let i = i as u32;
            if axes.contains(&i) {
                if keep_dims {
                    out.push(1);
                }
            } else {
                out.push(d);
            }
        }
        Shape { dims: out }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_relu_layer() {
        let mut g = GraphBuilder::new();
        let x = g.input(DType::F32, Shape::from(&[1, 4]));
        let w = g.input(DType::F32, Shape::from(&[4, 2]));
        let b = g.input(DType::F32, Shape::from(&[1, 2]));
        let zero = g.constant(DType::F32, Shape::from(&[1, 2]), vec![0u8; 8]);

        let xw = g.matmul(&x, &w);
        let xwb = g.add(&xw, &b);
        let y = g.max(&xwb, &zero);

        g.output(&y);
        let graph = g.build().expect("graph should validate");

        // Expected ops: Const (from constant()), MatMul, Add, Max => 4 ops.
        assert_eq!(graph.ops.len(), 4);
        assert_eq!(graph.outputs, vec![y.id]);
        assert_eq!(graph.inputs.len(), 3);
        assert_eq!(graph.constants.len(), 1);
    }

    #[test]
    #[should_panic(expected = "shape mismatch")]
    fn add_shape_mismatch_panics() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[3]));
        let b = g.input(DType::F32, Shape::from(&[4]));
        let _ = g.add(&a, &b);
    }

    #[test]
    #[should_panic(expected = "dtype mismatch")]
    fn add_dtype_mismatch_panics() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[3]));
        let b = g.input(DType::I32, Shape::from(&[3]));
        let _ = g.add(&a, &b);
    }

    #[test]
    #[should_panic(expected = "dtype must be a float")]
    fn sqrt_on_int_panics() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::I32, Shape::from(&[3]));
        let _ = g.sqrt(&a);
    }

    #[test]
    fn reshape_numel_check() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3]));
        let r = g.reshape(&a, Shape::from(&[6]));
        assert_eq!(r.shape, Shape::from(&[6]));
    }

    #[test]
    #[should_panic(expected = "numel mismatch")]
    fn reshape_numel_mismatch_panics() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3]));
        let _ = g.reshape(&a, Shape::from(&[5]));
    }

    #[test]
    fn transpose_reorders_dims() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3, 4]));
        let r = g.transpose(&a, vec![2, 0, 1]);
        assert_eq!(r.shape, Shape::from(&[4, 2, 3]));
    }

    #[test]
    #[should_panic(expected = "perm contains duplicate")]
    fn transpose_duplicate_perm_panics() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3]));
        let _ = g.transpose(&a, vec![0, 0]);
    }

    #[test]
    fn broadcast_basic() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[1, 3]));
        let r = g.broadcast(&a, Shape::from(&[4, 3]));
        assert_eq!(r.shape, Shape::from(&[4, 3]));
    }

    #[test]
    #[should_panic(expected = "broadcast")]
    fn broadcast_incompatible_panics() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3]));
        let _ = g.broadcast(&a, Shape::from(&[4, 3]));
    }

    #[test]
    fn matmul_shape_inference() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3]));
        let b = g.input(DType::F32, Shape::from(&[3, 4]));
        let r = g.matmul(&a, &b);
        assert_eq!(r.shape, Shape::from(&[2, 4]));
    }

    #[test]
    #[should_panic(expected = "inner dims mismatch")]
    fn matmul_inner_mismatch_panics() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3]));
        let b = g.input(DType::F32, Shape::from(&[5, 4]));
        let _ = g.matmul(&a, &b);
    }

    #[test]
    fn comparison_yields_u8() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[3]));
        let b = g.input(DType::F32, Shape::from(&[3]));
        let r = g.equal(&a, &b);
        assert_eq!(r.dtype, DType::U8);
    }

    #[test]
    fn where_basic() {
        let mut g = GraphBuilder::new();
        let p = g.input(DType::U8, Shape::from(&[3]));
        let t = g.input(DType::F32, Shape::from(&[3]));
        let f = g.input(DType::F32, Shape::from(&[3]));
        let r = g.where_(&p, &t, &f);
        assert_eq!(r.dtype, DType::F32);
        assert_eq!(r.shape, Shape::from(&[3]));
    }

    #[test]
    fn cast_changes_dtype_keeps_shape() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 2]));
        let r = g.cast(&a, DType::I32);
        assert_eq!(r.dtype, DType::I32);
        assert_eq!(r.shape, Shape::from(&[2, 2]));
    }

    #[test]
    fn reduce_sum_keepdims() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3, 4]));
        let r = g.reduce_sum(&a, vec![1], false);
        assert_eq!(r.shape, Shape::from(&[2, 4]));
        let r2 = g.reduce_sum(&a, vec![1], true);
        assert_eq!(r2.shape, Shape::from(&[2, 1, 4]));
    }

    #[test]
    fn reduce_all_to_scalar() {
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[2, 3]));
        let r = g.reduce_sum(&a, vec![], false);
        assert_eq!(r.shape, Shape::scalar());
    }

    #[test]
    fn reduce_shape_helper() {
        let s = reduce_shape(&Shape::from(&[2, 3, 4]), &[1], false);
        assert_eq!(s, Shape::from(&[2, 4]));
        let s = reduce_shape(&Shape::from(&[2, 3, 4]), &[1], true);
        assert_eq!(s, Shape::from(&[2, 1, 4]));
        let s = reduce_shape(&Shape::from(&[2, 3]), &[], true);
        assert_eq!(s, Shape::from(&[1, 1]));
        let s = reduce_shape(&Shape::from(&[2, 3]), &[], false);
        assert_eq!(s, Shape::scalar());
    }
}
