// # compression/mod.rs — TIFF Compression Dispatcher
//
// TIFF supports several compression schemes. The decoder selects the right
// decompressor based on the `Compression` tag in the IFD:
//
// | Code  | Name         | Description                                        |
// |-------|--------------|----------------------------------------------------|
// | 1     | Uncompressed | Raw pixel bytes, no encoding                       |
// | 5     | LZW          | Lempel–Ziv–Welch, MSB-first, 12-bit code table    |
// | 32773 | PackBits     | Simple byte-level RLE (Apple/Aldus invention)      |
//
// JPEG (code 7) is not implemented here — it would require a JPEG dependency.
// Callers receive a clear error message for unsupported compression codes.

pub mod lzw;
pub mod packbits;
pub mod uncompressed;

/// Decompress a single strip or tile of TIFF data.
///
/// # Arguments
///
/// - `compressed`: the raw bytes from the file (what's at the strip offset)
/// - `compression_code`: the value of the TIFF `Compression` tag
/// - `expected_bytes`: the expected decompressed size in bytes
///   Used for bounds checking and LZW output cap.
/// - `predictor`: the TIFF `Predictor` tag value (1=none, 2=horiz differencing)
/// - `width`: image width, needed to undo horizontal differencing
/// - `samples_per_pixel`: channels per pixel, needed for horizontal differencing
/// - `bits_per_sample`: bit depth per channel, needed for horizontal differencing
///
/// # Returns
///
/// The decompressed bytes, ready to be parsed into pixel values.
pub fn decompress(
    compressed: &[u8],
    compression_code: u16,
    expected_bytes: usize,
    predictor: u16,
    width: u32,
    samples_per_pixel: u16,
    bits_per_sample: u16,
) -> Result<Vec<u8>, String> {
    // ── Step 1: decompress according to the compression scheme ─────────────
    let mut raw = match compression_code {
        1 => uncompressed::decompress(compressed),
        5 => lzw::decompress(compressed, expected_bytes)?,
        32773 => packbits::decompress(compressed, expected_bytes)?,
        7 => {
            return Err(
                "TIFF: JPEG compression (code 7) is not supported in this crate; \
                 use image-codec-jpeg for JPEG strips"
                    .into(),
            );
        }
        c => {
            return Err(format!(
                "TIFF: unsupported compression code {} \
                 (supported: 1=uncompressed, 5=LZW, 32773=PackBits)",
                c
            ));
        }
    };

    // ── Step 2: undo horizontal differencing predictor ──────────────────────
    //
    // When `Predictor = 2` (horizontal differencing), each pixel was stored as
    // `pixel[x] - pixel[x-1]` rather than the absolute value. To recover
    // the original values, we apply a cumulative sum row by row.
    //
    // This is a lossless transform that improves LZW compression ratios for
    // natural images. It must be undone after decompression.
    if predictor == 2 && bits_per_sample == 8 {
        undo_horizontal_differencing(&mut raw, width as usize, samples_per_pixel as usize);
    }

    Ok(raw)
}

/// Undo the horizontal differencing predictor.
///
/// The TIFF spec describes this as: `delta[x] = pixel[x] - pixel[x-1]`.
/// So on decode we compute: `pixel[x] = delta[x] + pixel[x-1]`.
///
/// This is applied per-row, per-channel separately:
///
/// ```text
/// Row bytes: [R0, G0, B0,  ΔR1, ΔG1, ΔB1,  ΔR2, ΔG2, ΔB2, ...]
///                           ^--- these are differences, not absolutes
/// After undo:
///            [R0, G0, B0,  R0+ΔR1, G0+ΔG1, B0+ΔB1, ...]
/// ```
///
/// The arithmetic wraps at u8 boundaries (intentional — any high-bit noise
/// disappears when adding back the cumulative sum).
fn undo_horizontal_differencing(data: &mut [u8], width: usize, samples: usize) {
    // Each row is `width * samples` bytes wide.
    let row_bytes = width * samples;
    if row_bytes == 0 {
        return;
    }

    for row_start in (0..data.len()).step_by(row_bytes) {
        let row_end = (row_start + row_bytes).min(data.len());
        let row = &mut data[row_start..row_end];

        // Process each channel independently.
        // Channel c occupies bytes c, c+samples, c+2*samples, ...
        for c in 0..samples {
            // First pixel of each row is already absolute.
            // Start from second pixel.
            let mut prev = row[c];
            let mut x = c + samples;
            while x < row.len() {
                let curr = row[x].wrapping_add(prev);
                row[x] = curr;
                prev = curr;
                x += samples;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_uncompressed_passthrough() {
        let data = vec![1u8, 2, 3, 4];
        let result = decompress(&data, 1, 4, 1, 2, 2, 8).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn decompress_unknown_code_error() {
        let result = decompress(&[1, 2, 3], 999, 3, 1, 1, 1, 8);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported compression code 999"));
    }

    #[test]
    fn decompress_jpeg_error() {
        let result = decompress(&[], 7, 0, 1, 1, 1, 8);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JPEG"));
    }

    #[test]
    fn horizontal_differencing_undo() {
        // 2 pixels, 3 channels (RGB), width=2, samples=3
        // Row: [10, 20, 30, ΔR=5, ΔG=−3, ΔB=10]
        // In u8: [10, 20, 30, 5, 253, 10]  (−3 wraps to 253)
        // After undo: [10, 20, 30, 15, 17, 40]
        let mut data = vec![10u8, 20, 30, 5, 253, 10];
        undo_horizontal_differencing(&mut data, 2, 3);
        assert_eq!(data, vec![10, 20, 30, 15, 17, 40]);
    }
}
