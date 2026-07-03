//! # `coding-adventures-docx-writer` — build a valid `.docx` from a model
//!
//! Milestone **C2** of the OOXML effort (see `code/specs/DOCXW01-docx-writer.md`).
//! This is the word-processing *write* sibling of the spreadsheet `xlsx-writer`,
//! and the mirror image of the read-side [`wordprocessingml`] crate: where that
//! crate *opens* a `.docx`'s bytes into a `Document → Block → Paragraph/Table`
//! model, this crate *assembles* such a model back into a valid `.docx`.
//!
//! ## Where it sits
//!
//! ```text
//! Document model → docx-writer (C2, HERE) → opc-writer (C1) → zip → bytes
//! ```
//!
//! `docx-writer` knows exactly one thing: the **WordprocessingML** vocabulary —
//! how a document body is spelled as `<w:body>` full of `<w:p>` paragraphs (each
//! a run of `<w:r>` → `<w:t>` text) and `<w:tbl>` tables. Everything below that —
//! synthesizing `[Content_Types].xml`, wiring the package-root relationship,
//! DEFLATE-compressing each part into a ZIP — is delegated to the generic
//! [`coding_adventures_opc_writer`] packaging layer. We never touch ZIP bytes
//! ourselves.
//!
//! ## The part tree we emit
//!
//! ```text
//! example.docx  (a ZIP archive)
//! ├── [Content_Types].xml     ← media type of each part (synthesized by opc-writer)
//! ├── _rels/.rels             ← rId1 …/officeDocument → word/document.xml
//! └── word/document.xml       ← the body: paragraphs, runs, tables
//! ```
//!
//! The reader's `open_docx` follows the `/officeDocument` relationship in
//! `_rels/.rels` to *find* `word/document.xml`, so emitting that relationship is
//! precisely what makes our output openable by the reader — this is the pact the
//! round-trip test verifies.
//!
//! ## The paragraph → run → text model
//!
//! A word processor lets you bold one word mid-sentence. WordprocessingML records
//! that by splitting a paragraph's text into **runs** — maximal spans of uniform
//! formatting. "Second **paragraph**." is therefore two runs, `"Second "` and
//! `"paragraph."`. The reader concatenates runs with *no* separator, so they
//! rejoin to exactly `"Second paragraph."` — provided the trailing space of the
//! first run survives. That survival is guaranteed by `xml:space="preserve"` on
//! every `<w:t>`, which we always emit.
//!
//! ## Example
//!
//! ```
//! use coding_adventures_docx_writer::{Document, write_docx};
//!
//! let mut doc = Document::new();
//! doc.add_paragraph("Hello, DOCX!");
//! doc.add_paragraph_runs(&["Second ", "paragraph."]);
//! doc.add_table(&[vec!["A1cell".to_string(), "B1cell".to_string()]]);
//!
//! let bytes = write_docx(&doc);
//! assert_eq!(&bytes[..2], b"PK"); // a ZIP, hence an OPC package
//! ```

#![forbid(unsafe_code)]

use coding_adventures_opc_writer::{xml_escape, PackageWriter, RelationshipsBuilder};

// ===========================================================================
// Constants — the fixed vocabulary of a minimal `.docx`
// ===========================================================================

/// The WordprocessingML "main" namespace, bound to the `w:` prefix on
/// `<w:document>`. Every structural element (`body`, `p`, `r`, `t`, `tbl`, `tr`,
/// `tc`) lives here; the reader checks for exactly this URI.
const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// The content type registered as an `<Override>` for `/word/document.xml`. The
/// `.main+xml` suffix marks it as *the* WordprocessingML document part.
const DOCUMENT_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";

/// The relationship type URI for the package → main-document link. The reader's
/// `main_document_part()` locates the body by finding the relationship whose Type
/// ends with `/officeDocument`, so this exact string is load-bearing.
const OFFICE_DOCUMENT_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// Content type of every `.rels` part, registered as a `<Default>` for the
/// `rels` extension.
const RELS_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-package.relationships+xml";

/// Content type registered as the `<Default>` for the `xml` extension.
const XML_CONTENT_TYPE: &str = "application/xml";

/// The XML declaration OOXML producers put at the head of `document.xml`.
const XML_DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n";

// ===========================================================================
// The document model
// ===========================================================================
//
// This mirrors — deliberately simplified — the read-side model: a document is an
// ordered list of block-level items, each a paragraph (a list of run strings) or
// a table (rows of cell strings). We keep only *text*; formatting is out of
// scope at this milestone, exactly as the reader keeps only run text.

/// One block-level item in the body.
enum Block {
    /// A paragraph: an ordered list of run texts. A single-run paragraph has one
    /// element; a multi-run paragraph has several, which the reader rejoins.
    Paragraph(Vec<String>),
    /// A table: rows of cells, each cell a single string of text.
    Table(Vec<Vec<String>>),
}

/// A word-processing document under construction: its body's blocks in order.
///
/// Build one with [`new`](Document::new), push content with
/// [`add_paragraph`](Document::add_paragraph),
/// [`add_paragraph_runs`](Document::add_paragraph_runs), and
/// [`add_table`](Document::add_table), then serialize with [`write_docx`].
#[derive(Default)]
pub struct Document {
    blocks: Vec<Block>,
}

impl Document {
    /// A new, empty document. Serializing it yields a valid `.docx` with an empty
    /// body — degenerate but well-formed, never an error.
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    /// Append a **single-run** paragraph carrying `text`.
    ///
    /// The text is stored verbatim and escaped only at serialization time (so the
    /// model holds the *logical* string, not XML). `"a & b"` round-trips to
    /// `"a & b"`, not `"a &amp; b"`.
    pub fn add_paragraph(&mut self, text: &str) {
        self.blocks.push(Block::Paragraph(vec![text.to_string()]));
    }

    /// Append a **multi-run** paragraph: one `<w:r>` per element of `runs`, in
    /// order. The reader concatenates them with no separator, so
    /// `add_paragraph_runs(&["Second ", "paragraph."])` reads back as the single
    /// string `"Second paragraph."`.
    ///
    /// An empty slice yields an empty paragraph (a `<w:p/>` with no runs), which
    /// is valid and reads back as empty text.
    pub fn add_paragraph_runs(&mut self, runs: &[&str]) {
        self.blocks
            .push(Block::Paragraph(runs.iter().map(|r| r.to_string()).collect()));
    }

    /// Append a table: `rows` of cells, each cell a string of text. Each cell
    /// becomes a `<w:tc>` holding one single-run paragraph. An empty `rows`
    /// yields an empty `<w:tbl>`; a row with no cells yields an empty `<w:tr>` —
    /// both valid.
    pub fn add_table(&mut self, rows: &[Vec<String>]) {
        self.blocks.push(Block::Table(rows.to_vec()));
    }
}

// ===========================================================================
// Serialization: model → document.xml
// ===========================================================================

/// Serialize the whole document into the bytes of `word/document.xml`.
///
/// We build the XML by string concatenation — no allocation-per-node tree, just
/// a growing `String` — because the structure is small and fixed. Every scrap of
/// user text passes through [`xml_escape`]; nothing else is caller-controlled.
fn document_xml(doc: &Document) -> Vec<u8> {
    let mut xml = String::new();
    xml.push_str(XML_DECL);
    xml.push_str("<w:document xmlns:w=\"");
    xml.push_str(W_NS);
    xml.push_str("\"><w:body>");

    for block in &doc.blocks {
        match block {
            Block::Paragraph(runs) => push_paragraph(&mut xml, runs),
            Block::Table(rows) => push_table(&mut xml, rows),
        }
    }

    xml.push_str("</w:body></w:document>");
    xml.into_bytes()
}

/// Append one `<w:p>` built from an ordered list of run texts.
///
/// Each run is `<w:r><w:t xml:space="preserve">…escaped…</w:t></w:r>`. The
/// `xml:space="preserve"` attribute stops an XML processor from collapsing
/// leading/trailing whitespace, so a run like `"Second "` keeps its trailing
/// space when the reader rejoins runs.
fn push_paragraph(xml: &mut String, runs: &[String]) {
    xml.push_str("<w:p>");
    for run in runs {
        push_run(xml, run);
    }
    xml.push_str("</w:p>");
}

/// Append one `<w:r><w:t xml:space="preserve">…</w:t></w:r>` for `text`.
fn push_run(xml: &mut String, text: &str) {
    xml.push_str("<w:r><w:t xml:space=\"preserve\">");
    xml.push_str(&xml_escape(text));
    xml.push_str("</w:t></w:r>");
}

/// Append one `<w:tbl>` built from rows of cell strings. Each cell is a `<w:tc>`
/// wrapping a single-run paragraph — the minimal valid cell content.
fn push_table(xml: &mut String, rows: &[Vec<String>]) {
    xml.push_str("<w:tbl>");
    for row in rows {
        xml.push_str("<w:tr>");
        for cell in row {
            xml.push_str("<w:tc>");
            // A cell must contain at least one paragraph to be valid; we give it
            // exactly one single-run paragraph carrying the cell's text.
            push_paragraph(xml, std::slice::from_ref(cell));
            xml.push_str("</w:tc>");
        }
        xml.push_str("</w:tr>");
    }
    xml.push_str("</w:tbl>");
}

// ===========================================================================
// Packaging: document.xml → .docx bytes
// ===========================================================================

/// Serialize `doc` into the bytes of a complete, valid `.docx` file.
///
/// The three-step assembly:
/// 1. Register the two `<Default>` content types (`rels`, `xml`) every OPC
///    package needs.
/// 2. Add the package-root `_rels/.rels` relationship (`rId1` →
///    `word/document.xml`) so the reader can find the body.
/// 3. Add `word/document.xml` with its `<Override>` content type, then let
///    `opc-writer` synthesize `[Content_Types].xml` and ZIP everything up.
///
/// Total and panic-free: it performs pure in-memory computation over the model
/// and returns the resulting bytes.
pub fn write_docx(doc: &Document) -> Vec<u8> {
    let mut pkg = PackageWriter::new();

    // (1) The two Defaults every OPC package needs.
    pkg.add_default("rels", RELS_CONTENT_TYPE);
    pkg.add_default("xml", XML_CONTENT_TYPE);

    // (2) The package-root relationship: package → main document. The Target is
    // relative to `_rels/.rels`'s directory (the package root), hence no leading
    // slash. The reader keys off the Type suffix `/officeDocument`.
    let mut root_rels = RelationshipsBuilder::new();
    root_rels.add("rId1", OFFICE_DOCUMENT_REL_TYPE, "word/document.xml");
    pkg.add_part_defaulted("/_rels/.rels", &root_rels.build());

    // (3) The body, with an <Override> marking it as the main document part.
    let body = document_xml(doc);
    pkg.add_part("/word/document.xml", DOCUMENT_CONTENT_TYPE, &body);

    pkg.finish()
}

#[cfg(test)]
mod tests;
