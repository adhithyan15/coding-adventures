//! IEEE 754 floating-point formats — the bit-level anatomy of a float.
//!
//! # What is a floating-point format?
//!
//! Floating-point is how computers represent real numbers (like 3.14 or -0.001).
//! It works like scientific notation, but in binary:
//!
//! ```text
//! Scientific notation:   -6.022 x 10^23
//! IEEE 754 (binary):     (-1)^sign x 1.mantissa x 2^(exponent - bias)
//! ```
//!
//! A floating-point number is stored as three bit fields packed into a fixed-width
//! binary word:
//!
//! ```text
//! FP32 (32 bits):  [sign(1)] [exponent(8)] [mantissa(23)]
//!                   ^         ^              ^
//!                   |         |              |
//!                   |         |              +-- fractional part (after the "1.")
//!                   |         +-- power of 2 (biased: stored value - 127)
//!                   +-- 0 = positive, 1 = negative
//! ```
//!
//! # The three formats we support
//!
//! ```text
//! Format  Total  Exp  Mantissa  Bias   Used by
//! ------  -----  ---  --------  ----   -------
//! FP32     32     8     23      127    CPU, GPU (default precision)
//! FP16     16     5     10       15    GPU training (mixed precision)
//! BF16     16     8      7      127    TPU (native), ML training
//! ```
//!
//! # Why BF16 exists
//!
//! BF16 (Brain Float 16) was invented by Google for TPU hardware. It keeps the
//! same exponent range as FP32 (8-bit exponent, bias 127) but truncates the
//! mantissa from 23 bits to just 7. This means:
//!
//! - Same range as FP32 (can represent very large and very small numbers)
//! - Much less precision (~2-3 decimal digits vs ~7 for FP32)
//! - Perfect for ML: gradients can be huge or tiny (need range), but don't
//!   need to be super precise (need less precision)
//! - Trivial conversion from FP32: just truncate the lower 16 bits!
//!
//! # The implicit leading 1
//!
//! For normal (non-zero, non-denormal) numbers, the mantissa has an implicit
//! leading 1 that is not stored. So a stored mantissa of `[1, 0, 1, ...]` actually
//! represents `1.101...` in binary. This trick gives us one extra bit of precision
//! for free.
//!
//! ```text
//! Stored bits:   [1, 0, 1, 0, 0, ...]
//! Actual value:  1.10100...  (the "1." is implicit)
//! ```
//!
//! The only exception is denormalized numbers (exponent = all zeros), where the
//! implicit bit is 0 instead of 1, allowing representation of very small numbers
//! near zero.

// =========================================================================
// FloatFormat — describes the shape of a floating-point format
// =========================================================================

/// Describes the bit layout of an IEEE 754 floating-point format.
///
/// This is an immutable struct (all fields are public but the struct is `Copy`,
/// so there is no interior mutability concern). Rust's ownership system ensures
/// that format constants cannot be accidentally modified.
///
/// # Fields
///
/// - `name`: Human-readable name ("fp32", "fp16", "bf16").
/// - `total_bits`: Total width of the format in bits.
/// - `exponent_bits`: Number of bits in the exponent field.
/// - `mantissa_bits`: Number of explicit mantissa bits (without the implicit
///   leading 1). The actual precision is `mantissa_bits + 1`.
/// - `bias`: The exponent bias. The true exponent is `(stored_exponent - bias)`.
///   For FP32: bias=127, so stored exponent 127 means true exponent 0,
///   stored exponent 128 means true exponent 1, etc.
///
/// # Example
///
/// ```
/// use fp_arithmetic::FP32;
/// assert_eq!(FP32.total_bits, 32);
/// assert_eq!(FP32.exponent_bits, 8);
/// assert_eq!(FP32.mantissa_bits, 23);
/// assert_eq!(FP32.bias, 127);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatFormat {
    pub name: &'static str,
    pub total_bits: u32,
    pub exponent_bits: u32,
    pub mantissa_bits: u32,
    pub bias: i32,
}

// =========================================================================
// Standard format constants
// =========================================================================
//
// These are module-level constants. All code that works with floating-point
// should reference these constants rather than constructing FloatFormat manually.

/// FP32 (single precision) — the workhorse of computing.
///
/// ```text
/// [sign(1)] [exponent(8)] [mantissa(23)]
///  bit 31    bits 30-23    bits 22-0
/// ```
///
/// Used by CPU FPUs, GPU CUDA cores, and as the default for most computation.
/// Range: ~1.18e-38 to ~3.40e38, precision: ~7 decimal digits.
pub const FP32: FloatFormat = FloatFormat {
    name: "fp32",
    total_bits: 32,
    exponent_bits: 8,
    mantissa_bits: 23,
    bias: 127,
};

/// FP16 (half precision) — GPU mixed-precision training.
///
/// ```text
/// [sign(1)] [exponent(5)] [mantissa(10)]
///  bit 15    bits 14-10    bits 9-0
/// ```
///
/// Used for GPU training in mixed precision and inference. Saves memory and
/// bandwidth at the cost of range and precision.
/// Range: ~5.96e-8 to ~65504, precision: ~3-4 decimal digits.
pub const FP16: FloatFormat = FloatFormat {
    name: "fp16",
    total_bits: 16,
    exponent_bits: 5,
    mantissa_bits: 10,
    bias: 15,
};

/// BF16 (brain float) — Google's TPU native format.
///
/// ```text
/// [sign(1)] [exponent(8)] [mantissa(7)]
///  bit 15    bits 14-7     bits 6-0
/// ```
///
/// Same exponent range as FP32, but with only 7 mantissa bits (vs 23).
/// Converting FP32 -> BF16 is trivial: just drop the lower 16 bits.
/// Range: same as FP32, precision: ~2-3 decimal digits.
pub const BF16: FloatFormat = FloatFormat {
    name: "bf16",
    total_bits: 16,
    exponent_bits: 8,
    mantissa_bits: 7,
    bias: 127,
};

// =========================================================================
// FloatBits — the actual bit pattern of a floating-point number
// =========================================================================

/// The bit-level representation of an IEEE 754 floating-point number.
///
/// This stores the actual 0s and 1s that make up the number, decomposed into
/// the three fields (sign, exponent, mantissa). All bit vectors are stored
/// MSB-first (index 0 = most significant bit).
///
/// # Bit layout (FP32 example)
///
/// Consider the number 3.14:
///
/// ```text
/// Binary: 1.10010001111010111000011 x 2^1
/// Sign: 0 (positive)
/// Exponent: 128 (= 1 + 127 bias) = [1,0,0,0,0,0,0,0]
/// Mantissa: [1,0,0,1,0,0,0,1,1,1,1,0,1,0,1,1,1,0,0,0,0,1,1]
/// ```
///
/// Packed as 32 bits:
///
/// ```text
/// [0] [10000000] [10010001111010111000011]
/// sign  exponent        mantissa
/// ```
///
/// # Fields
///
/// - `sign`: 0 for positive, 1 for negative.
/// - `exponent`: Vec of exponent bits, MSB first. Length = `fmt.exponent_bits`.
/// - `mantissa`: Vec of mantissa bits, MSB first. Length = `fmt.mantissa_bits`.
///   These are the explicit bits only (no implicit leading 1).
/// - `fmt`: The `FloatFormat` this number is encoded in.
#[derive(Debug, Clone, PartialEq)]
pub struct FloatBits {
    pub sign: u8,
    pub exponent: Vec<u8>,
    pub mantissa: Vec<u8>,
    pub fmt: FloatFormat,
}

// =========================================================================
// Helper constructors for common special values
// =========================================================================

/// Creates a quiet NaN in the given format.
///
/// NaN (Not a Number) is represented by exponent = all 1s and mantissa != 0.
/// The MSB of the mantissa being 1 makes it a "quiet" NaN (as opposed to a
/// "signaling" NaN with MSB 0).
pub fn make_nan(fmt: FloatFormat) -> FloatBits {
    let mut mantissa = vec![0u8; fmt.mantissa_bits as usize];
    mantissa[0] = 1; // quiet NaN: MSB of mantissa is 1
    FloatBits {
        sign: 0,
        exponent: vec![1u8; fmt.exponent_bits as usize],
        mantissa,
        fmt,
    }
}

/// Creates positive or negative infinity in the given format.
///
/// Infinity is represented by exponent = all 1s and mantissa = all 0s.
pub fn make_inf(sign: u8, fmt: FloatFormat) -> FloatBits {
    FloatBits {
        sign,
        exponent: vec![1u8; fmt.exponent_bits as usize],
        mantissa: vec![0u8; fmt.mantissa_bits as usize],
        fmt,
    }
}

/// Creates positive or negative zero in the given format.
///
/// Zero is represented by exponent = all 0s and mantissa = all 0s.
/// IEEE 754 has both +0 and -0 — they compare equal but have different bits.
pub fn make_zero(sign: u8, fmt: FloatFormat) -> FloatBits {
    FloatBits {
        sign,
        exponent: vec![0u8; fmt.exponent_bits as usize],
        mantissa: vec![0u8; fmt.mantissa_bits as usize],
        fmt,
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that the three standard IEEE 754 format constants have the correct
    /// parameters. These constants define the bit layout of every floating-point
    /// number in this crate, so getting them wrong would break everything.
    #[test]
    fn test_float_format_constants() {
        // FP32: 1 sign + 8 exponent + 23 mantissa = 32
        assert_eq!(FP32.total_bits, 32);
        assert_eq!(FP32.exponent_bits, 8);
        assert_eq!(FP32.mantissa_bits, 23);
        assert_eq!(FP32.bias, 127);
        assert_eq!(FP32.name, "fp32");

        // FP16: 1 sign + 5 exponent + 10 mantissa = 16
        assert_eq!(FP16.total_bits, 16);
        assert_eq!(FP16.exponent_bits, 5);
        assert_eq!(FP16.mantissa_bits, 10);
        assert_eq!(FP16.bias, 15);

        // BF16: 1 sign + 8 exponent + 7 mantissa = 16
        assert_eq!(BF16.total_bits, 16);
        assert_eq!(BF16.exponent_bits, 8);
        assert_eq!(BF16.mantissa_bits, 7);
        assert_eq!(BF16.bias, 127);
    }

    /// Verify that bit counts add up: sign(1) + exponent + mantissa = total.
    #[test]
    fn test_bit_counts_add_up() {
        for fmt in &[FP32, FP16, BF16] {
            let total = 1 + fmt.exponent_bits + fmt.mantissa_bits;
            assert_eq!(
                total, fmt.total_bits,
                "{}: 1 + {} + {} = {}, want {}",
                fmt.name, fmt.exponent_bits, fmt.mantissa_bits, total, fmt.total_bits
            );
        }
    }

    /// Test the NaN constructor.
    #[test]
    fn test_make_nan() {
        let nan = make_nan(FP32);
        assert_eq!(nan.sign, 0);
        assert_eq!(nan.exponent.len(), 8);
        // All exponent bits should be 1
        for &bit in &nan.exponent {
            assert_eq!(bit, 1);
        }
        // First mantissa bit should be 1 (quiet NaN)
        assert_eq!(nan.mantissa[0], 1);
    }

    /// Test the infinity constructor.
    #[test]
    fn test_make_inf() {
        let pos_inf = make_inf(0, FP32);
        let neg_inf = make_inf(1, FP32);
        assert_eq!(pos_inf.sign, 0);
        assert_eq!(neg_inf.sign, 1);
        // Mantissa should be all zeros
        for &bit in &pos_inf.mantissa {
            assert_eq!(bit, 0);
        }
    }

    /// Test the zero constructor.
    #[test]
    fn test_make_zero() {
        let pos_zero = make_zero(0, FP32);
        let neg_zero = make_zero(1, FP32);
        assert_eq!(pos_zero.sign, 0);
        assert_eq!(neg_zero.sign, 1);
        for &bit in &pos_zero.exponent {
            assert_eq!(bit, 0);
        }
    }
}
