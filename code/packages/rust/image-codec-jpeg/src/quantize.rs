// # quantize.rs — JPEG quantization tables and quality scaling
//
// ## What is quantization?
//
// After the 8×8 DCT, we have 64 floating-point frequency coefficients. We
// can't store all of them at full precision without wasting space. Quantization
// is where JPEG's lossy compression actually happens: we divide each coefficient
// by a "step size" (the quantization table entry) and round to the nearest
// integer. Small coefficients near zero become zero entirely and disappear from
// the bitstream. Only the significant ones survive.
//
// Think of it like counting money in dollars instead of cents: $12.37 becomes
// $12. You lose the 37 cents (precision), but the number is much easier to
// store and transmit.
//
// ## Why separate luma and chroma tables?
//
// Human vision is more sensitive to luma (brightness) detail than chroma
// (colour) detail — we can tolerate more colour blurring than brightness
// blurring without noticing. The standard luma table uses smaller step sizes
// (finer quantization) for low-frequency coefficients and larger step sizes
// (coarser quantization) for high frequencies. The chroma table is coarser
// across the board.
//
// ## Annex K tables
//
// The JPEG standard (ITU-T T.81, Annex K) provides "suggested" quantization
// tables calibrated for quality level 50. These are the de-facto standard tables
// used by virtually every JPEG implementation. We use them as the base, then
// scale them according to the requested quality level (1–100).
//
// ## Quality scaling formula
//
// The standard formula (used by libjpeg and virtually every JPEG toolkit):
//
//   if quality < 50:  scale = 5000 / quality
//   if quality >= 50: scale = 200 - 2 * quality
//
// Each table entry is then multiplied by scale/100 and clamped to [1, 255].
// Quality 50 → scale=100 → tables unchanged.
// Quality 100 → scale=0 → all entries become 1 (finest quantization, near-lossless).
// Quality 1 → scale=5000 → all entries become 255 (coarsest quantization, tiny files).

// ---------------------------------------------------------------------------
// Standard Annex K base quantization tables (quality 50)
// ---------------------------------------------------------------------------

/// JPEG standard luma quantization table (Annex K, T.81).
///
/// These 64 values are in row-major 8×8 order (NOT zigzag order).
/// The table is designed to match human visual sensitivity:
/// - Small values (fine quantization) at top-left: low-frequency, DC area
/// - Large values (coarse quantization) at bottom-right: high-frequency area
///
/// Example: entry [0] = 16 means the DC coefficient is divided by 16.
/// Entry [63] = 99 means the highest-frequency AC is divided by 99.
pub const LUMA_QTABLE: [u8; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61,
    12, 12, 14, 19, 26, 58, 60, 55,
    14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62,
    18, 22, 37, 56, 68,109,103, 77,
    24, 35, 55, 64, 81,104,113, 92,
    49, 64, 78, 87,103,121,120,101,
    72, 92, 95, 98,112,100,103, 99,
];

/// JPEG standard chroma quantization table (Annex K, T.81).
///
/// Much coarser than the luma table — we lose more colour detail than brightness
/// detail, matching human visual perception.
pub const CHROMA_QTABLE: [u8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99,
    18, 21, 26, 66, 99, 99, 99, 99,
    24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
];

// ---------------------------------------------------------------------------
// Zigzag scan order
// ---------------------------------------------------------------------------

/// Zigzag scan permutation: maps DCT coefficient position (row-major) to
/// zigzag position.
///
/// JPEG serialises the 64 DCT coefficients in zigzag order rather than
/// row-major order. Why? The zigzag traverses from the low-frequency corner
/// (DC, top-left) to the high-frequency corner (bottom-right) in a diagonal
/// sweep. High-frequency coefficients are usually zero after quantization, so
/// they cluster at the end of the zigzag sequence — perfect for run-length
/// encoding (the AC entropy coder encodes runs of zeros efficiently).
///
/// Visually, the zigzag for an 8×8 block looks like:
///
/// ```text
///  0  1  5  6 14 15 27 28
///  2  4  7 13 16 26 29 42
///  3  8 12 17 25 30 41 43
///  9 11 18 24 31 40 44 53
/// 10 19 23 32 39 45 52 54
/// 20 22 33 38 46 51 55 60
/// 21 34 37 47 50 56 59 61
/// 35 36 48 49 57 58 62 63
/// ```
///
/// `ZIGZAG[i]` gives the zigzag output index for row-major position `i`.
pub const ZIGZAG: [usize; 64] = [
     0,  1,  5,  6, 14, 15, 27, 28,
     2,  4,  7, 13, 16, 26, 29, 42,
     3,  8, 12, 17, 25, 30, 41, 43,
     9, 11, 18, 24, 31, 40, 44, 53,
    10, 19, 23, 32, 39, 45, 52, 54,
    20, 22, 33, 38, 46, 51, 55, 60,
    21, 34, 37, 47, 50, 56, 59, 61,
    35, 36, 48, 49, 57, 58, 62, 63,
];

/// Inverse zigzag: `IZIGZAG[zz_pos]` gives the row-major position.
///
/// Built from ZIGZAG: if ZIGZAG[row_major] = zz_pos, then IZIGZAG[zz_pos] = row_major.
pub const IZIGZAG: [usize; 64] = {
    let mut iz = [0usize; 64];
    let mut i = 0;
    while i < 64 {
        iz[ZIGZAG[i]] = i;
        i += 1;
    }
    iz
};

// ---------------------------------------------------------------------------
// Quality scaling
// ---------------------------------------------------------------------------

/// Scale an Annex K base quantization table by a quality factor (1–100).
///
/// Returns a [u16; 64] because after downscaling at quality 1, some entries
/// can exceed u8::MAX (they're clamped at 255, but we keep u16 for future
/// flexibility). All entries are guaranteed to be in [1, 255].
///
/// # Quality semantics
///
/// | Quality | Effect                                           |
/// |---------|--------------------------------------------------|
/// | 100     | Step sizes all = 1 → near-lossless (large files)|
/// | 75      | Default quality for most JPEG tools              |
/// | 50      | Annex K base tables unchanged                    |
/// | 1       | Step sizes all = 255 → very lossy (tiny files)  |
pub fn scale_qtable(base: &[u8; 64], quality: u8) -> [u16; 64] {
    // Clamp quality to a valid range first.
    let q = quality.clamp(1, 100) as u32;

    // The libjpeg-turbo formula, which is the de-facto standard:
    //   quality < 50 → scale = 5000 / quality   (e.g. q=25 → scale=200)
    //   quality ≥ 50 → scale = 200 - 2 * quality (e.g. q=75 → scale=50)
    //
    // scale=100 leaves the table unchanged (÷100×100 = identity).
    // scale=0 (quality=100) is special-cased: all entries become 1.
    let scale = if q < 50 { 5000 / q } else { 200 - 2 * q };

    let mut out = [0u16; 64];
    for i in 0..64 {
        if scale == 0 {
            // Quality 100: finest possible quantization (step = 1).
            out[i] = 1;
        } else {
            // Apply the scale and round to the nearest integer.
            // The +50 before ÷100 provides rounding (equivalent to +0.5 before truncation).
            let v = ((base[i] as u32 * scale + 50) / 100).clamp(1, 255);
            out[i] = v as u16;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Coefficient quantize / dequantize
// ---------------------------------------------------------------------------

/// Quantize a DCT coefficient by dividing by the table entry and rounding.
///
/// This is the lossy step. The coefficient `coeff` is a floating-point DCT
/// value; `qtable_entry` is the step size (from the scaled quantization table).
///
/// Dividing and rounding to the nearest integer is equivalent to "snapping" the
/// continuous coefficient to a grid with spacing `qtable_entry`. Everything
/// between –qtable_entry/2 and +qtable_entry/2 snaps to zero and disappears.
pub fn quantize(coeff: f32, qtable_entry: u16) -> i16 {
    (coeff / qtable_entry as f32).round() as i16
}

/// Dequantize a quantized coefficient by multiplying by the table entry.
///
/// This is the inverse of quantize, applied during decoding. It converts the
/// integer quantized coefficient back to an approximate floating-point DCT
/// value. The result won't match the original exactly — that precision was
/// lost during quantization — but it will be within ±(qtable_entry / 2).
pub fn dequantize(qcoeff: i16, qtable_entry: u16) -> f32 {
    qcoeff as f32 * qtable_entry as f32
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// At quality 50 the scale factor is 100, so the table is unchanged.
    #[test]
    fn quality_50_is_identity() {
        let scaled = scale_qtable(&LUMA_QTABLE, 50);
        for i in 0..64 {
            assert_eq!(scaled[i], LUMA_QTABLE[i] as u16,
                "entry {i}: expected {}, got {}", LUMA_QTABLE[i], scaled[i]);
        }
    }

    /// At quality 100 every entry should be 1 (finest possible quantization).
    #[test]
    fn quality_100_all_ones() {
        let scaled = scale_qtable(&LUMA_QTABLE, 100);
        for (i, &entry) in scaled.iter().enumerate() {
            assert_eq!(entry, 1, "entry {i} should be 1 at quality 100");
        }
    }

    /// At quality 1 every entry should be 255 (coarsest possible).
    #[test]
    fn quality_1_all_255() {
        let luma = scale_qtable(&LUMA_QTABLE, 1);
        // At quality=1, scale=5000. The smallest base entry is 10 (luma table).
        // 10 * 5000 / 100 = 500 → clamped to 255.
        for (i, &entry) in luma.iter().enumerate() {
            assert_eq!(entry, 255, "entry {i} should be 255 at quality 1");
        }
    }

    /// Quality clamping: quality=0 should behave like quality=1.
    #[test]
    fn quality_clamps_at_1() {
        let q0 = scale_qtable(&LUMA_QTABLE, 0);
        let q1 = scale_qtable(&LUMA_QTABLE, 1);
        assert_eq!(q0, q1);
    }

    /// Quality clamping: quality=200 should behave like quality=100.
    #[test]
    fn quality_clamps_at_100() {
        let q200 = scale_qtable(&LUMA_QTABLE, 200);
        let q100 = scale_qtable(&LUMA_QTABLE, 100);
        assert_eq!(q200, q100);
    }

    /// No entry should be zero (would cause divide-by-zero during dequantization).
    #[test]
    fn all_entries_nonzero_across_quality_range() {
        for q in [1u8, 25, 50, 75, 100] {
            let t = scale_qtable(&LUMA_QTABLE, q);
            for (i, &v) in t.iter().enumerate() {
                assert!(v >= 1, "quality {q} entry {i} = 0");
            }
        }
    }

    /// Quantize(coeff, 1) should be exactly the rounded coefficient.
    #[test]
    fn quantize_with_step_1() {
        assert_eq!(quantize(5.7, 1), 6);
        assert_eq!(quantize(-3.2, 1), -3);
        assert_eq!(quantize(0.0, 1), 0);
    }

    /// Quantize then dequantize should produce a value within ±step_size/2.
    #[test]
    fn quantize_dequantize_within_half_step() {
        let step = 16u16; // typical luma DC step at quality 50
        for coeff in [-100.0f32, -50.0, 0.0, 50.0, 127.5] {
            let q = quantize(coeff, step);
            let dq = dequantize(q, step);
            assert!((dq - coeff).abs() <= step as f32 / 2.0,
                "coeff={coeff} step={step}: dequantize gave {dq}");
        }
    }

    /// ZIGZAG is a permutation of 0..64 (every index appears exactly once).
    #[test]
    fn zigzag_is_permutation() {
        let mut seen = [false; 64];
        for &v in &ZIGZAG {
            assert!(!seen[v], "zigzag value {v} appeared twice");
            seen[v] = true;
        }
        assert!(seen.iter().all(|&s| s), "zigzag missing some values");
    }

    /// IZIGZAG is the exact inverse of ZIGZAG.
    #[test]
    fn izigzag_is_inverse_of_zigzag() {
        for i in 0..64 {
            assert_eq!(IZIGZAG[ZIGZAG[i]], i, "IZIGZAG[ZIGZAG[{i}]] != {i}");
            assert_eq!(ZIGZAG[IZIGZAG[i]], i, "ZIGZAG[IZIGZAG[{i}]] != {i}");
        }
    }
}
