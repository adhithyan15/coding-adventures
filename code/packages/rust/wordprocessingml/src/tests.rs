//! Tests for the WordprocessingML reader.
//!
//! The headline test opens the real DEFLATE-compressed [`MINIMAL_DOCX`] fixture
//! and asserts the full paragraph/table extraction end-to-end. The rest exercise
//! run-joining, tabs/breaks, table structure, `Document::text()`, the error
//! paths, and empty/whitespace handling — all through hand-built XML fed through
//! the same reader helpers.

use super::*;
use crate::fixture::MINIMAL_DOCX;

/// Parse a `<w:document>` XML string straight through the reader, bypassing the
/// OPC layer. Lets us unit-test body/paragraph/table handling on tiny inputs.
fn doc_from_xml(document_xml: &str) -> Document {
    let root = parse_xml(document_xml).expect("test XML parses").root;
    let body = match root.get_child(Some(W_NS), "body") {
        Some(b) => b,
        None => return Document { blocks: Vec::new() },
    };
    Document {
        blocks: read_blocks(body),
    }
}

/// A `<w:document>` wrapper with the `w` namespace bound, around a body.
fn wrap(body_inner: &str) -> String {
    format!(
        "<w:document xmlns:w=\"{W_NS}\"><w:body>{body_inner}</w:body></w:document>"
    )
}

// ---------------------------------------------------------------------------
// End-to-end: the real .docx fixture
// ---------------------------------------------------------------------------

#[test]
fn opens_minimal_docx_end_to_end() {
    let doc = open_docx(MINIMAL_DOCX).expect("fixture opens");

    // Three blocks in order: para, para, table.
    assert_eq!(doc.blocks.len(), 3, "blocks: {:#?}", doc.blocks);

    // 1) "Hello, DOCX!"
    match &doc.blocks[0] {
        Block::Paragraph(p) => assert_eq!(p.text, "Hello, DOCX!"),
        other => panic!("block 0 should be a paragraph, got {other:?}"),
    }

    // 2) "Second paragraph." as two runs "Second " + "paragraph."
    match &doc.blocks[1] {
        Block::Paragraph(p) => {
            assert_eq!(p.runs.len(), 2, "expected two runs, got {:?}", p.runs);
            assert_eq!(p.runs[0].text, "Second ");
            assert_eq!(p.runs[1].text, "paragraph.");
            assert_eq!(p.text, "Second paragraph.");
        }
        other => panic!("block 1 should be a paragraph, got {other:?}"),
    }

    // 3) one-row table with cells "A1cell" / "B1cell".
    match &doc.blocks[2] {
        Block::Table(t) => {
            assert_eq!(t.rows.len(), 1, "one row");
            assert_eq!(t.rows[0].len(), 2, "two cells");
            assert_eq!(t.rows[0][0].text, "A1cell");
            assert_eq!(t.rows[0][1].text, "B1cell");
        }
        other => panic!("block 2 should be a table, got {other:?}"),
    }
}

#[test]
fn document_text_contains_everything() {
    let doc = open_docx(MINIMAL_DOCX).expect("fixture opens");
    let text = doc.text();
    for needle in ["Hello, DOCX!", "Second paragraph.", "A1cell", "B1cell"] {
        assert!(text.contains(needle), "text {text:?} missing {needle:?}");
    }
    // Paragraphs come before the table's cells, in document order.
    let hello = text.find("Hello, DOCX!").unwrap();
    let second = text.find("Second paragraph.").unwrap();
    let a1 = text.find("A1cell").unwrap();
    assert!(hello < second && second < a1, "wrong order in {text:?}");
}

#[test]
fn paragraphs_and_tables_iterators() {
    let doc = open_docx(MINIMAL_DOCX).expect("fixture opens");
    assert_eq!(doc.paragraphs().count(), 2);
    assert_eq!(doc.tables().count(), 1);
}

// ---------------------------------------------------------------------------
// Run joining, tabs, breaks
// ---------------------------------------------------------------------------

#[test]
fn joins_runs_without_inserting_separators() {
    let doc = doc_from_xml(&wrap(
        "<w:p><w:r><w:t xml:space=\"preserve\">Second </w:t></w:r>\
         <w:r><w:t>paragraph.</w:t></w:r></w:p>",
    ));
    let p = doc.paragraphs().next().unwrap();
    assert_eq!(p.text, "Second paragraph.");
}

#[test]
fn tab_and_break_become_characters_in_order() {
    // A run with text, a tab, more text, a break, more text — order must hold.
    let doc = doc_from_xml(&wrap(
        "<w:p><w:r><w:t>a</w:t><w:tab/><w:t>b</w:t><w:br/><w:t>c</w:t></w:r></w:p>",
    ));
    let p = doc.paragraphs().next().unwrap();
    assert_eq!(p.text, "a\tb\nc");
    assert_eq!(p.runs.len(), 1);
    assert_eq!(p.runs[0].text, "a\tb\nc");
}

#[test]
fn run_properties_contribute_no_text() {
    // <w:rPr> (run properties, e.g. bold) must not leak into text.
    let doc = doc_from_xml(&wrap(
        "<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>bold</w:t></w:r></w:p>",
    ));
    assert_eq!(doc.paragraphs().next().unwrap().text, "bold");
}

#[test]
fn empty_paragraph_yields_empty_text() {
    let doc = doc_from_xml(&wrap("<w:p></w:p>"));
    let p = doc.paragraphs().next().unwrap();
    assert_eq!(p.text, "");
    assert!(p.runs.is_empty());
}

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

#[test]
fn table_multi_row_multi_cell_with_paragraphs() {
    let doc = doc_from_xml(&wrap(
        "<w:tbl>\
           <w:tr>\
             <w:tc><w:p><w:r><w:t>r1c1</w:t></w:r></w:p></w:tc>\
             <w:tc><w:p><w:r><w:t>r1c2</w:t></w:r></w:p></w:tc>\
           </w:tr>\
           <w:tr>\
             <w:tc><w:p><w:r><w:t>r2c1</w:t></w:r></w:p></w:tc>\
             <w:tc><w:p><w:r><w:t>r2c2</w:t></w:r></w:p></w:tc>\
           </w:tr>\
         </w:tbl>",
    ));
    let table = doc.tables().next().unwrap();
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0][0].text, "r1c1");
    assert_eq!(table.rows[0][1].text, "r1c2");
    assert_eq!(table.rows[1][0].text, "r2c1");
    assert_eq!(table.rows[1][1].text, "r2c2");
}

#[test]
fn cell_with_multiple_paragraphs_newline_joins() {
    let doc = doc_from_xml(&wrap(
        "<w:tbl><w:tr><w:tc>\
           <w:p><w:r><w:t>line one</w:t></w:r></w:p>\
           <w:p><w:r><w:t>line two</w:t></w:r></w:p>\
         </w:tc></w:tr></w:tbl>",
    ));
    let cell = &doc.tables().next().unwrap().rows[0][0];
    assert_eq!(cell.text, "line one\nline two");
    assert_eq!(cell.paragraphs.len(), 2);
}

#[test]
fn paragraph_text_does_not_leak_nested_table_text() {
    // A paragraph followed by a table: the paragraph's own text must not
    // include the table's cell text. (Guards against text_content() misuse.)
    let doc = doc_from_xml(&wrap(
        "<w:p><w:r><w:t>outer</w:t></w:r></w:p>\
         <w:tbl><w:tr><w:tc><w:p><w:r><w:t>inner</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
    ));
    let first_para = doc.paragraphs().next().unwrap();
    assert_eq!(first_para.text, "outer");
    assert!(!first_para.text.contains("inner"));
}

// ---------------------------------------------------------------------------
// Block ordering & sectPr
// ---------------------------------------------------------------------------

#[test]
fn sect_pr_is_ignored() {
    let doc = doc_from_xml(&wrap(
        "<w:p><w:r><w:t>only para</w:t></w:r></w:p>\
         <w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/></w:sectPr>",
    ));
    assert_eq!(doc.blocks.len(), 1);
    assert_eq!(doc.paragraphs().next().unwrap().text, "only para");
}

#[test]
fn unknown_block_elements_are_skipped() {
    let doc = doc_from_xml(&wrap(
        "<w:p><w:r><w:t>keep</w:t></w:r></w:p>\
         <w:sdt><w:sdtContent/></w:sdt>",
    ));
    assert_eq!(doc.blocks.len(), 1);
}

// ---------------------------------------------------------------------------
// Empty / whitespace / degenerate bodies
// ---------------------------------------------------------------------------

#[test]
fn empty_body_yields_no_blocks() {
    let doc = doc_from_xml(&wrap(""));
    assert!(doc.blocks.is_empty());
    assert_eq!(doc.text(), "");
}

#[test]
fn whitespace_only_body_yields_no_blocks() {
    let doc = doc_from_xml(&wrap("   \n\t  "));
    assert!(doc.blocks.is_empty());
    assert_eq!(doc.text(), "");
}

#[test]
fn document_without_body_yields_empty_document() {
    let doc = doc_from_xml(&format!("<w:document xmlns:w=\"{W_NS}\"/>"));
    assert!(doc.blocks.is_empty());
}

#[test]
fn trailing_whitespace_in_run_text_survives() {
    // The load-bearing case for our fixture: a run "Second " keeps its trailing
    // space, so joining two runs gives "Second paragraph." (no lost space). The
    // underlying xml-parser preserves trailing text-node whitespace verbatim.
    // (Note: it *does* trim leading whitespace of a text node — it does not
    // interpret `xml:space="preserve"` — but WordprocessingML runs that need a
    // space put it trailing, which is exactly what survives.)
    let doc = doc_from_xml(&wrap(
        "<w:p><w:r><w:t xml:space=\"preserve\">Second </w:t></w:r>\
         <w:r><w:t>word</w:t></w:r></w:p>",
    ));
    assert_eq!(doc.paragraphs().next().unwrap().text, "Second word");
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn non_docx_bytes_error() {
    // Random bytes are not a ZIP → Opc error.
    let err = open_docx(b"not a zip file at all").unwrap_err();
    assert!(matches!(err, DocxError::Opc(_)), "got {err:?}");
    // Display is non-empty and human-readable.
    assert!(!format!("{err}").is_empty());
}

#[test]
fn empty_bytes_error() {
    let err = open_docx(&[]).unwrap_err();
    assert!(matches!(err, DocxError::Opc(_)), "got {err:?}");
}

#[test]
fn error_display_and_from_opc() {
    // Exercise every Display arm + the From<OpcError> conversion path.
    let missing = DocxError::MissingDocument;
    assert!(format!("{missing}").contains("document"));
    let utf8 = DocxError::NotUtf8("/word/document.xml".into());
    assert!(format!("{utf8}").contains("UTF-8"));
    let bad = DocxError::MalformedXml("boom".into());
    assert!(format!("{bad}").contains("boom"));

    // From<OpcError>: build one via a bad open and confirm it wraps.
    let opc_err = Package::open(b"x").unwrap_err();
    let wrapped: DocxError = opc_err.into();
    assert!(matches!(wrapped, DocxError::Opc(_)));
}

#[test]
fn error_is_std_error() {
    fn assert_error<E: std::error::Error>(_: &E) {}
    assert_error(&DocxError::MissingDocument);
}
