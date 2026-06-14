// # encoder.rs — minimal ORF encoder
//
// ORF is a TIFF container.  For the purposes of this crate, encoding means
// writing a standard uncompressed TIFF file.  Real Olympus cameras would write
// proprietary MakerNote IFDs, CFA patterns, and possibly Olympus compressed
// pixel data (Compression=32767), but none of that is needed for round-trip
// testing.
//
// We delegate entirely to image_codec_tiff::encode_tiff, which produces:
//   - Little-endian byte order ("II")
//   - TIFF magic 42
//   - Compression=1 (uncompressed)
//   - RGB chunky layout, 8 bits per channel
//   - A single strip containing all pixel data
//
// This output is accepted by decode_orf because:
//   1. It starts with "II" (LE) ✓
//   2. Magic is standard 42 — no IIRO patching needed ✓
//   3. No Make tag → no make-tag rejection ✓
//   4. PhotometricInterpretation is RGB (2), not CFA (32803), so we fall back
//      to ifd_index=0 and the TIFF decoder handles it as a normal RGB image ✓

use pixel_container::PixelContainer;

/// Encode a `PixelContainer` as an uncompressed TIFF-based ORF byte stream.
///
/// The output is a valid TIFF file that `decode_orf` will accept.  It is NOT
/// a camera-native ORF (no MakerNote, no CFA pattern, no Olympus extensions),
/// but it is sufficient for round-trip and regression testing.
///
/// # Arguments
///
/// * `pixels` — The source RGBA8 pixel container.
///
/// # Returns
///
/// A `Vec<u8>` containing the TIFF-encoded pixel data.
pub fn encode_orf(pixels: &PixelContainer) -> Vec<u8> {
    image_codec_tiff::encode_tiff(pixels)
}
