//! CPU **reference** implementations of the core array operations.
//!
//! These compute exact results today. In a later MA item the same ops will run
//! through the planned `ComputeGraph` on the chosen backend (CPU/GPU) — the
//! lowering and dispatch decision already live in [`crate::accel`]. Keeping a
//! correct reference path here means every PR is usable while the execution
//! layer is built out.

use crate::value::Array;

/// A binary elementwise operator (`+ - * /`, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl BinOp {
    fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            BinOp::Add => a + b,
            BinOp::Sub => a - b,
            BinOp::Mul => a * b,
            BinOp::Div => a / b,
        }
    }
}

/// Elementwise binary op with scalar broadcasting. Either operand may be a
/// scalar; otherwise the shapes must match exactly. (Full NumPy/MATLAB
/// broadcasting follows in a later item.)
pub fn elementwise(op: BinOp, a: &Array, b: &Array) -> Result<Array, String> {
    let (ad, bd) = (a.data(), b.data());
    let data: Vec<f64> = match (a.is_scalar(), b.is_scalar()) {
        (true, _) => bd.iter().map(|&y| op.apply(ad[0], y)).collect(),
        (_, true) => ad.iter().map(|&x| op.apply(x, bd[0])).collect(),
        _ => {
            if a.shape() != b.shape() {
                return Err(format!(
                    "non-conformable arrays: {:?} vs {:?}",
                    a.shape(),
                    b.shape()
                ));
            }
            ad.iter().zip(bd).map(|(&x, &y)| op.apply(x, y)).collect()
        }
    };
    // Result takes the non-scalar operand's shape (or the scalar's if both are).
    let shape = if a.is_scalar() { b.shape() } else { a.shape() };
    Array::from_shape(data, shape.to_vec())
}

pub fn add(a: &Array, b: &Array) -> Result<Array, String> {
    elementwise(BinOp::Add, a, b)
}
pub fn sub(a: &Array, b: &Array) -> Result<Array, String> {
    elementwise(BinOp::Sub, a, b)
}
pub fn mul(a: &Array, b: &Array) -> Result<Array, String> {
    elementwise(BinOp::Mul, a, b)
}
pub fn div(a: &Array, b: &Array) -> Result<Array, String> {
    elementwise(BinOp::Div, a, b)
}

/// Matrix product `[m, k] · [k, n] → [m, n]` (column-major throughout).
pub fn matmul(a: &Array, b: &Array) -> Result<Array, String> {
    let (m, ka) = (a.nrows(), a.ncols());
    let (kb, n) = (b.nrows(), b.ncols());
    if ka != kb {
        return Err(format!(
            "matmul: inner dimensions disagree ({m}x{ka} · {kb}x{n})"
        ));
    }
    let (ad, bd) = (a.data(), b.data());
    let mut out = vec![0.0; m * n];
    for j in 0..n {
        for i in 0..m {
            let mut acc = 0.0;
            for p in 0..ka {
                acc += ad[p * m + i] * bd[j * kb + p]; // column-major indexing
            }
            out[j * m + i] = acc;
        }
    }
    Array::from_shape(out, vec![m, n])
}

/// Matrix transpose.
pub fn transpose(a: &Array) -> Array {
    let (m, n) = (a.nrows(), a.ncols());
    let ad = a.data();
    let mut out = vec![0.0; m * n];
    for j in 0..n {
        for i in 0..m {
            out[i * n + j] = ad[j * m + i];
        }
    }
    Array::from_shape(out, vec![n, m]).expect("transpose preserves element count")
}

/// Whole-array reductions.
pub fn sum(a: &Array) -> f64 {
    a.data().iter().sum()
}
pub fn mean(a: &Array) -> f64 {
    if a.is_empty() {
        f64::NAN
    } else {
        sum(a) / a.len() as f64
    }
}
pub fn max(a: &Array) -> f64 {
    a.data().iter().copied().fold(f64::NEG_INFINITY, f64::max)
}
pub fn min(a: &Array) -> f64 {
    a.data().iter().copied().fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Array;

    #[test]
    fn elementwise_equal_shapes() {
        let a = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let b = Array::from_vec(vec![10.0, 20.0, 30.0]);
        assert_eq!(add(&a, &b).unwrap().data(), &[11.0, 22.0, 33.0]);
        assert_eq!(sub(&b, &a).unwrap().data(), &[9.0, 18.0, 27.0]);
        assert_eq!(mul(&a, &b).unwrap().data(), &[10.0, 40.0, 90.0]);
        assert_eq!(div(&b, &a).unwrap().data(), &[10.0, 10.0, 10.0]);
    }

    #[test]
    fn scalar_broadcasts_either_side() {
        let v = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let s = Array::scalar(2.0);
        assert_eq!(mul(&s, &v).unwrap().data(), &[2.0, 4.0, 6.0]); // scalar lhs
        assert_eq!(add(&v, &s).unwrap().data(), &[3.0, 4.0, 5.0]); // scalar rhs
                                                                   // The result keeps the array operand's shape, not the scalar's.
        assert_eq!(add(&v, &s).unwrap().shape(), &[3]);
    }

    #[test]
    fn two_scalars_stay_scalar() {
        let r = add(&Array::scalar(2.0), &Array::scalar(40.0)).unwrap();
        assert!(r.is_scalar());
        assert_eq!(r.data(), &[42.0]);
    }

    #[test]
    fn nonconformable_is_an_error() {
        let a = Array::from_vec(vec![1.0, 2.0]);
        let b = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert!(add(&a, &b).is_err());
    }

    #[test]
    fn nan_and_inf_propagate() {
        let a = Array::from_vec(vec![1.0, 0.0]);
        let b = Array::from_vec(vec![0.0, 0.0]);
        let q = div(&a, &b).unwrap();
        assert!(q.data()[0].is_infinite());
        assert!(q.data()[1].is_nan());
    }

    #[test]
    fn matmul_identity_and_product() {
        // [[1,2],[3,4]] · I == itself.
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        assert_eq!(matmul(&a, &Array::eye(2)).unwrap().data(), a.data());

        // [[1,2],[3,4]] · [[5,6],[7,8]] = [[19,22],[43,50]].
        let b = Array::from_rows(vec![vec![5.0, 6.0], vec![7.0, 8.0]]).unwrap();
        let c = matmul(&a, &b).unwrap();
        assert_eq!(c.shape(), &[2, 2]);
        assert_eq!(c.get(0, 0), Some(19.0));
        assert_eq!(c.get(0, 1), Some(22.0));
        assert_eq!(c.get(1, 0), Some(43.0));
        assert_eq!(c.get(1, 1), Some(50.0));
    }

    #[test]
    fn matmul_nonsquare() {
        // [2x3] · [3x1] -> [2x1].
        let a = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let x = Array::from_rows(vec![vec![1.0], vec![0.0], vec![-1.0]]).unwrap();
        let y = matmul(&a, &x).unwrap();
        assert_eq!(y.shape(), &[2, 1]);
        assert_eq!(y.data(), &[-2.0, -2.0]);
    }

    #[test]
    fn matmul_dimension_mismatch_errors() {
        let a = Array::from_rows(vec![vec![1.0, 2.0]]).unwrap(); // 1x2
        let b = Array::from_rows(vec![vec![1.0, 2.0]]).unwrap(); // 1x2
        assert!(matmul(&a, &b).is_err());
    }

    #[test]
    fn transpose_swaps_axes() {
        let a = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let t = transpose(&a);
        assert_eq!(t.shape(), &[3, 2]);
        assert_eq!(t.get(0, 0), Some(1.0));
        assert_eq!(t.get(2, 1), Some(6.0));
        // Transpose is an involution.
        assert_eq!(transpose(&t).data(), a.data());
    }

    #[test]
    fn reductions() {
        let a = Array::from_vec(vec![2.0, 4.0, 6.0, 8.0]);
        assert_eq!(sum(&a), 20.0);
        assert_eq!(mean(&a), 5.0);
        assert_eq!(max(&a), 8.0);
        assert_eq!(min(&a), 2.0);
    }

    #[test]
    fn mean_of_empty_is_nan() {
        let empty = Array::from_shape(vec![], vec![0, 0]).unwrap();
        assert!(mean(&empty).is_nan());
    }
}
