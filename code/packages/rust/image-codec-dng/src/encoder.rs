// # encoder.rs — Minimal DNG Encoder (for round-trip tests)
//
// DNG is a strict superset of TIFF. Any valid TIFF with appropriate tags is a
// valid DNG. For round-trip testing, we produce a synthetic DNG that is simply
// an uncompressed TIFF — the simplest possible valid DNG.
//
// ## What makes a valid minimal DNG?
//
// At minimum, a "DNG" file just needs to be a TIFF that the decoder can read.
// The decoder's IFD selection logic falls back to IFD0 when no CFA/LinearRaw
// IFD is found, so a plain TIFF with PhotometricInterpretation=2 (RGB) works
// as a round-trip test vehicle.
//
// ## Limitations
//
// This encoder does NOT produce a real camera DNG:
// - No DNG version tag (50706)
// - No AsShotNeutral (the decode will use [1,1,1] identity WB)
// - No colour calibration matrices
// - PhotometricInterpretation=2 (RGB), not CFA or LinearRaw
//
// These limitations are intentional — the encoder's sole purpose is to provide
// a byte stream that `decode_dng` can ingest for round-trip tests. For
// production DNG encoding (e.g. for archiving), a full implementation would
// need to embed the camera-specific calibration data.
//
// ## Why not write DNG tags?
//
// A proper DNG encoder would need to know the camera model, its ForwardMatrix,
// and AsShotNeutral — none of which are stored in a `PixelContainer`. The
// `PixelContainer` holds only RGBA8 output pixels, not raw camera sensor data.

use image_codec_tiff::encode_tiff;
use pixel_container::PixelContainer;

/// Encode a `PixelContainer` as a minimal DNG-compatible TIFF file.
///
/// The output is a valid TIFF (and therefore a valid DNG, since DNG ⊇ TIFF)
/// containing uncompressed RGB pixel data.
///
/// ## Why encode as plain TIFF?
///
/// The `PixelContainer` holds RGBA8 output pixels — already colour-corrected,
/// gamma-encoded, 8-bit values. Re-encoding as a CFA would require knowing the
/// original raw sensor data, which is lost after decoding. For the round-trip
/// test (`encode → decode → same dimensions`), a plain TIFF is sufficient.
///
/// ## Output format
///
/// - Little-endian byte order
/// - Uncompressed pixel data (Compression=1)
/// - RGB chunky (SamplesPerPixel=3, PlanarConfiguration=1)
/// - 8-bit per channel
/// - No DNG private tags
///
/// ## Example
///
/// ```rust,ignore
/// use image_codec_dng::encode_dng;
/// use pixel_container::PixelContainer;
///
/// let mut pc = PixelContainer::new(10, 10);
/// pc.fill(255, 128, 0, 255); // orange
/// let dng_bytes = encode_dng(&pc);
/// // dng_bytes is a valid TIFF that decode_dng() can read back.
/// ```
pub fn encode_dng(pixels: &PixelContainer) -> Vec<u8> {
    // For round-trip tests, encode as a standard uncompressed RGB TIFF.
    // This is a valid minimal DNG (DNG is a superset of TIFF).
    encode_tiff(pixels)
}
