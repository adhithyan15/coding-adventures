// # unpack.rs — 12-bit little-endian packed pixel reader
//
// ## Bit Packing Scheme
//
// Panasonic packs two 12-bit pixel values into three bytes (24 bits total),
// discarding zero waste bits. The scheme is little-endian:
//
//   Byte index:   |    byte0    |    byte1    |    byte2    |
//   Bit indices:  | b7 .. b0    | b7 .. b0    | b7 .. b0    |
//
//   Pixel 0 (p0):  b0[7:0] | b1[3:0] << 8   (i.e. bits [7:0] then [11:8])
//   Pixel 1 (p1):  b1[7:4] >> 4 | b2[7:0] << 4
//
// In other words:
//   byte0         = p0[7:0]          (low 8 bits of p0)
//   byte1 low  4  = p0[11:8]         (high nibble of p0)
//   byte1 high 4  = p1[3:0]          (low nibble of p1)
//   byte2         = p1[11:4]         (high 8 bits of p1)
//
// ## Row Stride
//
// A complete row of `width` pixels occupies:
//
//   stride = ⌈width × 12 / 8⌉ = (width × 12 + 7) / 8 bytes
//
// The last 3-byte group in a row may encode a partial pixel if `width` is odd.
// We always emit exactly `num_pixels` values and ignore trailing bits.

/// Unpack `num_pixels` 12-bit LE pixels from a packed byte slice.
///
/// Each pair of adjacent pixels occupies 3 consecutive bytes. Pixels beyond
/// the available input are silently omitted (the returned `Vec` may be shorter
/// than `num_pixels` if `data` is too short — callers should check).
///
/// # Examples
///
/// ```
/// // The 3-byte sequence 0x34, 0x12, 0x56 encodes:
/// //   p0 = 0x034 | (0x12 & 0x0F) << 8 = 0x234
/// //   p1 = (0x12 >> 4) | (0x56 << 4) = 0x561
/// // That is:
/// //   byte0 = 0x34 → p0[7:0] = 0x34
/// //   byte1 = 0x12 → p0[11:8] = 0x02, p1[3:0] = 0x01
/// //   byte2 = 0x56 → p1[11:4] = 0x56
/// // p0 = 0x34 | (0x02 << 8) = 0x234
/// // p1 = 0x01 | (0x56 << 4) = 0x561
/// ```
pub fn unpack_12bit_le(data: &[u8], num_pixels: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(num_pixels);
    let mut i = 0usize;

    while out.len() < num_pixels {
        // Need at least 3 bytes to decode a pair.
        if i + 2 >= data.len() {
            break;
        }

        let b0 = data[i] as u16;
        let b1 = data[i + 1] as u16;
        let b2 = data[i + 2] as u16;

        // Pixel 0: low 8 bits from b0, high 4 bits from low nibble of b1.
        //   p0 = b0 | ((b1 & 0x0F) << 8)
        let p0 = b0 | ((b1 & 0x0F) << 8);
        out.push(p0);

        if out.len() < num_pixels {
            // Pixel 1: low 4 bits from high nibble of b1, high 8 bits from b2.
            //   p1 = (b1 >> 4) | (b2 << 4)
            let p1 = (b1 >> 4) | (b2 << 4);
            out.push(p1);
        }

        i += 3;
    }

    out
}

/// Compute the row stride in bytes for a given sensor width packed at 12 bpp.
///
/// stride = ⌈width × 12 / 8⌉
///
/// For example:
/// - width = 4000 → stride = 6000 bytes  (4000 × 12 / 8 = 6000 exactly)
/// - width = 3    → stride = 5 bytes     ((3 × 12 + 7) / 8 = 5)
pub fn row_stride_bytes(width: u32) -> usize {
    // Use usize to avoid overflow on large sensors.
    let bits = width as usize * 12;
    bits.div_ceil(8)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_known_pair() {
        // Hand-computed test vector.
        //
        // Let p0 = 0x123 and p1 = 0x456.
        //
        // Encoding:
        //   byte0 = p0[7:0]          = 0x23
        //   byte1 = p0[11:8] | (p1[3:0] << 4) = 0x01 | (0x06 << 4) = 0x61
        //   byte2 = p1[11:4]         = 0x45
        let data = [0x23u8, 0x61, 0x45];
        let pixels = unpack_12bit_le(&data, 2);
        assert_eq!(pixels.len(), 2);
        assert_eq!(pixels[0], 0x123, "p0 mismatch: got {:03X}", pixels[0]);
        assert_eq!(pixels[1], 0x456, "p1 mismatch: got {:03X}", pixels[1]);
    }

    #[test]
    fn unpack_all_zeros() {
        let data = [0u8; 6];
        let pixels = unpack_12bit_le(&data, 4);
        assert_eq!(pixels, vec![0, 0, 0, 0]);
    }

    #[test]
    fn unpack_all_max_12bit() {
        // Max 12-bit value = 0xFFF = 4095.
        // Encoding two 0xFFF pixels:
        //   byte0 = 0xFF  (p0[7:0] = 0xFF)
        //   byte1 = 0xFF  (p0[11:8] = 0x0F << 0 = 0xF, p1[3:0] = 0xF << 4 = 0xF0)
        //           → 0x0F | 0xF0 = 0xFF
        //   byte2 = 0xFF  (p1[11:4] = 0xFF)
        let data = [0xFFu8, 0xFF, 0xFF];
        let pixels = unpack_12bit_le(&data, 2);
        assert_eq!(pixels[0], 0xFFF);
        assert_eq!(pixels[1], 0xFFF);
    }

    #[test]
    fn unpack_respects_num_pixels() {
        // 6 bytes could decode 4 pixels, but we only request 3.
        let data = [0x23u8, 0x61, 0x45, 0x23, 0x61, 0x45];
        let pixels = unpack_12bit_le(&data, 3);
        assert_eq!(pixels.len(), 3);
    }

    #[test]
    fn row_stride_even_width() {
        // 4000 pixels × 12 bits / 8 = 6000 bytes exactly.
        assert_eq!(row_stride_bytes(4000), 6000);
    }

    #[test]
    fn row_stride_odd_width() {
        // 3 pixels × 12 bits = 36 bits → ⌈36/8⌉ = 5 bytes.
        assert_eq!(row_stride_bytes(3), 5);
    }

    #[test]
    fn row_stride_single_pixel() {
        // 1 pixel × 12 bits = 12 bits → ⌈12/8⌉ = 2 bytes.
        assert_eq!(row_stride_bytes(1), 2);
    }
}
