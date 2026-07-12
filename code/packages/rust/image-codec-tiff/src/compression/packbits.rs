// # packbits.rs — PackBits RLE Decompressor
//
// PackBits is a simple run-length encoding scheme invented by Apple for
// MacPaint (1984) and later standardised in the TIFF spec as Compression=32773.
// It's used by macOS Preview when saving TIFFs and by many older scanners.
//
// ## Algorithm
//
// The compressed stream consists of "runs". Each run starts with a one-byte
// *header* byte `h`, interpreted as a signed integer:
//
// ```text
// h == -128 (0x80):   NOP — skip this byte, continue to the next run.
//
// -127 <= h <= -1:    Run-length run.
//                     Read the NEXT byte and repeat it (1 - h) times.
//                     e.g. h=-3 means repeat next byte (1-(-3))=4 times.
//
//  0   <= h <= 127:   Literal run.
//                     Copy the NEXT (h + 1) bytes verbatim.
//                     e.g. h=2 means copy next 3 bytes.
// ```
//
// ## Stopping Condition
//
// Stop when `expected_bytes` of output have been produced. A well-formed
// TIFF file encodes exactly the right number of bytes; if we've reached
// `expected_bytes` we stop rather than reading further.
//
// ## Example
//
// Compressed: [0xFE, 0xAA, 0x02, 0x80, 0x00, 0x2A, 0x01, 0x80, 0xFF]
//
// | Header | Meaning                   | Output         |
// |--------|---------------------------|----------------|
// | 0xFE   | h=-2 → repeat 3 times     | AA AA AA       |
// | 0x02   | h=2 → copy 3 bytes        | 80 00 2A       |
// | 0x01   | h=1 → copy 2 bytes        | 80 FF          |
// → Output: AA AA AA 80 00 2A 80 FF

/// Maximum decompressed output size.
///
/// PackBits can expand at most 128× in the worst case (header byte 0 → copy 1
/// byte, but with the header overhead, a single run emits 1 byte per 2 input
/// bytes). We cap at 4× the compressed size as an extra safety margin, since
/// we also have `expected_bytes` as a tighter bound.
const MAX_EXPANSION: usize = 4;

/// Decompress a PackBits-encoded byte stream.
///
/// # Arguments
///
/// - `compressed`: raw compressed bytes from the TIFF strip
/// - `expected_bytes`: expected number of output bytes (from StripByteCounts
///   divided by actual-strip-count, or RowsPerStrip × row_stride)
///
/// # Returns
///
/// The decompressed bytes, up to `expected_bytes` in length.
///
/// # Errors
///
/// Returns `Err` if the stream is truncated (header byte present but
/// referenced data bytes are missing).
pub fn decompress(compressed: &[u8], expected_bytes: usize) -> Result<Vec<u8>, String> {
    // Safety cap: do not allocate more than this many bytes.
    // `expected_bytes` is usually the tighter bound, but we guard against
    // malformed expected_bytes values too.
    let cap = expected_bytes
        .min(
            compressed
                .len().saturating_mul(MAX_EXPANSION),
        )
        .min(expected_bytes.saturating_add(1)); // allow exactly expected_bytes

    let mut output = Vec::with_capacity(cap.min(64 * 1024)); // don't pre-alloc huge buffers
    let mut i = 0;

    while i < compressed.len() && output.len() < expected_bytes {
        // Read the header byte as a signed integer.
        let h = compressed[i] as i8;
        i += 1;

        if h == -128 {
            // NOP — skip this byte, do nothing.
            continue;
        }

        if h < 0 {
            // Run-length run: repeat the next byte (1 - h) times.
            //
            // h = -1  → repeat 2 times
            // h = -2  → repeat 3 times
            // ...
            // h = -127 → repeat 128 times
            if i >= compressed.len() {
                return Err("PackBits: truncated run-length run (missing data byte)".into());
            }
            let repeat_byte = compressed[i];
            i += 1;
            let count = (1i16 - h as i16) as usize;

            for _ in 0..count {
                if output.len() >= expected_bytes {
                    break;
                }
                output.push(repeat_byte);
            }
        } else {
            // Literal run: copy the next (h + 1) bytes verbatim.
            //
            // h = 0 → copy 1 byte
            // h = 1 → copy 2 bytes
            // ...
            // h = 127 → copy 128 bytes
            let count = (h as usize) + 1;

            if i + count > compressed.len() {
                return Err(format!(
                    "PackBits: truncated literal run: need {} bytes at offset {}, have {}",
                    count,
                    i,
                    compressed.len() - i
                ));
            }
            let end = i + count;
            let remaining = expected_bytes - output.len();
            let to_copy = count.min(remaining);
            output.extend_from_slice(&compressed[i..i + to_copy]);
            i = end;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packbits_literal_run() {
        // h=2 → copy next 3 bytes: [0xAA, 0xBB, 0xCC]
        let compressed = vec![0x02u8, 0xAA, 0xBB, 0xCC];
        let out = decompress(&compressed, 3).unwrap();
        assert_eq!(out, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn packbits_rle_run() {
        // h=-3 (0xFD) → repeat next byte 4 times: [0x42, 0x42, 0x42, 0x42]
        let compressed = vec![0xFDu8, 0x42];
        let out = decompress(&compressed, 4).unwrap();
        assert_eq!(out, vec![0x42, 0x42, 0x42, 0x42]);
    }

    #[test]
    fn packbits_nop() {
        // h=-128 (0x80) → NOP
        // followed by h=0 (0x00) → copy 1 byte
        let compressed = vec![0x80u8, 0x00, 0xAB];
        let out = decompress(&compressed, 1).unwrap();
        assert_eq!(out, vec![0xAB]);
    }

    #[test]
    fn packbits_mixed() {
        // Example from the TIFF spec:
        // FE AA → repeat 0xAA 3 times
        // 02 80 00 2A → copy 3 bytes
        // 01 80 FF → copy 2 bytes
        let compressed = vec![0xFEu8, 0xAA, 0x02, 0x80, 0x00, 0x2A, 0x01, 0x80, 0xFF];
        let out = decompress(&compressed, 8).unwrap();
        assert_eq!(out, vec![0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0x80, 0xFF]);
    }

    #[test]
    fn packbits_truncated_rle_error() {
        // h=-1 (0xFF) → needs one more byte, but stream ends
        let compressed = vec![0xFFu8];
        assert!(decompress(&compressed, 2).is_err());
    }

    #[test]
    fn packbits_truncated_literal_error() {
        // h=2 → needs 3 bytes, but only 2 follow
        let compressed = vec![0x02u8, 0xAA, 0xBB];
        assert!(decompress(&compressed, 3).is_err());
    }

    #[test]
    fn packbits_stops_at_expected() {
        // Even if the compressed stream has more data, stop after expected_bytes.
        let compressed = vec![0x03u8, 0x01, 0x02, 0x03, 0x04, // copy 4 bytes
                               0x01u8, 0x10, 0x20]; // copy 2 more
        let out = decompress(&compressed, 2).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out, vec![0x01, 0x02]);
    }

    #[test]
    fn packbits_empty_input() {
        let out = decompress(&[], 0).unwrap();
        assert!(out.is_empty());
    }
}
