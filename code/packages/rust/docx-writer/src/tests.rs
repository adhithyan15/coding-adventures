//! Tests for `docx-writer`.
//!
//! The headline is the **round-trip proof**: build a document with the public
//! API, serialize it, then reopen the bytes with the independent read-side
//! `wordprocessingml` crate and assert the model comes back intact. Passing it
//! exercises the entire write path — WordprocessingML serialization, OPC
//! packaging, content types, relationships, ZIP — end to end.
//!
//! The remaining unit tests pin down escaping, Unicode, degenerate inputs
//! (empty document), and paragraph ordering.

use super::*;
use coding_adventures_wordprocessingml::{open_docx, Block as RBlock};

/// Every `.docx` must begin with the ZIP local-file-header magic `PK`.
#[test]
fn output_is_a_zip() {
    let doc = Document::new();
    let bytes = write_docx(&doc);
    assert_eq!(&bytes[..2], b"PK", "output must be a ZIP / OPC package");
}

/// THE PROOF. Build paragraph "Hello, DOCX!"; a two-run paragraph
/// "Second " + "paragraph."; a one-row table "A1cell"/"B1cell". Write, reopen
/// with the reader, and assert everything comes back.
#[test]
fn round_trips_through_wordprocessingml() {
    let mut doc = Document::new();
    doc.add_paragraph("Hello, DOCX!");
    doc.add_paragraph_runs(&["Second ", "paragraph."]);
    doc.add_table(&[vec!["A1cell".to_string(), "B1cell".to_string()]]);

    let bytes = write_docx(&doc);
    let read = open_docx(&bytes).expect("reader must open our .docx");

    // --- whole-document text extraction ---
    let text = read.text();
    assert!(text.contains("Hello, DOCX!"), "text was: {text:?}");
    assert!(
        text.contains("Second paragraph."),
        "the two runs must concatenate; text was: {text:?}"
    );
    assert!(text.contains("A1cell"), "text was: {text:?}");
    assert!(text.contains("B1cell"), "text was: {text:?}");

    // --- paragraph-by-paragraph ---
    let paras: Vec<_> = read.paragraphs().collect();
    assert_eq!(paras.len(), 2, "two top-level paragraphs (table excluded)");
    assert_eq!(paras[0].text, "Hello, DOCX!");
    assert_eq!(
        paras[1].text, "Second paragraph.",
        "two runs joined, trailing space of 'Second ' preserved"
    );
    // The second paragraph really is stored as two runs.
    assert_eq!(paras[1].runs.len(), 2);
    assert_eq!(paras[1].runs[0].text, "Second ");
    assert_eq!(paras[1].runs[1].text, "paragraph.");

    // --- the table ---
    let tables: Vec<_> = read.tables().collect();
    assert_eq!(tables.len(), 1);
    let row0 = &tables[0].rows[0];
    assert_eq!(row0.len(), 2, "two cells in row 0");
    assert_eq!(row0[0].text, "A1cell");
    assert_eq!(row0[1].text, "B1cell");
}

/// XML special characters in paragraph text must survive escaping and decode
/// back to the exact original string.
///
/// We deliberately write the specials *without* surrounding spaces. The reader's
/// xml-lexer has a documented limitation — it drops whitespace that sits at a
/// token boundary adjacent to an entity reference (see xml-parser's
/// `test_entities_in_text`). That is a property of the *reader*, not of our
/// output, which escapes correctly. Using adjacent specials exercises our
/// escaping of all five characters while staying inside what the reader can
/// faithfully decode.
#[test]
fn escapes_xml_specials() {
    let mut doc = Document::new();
    doc.add_paragraph("x&y<z>w\"v'u");

    let bytes = write_docx(&doc);
    let read = open_docx(&bytes).expect("open");
    let paras: Vec<_> = read.paragraphs().collect();
    assert_eq!(paras[0].text, "x&y<z>w\"v'u");
}

/// Sanity-check the escaping at the *bytes* level too, independent of the
/// reader: the serialized `document.xml` must contain the entity references, and
/// must NOT contain a raw `&`/`<`/`>` from the user text.
#[test]
fn document_xml_contains_entity_references() {
    let mut doc = Document::new();
    doc.add_paragraph("a & b < c");
    // The word/document.xml part is DEFLATE-compressed inside the ZIP, so we
    // can't grep the packaged bytes directly. Verify escaping at the serializer
    // seam by calling the private `document_xml` helper.
    let xml = String::from_utf8(super::document_xml(&doc)).expect("utf8");
    assert!(xml.contains("a &amp; b &lt; c"), "xml was: {xml}");
}

/// Unicode (including multi-byte code points and emoji) passes through unchanged.
#[test]
fn preserves_unicode() {
    let mut doc = Document::new();
    doc.add_paragraph("héllo — 世界 — 🦀");

    let bytes = write_docx(&doc);
    let read = open_docx(&bytes).expect("open");
    let paras: Vec<_> = read.paragraphs().collect();
    assert_eq!(paras[0].text, "héllo — 世界 — 🦀");
}

/// An empty document is valid: it opens and has an empty body (no blocks).
#[test]
fn empty_document_is_valid() {
    let doc = Document::new();
    let bytes = write_docx(&doc);
    let read = open_docx(&bytes).expect("empty doc must still open");
    assert!(read.blocks.is_empty(), "empty body → no blocks");
    assert_eq!(read.text(), "");
}

/// Multiple paragraphs come back in insertion order.
#[test]
fn preserves_paragraph_order() {
    let mut doc = Document::new();
    doc.add_paragraph("first");
    doc.add_paragraph("second");
    doc.add_paragraph("third");

    let bytes = write_docx(&doc);
    let read = open_docx(&bytes).expect("open");
    let texts: Vec<&str> = read.paragraphs().map(|p| p.text.as_str()).collect();
    assert_eq!(texts, vec!["first", "second", "third"]);
}

/// A run's **trailing** space survives — this is the case the round-trip relies
/// on ("Second " + "paragraph." → "Second paragraph."). A trailing space sits
/// inside the single TEXT token right before `</w:t>`, so the reader's lexer
/// keeps it.
///
/// (A *leading* space directly after `>` is consumed by the reader's lexer, a
/// documented xml-parser limitation — not a defect in our `xml:space="preserve"`
/// output — so we don't assert that here.)
#[test]
fn preserves_trailing_whitespace_across_runs() {
    let mut doc = Document::new();
    doc.add_paragraph_runs(&["alpha ", "beta ", "gamma"]);

    let bytes = write_docx(&doc);
    let read = open_docx(&bytes).expect("open");
    let paras: Vec<_> = read.paragraphs().collect();
    assert_eq!(paras[0].text, "alpha beta gamma");
    // And the whitespace lives inside `xml:space="preserve"` in our output.
    let xml = String::from_utf8(super::document_xml(&doc)).expect("utf8");
    assert!(xml.contains("xml:space=\"preserve\""));
}

/// An empty table is valid and round-trips as a table with no rows.
#[test]
fn empty_table_is_valid() {
    let mut doc = Document::new();
    doc.add_table(&[]);

    let bytes = write_docx(&doc);
    let read = open_docx(&bytes).expect("open");
    // The block exists as a table (not a paragraph) with zero rows.
    assert_eq!(read.blocks.len(), 1);
    match &read.blocks[0] {
        RBlock::Table(t) => assert!(t.rows.is_empty()),
        other => panic!("expected an empty table, got {other:?}"),
    }
}

/// A multi-row, multi-cell table round-trips with grid structure intact.
#[test]
fn multi_row_table_round_trips() {
    let mut doc = Document::new();
    doc.add_table(&[
        vec!["r0c0".to_string(), "r0c1".to_string()],
        vec!["r1c0".to_string(), "r1c1".to_string()],
    ]);

    let bytes = write_docx(&doc);
    let read = open_docx(&bytes).expect("open");
    let tables: Vec<_> = read.tables().collect();
    assert_eq!(tables[0].rows.len(), 2);
    assert_eq!(tables[0].rows[0][0].text, "r0c0");
    assert_eq!(tables[0].rows[0][1].text, "r0c1");
    assert_eq!(tables[0].rows[1][0].text, "r1c0");
    assert_eq!(tables[0].rows[1][1].text, "r1c1");
}
