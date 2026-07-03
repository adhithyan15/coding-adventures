// # image-convert (library)
//
// The library part of the universal image format converter. Exposes format
// detection, decoding, and encoding without any filesystem I/O — making
// everything testable in unit tests without touching real files.
//
// The `main.rs` binary wraps these functions with file I/O and a CLI.

pub mod codecs;
pub mod detect;

pub use codecs::{decode_image, encode_image};
pub use detect::{
    detect_by_extension, detect_format, extension_from_path, ImageFormat,
};
pub use pixel_container::PixelContainer;

/// Format a human-readable list of all supported input and output formats,
/// suitable for the `--list-formats` flag.
pub fn list_formats() -> String {
    let mut s = String::new();

    s.push_str("Input formats (decode):\n");
    let inputs = [
        (ImageFormat::Png,  ".png"),
        (ImageFormat::Bmp,  ".bmp"),
        (ImageFormat::Ppm,  ".ppm / .pgm"),
        (ImageFormat::Qoi,  ".qoi"),
        (ImageFormat::Jpeg, ".jpg / .jpeg"),
        (ImageFormat::WebP, ".webp"),
        (ImageFormat::Jxl,  ".jxl"),
        (ImageFormat::Gif,  ".gif"),
        (ImageFormat::Ico,  ".ico / .cur"),
        (ImageFormat::Tiff, ".tif / .tiff"),
        (ImageFormat::Dng,  ".dng"),
        (ImageFormat::Cr2,  ".cr2"),
        (ImageFormat::Nef,  ".nef"),
        (ImageFormat::Arw,  ".arw"),
        (ImageFormat::Raf,  ".raf"),
        (ImageFormat::Orf,  ".orf"),
        (ImageFormat::Rw2,  ".rw2"),
    ];
    for (fmt, exts) in &inputs {
        s.push_str(&format!("  {:12}  {}\n", exts, fmt.name()));
    }

    s.push_str("\nOutput formats (encode):\n");
    let outputs = [
        (ImageFormat::Png,  ".png"),
        (ImageFormat::Bmp,  ".bmp"),
        (ImageFormat::Ppm,  ".ppm"),
        (ImageFormat::Qoi,  ".qoi"),
        (ImageFormat::Jpeg, ".jpg / .jpeg"),
        (ImageFormat::WebP, ".webp"),
        (ImageFormat::Jxl,  ".jxl"),
        (ImageFormat::Gif,  ".gif"),
        (ImageFormat::Ico,  ".ico"),
        (ImageFormat::Tiff, ".tif / .tiff"),
    ];
    for (fmt, exts) in &outputs {
        s.push_str(&format!("  {:12}  {}\n", exts, fmt.name()));
    }

    s.push_str("\nRAW formats (DNG/CR2/NEF/ARW/RAF/ORF/RW2) are input-only.\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_formats_contains_all_inputs() {
        let s = list_formats();
        for name in ["PNG", "JPEG", "WebP", "TIFF", "DNG", "Canon CR2",
                     "Nikon NEF", "Sony ARW", "Fujifilm RAF"] {
            assert!(s.contains(name), "list_formats missing: {}", name);
        }
    }

    #[test]
    fn list_formats_mentions_raw_input_only() {
        let s = list_formats();
        assert!(s.contains("input-only"), "Should note RAW formats are input-only");
    }

    #[test]
    fn end_to_end_encode_decode_png() {
        let mut px = PixelContainer::new(2, 2);
        px.fill(100, 150, 200, 255);
        let fmt = ImageFormat::Png;
        let bytes = encode_image(&px, &fmt, 85).unwrap();
        assert!(bytes.starts_with(b"\x89PNG"));
        let decoded = decode_image(&bytes, &fmt).unwrap();
        assert_eq!(decoded.pixel_at(0, 0), (100, 150, 200, 255));
    }

    #[test]
    fn detect_format_png_magic() {
        let magic = b"\x89PNG\r\n\x1a\ndata";
        let fmt = detect_format(magic, Some("jpg")).unwrap();
        assert_eq!(fmt, ImageFormat::Png, "Magic should override extension");
    }
}
