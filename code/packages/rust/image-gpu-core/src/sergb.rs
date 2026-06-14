//! sRGB transfer functions (decode/encode).
//!
//! Identical to the v0.1 helpers — used by Rust pre/post-processing
//! around graphs that need linear-light arithmetic.

#[inline]
pub fn decode(byte: u8) -> f32 {
    let v = byte as f32 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055_f32).powf(2.4)
    }
}

#[inline]
pub fn encode(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let s = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_encode_round_trip() {
        // sRGB values should round-trip within ±1 LSB.
        for byte in [0u8, 1, 50, 100, 128, 200, 254, 255] {
            let lin = decode(byte);
            let encoded = encode(lin);
            assert!(
                (encoded as i16 - byte as i16).abs() <= 1,
                "round-trip for {} → {} → {}",
                byte,
                lin,
                encoded
            );
        }
    }

    #[test]
    fn encode_clamps() {
        assert_eq!(encode(-0.5), 0);
        assert_eq!(encode(2.0), 255);
    }
}
