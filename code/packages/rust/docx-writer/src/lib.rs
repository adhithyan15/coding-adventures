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
const RELS_CONTENT_TYPE: &str = "application/vnd.openxmlformats-package.relationships+xml";

/// The content type registered as an `<Override>` for `/word/styles.xml` — the
/// part holding the style definitions the body's `<w:pStyle>` references resolve
/// against. Only emitted when a non-`Normal` paragraph style is used.
const STYLES_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml";

/// The relationship type linking `word/document.xml` to its `word/styles.xml`.
/// Word follows this from the body's own `.rels` to find the style definitions.
const STYLES_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";

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

/// A **run** — a maximal span of uniform character formatting within a
/// paragraph, the unit WordprocessingML formats at. `"a **bold** word"` is three
/// runs: `"a "`, `"bold"` (bold), `" word"`.
///
/// The three flags map to *direct* (style-free) formatting: `bold` → `<w:b/>`,
/// `italic` → `<w:i/>`, `mono` → a monospace `<w:rFonts>` (inline `code`). The
/// reader ignores them (it keeps only text); Word renders them. A run with no
/// flags emits no `<w:rPr>`, so unformatted output is byte-for-byte unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The run's logical text (escaped only at serialization time).
    pub text: String,
    /// Bold — `<w:b/>`.
    pub bold: bool,
    /// Italic — `<w:i/>`.
    pub italic: bool,
    /// Monospace — a Consolas `<w:rFonts>` override (inline code).
    pub mono: bool,
}

impl Run {
    /// An unformatted run carrying `text`.
    pub fn plain(text: &str) -> Self {
        Self {
            text: text.to_string(),
            bold: false,
            italic: false,
            mono: false,
        }
    }
    /// This run, made bold (chainable: `Run::plain("x").bold().italic()`).
    #[must_use]
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    /// This run, made italic.
    #[must_use]
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    /// This run, made monospace (inline code).
    #[must_use]
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
}

/// A paragraph's **style** — the semantic role Word renders it as. Anything but
/// [`Normal`](ParagraphStyle::Normal) emits a `<w:pStyle>` reference and pulls in
/// the package's `styles.xml` (see [`write_docx`]); `Normal` emits nothing, so a
/// document that uses no styles has no `styles.xml` at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphStyle {
    /// Body text — no `<w:pStyle>`.
    Normal,
    /// A heading; the `u8` is the level, clamped to `1..=6` (`Heading1`…`Heading6`).
    Heading(u8),
    /// A monospace code-block line (`Code`).
    Code,
    /// A block quotation (`Quote`).
    Quote,
    /// A list item's paragraph (`ListParagraph`) — indented; the bullet/number is
    /// carried in the run text by the caller.
    List,
}

impl ParagraphStyle {
    /// The `w:styleId` this style references, or `None` for [`Normal`]
    /// (which needs no `<w:pStyle>`). Heading levels are clamped to `1..=6`.
    fn style_id(self) -> Option<String> {
        match self {
            ParagraphStyle::Normal => None,
            ParagraphStyle::Heading(level) => Some(format!("Heading{}", level.clamp(1, 6))),
            ParagraphStyle::Code => Some("Code".to_string()),
            ParagraphStyle::Quote => Some("Quote".to_string()),
            ParagraphStyle::List => Some("ListParagraph".to_string()),
        }
    }
}

/// One block-level item in the body.
enum Block {
    /// A paragraph: a style + an ordered list of formatted runs. A single-run
    /// paragraph has one element; a multi-run paragraph has several, which the
    /// reader rejoins.
    Paragraph {
        style: ParagraphStyle,
        runs: Vec<Run>,
    },
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
        self.blocks.push(Block::Paragraph {
            style: ParagraphStyle::Normal,
            runs: vec![Run::plain(text)],
        });
    }

    /// Append a **multi-run** paragraph: one `<w:r>` per element of `runs`, in
    /// order. The reader concatenates them with no separator, so
    /// `add_paragraph_runs(&["Second ", "paragraph."])` reads back as the single
    /// string `"Second paragraph."`.
    ///
    /// An empty slice yields an empty paragraph (a `<w:p/>` with no runs), which
    /// is valid and reads back as empty text.
    pub fn add_paragraph_runs(&mut self, runs: &[&str]) {
        self.blocks.push(Block::Paragraph {
            style: ParagraphStyle::Normal,
            runs: runs.iter().map(|r| Run::plain(r)).collect(),
        });
    }

    /// Append a paragraph carrying a [`ParagraphStyle`] and a list of formatted
    /// [`Run`]s — the general form the plain helpers above sugar over. A heading,
    /// a code line, a quote, or a list item with bold/italic/mono spans all go
    /// through here.
    ///
    /// Using any non-[`Normal`](ParagraphStyle::Normal) style causes
    /// [`write_docx`] to emit a `styles.xml` part defining it (Word needs the
    /// definition to render the style); an all-`Normal` document has none.
    pub fn add_styled_paragraph(&mut self, style: ParagraphStyle, runs: Vec<Run>) {
        self.blocks.push(Block::Paragraph { style, runs });
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
            Block::Paragraph { style, runs } => push_paragraph(&mut xml, *style, runs),
            Block::Table(rows) => push_table(&mut xml, rows),
        }
    }

    xml.push_str("</w:body></w:document>");
    xml.into_bytes()
}

/// Append one `<w:p>` with an optional `<w:pStyle>` (for non-`Normal` styles) and
/// an ordered list of formatted runs.
///
/// The paragraph properties, when present, come first:
/// `<w:pPr><w:pStyle w:val="Heading1"/></w:pPr>`. A `Normal` paragraph emits no
/// `<w:pPr>`, so its bytes are unchanged from the text-only era.
fn push_paragraph(xml: &mut String, style: ParagraphStyle, runs: &[Run]) {
    xml.push_str("<w:p>");
    if let Some(id) = style.style_id() {
        xml.push_str("<w:pPr><w:pStyle w:val=\"");
        xml.push_str(&id); // style ids are our own fixed ASCII identifiers
        xml.push_str("\"/></w:pPr>");
    }
    for run in runs {
        push_run(xml, run);
    }
    xml.push_str("</w:p>");
}

/// Append one `<w:r>` for `run`: an optional `<w:rPr>` of direct formatting, then
/// `<w:t xml:space="preserve">…escaped…</w:t>`.
///
/// The `xml:space="preserve"` attribute stops an XML processor from collapsing
/// leading/trailing whitespace, so a run like `"Second "` keeps its trailing
/// space when the reader rejoins runs. An unformatted run emits no `<w:rPr>`.
fn push_run(xml: &mut String, run: &Run) {
    xml.push_str("<w:r>");
    if run.bold || run.italic || run.mono {
        xml.push_str("<w:rPr>");
        if run.bold {
            xml.push_str("<w:b/>");
        }
        if run.italic {
            xml.push_str("<w:i/>");
        }
        if run.mono {
            // A direct monospace font override — inline `code` without needing a
            // character style defined in styles.xml.
            xml.push_str("<w:rFonts w:ascii=\"Consolas\" w:hAnsi=\"Consolas\" w:cs=\"Consolas\"/>");
        }
        xml.push_str("</w:rPr>");
    }
    xml.push_str("<w:t xml:space=\"preserve\">");
    xml.push_str(&xml_escape(&run.text));
    xml.push_str("</w:t></w:r>");
}

/// Append one `<w:tbl>` built from rows of cell strings. Each cell is a `<w:tc>`
/// wrapping a single-run `Normal` paragraph — the minimal valid cell content.
fn push_table(xml: &mut String, rows: &[Vec<String>]) {
    xml.push_str("<w:tbl>");
    for row in rows {
        xml.push_str("<w:tr>");
        for cell in row {
            xml.push_str("<w:tc>");
            // A cell must contain at least one paragraph to be valid; we give it
            // exactly one single-run paragraph carrying the cell's text.
            push_paragraph(xml, ParagraphStyle::Normal, &[Run::plain(cell)]);
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

    // (4) Styles — ONLY when a paragraph uses a non-Normal style. A `<w:pStyle>`
    // reference renders as its style only if the package defines it, so we add a
    // `word/styles.xml` part plus the `word/_rels/document.xml.rels` relationship
    // that points the body at it. An all-Normal document skips this entirely, so
    // its bytes are byte-for-byte what the text-only writer produced.
    if uses_styles(doc) {
        let mut doc_rels = RelationshipsBuilder::new();
        doc_rels.add("rId1", STYLES_REL_TYPE, "styles.xml");
        pkg.add_part_defaulted("/word/_rels/document.xml.rels", &doc_rels.build());
        pkg.add_part(
            "/word/styles.xml",
            STYLES_CONTENT_TYPE,
            styles_xml().as_bytes(),
        );
    }

    pkg.finish()
}

/// Whether any block uses a non-`Normal` paragraph style — the condition for
/// emitting `styles.xml`.
fn uses_styles(doc: &Document) -> bool {
    doc.blocks
        .iter()
        .any(|b| matches!(b, Block::Paragraph { style, .. } if style.style_id().is_some()))
}

/// The fixed `word/styles.xml` defining every style MD02 uses: `Heading1`…
/// `Heading6` (bold, decreasing size, with an `<w:outlineLvl>` so Word's outline
/// / navigation pane works), `Code` (monospace), `Quote` (indented italic), and
/// `ListParagraph` (indented). Word resolves a `<w:pStyle w:val="Heading1"/>` in
/// the body against these definitions.
fn styles_xml() -> String {
    let mut s = String::new();
    s.push_str(XML_DECL);
    s.push_str("<w:styles xmlns:w=\"");
    s.push_str(W_NS);
    s.push_str("\">");
    // Heading1..6: half-point sizes 32,28,26,24,22,20 (16pt..10pt), bold, with an
    // outline level (0-based) so they populate the document outline.
    let heading_sizes = [32u32, 28, 26, 24, 22, 20];
    for (i, size) in heading_sizes.iter().enumerate() {
        let level = i + 1;
        s.push_str(&format!(
            "<w:style w:type=\"paragraph\" w:styleId=\"Heading{level}\">\
               <w:name w:val=\"heading {level}\"/>\
               <w:basedOn w:val=\"Normal\"/>\
               <w:pPr><w:outlineLvl w:val=\"{outline}\"/></w:pPr>\
               <w:rPr><w:b/><w:sz w:val=\"{size}\"/></w:rPr>\
             </w:style>",
            outline = i,
        ));
    }
    // Code: a monospace block style.
    s.push_str(
        "<w:style w:type=\"paragraph\" w:styleId=\"Code\">\
           <w:name w:val=\"Code\"/>\
           <w:basedOn w:val=\"Normal\"/>\
           <w:rPr><w:rFonts w:ascii=\"Consolas\" w:hAnsi=\"Consolas\" w:cs=\"Consolas\"/></w:rPr>\
         </w:style>",
    );
    // Quote: indented italic.
    s.push_str(
        "<w:style w:type=\"paragraph\" w:styleId=\"Quote\">\
           <w:name w:val=\"Quote\"/>\
           <w:basedOn w:val=\"Normal\"/>\
           <w:pPr><w:ind w:left=\"720\"/></w:pPr>\
           <w:rPr><w:i/></w:rPr>\
         </w:style>",
    );
    // ListParagraph: indented body text (the bullet/number lives in the text).
    s.push_str(
        "<w:style w:type=\"paragraph\" w:styleId=\"ListParagraph\">\
           <w:name w:val=\"List Paragraph\"/>\
           <w:basedOn w:val=\"Normal\"/>\
           <w:pPr><w:ind w:left=\"720\"/></w:pPr>\
         </w:style>",
    );
    s.push_str("</w:styles>");
    s
}

#[cfg(test)]
mod tests;
