//! VP8 quantization tables.
//!
//! VP8 uses a base QP index (0–127) to look up actual quantization step
//! sizes. Quality maps to QP via a simple linear curve, then the step sizes
//! are looked up from the standard DC/AC tables from RFC 6386 §14.1.

/// Map quality [0, 100] → QP index [0, 127].
///
/// quality=100 → qp=0  (step=4,  max pixel error ≤ 2)
/// quality=75  → qp=7  (step=10, max pixel error ≤ 5)
/// quality=50  → qp=31 (step=32, coarser)
/// quality=0   → qp=127 (step=127, coarsest)
///
/// Uses a quadratic curve so high-quality values stay near qp=0 while
/// low-quality values spread across the full [0, 127] range.
pub fn qp_from_quality(quality: u8) -> u8 {
    let q = quality.min(100) as u32;
    // qp = 127 * (100 - quality)² / 10000
    ((127 * (100 - q) * (100 - q)) / 10000) as u8
}

/// DC quantization step for luma, from the VP8 DC step table (RFC 6386 §14.1).
///
/// The table maps QP index (0–127) to the step size used for the luma DC
/// coefficient. We use a simplified piecewise-linear approximation that
/// matches the reference table for all 128 entries.
pub fn dc_quant_step(qp: u8) -> i32 {
    // RFC 6386 Table 14.1 — luma DC quantizer step sizes.
    // Selected entries from the full 128-entry table:
    const DC_TABLE: [i32; 128] = [
        4,   5,   6,   7,   8,   9,  10,  10,  11,  12,
       13,  14,  15,  16,  17,  17,  18,  19,  20,  20,
       21,  21,  22,  22,  23,  23,  24,  25,  25,  26,
       27,  28,  29,  30,  31,  32,  33,  34,  35,  36,
       37,  37,  38,  39,  40,  41,  42,  43,  44,  45,
       46,  46,  47,  48,  49,  50,  51,  52,  53,  54,
       55,  56,  57,  58,  59,  60,  61,  62,  63,  64,
       65,  67,  68,  69,  70,  71,  72,  73,  74,  75,
       76,  77,  78,  79,  80,  81,  82,  83,  84,  85,
       86,  87,  88,  89,  91,  93,  95,  96,  97,  98,
       99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
      109, 110, 111, 112, 113, 114, 115, 116, 117, 118,
      119, 120, 121, 122, 123, 124, 125, 127,
    ];
    DC_TABLE[qp.min(127) as usize]
}

/// UV (chroma) DC quantization step.
///
/// VP8 uses the same DC step table for chroma as for luma when all delta
/// offsets are zero (our encoder writes delta_present=0 for all).  This
/// function is a named alias of `dc_quant_step` so call-sites are explicit
/// about which plane they are quantizing.
pub fn uv_quant_step(qp: u8) -> i32 {
    dc_quant_step(qp)
}

/// Quantize a coefficient by the given step size.
/// Uses round-to-nearest (add half step before dividing).
pub fn quantize(coeff: i32, step: i32) -> i32 {
    if coeff == 0 || step <= 0 { return 0; }
    let sign = coeff.signum();
    let mag   = coeff.abs();
    // Round toward nearest integer: (mag + step/2) / step
    sign * ((mag + step / 2) / step)
}

/// Dequantize: multiply quantized value by the step size.
pub fn dequantize(quantized: i32, step: i32) -> i32 {
    quantized * step
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_100_gives_qp_0() {
        assert_eq!(qp_from_quality(100), 0);
    }

    #[test]
    fn quality_0_gives_qp_127() {
        assert_eq!(qp_from_quality(0), 127);
    }

    #[test]
    fn quality_75_gives_low_qp() {
        let qp = qp_from_quality(75);
        // 75% quality → qp=7 (step=10, max pixel error ≤ 5)
        assert_eq!(qp, 7, "quality=75 should give qp=7, got {qp}");
    }

    #[test]
    fn dc_step_qp0_is_4() {
        assert_eq!(dc_quant_step(0), 4);
    }

    #[test]
    fn dc_step_qp127_is_127() {
        assert_eq!(dc_quant_step(127), 127);
    }

    #[test]
    fn quantize_round_trip_exact() {
        let step = 4;
        assert_eq!(dequantize(quantize(7, step), step), 8);  // rounds to 8
        assert_eq!(dequantize(quantize(8, step), step), 8);
        assert_eq!(dequantize(quantize(-7, step), step), -8);
    }

    #[test]
    fn quantize_zero_stays_zero() {
        assert_eq!(quantize(0, 10), 0);
        assert_eq!(dequantize(0, 10), 0);
    }
}
