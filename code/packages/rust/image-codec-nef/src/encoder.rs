// # encoder.rs — minimal NEF encoder for round-trip testing
//
// A "real" NEF encoder would need to write Nikon MakerNote metadata,
// encrypted white balance, and optionally Nikon's proprietary compressed
// format. That is far beyond the scope of v0.1.
//
// Instead, we encode as a standard uncompressed TIFF. The resulting file is
// structurally valid as a TIFF — and therefore as a bare-minimum NEF — and
// can be decoded back by `decode_nef` (because `decode_nef` accepts TIFF
// files without a Make tag).
//
// This is the same approach used by the RW2, RAF, and other RAW codec test
// encoders in this monorepo: the encoder's job is to produce bytes that
// allow round-trip testing of the decoder, not to produce camera-compatible
// output.

use pixel_container::PixelContainer;

/// Encode a `PixelContainer` as a minimal synthetic NEF file.
///
/// The output is a valid uncompressed TIFF. `decode_nef` will accept it
/// because no Make tag is present (missing Make = permitted for synthetic
/// files).
///
/// # Example
///
/// ```rust,ignore
/// let mut pc = PixelContainer::new(4, 4);
/// pc.fill(200, 100, 50, 255);
/// let nef_bytes = encode_nef(&pc);
/// let decoded = decode_nef(&nef_bytes).unwrap();
/// assert_eq!(decoded.width, 4);
/// ```
pub fn encode_nef(pixels: &PixelContainer) -> Vec<u8> {
    image_codec_tiff::encode_tiff(pixels)
}
