// # encoder.rs — minimal ARW encoder for round-trip testing
//
// A real ARW encoder would need to write Sony MakerNote metadata,
// white balance data in SonyMakerNote2 (tag 0x2001), and optionally the
// Sony compressed row format. That is beyond the scope of v0.1.
//
// Instead, we encode as a standard uncompressed TIFF. The resulting file is
// structurally valid as a TIFF — and therefore as a bare-minimum ARW —
// and can be decoded back by `decode_arw` (no Make tag = allowed for
// synthetic files).

use pixel_container::PixelContainer;

/// Encode a `PixelContainer` as a minimal synthetic ARW file.
///
/// The output is a valid uncompressed TIFF that `decode_arw` can round-trip.
/// It is NOT suitable for writing to a Sony camera — ARW is proprietary
/// and read-only in practice.
///
/// # Example
///
/// ```rust,ignore
/// let mut pc = PixelContainer::new(4, 4);
/// pc.fill(100, 150, 200, 255);
/// let arw_bytes = encode_arw(&pc);
/// let decoded = decode_arw(&arw_bytes).unwrap();
/// assert_eq!(decoded.width, 4);
/// ```
pub fn encode_arw(pixels: &PixelContainer) -> Vec<u8> {
    image_codec_tiff::encode_tiff(pixels)
}
