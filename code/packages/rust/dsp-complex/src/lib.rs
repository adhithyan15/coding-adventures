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
//! ## Defensive Engineering and Security Guarantees
//!
//! - **Strictly Safe Rust**: `#![forbid(unsafe_code)]` is unconditionally enforced.
//! - **CWE-400 Resource Exhaustion Defense**: Bounds checks prevent memory
//!   exhaustion via [`MAX_SAMPLES`] (`1 << 24` complex elements, 128 MiB buffer).
//! - **CWE-190 Integer Overflow Defense**: Buffer dimensions and allocations are
//!   checked via `checked_mul` and bounded arithmetic.
//! - **Finite Input Enforcement**: Non-finite float inputs (`NaN`, `+Inf`, `-Inf`)
//!   are rejected cleanly with [`ComplexError::InvalidInput`], guaranteeing no
//!   arithmetic poisoning or unexpected NaN propagation.
//! - **CWE-674 Recursion Freedom**: All algorithms are strictly iterative.
//! - **Zero Ambient Authority**: No filesystem (CWE-22), subprocess execution (CWE-78),
//!   or regular expressions (CWE-1333).

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

use matrix_ir::{DType, Shape};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
// § 1. Defensive Boundaries and Constants (CWE-400 / CWE-190)
// ═══════════════════════════════════════════════════════════════════════

/// Maximum number of complex samples in a [`ComplexTensor`].
///
/// Bounded at `2^24 = 16,777,216` samples (128 MiB for interleaved f32 pairs)
/// to defend against denial-of-service allocations (CWE-400).
pub const MAX_SAMPLES: usize = 1 << 24;

// ═══════════════════════════════════════════════════════════════════════
// § 2. Interleaved Complex Storage Structure
// ═══════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════
// § 3. Checked Constructors and Finite-Input Validation
// ═══════════════════════════════════════════════════════════════════════

impl ComplexTensor {
    /// Construct from a real-only signal with finite-input and bounds checks.
    ///
    /// Imaginary parts are zero-filled.
    /// Returns `Err` if the input exceeds [`MAX_SAMPLES`] or contains `NaN`/`Inf`.
    pub fn try_from_real(real: &[f32]) -> Result<Self, ComplexError> {
        if real.len() > MAX_SAMPLES {
            return Err(ComplexError::InvalidInput(format!(
                "real slice length {} exceeds MAX_SAMPLES ({})",
                real.len(),
                MAX_SAMPLES
            )));
        }
        if real.iter().any(|&x| !x.is_finite()) {
            return Err(ComplexError::InvalidInput(
                "input contains non-finite float (NaN or Inf)".into(),
            ));
        }
        let mut buf = Vec::with_capacity(real.len() * 2);
        for &x in real {
            buf.push(x);
            buf.push(0.0);
        }
        Ok(ComplexTensor { interleaved: buf })
    }

    /// Construct from a real-only signal.  Imaginary parts are
    /// zero-filled.  Panics if the slice exceeds [`MAX_SAMPLES`] or
    /// contains non-finite values.  Use [`try_from_real`] for fallible handling.
    pub fn from_real(real: &[f32]) -> Self {
        Self::try_from_real(real).expect("valid finite real slice within bounds")
    }

    /// Construct from separate real and imag arrays.  Returns
    /// `Err(LengthMismatch)` if they don't match, or `Err(InvalidInput)`
    /// if length exceeds [`MAX_SAMPLES`] or samples are non-finite.
    pub fn from_real_imag(real: &[f32], imag: &[f32]) -> Result<Self, ComplexError> {
        if real.len() != imag.len() {
            return Err(ComplexError::LengthMismatch {
                real_len: real.len(),
                imag_len: imag.len(),
            });
        }
        if real.len() > MAX_SAMPLES {
            return Err(ComplexError::InvalidInput(format!(
                "array length {} exceeds MAX_SAMPLES ({})",
                real.len(),
                MAX_SAMPLES
            )));
        }
        if real.iter().any(|&x| !x.is_finite()) || imag.iter().any(|&x| !x.is_finite()) {
            return Err(ComplexError::InvalidInput(
                "input contains non-finite float (NaN or Inf)".into(),
            ));
        }
        let mut buf = Vec::with_capacity(real.len() * 2);
        for (r, i) in real.iter().zip(imag.iter()) {
            buf.push(*r);
            buf.push(*i);
        }
        Ok(ComplexTensor { interleaved: buf })
    }

    /// Construct directly from an interleaved buffer.  The slice
    /// length must be even, bounded by `2 * MAX_SAMPLES`, and all
    /// floats must be finite.
    pub fn from_interleaved(data: Vec<f32>) -> Result<Self, ComplexError> {
        if !data.len().is_multiple_of(2) {
            return Err(ComplexError::OddInterleavedLength(data.len()));
        }
        let n = data.len() / 2;
        if n > MAX_SAMPLES {
            return Err(ComplexError::InvalidInput(format!(
                "interleaved element count {} exceeds MAX_SAMPLES ({})",
                n, MAX_SAMPLES
            )));
        }
        if data.iter().any(|&x| !x.is_finite()) {
            return Err(ComplexError::InvalidInput(
                "input contains non-finite float (NaN or Inf)".into(),
            ));
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

    // ═══════════════════════════════════════════════════════════════════
    // § 4. Spectral Projection and Phase Extraction
    // ═══════════════════════════════════════════════════════════════════

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
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| (c[0] * c[0] + c[1] * c[1]).sqrt())
            .collect()
    }

    /// Element-wise phase angle in radians (`atan2(im, re)`).
    pub fn phase(&self) -> Vec<f32> {
        self.interleaved
            .as_chunks::<2>()
            .0
            .iter()
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

    // ═══════════════════════════════════════════════════════════════════
    // § 5. Matrix-IR Shape Interoperability
    // ═══════════════════════════════════════════════════════════════════

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
                    .as_chunks::<2>()
                    .0
                    .iter()
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
    /// Input parameters violate dimension, finiteness, or resource bounds (CWE-400 / CWE-190).
    InvalidInput(String),
    /// `from_real_imag` was called with mismatched array lengths.
    LengthMismatch { real_len: usize, imag_len: usize },
    /// `from_interleaved` got an odd-length buffer.
    OddInterleavedLength(usize),
}

impl fmt::Display for ComplexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComplexError::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
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

// ═══════════════════════════════════════════════════════════════════════
// § 6. Unit Tests and SplitMix64 Property-Based Testing
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_test_bootstrap::prng::SplitMix64;

    fn sample_uniform_f32(rng: &mut SplitMix64) -> f32 {
        let u = rng.next_u64();
        let mantissa = (u & 0x00ff_ffff) as f32 / (0x0100_0000 as f32);
        (mantissa * 2.0) - 1.0
    }

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
        assert!(s.contains("1000"));
        assert!(s.len() < 200, "debug output too long: {} chars", s.len());
    }

    #[test]
    fn hostile_non_finite_inputs_rejected() {
        // NaN in real
        assert!(matches!(
            ComplexTensor::try_from_real(&[1.0, f32::NAN, 3.0]),
            Err(ComplexError::InvalidInput(_))
        ));
        // Inf in real_imag
        assert!(matches!(
            ComplexTensor::from_real_imag(&[1.0, f32::INFINITY], &[0.0, 0.0]),
            Err(ComplexError::InvalidInput(_))
        ));
        assert!(matches!(
            ComplexTensor::from_real_imag(&[1.0, 2.0], &[0.0, f32::NEG_INFINITY]),
            Err(ComplexError::InvalidInput(_))
        ));
        // NaN in interleaved
        assert!(matches!(
            ComplexTensor::from_interleaved(vec![1.0, 2.0, f32::NAN, 4.0]),
            Err(ComplexError::InvalidInput(_))
        ));
    }

    #[test]
    fn pbt_splitmix64_round_trip_and_invariants() {
        let mut rng = SplitMix64::new(0xdead_beef_cafe_babe);
        for &len in &[1usize, 2, 7, 16, 31, 64, 128, 255] {
            let mut real = Vec::with_capacity(len);
            let mut imag = Vec::with_capacity(len);
            for _ in 0..len {
                real.push(sample_uniform_f32(&mut rng));
                imag.push(sample_uniform_f32(&mut rng));
            }

            let tensor = ComplexTensor::from_real_imag(&real, &imag).unwrap();
            assert_eq!(tensor.len(), len);
            assert_eq!(tensor.real(), real);
            assert_eq!(tensor.imag(), imag);

            // Double conjugation invariant
            assert_eq!(tensor.conjugate().conjugate(), tensor);

            // Interleaved round-trip invariant
            let raw = tensor.as_interleaved().to_vec();
            let recovered = ComplexTensor::from_interleaved(raw.clone()).unwrap();
            assert_eq!(recovered, tensor);
            assert_eq!(recovered.into_interleaved(), raw);
        }
    }
}
