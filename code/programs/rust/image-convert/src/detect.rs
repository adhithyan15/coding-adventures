// # detect.rs — Image Format Detection
//
// Identifies the format of an image from its magic bytes and/or file extension.
//
// ## Detection Priority
//
// 1. Magic bytes (first 16 bytes) — most reliable, handles mislabelled files.
// 2. File extension — fallback for ambiguous TIFF-family files and truncated data.
//
// ## Why Magic Bytes First?
//
// A camera RAW file saved as ".tiff" is still a DNG/CR2/NEF. Reading the
// actual bytes — not just the filename — gives the correct answer. The
// extension is only consulted when magic bytes alone are ambiguous (e.g.,
// all TIFF-family files start with the same TIFF header).

/// Every image format this program knows about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageFormat {
    // ── Standard web/desktop formats ──────────────────────────────────────
    /// Portable Network Graphics — lossless, RGBA support.
    Png,
    /// Windows Bitmap — uncompressed, large.
    Bmp,
    /// Portable Pixmap — plain-text or binary RGB, for debugging.
    Ppm,
    /// Quite OK Image — fast lossless compression.
    Qoi,
    /// JPEG — lossy, ubiquitous.
    Jpeg,
    /// WebP — VP8L lossless or VP8 lossy.
    WebP,
    /// JPEG XL — next-gen lossless/lossy codec.
    Jxl,
    /// GIF — 256-colour animated/static images.
    Gif,
    /// Windows ICO/CUR — icon container.
    Ico,
    /// TIFF — flexible tagged image format; also RAW container.
    Tiff,
    // ── Camera RAW formats (input only) ────────────────────────────────────
    /// Adobe Digital Negative — open RAW standard.
    Dng,
    /// Canon CR2 — TIFF with lossless JPEG strips, signature "CR\x02".
    Cr2,
    /// Nikon NEF — TIFF with Nikon MakerNote.
    Nef,
    /// Sony ARW — TIFF with Sony-specific compression.
    Arw,
    /// Fujifilm RAF — proprietary container, X-Trans or Bayer sensor.
    Raf,
    /// Olympus ORF — TIFF variant (sometimes with "IIRO" magic).
    Orf,
    /// Panasonic RW2 — TIFF-like with magic `II\x55\x00`.
    Rw2,
}

impl ImageFormat {
    /// Returns `true` if this format supports encoding (output).
    ///
    /// Camera RAW formats are input-only: re-encoding to a proprietary
    /// sensor-data format would produce an invalid file that no camera
    /// firmware could read.
    pub fn is_encodable(&self) -> bool {
        matches!(
            self,
            ImageFormat::Png
                | ImageFormat::Bmp
                | ImageFormat::Ppm
                | ImageFormat::Qoi
                | ImageFormat::Jpeg
                | ImageFormat::WebP
                | ImageFormat::Jxl
                | ImageFormat::Gif
                | ImageFormat::Ico
                | ImageFormat::Tiff
        )
    }

    /// Short human-readable name for `--list-formats` output.
    pub fn name(&self) -> &'static str {
        match self {
            ImageFormat::Png  => "PNG",
            ImageFormat::Bmp  => "BMP",
            ImageFormat::Ppm  => "PPM",
            ImageFormat::Qoi  => "QOI",
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::WebP => "WebP",
            ImageFormat::Jxl  => "JPEG XL",
            ImageFormat::Gif  => "GIF",
            ImageFormat::Ico  => "ICO",
            ImageFormat::Tiff => "TIFF",
            ImageFormat::Dng  => "Adobe DNG",
            ImageFormat::Cr2  => "Canon CR2",
            ImageFormat::Nef  => "Nikon NEF",
            ImageFormat::Arw  => "Sony ARW",
            ImageFormat::Raf  => "Fujifilm RAF",
            ImageFormat::Orf  => "Olympus ORF",
            ImageFormat::Rw2  => "Panasonic RW2",
        }
    }

    /// MIME type string for the format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png  => "image/png",
            ImageFormat::Bmp  => "image/bmp",
            ImageFormat::Ppm  => "image/x-portable-pixmap",
            ImageFormat::Qoi  => "image/x-qoi",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Jxl  => "image/jxl",
            ImageFormat::Gif  => "image/gif",
            ImageFormat::Ico  => "image/x-icon",
            ImageFormat::Tiff => "image/tiff",
            ImageFormat::Dng  => "image/x-adobe-dng",
            ImageFormat::Cr2  => "image/x-canon-cr2",
            ImageFormat::Nef  => "image/x-nikon-nef",
            ImageFormat::Arw  => "image/x-sony-arw",
            ImageFormat::Raf  => "image/x-fuji-raf",
            ImageFormat::Orf  => "image/x-olympus-orf",
            ImageFormat::Rw2  => "image/x-panasonic-rw2",
        }
    }
}

// ─── Format detection ─────────────────────────────────────────────────────────

/// Detect the image format of `bytes` using magic bytes, with `ext` as fallback.
///
/// `ext` should be the file extension without the dot (e.g. `"jpg"`), already
/// lowercased. Pass `None` if no path is known (e.g. when reading from stdin).
///
/// Returns `None` if the format cannot be determined from either source.
pub fn detect_format(bytes: &[u8], ext: Option<&str>) -> Option<ImageFormat> {
    // 1. Try magic bytes for formats with unambiguous signatures.
    if let Some(fmt) = detect_by_magic(bytes) {
        return Some(fmt);
    }
    // 2. Fall back to file extension.
    ext.and_then(detect_by_extension)
}

/// Detect from the leading magic bytes of the file.
///
/// Returns `None` for formats whose magic is ambiguous (TIFF family).
/// The caller falls back to the extension for those cases.
fn detect_by_magic(bytes: &[u8]) -> Option<ImageFormat> {
    // Helper: check prefix match.
    let starts_with = |pat: &[u8]| bytes.len() >= pat.len() && bytes[..pat.len()] == *pat;

    // ── PNG: 8-byte signature ─────────────────────────────────────────────
    // \x89 P N G \r \n \x1a \n
    if starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(ImageFormat::Png);
    }

    // ── JPEG: SOI marker ──────────────────────────────────────────────────
    // \xFF \xD8 \xFF
    if starts_with(b"\xFF\xD8\xFF") {
        return Some(ImageFormat::Jpeg);
    }

    // ── GIF: "GIF87a" or "GIF89a" ────────────────────────────────────────
    if starts_with(b"GIF87a") || starts_with(b"GIF89a") {
        return Some(ImageFormat::Gif);
    }

    // ── WebP: "RIFF????WEBP" ──────────────────────────────────────────────
    // Bytes 0-3 = "RIFF", bytes 8-11 = "WEBP".
    if bytes.len() >= 12
        && &bytes[0..4] == b"RIFF"
        && &bytes[8..12] == b"WEBP"
    {
        return Some(ImageFormat::WebP);
    }

    // ── JPEG XL: naked codestream \xFF\x0A or ISOBMFF box "JXL " ─────────
    if starts_with(b"\xFF\x0A") {
        return Some(ImageFormat::Jxl);
    }
    if bytes.len() >= 12 && &bytes[4..12] == b"ftypJXL " {
        return Some(ImageFormat::Jxl);
    }

    // ── QOI: "qoif" ──────────────────────────────────────────────────────
    if starts_with(b"qoif") {
        return Some(ImageFormat::Qoi);
    }

    // ── Fujifilm RAF: 16-byte magic ───────────────────────────────────────
    if starts_with(b"FUJIFILMCCD-RAW ") {
        return Some(ImageFormat::Raf);
    }

    // ── Panasonic RW2: "II\x55\x00" ──────────────────────────────────────
    // Version byte 0x55 (85) instead of standard TIFF 42.
    if starts_with(b"II\x55\x00") {
        return Some(ImageFormat::Rw2);
    }

    // ── ICO/CUR: "\x00\x00\x01\x00" ──────────────────────────────────────
    if starts_with(b"\x00\x00\x01\x00") || starts_with(b"\x00\x00\x02\x00") {
        return Some(ImageFormat::Ico);
    }

    // ── BMP: "BM" ────────────────────────────────────────────────────────
    if starts_with(b"BM") {
        return Some(ImageFormat::Bmp);
    }

    // ── TIFF family: "II\x2A\x00" or "MM\x00\x2A" or Olympus "IIRO" ─────
    // These all share the same outer magic; we discriminate by extension.
    let is_le_tiff = starts_with(b"II\x2A\x00");
    let is_be_tiff = starts_with(b"MM\x00\x2A");
    let is_iiro   = starts_with(b"II\x52\x4F"); // Olympus IIRO variant

    if is_le_tiff || is_be_tiff || is_iiro {
        // CR2 has a distinctive "CR\x02" at offset 8.
        if bytes.len() >= 11 && &bytes[8..10] == b"CR" && bytes[10] == 2 {
            return Some(ImageFormat::Cr2);
        }
        // Olympus IIRO is unambiguous.
        if is_iiro {
            return Some(ImageFormat::Orf);
        }
        // For remaining TIFF-family files, we cannot distinguish DNG/NEF/ARW/ORF
        // from plain TIFF without parsing the IFDs. Return None and let the
        // extension handler do the disambiguation.
        return None;
    }

    // ── PPM family: "P6\n", "P5\n", "P3\n", "P2\n" ──────────────────────
    if bytes.len() >= 2 && bytes[0] == b'P' && matches!(bytes[1], b'2' | b'3' | b'5' | b'6') {
        return Some(ImageFormat::Ppm);
    }

    None
}

/// Detect format from a lowercased file extension (without the dot).
pub fn detect_by_extension(ext: &str) -> Option<ImageFormat> {
    match ext {
        "png"            => Some(ImageFormat::Png),
        "bmp"            => Some(ImageFormat::Bmp),
        "ppm" | "pgm" | "pnm" => Some(ImageFormat::Ppm),
        "qoi"            => Some(ImageFormat::Qoi),
        "jpg" | "jpeg"   => Some(ImageFormat::Jpeg),
        "webp"           => Some(ImageFormat::WebP),
        "jxl"            => Some(ImageFormat::Jxl),
        "gif"            => Some(ImageFormat::Gif),
        "ico" | "cur"    => Some(ImageFormat::Ico),
        "tif" | "tiff"   => Some(ImageFormat::Tiff),
        "dng"            => Some(ImageFormat::Dng),
        "cr2"            => Some(ImageFormat::Cr2),
        "nef"            => Some(ImageFormat::Nef),
        "arw"            => Some(ImageFormat::Arw),
        "raf"            => Some(ImageFormat::Raf),
        "orf"            => Some(ImageFormat::Orf),
        "rw2"            => Some(ImageFormat::Rw2),
        _                => None,
    }
}

/// Extract the lowercased extension from a file path, without the dot.
///
/// ```text
/// "photo.NEF"     → Some("nef")
/// "archive.tar.gz"→ Some("gz")
/// "README"        → None
/// ```
pub fn extension_from_path(path: &str) -> Option<String> {
    path.rsplit('.').next()
        .filter(|e| !e.is_empty() && *e != path) // no extension if the whole name is returned
        .map(|e| e.to_lowercase())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Magic byte tests ──────────────────────────────────────────────────

    #[test]
    fn detects_png_by_magic() {
        let magic = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        assert_eq!(detect_by_magic(magic), Some(ImageFormat::Png));
    }

    #[test]
    fn detects_jpeg_by_magic() {
        let magic = b"\xFF\xD8\xFF\xE0\x00\x10JFIF";
        assert_eq!(detect_by_magic(magic), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn detects_gif87_by_magic() {
        assert_eq!(detect_by_magic(b"GIF87a\x00"), Some(ImageFormat::Gif));
    }

    #[test]
    fn detects_gif89_by_magic() {
        assert_eq!(detect_by_magic(b"GIF89a\x00"), Some(ImageFormat::Gif));
    }

    #[test]
    fn detects_webp_by_magic() {
        let mut magic = [0u8; 12];
        magic[0..4].copy_from_slice(b"RIFF");
        magic[4..8].copy_from_slice(&[0x10, 0x00, 0x00, 0x00]);
        magic[8..12].copy_from_slice(b"WEBP");
        assert_eq!(detect_by_magic(&magic), Some(ImageFormat::WebP));
    }

    #[test]
    fn detects_jxl_naked_codestream() {
        assert_eq!(detect_by_magic(b"\xFF\x0A\x00\x00"), Some(ImageFormat::Jxl));
    }

    #[test]
    fn detects_qoi_by_magic() {
        assert_eq!(detect_by_magic(b"qoif\x00\x00\x00\x04"), Some(ImageFormat::Qoi));
    }

    #[test]
    fn detects_raf_by_magic() {
        assert_eq!(detect_by_magic(b"FUJIFILMCCD-RAW \x00"), Some(ImageFormat::Raf));
    }

    #[test]
    fn detects_rw2_by_magic() {
        assert_eq!(detect_by_magic(b"II\x55\x00\x08\x00\x00\x00"), Some(ImageFormat::Rw2));
    }

    #[test]
    fn detects_ico_by_magic() {
        assert_eq!(detect_by_magic(b"\x00\x00\x01\x00"), Some(ImageFormat::Ico));
    }

    #[test]
    fn detects_bmp_by_magic() {
        assert_eq!(detect_by_magic(b"BM\x46\x00\x00\x00"), Some(ImageFormat::Bmp));
    }

    #[test]
    fn detects_cr2_by_magic() {
        // II + TIFF magic + CR2 signature at offset 8
        let mut bytes = [0u8; 12];
        bytes[0..4].copy_from_slice(b"II\x2A\x00");
        bytes[4..8].copy_from_slice(&[0x10, 0x00, 0x00, 0x00]);
        bytes[8..11].copy_from_slice(b"CR\x02");
        assert_eq!(detect_by_magic(&bytes), Some(ImageFormat::Cr2));
    }

    #[test]
    fn detects_orf_iiro_by_magic() {
        assert_eq!(detect_by_magic(b"II\x52\x4F\x08\x00\x00\x00"), Some(ImageFormat::Orf));
    }

    #[test]
    fn tiff_family_magic_returns_none() {
        // Plain LE TIFF — cannot distinguish DNG/NEF/ARW/generic from magic alone
        let bytes = b"II\x2A\x00\x08\x00\x00\x00\x00\x00";
        assert_eq!(detect_by_magic(bytes), None);
    }

    #[test]
    fn detects_ppm_by_magic() {
        assert_eq!(detect_by_magic(b"P6\n"), Some(ImageFormat::Ppm));
        assert_eq!(detect_by_magic(b"P5\n"), Some(ImageFormat::Ppm));
    }

    // ── Extension tests ───────────────────────────────────────────────────

    #[test]
    fn detects_jpeg_by_ext_jpg() {
        assert_eq!(detect_by_extension("jpg"), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn detects_jpeg_by_ext_jpeg() {
        assert_eq!(detect_by_extension("jpeg"), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn detects_nef_by_ext() {
        assert_eq!(detect_by_extension("nef"), Some(ImageFormat::Nef));
    }

    #[test]
    fn detects_arw_by_ext() {
        assert_eq!(detect_by_extension("arw"), Some(ImageFormat::Arw));
    }

    #[test]
    fn detects_dng_by_ext() {
        assert_eq!(detect_by_extension("dng"), Some(ImageFormat::Dng));
    }

    #[test]
    fn unknown_extension_returns_none() {
        assert_eq!(detect_by_extension("xyz"), None);
    }

    // ── detect_format combining both ──────────────────────────────────────

    #[test]
    fn magic_beats_wrong_extension() {
        // PNG magic bytes but ".bmp" extension → PNG wins
        let magic = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        assert_eq!(detect_format(magic, Some("bmp")), Some(ImageFormat::Png));
    }

    #[test]
    fn extension_used_when_magic_ambiguous() {
        // Plain TIFF bytes, ".dng" extension → DNG
        let bytes = b"II\x2A\x00\x08\x00\x00\x00\x00\x00";
        assert_eq!(detect_format(bytes, Some("dng")), Some(ImageFormat::Dng));
        assert_eq!(detect_format(bytes, Some("nef")), Some(ImageFormat::Nef));
        assert_eq!(detect_format(bytes, Some("tif")), Some(ImageFormat::Tiff));
    }

    #[test]
    fn no_magic_no_extension_returns_none() {
        assert_eq!(detect_format(&[0x00, 0x01, 0x02], None), None);
    }

    // ── extension_from_path ───────────────────────────────────────────────

    #[test]
    fn extension_uppercase_lowercased() {
        assert_eq!(extension_from_path("PHOTO.NEF"), Some("nef".into()));
    }

    #[test]
    fn extension_no_dot_returns_none() {
        assert_eq!(extension_from_path("README"), None);
    }

    #[test]
    fn extension_multi_dot() {
        assert_eq!(extension_from_path("archive.tar.gz"), Some("gz".into()));
    }

    // ── ImageFormat properties ─────────────────────────────────────────────

    #[test]
    fn raw_formats_not_encodable() {
        for fmt in [
            ImageFormat::Dng, ImageFormat::Cr2, ImageFormat::Nef,
            ImageFormat::Arw, ImageFormat::Raf, ImageFormat::Orf, ImageFormat::Rw2,
        ] {
            assert!(!fmt.is_encodable(), "{} should not be encodable", fmt.name());
        }
    }

    #[test]
    fn standard_formats_encodable() {
        for fmt in [
            ImageFormat::Png, ImageFormat::Bmp, ImageFormat::Jpeg,
            ImageFormat::Tiff, ImageFormat::WebP, ImageFormat::Gif,
        ] {
            assert!(fmt.is_encodable(), "{} should be encodable", fmt.name());
        }
    }

    #[test]
    fn format_names_non_empty() {
        for fmt in [
            ImageFormat::Png, ImageFormat::Dng, ImageFormat::Cr2,
            ImageFormat::Nef, ImageFormat::Raf,
        ] {
            assert!(!fmt.name().is_empty());
        }
    }
}
