//! Where the ink actually lands, according to somebody else's renderer.
//!
//! `qpdf --check` (see `qpdf_gate.rs`) verifies that a file is *structurally*
//! sound: objects resolve, streams have the lengths they claim, the xref
//! agrees with reality. It cannot tell you the page is upside down, because an
//! upside-down page is structurally perfect.
//!
//! That is the failure this file exists for. **PDF's origin is bottom-left
//! with y growing upward**; screens, box trees and SVG all put it top-left with
//! y growing downward. A renderer that reconciles those wrongly produces output
//! where text appears, paths appear, every structural check passes — and the
//! page is mirrored. It looks entirely plausible until someone reads it.
//!
//! So these tests rasterise with **poppler** (`pdftoppm`), an independent
//! implementation, and ask where the dark pixels are. A flip moves them from
//! one half of the image to the other, which no amount of structural
//! validation would notice.
//!
//! ## Absence is a failure, not a skip
//!
//! [`require_pdftoppm`] panics when poppler is missing, matching `qpdf_gate.rs`
//! and for the same reason: a skipped oracle reports green while checking
//! nothing, which is worse than no oracle at all because it looks like
//! coverage.

use std::process::Command;

use pdf::{ColorTarget, Content, Document, Page, Paint, StandardFont, LETTER};

/// Locate `pdftoppm`, or fail loudly with instructions.
fn require_pdftoppm() -> &'static str {
    let found = Command::new("pdftoppm")
        .arg("-v")
        .output()
        .map(|out| out.status.success() || !out.stderr.is_empty())
        .unwrap_or(false);
    assert!(
        found,
        "`pdftoppm` (poppler-utils) is required: it is the independent renderer \
         that catches coordinate-flip bugs structural validation cannot see. \
         Install it with `brew install poppler` (macOS) or \
         `apt-get install poppler-utils` (Debian/Ubuntu)."
    );
    "pdftoppm"
}

/// A rasterised page: 8-bit RGB, row-major.
struct Raster {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Raster {
    /// Parse binary PPM (`P6`), which is what `pdftoppm` writes by default.
    ///
    /// Hand-parsed rather than pulled from a crate: the format is a magic
    /// number, three integers and a block of bytes, and this crate has no
    /// dependencies for good reasons that a test should not quietly undo.
    fn from_ppm(bytes: &[u8]) -> Self {
        let mut fields = Vec::new();
        let mut cursor = 0usize;
        while fields.len() < 4 {
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            // Comments run to end of line and may appear between any fields.
            if bytes.get(cursor) == Some(&b'#') {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
                continue;
            }
            let start = cursor;
            while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            fields.push(String::from_utf8_lossy(&bytes[start..cursor]).to_string());
        }
        assert_eq!(fields[0], "P6", "expected a binary PPM from pdftoppm");
        let width: usize = fields[1].parse().expect("ppm width");
        let height: usize = fields[2].parse().expect("ppm height");
        assert_eq!(fields[3], "255", "expected 8-bit samples");
        // Exactly one whitespace byte separates the header from the data.
        cursor += 1;

        let pixels = bytes[cursor..].to_vec();
        assert!(
            pixels.len() >= width * height * 3,
            "ppm is short: {} bytes for {width}x{height}",
            pixels.len()
        );
        Raster {
            width,
            height,
            pixels,
        }
    }

    /// Fraction of pixels darker than mid-grey within a region, as fractions
    /// of the page (0.0–1.0, origin top-left as the image itself is stored).
    fn ink(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        let px0 = (x0 * self.width as f64) as usize;
        let px1 = ((x1 * self.width as f64) as usize).min(self.width);
        let py0 = (y0 * self.height as f64) as usize;
        let py1 = ((y1 * self.height as f64) as usize).min(self.height);

        let mut dark = 0usize;
        let mut total = 0usize;
        for y in py0..py1 {
            for x in px0..px1 {
                let i = (y * self.width + x) * 3;
                let luma = (u16::from(self.pixels[i])
                    + u16::from(self.pixels[i + 1])
                    + u16::from(self.pixels[i + 2]))
                    / 3;
                if luma < 128 {
                    dark += 1;
                }
                total += 1;
            }
        }
        if total == 0 {
            0.0
        } else {
            dark as f64 / total as f64
        }
    }
}

/// Write a PDF, rasterise it with poppler, and return the page image.
fn rasterise(pdf_bytes: &[u8], name: &str) -> Raster {
    let tool = require_pdftoppm();
    let dir = std::env::temp_dir().join(format!("pdf-render-gate-{name}"));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let pdf_path = dir.join("page.pdf");
    std::fs::write(&pdf_path, pdf_bytes).expect("write pdf");

    let out_prefix = dir.join("out");
    let status = Command::new(tool)
        // 36 dpi: half a point per pixel, plenty to tell one half of a page
        // from the other, and small enough to scan quickly. PPM is pdftoppm's
        // default output -- there is no `-ppm` flag, and passing one makes it
        // print its usage and exit 0, which reads as "produced no image".
        .args(["-r", "36"])
        .arg(&pdf_path)
        .arg(&out_prefix)
        .output()
        .expect("run pdftoppm");
    assert!(
        status.status.success(),
        "pdftoppm failed on {name}: {}\n{}",
        String::from_utf8_lossy(&status.stderr),
        String::from_utf8_lossy(&status.stdout)
    );

    // pdftoppm appends the page number, whose width depends on the page count.
    let mut candidates: Vec<_> = std::fs::read_dir(&dir)
        .expect("read temp dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "ppm"))
        .collect();
    candidates.sort();
    assert!(
        !candidates.is_empty(),
        "pdftoppm produced no image for {name}"
    );

    let bytes = std::fs::read(&candidates[0]).expect("read ppm");
    Raster::from_ppm(&bytes)
}

/// A page with a filled square in one corner, described in top-down space.
fn page_with_corner_square() -> Vec<u8> {
    let (width, height) = LETTER;
    let mut content = Content::top_down(height);
    content
        .gray(0.0, ColorTarget::Fill)
        // Top-down: 40 points from the left, 40 from the TOP.
        .rect(40.0, 40.0, 200.0, 200.0)
        .paint(Paint::Fill);

    let mut doc = Document::new();
    doc.add_page(Page::with_content(width, height, content).expect("matching height"));
    doc.finish().expect("write pdf")
}

#[test]
fn a_top_down_rectangle_lands_in_the_top_left_of_the_rendered_page() {
    let raster = rasterise(&page_with_corner_square(), "top-left-square");

    let top_left = raster.ink(0.0, 0.0, 0.5, 0.5);
    let bottom_left = raster.ink(0.0, 0.5, 0.5, 1.0);

    // The whole point: the square was described 40 points from the top, so it
    // must be rendered near the top. If the y conversion were dropped or
    // inverted this assertion flips and nothing else in the suite notices --
    // the file is structurally identical either way.
    assert!(
        top_left > 0.2,
        "expected ink in the top-left quadrant, found {top_left:.3}"
    );
    assert!(
        bottom_left < 0.01,
        "found {bottom_left:.3} ink in the BOTTOM-left quadrant; the page is \
         mirrored, which qpdf --check cannot see"
    );
}

#[test]
fn the_same_square_in_pdf_space_lands_in_the_bottom_left() {
    // The mirror image of the test above, so neither can pass by ignoring the
    // coordinate space: `Space::Pdf` must NOT be flipped.
    let (width, height) = LETTER;
    let mut content = Content::pdf_space();
    content
        .gray(0.0, ColorTarget::Fill)
        .rect(40.0, 40.0, 200.0, 200.0)
        .paint(Paint::Fill);

    let mut doc = Document::new();
    doc.add_page(Page::with_content(width, height, content).expect("pdf space fits any page"));
    let raster = rasterise(&doc.finish().expect("write pdf"), "bottom-left-square");

    assert!(
        raster.ink(0.0, 0.5, 0.5, 1.0) > 0.2,
        "PDF-space y=40 means 40 from the BOTTOM"
    );
    assert!(
        raster.ink(0.0, 0.0, 0.5, 0.5) < 0.01,
        "a pdf-space page must not be flipped"
    );
}

#[test]
fn text_positioned_near_the_top_renders_near_the_top_and_upright() {
    let (width, height) = LETTER;
    let mut content = Content::top_down(height);
    content
        .begin_text()
        .font("F1", 48.0)
        .text_position(60.0, 90.0)
        .show_text(b"TOP")
        .end_text();

    let mut page = Page::with_content(width, height, content).expect("matching height");
    page.add_font("F1", StandardFont::HelveticaBold);
    let mut doc = Document::new();
    doc.add_page(page);
    let raster = rasterise(&doc.finish().expect("write pdf"), "text-top");

    let top = raster.ink(0.0, 0.0, 1.0, 0.5);
    let bottom = raster.ink(0.0, 0.5, 1.0, 1.0);
    assert!(
        top > 0.001,
        "expected glyphs in the top half, found {top:.4}"
    );
    assert!(
        bottom < 0.0005,
        "found {bottom:.4} ink in the bottom half; the text baseline was \
         mirrored the wrong way"
    );
}

#[test]
fn text_is_not_mirrored_by_the_top_down_space() {
    // A flip implemented as `1 0 0 -1 0 h cm` puts text in the right PLACE and
    // renders every glyph reversed. Poppler will happily extract the text
    // either way, so this compares the rendering against the same string drawn
    // in PDF space at the mirrored position: the two must be identical images.
    let (width, height) = LETTER;

    let mut top_down = Content::top_down(height);
    top_down
        .begin_text()
        .font("F1", 36.0)
        .text_position(72.0, 200.0)
        .show_text(b"Rp")
        .end_text();

    let mut native = Content::pdf_space();
    native
        .begin_text()
        .font("F1", 36.0)
        // The same baseline, expressed from the bottom.
        .text_position(72.0, height - 200.0)
        .show_text(b"Rp")
        .end_text();

    let render = |content: Content, name: &str| {
        let mut page = Page::with_content(width, height, content).expect("height");
        page.add_font("F1", StandardFont::Helvetica);
        let mut doc = Document::new();
        doc.add_page(page);
        rasterise(&doc.finish().expect("write pdf"), name)
    };

    let a = render(top_down, "text-topdown");
    let b = render(native, "text-native");

    assert_eq!(a.width, b.width);
    assert_eq!(a.height, b.height);
    assert_eq!(
        a.pixels, b.pixels,
        "top-down text differs from the same text placed natively; the space \
         is transforming the glyphs, not just the position"
    );
    // And it must actually have drawn something, or the comparison is vacuous.
    assert!(a.ink(0.0, 0.0, 1.0, 1.0) > 0.0005, "no glyphs were drawn");
}
