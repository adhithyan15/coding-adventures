//! An embedded subsetted font, checked by an independent reader.
//!
//! This is PDF-3's "done when": a PDF with an embedded subsetted TrueType font
//! renders correct glyphs in an independent renderer, and `pdftotext` recovers
//! the original string.
//!
//! ## Two oracles, because they see different failures
//!
//! **`pdftotext`** exercises the `/ToUnicode` CMap, which is invisible to
//! rendering. A PDF without one draws perfectly and yields gibberish when
//! selected, searched, or read aloud — a defect that survives every visual
//! check ever run against it.
//!
//! **`pdftoppm`** exercises the glyphs. A correct CMap over a broken font
//! extracts perfect text from a blank page.
//!
//! Neither alone is enough, which is the point of running both.
//!
//! Linux and macOS only, matching `render_gate.rs`: poppler has no reliable
//! Chocolatey package, and this crate writes byte-identical PDFs everywhere so
//! a third platform adds no coverage.
#![cfg(not(target_os = "windows"))]

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use pdf::{Content, Document, EmbeddedFont, EmbeddedGlyph, Page, LETTER};

fn require(tool: &str) {
    let found = Command::new(tool)
        .arg("-v")
        .output()
        .map(|out| out.status.success() || !out.stderr.is_empty())
        .unwrap_or(false);
    assert!(
        found,
        "`{tool}` (poppler-utils) is required: it is the independent reader \
         that checks what we embedded. Install with `brew install poppler` or \
         `apt-get install poppler-utils`."
    );
}

/// Build a one-page PDF showing `text` in an embedded subset of `font_path`.
///
/// Returns the PDF bytes and the string that should come back out.
fn pdf_with_embedded_text(font_path: &str, text: &str) -> (Vec<u8>, String) {
    let bytes = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(font_path))
        .expect("font fixture");
    let font = font_parser::load(&bytes).expect("font loads");
    let metrics = font_parser::font_metrics(&font);

    // Map each character to its glyph through the font's own cmap, so the
    // subset contains exactly what the text needs.
    let mut wanted = BTreeSet::new();
    let mut glyph_ids = Vec::new();
    for character in text.chars() {
        let id = font_parser::glyph_id(&font, character as u32)
            .unwrap_or_else(|| panic!("{font_path} has no glyph for {character:?}"));
        wanted.insert(id);
        glyph_ids.push(id);
    }

    let subset = font_subset::subset(&font, &wanted).expect("subset");

    let mut glyphs: BTreeMap<u16, EmbeddedGlyph> = BTreeMap::new();
    for (character, id) in text.chars().zip(&glyph_ids) {
        let advance = font_parser::glyph_metrics(&font, *id)
            .map(|m| m.advance_width)
            .unwrap_or(0);
        glyphs.insert(
            *id,
            EmbeddedGlyph {
                advance,
                text: character.to_string(),
            },
        );
    }

    let embedded = EmbeddedFont::new(
        "TestSubset",
        subset.font,
        metrics.units_per_em,
        metrics.ascender,
        metrics.descender,
        [
            0,
            metrics.descender,
            metrics.units_per_em as i16,
            metrics.ascender,
        ],
        glyphs,
    );

    let (width, height) = LETTER;
    let mut content = Content::top_down(height);
    content
        .begin_text()
        .font("F1", 48.0)
        .text_position(72.0, 120.0)
        // Glyph IDS, not characters -- the encoding is Identity-H.
        .show_glyphs(&glyph_ids)
        .end_text();

    let mut page = Page::with_content(width, height, content).expect("page");
    page.add_embedded_font("F1", embedded);
    let mut doc = Document::new();
    doc.add_page(page);
    (doc.finish().expect("write pdf"), text.to_string())
}

fn write_temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("pdf-embed-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("doc.pdf");
    std::fs::write(&path, bytes).expect("write pdf");
    path
}

fn extract_text(pdf: &[u8], name: &str) -> String {
    require("pdftotext");
    let path = write_temp(name, pdf);
    let output = Command::new("pdftotext")
        .arg(&path)
        .arg("-")
        .output()
        .expect("run pdftotext");
    assert!(
        output.status.success(),
        "pdftotext failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn ink_fraction(pdf: &[u8], name: &str) -> f64 {
    require("pdftoppm");
    let path = write_temp(name, pdf);
    let prefix = path.with_file_name("page");
    let status = Command::new("pdftoppm")
        .args(["-r", "36"])
        .arg(&path)
        .arg(&prefix)
        .status()
        .expect("run pdftoppm");
    assert!(status.success(), "pdftoppm failed on {name}");

    let mut images: Vec<_> = std::fs::read_dir(path.parent().unwrap())
        .expect("read dir")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "ppm"))
        .collect();
    images.sort();
    assert!(!images.is_empty(), "pdftoppm produced no image for {name}");

    let bytes = std::fs::read(&images[0]).expect("read ppm");
    // P6 header: magic, width, height, maxval, then one whitespace byte.
    let mut fields = Vec::new();
    let mut cursor = 0usize;
    while fields.len() < 4 {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let start = cursor;
        while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        fields.push(String::from_utf8_lossy(&bytes[start..cursor]).to_string());
    }
    cursor += 1;
    let pixels = &bytes[cursor..];
    let dark = pixels
        .chunks(3)
        .filter(|p| p.len() == 3 && (u16::from(p[0]) + u16::from(p[1]) + u16::from(p[2])) / 3 < 128)
        .count();
    dark as f64 / (pixels.len() / 3).max(1) as f64
}

/// Latin, the easy case, so a failure here is about embedding rather than script.
#[test]
fn an_embedded_latin_subset_renders_and_extracts() {
    let (pdf, expected) =
        pdf_with_embedded_text("../../../fixtures/fonts/Inter-Regular.ttf", "Engram");

    let extracted = extract_text(&pdf, "latin");
    assert_eq!(
        extracted, expected,
        "pdftotext should recover the original string through /ToUnicode"
    );
    assert!(
        ink_fraction(&pdf, "latin") > 0.0005,
        "the glyphs should actually be drawn"
    );
}

/// The scripts that force embedding in the first place.
///
/// Nothing in the base-14 fonts can draw either, so this is the case PDF-3
/// exists for — and a font pipeline verified only on Latin is verified on the
/// half that already worked.
#[test]
fn embedded_tamil_and_japanese_render_and_extract() {
    for (font, text, name) in [
        (
            "../../../learning/human-languages/_fonts/NotoSansTamil-Static.ttf",
            "\u{0B85}\u{0B86}",
            "tamil",
        ),
        (
            "../../../learning/human-languages/_fonts/NotoSansJP-Subset.ttf",
            "\u{3042}",
            "japanese",
        ),
    ] {
        let (pdf, expected) = pdf_with_embedded_text(font, text);

        let extracted = extract_text(&pdf, name);
        assert_eq!(
            extracted, expected,
            "{name}: pdftotext should recover the original text -- without a \
             correct /ToUnicode this is where a PDF that LOOKS perfect returns \
             gibberish"
        );
        assert!(
            ink_fraction(&pdf, name) > 0.0005,
            "{name}: the glyphs should actually be drawn"
        );
    }
}

/// The embedded font is a subset, not the whole face.
///
/// Without this the suite would pass on a pipeline that embedded the entire
/// font — correct output, and a 20 MB export for a two-page study sheet.
#[test]
fn the_embedded_font_is_much_smaller_than_the_original() {
    let original = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../learning/human-languages/_fonts/NotoSansJP-Subset.ttf"),
    )
    .expect("font fixture");
    let (pdf, _) = pdf_with_embedded_text(
        "../../../learning/human-languages/_fonts/NotoSansJP-Subset.ttf",
        "\u{3042}",
    );
    assert!(
        pdf.len() * 2 < original.len(),
        "the whole PDF ({}) should be well under the unsubsetted font ({})",
        pdf.len(),
        original.len()
    );
}
