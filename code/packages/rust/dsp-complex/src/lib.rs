//! # `dsp-complex` — complex-number helper for the DSP layer
//!
//! **DSP01 Phase 2.**  Provides the [`ComplexTensor`] type that the
//! DSP00 spec calls for: an interleaved `[real, imag]` view of f32
//! data where the trailing axis is size 2.
//!
//! ## V1 storage layout
//!
//! Per DSP00's complex-number convention, a length-`N` complex
//! signal is stored as a length-`2N` `Vec<f32>` with layout:
//!
//! ```text
//! [re_0, im_0, re_1, im_1, …, re_{N-1}, im_{N-1}]
//! ```
//!
//! Matches matrix-ir's `Shape::from(&[N, 2])` with dtype `F32`.  Once
//! a `Slice` op lands in matrix-ir (DSP01 Phase 3 dependency), this
//! crate will introduce a `Graph`-backed `ComplexTensor` variant
//! alongside the scalar one — the public method names (real / imag /
//! magnitude / phase / conjugate) won't change.
//!
//! ## What this crate is NOT
//!
//! - **Not a full Complex32 type.**  We're not implementing
//!   `std::ops::*` traits, transcendentals (`Complex::exp`), or
//!   trigonometric helpers.  When user code needs those it does the
//!   math on the underlying `Vec<f32>` interleaved layout directly,
//!   or composes via the matrix-ir graph builder.  A future
//!   `dsp-complex-math` crate could add that surface; YAGNI for V1.
//! - **Not a tensor.**  The matrix-ir-backed variant lands in Phase 3.
//!   V1 is scalar host data only — same shape and bit-exact behaviour
//!   the matrix-ir version will match.

#![warn(rust_2018_idioms)]

use matrix_ir::{DType, Shape};
use std::fmt;

/// A complex-valued signal/spectrum laid out as interleaved
/// `[re, im, re, im, …]` f32 pairs.
///
/// **MX06 Phase 2 storage**: pure `Vec<f32>` of length `2 * N`,
/// where `N` is the number of complex elements.  Phase 3 will add
/// a matrix-ir-backed variant; the public method names won't change.
#[derive(Clone, PartialEq)]
pub struct ComplexTensor {
    /// Interleaved real / imag pairs.  Always `2 * num_complex`
    /// elements; an empty signal is `vec![]` with `num_complex = 0`.
    interleaved: Vec<f32>,
}

impl ComplexTensor {
    /// Construct from a real-only signal.  Imaginary parts are
    /// zero-filled.  Cheap memory-wise: `2N` floats for `N` real
    /// samples, half of them zero.  Phase 3's graph-backed variant
    /// will reuse the original input tensor and only generate zeros
    /// where needed.
    pub fn from_real(real: &[f32]) -> Self {
        let mut buf = Vec::with_capacity(real.len() * 2);
        for &x in real {
            buf.push(x);
            buf.push(0.0);
        }
        ComplexTensor { interleaved: buf }
    }

    /// Construct from separate real and imag arrays.  Returns
    /// `Err(LengthMismatch)` if they don't match.
    pub fn from_real_imag(real: &[f32], imag: &[f32]) -> Result<Self, ComplexError> {
        if real.len() != imag.len() {
            return Err(ComplexError::LengthMismatch {
                real_len: real.len(),
                imag_len: imag.len(),
            });
        }
        let mut buf = Vec::with_capacity(real.len() * 2);
        for (r, i) in real.iter().zip(imag.iter()) {
            buf.push(*r);
            buf.push(*i);
        }
        Ok(ComplexTensor { interleaved: buf })
    }

    /// Construct directly from an interleaved buffer.  The slice
    /// length must be even.  Used by FFT primitives that already
    /// produce interleaved output.
    pub fn from_interleaved(data: Vec<f32>) -> Result<Self, ComplexError> {
        if data.len() % 2 != 0 {
            return Err(ComplexError::OddInterleavedLength(data.len()));
        }
        Ok(ComplexTensor { interleaved: data })
    }

    /// Number of complex elements (half the interleaved length).
    pub fn len(&self) -> usize {
        self.interleaved.len() / 2
    }

    /// Whether the tensor has zero complex elements.
    pub fn is_empty(&self) -> bool {
        self.interleaved.is_empty()
    }

    /// Borrow the underlying interleaved buffer.  Read-only; callers
    /// that want to mutate should construct a new `ComplexTensor`.
    pub fn as_interleaved(&self) -> &[f32] {
        &self.interleaved
    }

    /// Consume the tensor and return the underlying buffer.  Used by
    /// FFT primitives that take ownership.
    pub fn into_interleaved(self) -> Vec<f32> {
        self.interleaved
    }

    // ── Accessors that produce real-valued vectors ─────────────────
    //
    // These match the DSP00 spec API.  Phase 3 will reintroduce them
    // as graph-builder methods that emit Slice/Mul/Sqrt/Atan2 ops;
    // the names stay the same so user code ports forward.

    /// Real components: `[re_0, re_1, …, re_{N-1}]`.
    pub fn real(&self) -> Vec<f32> {
        self.interleaved.iter().step_by(2).copied().collect()
    }

    /// Imaginary components: `[im_0, im_1, …, im_{N-1}]`.
    pub fn imag(&self) -> Vec<f32> {
        self.interleaved.iter().skip(1).step_by(2).copied().collect()
    }

    /// Element-wise magnitude (`√(re² + im²)`).
    pub fn magnitude(&self) -> Vec<f32> {
        self.interleaved
            .chunks_exact(2)
            .map(|c| (c[0] * c[0] + c[1] * c[1]).sqrt())
            .collect()
    }

    /// Element-wise phase angle in radians (`atan2(im, re)`).
    pub fn phase(&self) -> Vec<f32> {
        self.interleaved
            .chunks_exact(2)
            .map(|c| c[1].atan2(c[0]))
            .collect()
    }

    /// Element-wise complex conjugate: `(re, im) → (re, -im)`.
    pub fn conjugate(&self) -> ComplexTensor {
        let mut out = self.interleaved.clone();
        for i in (1..out.len()).step_by(2) {
            out[i] = -out[i];
        }
        ComplexTensor { interleaved: out }
    }

    /// Shape this tensor will have when promoted to a matrix-ir
    /// tensor in Phase 3.  Returns `[len, 2]` with dtype `F32`.
    /// Useful for callers building graphs that need to allocate
    /// matching tensors.
    pub fn matrix_shape(&self) -> Shape {
        Shape::from(&[self.len() as u32, 2])
    }

    /// Dtype matching the matrix-ir convention.  Always `F32` in V1.
    pub fn matrix_dtype(&self) -> DType {
        DType::F32
    }
}

impl fmt::Debug for ComplexTensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Show first few elements + length so big spectra don't dump
        // megabytes into the test log.
        let max_show = 4usize;
        f.debug_struct("ComplexTensor")
            .field("len", &self.len())
            .field(
                "head",
                &self
                    .interleaved
                    .chunks_exact(2)
                    .take(max_show)
                    .map(|c| (c[0], c[1]))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Errors produced by [`ComplexTensor`] constructors.
#[derive(Debug, Clone, PartialEq)]
pub enum ComplexError {
    /// `from_real_imag` was called with mismatched array lengths.
    LengthMismatch { real_len: usize, imag_len: usize },
    /// `from_interleaved` got an odd-length buffer.
    OddInterleavedLength(usize),
}

impl fmt::Display for ComplexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComplexError::LengthMismatch { real_len, imag_len } => write!(
                f,
                "real array has {} elements, imag has {}; must match",
                real_len, imag_len
            ),
            ComplexError::OddInterleavedLength(n) => write!(
                f,
                "interleaved buffer has odd length {}; must be even",
                n
            ),
        }
    }
}

impl std::error::Error for ComplexError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_real_zero_fills_imag() {
        let c = ComplexTensor::from_real(&[1.0, 2.0, 3.0]);
        assert_eq!(c.len(), 3);
        assert_eq!(c.real(), vec![1.0, 2.0, 3.0]);
        assert_eq!(c.imag(), vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn from_real_imag_pairs_correctly() {
        let c = ComplexTensor::from_real_imag(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]).unwrap();
        assert_eq!(c.len(), 3);
        assert_eq!(c.real(), vec![1.0, 2.0, 3.0]);
        assert_eq!(c.imag(), vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn from_real_imag_mismatch_errors() {
        let err = ComplexTensor::from_real_imag(&[1.0], &[2.0, 3.0]).unwrap_err();
        assert_eq!(
            err,
            ComplexError::LengthMismatch {
                real_len: 1,
                imag_len: 2
            }
        );
    }

    #[test]
    fn from_interleaved_validates_even_length() {
        assert!(ComplexTensor::from_interleaved(vec![1.0, 2.0, 3.0, 4.0]).is_ok());
        let err = ComplexTensor::from_interleaved(vec![1.0, 2.0, 3.0]).unwrap_err();
        assert_eq!(err, ComplexError::OddInterleavedLength(3));
    }

    #[test]
    fn magnitude_handles_known_values() {
        // (3, 4) → 5, (-3, -4) → 5, (0, 0) → 0, (1, 0) → 1
        let c = ComplexTensor::from_real_imag(&[3.0, -3.0, 0.0, 1.0], &[4.0, -4.0, 0.0, 0.0])
            .unwrap();
        let mag = c.magnitude();
        assert_eq!(mag, vec![5.0, 5.0, 0.0, 1.0]);
    }

    #[test]
    fn phase_handles_known_angles() {
        let c = ComplexTensor::from_real_imag(&[1.0, 0.0, -1.0, 0.0], &[0.0, 1.0, 0.0, -1.0])
            .unwrap();
        let p = c.phase();
        // atan2(0, 1) = 0, atan2(1, 0) = π/2, atan2(0, -1) = π, atan2(-1, 0) = -π/2
        let pi = std::f32::consts::PI;
        let tolerance = 1e-6;
        assert!((p[0] - 0.0).abs() < tolerance);
        assert!((p[1] - pi / 2.0).abs() < tolerance);
        assert!((p[2] - pi).abs() < tolerance);
        assert!((p[3] - (-pi / 2.0)).abs() < tolerance);
    }

    #[test]
    fn conjugate_negates_imag_only() {
        let c = ComplexTensor::from_real_imag(&[1.0, 2.0], &[3.0, -4.0]).unwrap();
        let conj = c.conjugate();
        assert_eq!(conj.real(), vec![1.0, 2.0]);
        assert_eq!(conj.imag(), vec![-3.0, 4.0]);
    }

    #[test]
    fn conjugate_twice_is_identity() {
        let c = ComplexTensor::from_real_imag(&[1.0, 2.0, 3.0], &[-4.0, 5.0, -6.0]).unwrap();
        let cc = c.conjugate().conjugate();
        assert_eq!(c.as_interleaved(), cc.as_interleaved());
    }

    #[test]
    fn empty_tensor_is_well_defined() {
        let c = ComplexTensor::from_real(&[]);
        assert_eq!(c.len(), 0);
        assert!(c.is_empty());
        assert!(c.real().is_empty());
        assert!(c.imag().is_empty());
        assert!(c.magnitude().is_empty());
        assert!(c.phase().is_empty());
    }

    #[test]
    fn matrix_shape_and_dtype_match_dsp00_convention() {
        let c = ComplexTensor::from_real(&[1.0; 8]);
        let shape = c.matrix_shape();
        assert_eq!(shape.dims, vec![8, 2]);
        assert_eq!(c.matrix_dtype(), DType::F32);
    }

    #[test]
    fn as_interleaved_and_into_interleaved_round_trip() {
        let original = vec![1.0, 2.0, 3.0, 4.0];
        let c = ComplexTensor::from_interleaved(original.clone()).unwrap();
        assert_eq!(c.as_interleaved(), &original[..]);
        assert_eq!(c.into_interleaved(), original);
    }

    #[test]
    fn debug_impl_truncates_for_large_signals() {
        let c = ComplexTensor::from_real(&[0.0; 1000]);
        let s = format!("{:?}", c);
        // Should mention len: 1000 but not dump 1000 pairs.
        assert!(s.contains("1000"));
        assert!(s.len() < 200, "debug output too long: {} chars", s.len());
    }
}
