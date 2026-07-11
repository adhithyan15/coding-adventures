// # unpack.rs — 12-bit big-endian packed pixel reader
//
// Fujifilm RAF stores raw sensor data as 12-bit values packed two-per-three-
// bytes, in big-endian bit order.  This is the same packing used by many
// early Nikon and Canon RAW formats.
//
// ## The packing scheme
//
// Two 12-bit pixels (p0 and p1) are stored in three consecutive bytes
// (b0, b1, b2):
//
// ```text
// b0 = p0[11:4]          — the high 8 bits of pixel 0
// b1 = (p0[3:0] << 4)    — the low  4 bits of pixel 0 in the high nibble …
//    | (p1[11:8])         — … and the high 4 bits of pixel 1 in the low nibble
// b2 = p1[7:0]           — the low  8 bits of pixel 1
// ```
//
// Reading back:
//
// ```text
// p0 = (b0 << 4) | (b1 >> 4)
// p1 = ((b1 & 0x0F) << 8) | b2
// ```
//
// ## Row padding
//
// The row width in the packed data may be padded to the next multiple of 16
// pixels so that each row starts on a 24-byte (= 16 pixels × 3/2 bytes)
// boundary.  The caller should pass `raw_width` (from CFA header tag 0x0110)
// as the number of *valid* pixels per row; any extra padding pixels at the
// right edge are discarded.

/// Unpack 12-bit big-endian packed pixels from `data`.
///
/// Returns a `Vec<u16>` of exactly `num_pixels` values, each in the range
/// `[0, 4095]`.  Stops as soon as `num_pixels` have been read or the input
/// is exhausted (whichever comes first).
///
/// # Arguments
///
/// * `data`       — raw byte slice from the CFA pixel section of the RAF file
/// * `num_pixels` — the exact number of 12-bit values expected
///
/// # Example
///
/// ```text
/// bytes = [0x12, 0x34, 0x56]
/// p0 = (0x12 << 4) | (0x34 >> 4) = 0x120 | 0x03 = 0x123 = 291
/// p1 = ((0x34 & 0x0F) << 8) | 0x56 = (0x04 << 8) | 0x56 = 0x456 = 1110
/// ```
pub fn unpack_12bit_be(data: &[u8], num_pixels: usize) -> Vec<u16> {
    // Pre-allocate the exact capacity we need.
    let mut out = Vec::with_capacity(num_pixels);
    let mut i = 0usize; // byte index into `data`

    // Each iteration of the loop consumes 3 bytes and produces up to 2 pixels.
    // The last "pair" may be an odd pixel encoded in only 2 bytes — handle it
    // by reading b2 only when it is needed (and available).
    while out.len() < num_pixels {
        // Guard: need at least 2 bytes for the first pixel of any pair.
        if i + 1 >= data.len() {
            break;
        }

        let b0 = data[i]     as u16;
        let b1 = data[i + 1] as u16;
        // b2 is needed only for the second pixel; read conditionally below.
        i += 2;

        // Pixel 0: top 8 bits from b0, bottom 4 from b1's high nibble.
        //   p0 = (b0 << 4) | (b1 >> 4)
        let p0 = (b0 << 4) | (b1 >> 4);
        out.push(p0);

        // Pixel 1: top 4 bits from b1's low nibble, bottom 8 from b2.
        //   p1 = ((b1 & 0x0F) << 8) | b2
        // Only consume a b2 byte if we still need another pixel and it exists.
        if out.len() < num_pixels && i < data.len() {
            let b2 = data[i] as u16;
            i += 1;
            let p1 = ((b1 & 0x0F) << 8) | b2;
            out.push(p1);
        }
    }

    out
}

/// Compute how many bytes are needed to store `pixels` packed 12-bit values.
///
/// Each pair of pixels takes exactly 3 bytes.  An odd trailing pixel takes 2
/// bytes (the second byte is half-empty but must be present).
///
/// ```text
/// bytes_needed(2) = 3  (1 full pair)
/// bytes_needed(3) = 5  (1 pair + 1 odd pixel → 2 extra bytes)
/// bytes_needed(4) = 6  (2 full pairs)
/// ```
pub fn packed_byte_count(pixels: usize) -> usize {
    // Integer division rounds down; multiply back; add 2 bytes for any odd
    // pixel (the 3rd byte of the "pair" even if pixel 1 is absent).
    let pairs = pixels / 2;
    let odd   = pixels % 2;
    pairs * 3 + odd * 2
}

/// Pack a slice of 12-bit pixel values into 12-bit big-endian packed bytes.
///
/// This is the inverse of `unpack_12bit_be`.  Values are clamped to `[0, 4095]`
/// before packing; no error is raised for out-of-range inputs.
///
/// Used by the test encoder (`encoder.rs`) to build synthetic RAF files.
pub fn pack_12bit_be(pixels: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(packed_byte_count(pixels.len()));
    let mut iter = pixels.iter();

    // Take up to two pixels per iteration.
    while let Some(&p0) = iter.next() {
        let p0 = p0 & 0x0FFF; // clamp to 12 bits

        match iter.next() {
            Some(&p1) => {
                let p1 = p1 & 0x0FFF;
                // b0 = p0[11:4]
                let b0 = (p0 >> 4) as u8;
                // b1 = (p0[3:0] << 4) | p1[11:8]
                let b1 = (((p0 & 0x0F) << 4) | (p1 >> 8)) as u8;
                // b2 = p1[7:0]
                let b2 = (p1 & 0xFF) as u8;
                out.push(b0);
                out.push(b1);
                out.push(b2);
            }
            None => {
                // Odd final pixel: fill second byte's low nibble with zero.
                let b0 = (p0 >> 4) as u8;
                let b1 = ((p0 & 0x0F) << 4) as u8;
                out.push(b0);
                out.push(b1);
            }
        }
    }

    out
}
