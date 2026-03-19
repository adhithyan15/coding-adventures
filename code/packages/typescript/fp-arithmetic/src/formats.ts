/**
 * IEEE 754 floating-point formats -- the bit-level anatomy of a float.
 *
 * === What is a floating-point format? ===
 *
 * Floating-point is how computers represent real numbers (like 3.14 or -0.001).
 * It works like scientific notation, but in binary:
 *
 *     Scientific notation:   -6.022 x 10^23
 *     IEEE 754 (binary):     (-1)^sign x 1.mantissa x 2^(exponent - bias)
 *
 * A floating-point number is stored as three bit fields packed into a fixed-width
 * binary word:
 *
 *     FP32 (32 bits):  [sign(1)] [exponent(8)] [mantissa(23)]
 *                       ^         ^              ^
 *                       |         |              |
 *                       |         |              +-- fractional part (after the "1.")
 *                       |         +-- power of 2 (biased: stored value - 127)
 *                       +-- 0 = positive, 1 = negative
 *
 * === The three formats we support ===
 *
 *     Format  Total  Exp  Mantissa  Bias   Used by
 *     ------  -----  ---  --------  ----   -------
 *     FP32     32     8     23      127    CPU, GPU (default precision)
 *     FP16     16     5     10       15    GPU training (mixed precision)
 *     BF16     16     8      7      127    TPU (native), ML training
 *
 * === Why BF16 exists ===
 *
 * BF16 (Brain Float 16) was invented by Google for TPU hardware. It keeps the
 * same exponent range as FP32 (8-bit exponent, bias 127) but truncates the
 * mantissa from 23 bits to just 7. This means:
 *
 *   - Same range as FP32 (can represent very large and very small numbers)
 *   - Much less precision (~2-3 decimal digits vs ~7 for FP32)
 *   - Perfect for ML: gradients can be huge or tiny (need range), but don't
 *     need to be super precise (need less precision)
 *   - Trivial conversion from FP32: just truncate the lower 16 bits!
 *
 * === The implicit leading 1 ===
 *
 * For normal (non-zero, non-denormal) numbers, the mantissa has an implicit
 * leading 1 that is not stored. So a stored mantissa of [1, 0, 1, ...] actually
 * represents 1.101... in binary. This trick gives us one extra bit of precision
 * for free.
 *
 *     Stored bits:   [1, 0, 1, 0, 0, ...]
 *     Actual value:  1.10100...  (the "1." is implicit)
 *
 * The only exception is denormalized numbers (exponent = all zeros), where the
 * implicit bit is 0 instead of 1, allowing representation of very small numbers
 * near zero.
 */

// ---------------------------------------------------------------------------
// FloatFormat -- describes the shape of a floating-point format
// ---------------------------------------------------------------------------

/**
 * Describes the bit layout of an IEEE 754 floating-point format.
 *
 * This is an immutable interface (all fields are readonly). Once created,
 * the fields cannot be changed, preventing bugs where code accidentally
 * mutates a format constant.
 *
 * Properties:
 *   name:          Human-readable name ("fp32", "fp16", "bf16").
 *   totalBits:     Total width of the format in bits.
 *   exponentBits:  Number of bits in the exponent field.
 *   mantissaBits:  Number of explicit mantissa bits (without the implicit
 *                  leading 1). The actual precision is mantissaBits + 1.
 *   bias:          The exponent bias. The true exponent is (stored_exponent - bias).
 *                  For FP32: bias=127, so stored exponent 127 means true exponent 0,
 *                  stored exponent 128 means true exponent 1, etc.
 *
 * Example:
 *     FP32.totalBits     // 32
 *     FP32.exponentBits  // 8
 *     FP32.mantissaBits  // 23
 *     FP32.bias          // 127
 */
export interface FloatFormat {
  readonly name: string;
  readonly totalBits: number;
  readonly exponentBits: number;
  readonly mantissaBits: number;
  readonly bias: number;
}

// ---------------------------------------------------------------------------
// Standard format constants
// ---------------------------------------------------------------------------
// These are module-level singletons. All code that works with floating-point
// should reference these constants rather than constructing FloatFormat manually.

/**
 * FP32 (single precision) -- the workhorse of computing.
 *
 *     [sign(1)] [exponent(8)] [mantissa(23)]
 *      bit 31    bits 30-23    bits 22-0
 *
 * Used by CPU FPUs, GPU CUDA cores, and as the default for most computation.
 * Range: ~1.18e-38 to ~3.40e38, precision: ~7 decimal digits.
 */
export const FP32: FloatFormat = Object.freeze({
  name: "fp32",
  totalBits: 32,
  exponentBits: 8,
  mantissaBits: 23,
  bias: 127,
});

/**
 * FP16 (half precision) -- GPU mixed-precision training.
 *
 *     [sign(1)] [exponent(5)] [mantissa(10)]
 *      bit 15    bits 14-10    bits 9-0
 *
 * Used for GPU training in mixed precision and inference. Saves memory and
 * bandwidth at the cost of range and precision.
 * Range: ~5.96e-8 to ~65504, precision: ~3-4 decimal digits.
 */
export const FP16: FloatFormat = Object.freeze({
  name: "fp16",
  totalBits: 16,
  exponentBits: 5,
  mantissaBits: 10,
  bias: 15,
});

/**
 * BF16 (brain float) -- Google's TPU native format.
 *
 *     [sign(1)] [exponent(8)] [mantissa(7)]
 *      bit 15    bits 14-7     bits 6-0
 *
 * Same exponent range as FP32, but with only 7 mantissa bits (vs 23).
 * Converting FP32 -> BF16 is trivial: just drop the lower 16 bits.
 * Range: same as FP32, precision: ~2-3 decimal digits.
 */
export const BF16: FloatFormat = Object.freeze({
  name: "bf16",
  totalBits: 16,
  exponentBits: 8,
  mantissaBits: 7,
  bias: 127,
});

// ---------------------------------------------------------------------------
// FloatBits -- the actual bit pattern of a floating-point number
// ---------------------------------------------------------------------------

/**
 * The bit-level representation of an IEEE 754 floating-point number.
 *
 * This stores the actual 0s and 1s that make up the number, decomposed into
 * the three fields (sign, exponent, mantissa). All bit arrays are stored
 * MSB-first (index 0 = most significant bit).
 *
 * === Bit layout (FP32 example) ===
 *
 * Consider the number 3.14:
 *
 *     Binary: 1.10010001111010111000011 x 2^1
 *     Sign: 0 (positive)
 *     Exponent: 128 (= 1 + 127 bias) = [1,0,0,0,0,0,0,0]
 *     Mantissa: [1,0,0,1,0,0,0,1,1,1,1,0,1,0,1,1,1,0,0,0,0,1,1]
 *
 * Packed as 32 bits:
 *     [0] [10000000] [10010001111010111000011]
 *     sign  exponent        mantissa
 *
 * Properties:
 *     sign:      0 for positive, 1 for negative.
 *     exponent:  Array of exponent bits, MSB first. Length = fmt.exponentBits.
 *     mantissa:  Array of mantissa bits, MSB first. Length = fmt.mantissaBits.
 *                These are the explicit bits only (no implicit leading 1).
 *     fmt:       The FloatFormat this number is encoded in.
 *
 * Example:
 *     const bits: FloatBits = {
 *       sign: 0,
 *       exponent: [0,1,1,1,1,1,1,1],
 *       mantissa: new Array(23).fill(0),
 *       fmt: FP32,
 *     };
 *     // This is +1.0 in FP32: exponent 127-127=0, mantissa 1.000...
 */
export interface FloatBits {
  readonly sign: number;
  readonly exponent: readonly number[];
  readonly mantissa: readonly number[];
  readonly fmt: FloatFormat;
}
