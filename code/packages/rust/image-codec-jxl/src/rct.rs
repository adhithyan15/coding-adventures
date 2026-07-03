//! # Reversible Colour Transform (RCT / YCoCg)
//!
//! JXL Modular can apply a lossless colour transform before the per-channel
//! gradient predictor runs.  After transforming, the three colour channels
//! become much less correlated with each other, which improves compression.
//!
//! ## YCoCg (RCT type 6 in the JXL specification)
//!
//! The transform is an integer variant of YCoCg derived from the Malvar-Sullivan
//! "lossless" YCoCg transform:
//!
//! **Forward (RGB → YCoCg):**
//!
//! ```text
//! Co  = R − B
//! tmp = B + (Co >> 1)     (integer arithmetic, arithmetic right-shift)
//! Cg  = G − tmp
//! Y   = tmp + (Cg >> 1)
//! ```
//!
//! Output channels: (Y, Co, Cg) replace (R, G, B).
//!
//! **Inverse (YCoCg → RGB):**
//!
//! ```text
//! tmp = Y − (Cg >> 1)
//! G   = Cg + tmp
//! B   = tmp − (Co >> 1)
//! R   = B + Co
//! ```
//!
//! ## Usage note
//!
//! Our encoder does **not** apply RCT — it encodes channels directly in RGB
//! order, which is simpler and already gives good results because the gradient
//! predictor removes most spatial redundancy.
//!
//! These functions are included so that the decoder can, in principle, handle
//! RCT-coded images produced by libjxl.  They are also tested directly as
//! pure math functions regardless of whether the encoder uses them.

/// Forward RCT (type 6): RGB → (Y, Co, Cg) using integer YCoCg.
///
/// All arithmetic is integer; right-shifts are *arithmetic* (sign-preserving).
/// Input values need not be in [0, 255] — the transform is defined over ℤ.
///
/// Returns `(y, co, cg)`.
pub fn rct_forward(r: i32, g: i32, b: i32) -> (i32, i32, i32) {
    // Step 1: compute chroma-orange (difference of R and B).
    let co = r - b;
    // Step 2: shift the blue channel up by half of Co.
    //         `>> 1` is arithmetic right-shift — rounds toward negative infinity.
    let tmp = b + (co >> 1);
    // Step 3: chroma-green (difference of G from adjusted luma).
    let cg = g - tmp;
    // Step 4: luma (adjust for Cg).
    let y = tmp + (cg >> 1);
    (y, co, cg)
}

/// Inverse RCT (type 6): (Y, Co, Cg) → RGB.
///
/// Exact inverse of `rct_forward`; applying both in succession leaves the
/// original values unchanged.
///
/// Returns `(r, g, b)`.
pub fn rct_inverse(y: i32, co: i32, cg: i32) -> (i32, i32, i32) {
    // Undo step 4.
    let tmp = y - (cg >> 1);
    // Undo step 3.
    let g = cg + tmp;
    // Undo step 2.
    let b = tmp - (co >> 1);
    // Undo step 1.
    let r = b + co;
    (r, g, b)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(r: i32, g: i32, b: i32) {
        let (y, co, cg) = rct_forward(r, g, b);
        let (rr, gg, bb) = rct_inverse(y, co, cg);
        assert_eq!(
            (r, g, b), (rr, gg, bb),
            "RCT round-trip failed for ({}, {}, {})",
            r, g, b
        );
    }

    #[test]
    fn round_trip_black() { round_trip(0, 0, 0); }

    #[test]
    fn round_trip_white() { round_trip(255, 255, 255); }

    #[test]
    fn round_trip_red() { round_trip(255, 0, 0); }

    #[test]
    fn round_trip_green() { round_trip(0, 255, 0); }

    #[test]
    fn round_trip_blue() { round_trip(0, 0, 255); }

    #[test]
    fn round_trip_arbitrary() {
        for r in (0u8..=255).step_by(17) {
            for g in (0u8..=255).step_by(23) {
                for b in (0u8..=255).step_by(31) {
                    round_trip(r as i32, g as i32, b as i32);
                }
            }
        }
    }

    #[test]
    fn luminance_of_grey_is_grey() {
        // For (k, k, k): Co=0, Cg=0, Y=k.
        let (y, co, cg) = rct_forward(100, 100, 100);
        assert_eq!(co, 0);
        assert_eq!(cg, 0);
        assert_eq!(y, 100);
    }
}
