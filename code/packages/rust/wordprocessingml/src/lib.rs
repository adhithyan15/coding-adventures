//! # `coding-adventures-wordprocessingml` — read a `.docx` as extractable text
//!
//! This is milestone **W3** of the OOXML effort (see `code/specs/WML01`) — the
//! word-processing sibling of [`coding-adventures-spreadsheetml`]. It takes the
//! raw bytes of a `.docx` file and produces a [`Document`] → [`Block`] →
//! [`Paragraph`]/[`Table`] model, where each paragraph exposes its plain text
//! (and its individual [`Run`]s) and each table exposes its rows × [`Cell`]s.
//!
//! It sits directly on two lower layers that already did the hard plumbing:
//!
//! ```text
//! bytes → zip (M0) → xml-parser (M1) → opc (M2) → wordprocessingml (W3, HERE)
//! ```
//!
//! * The **`opc`** crate opens the ZIP, exposes named parts, and points us at
//!   the main document part (`/word/document.xml` for a `.docx`).
//! * The **`xml-parser`** crate parses that part's UTF-8 XML into a namespaced
//!   element tree with entity decoding already done.
//!
//! ## The paragraph → run → text model
//!
//! Unlike a spreadsheet — normalized with two indirections (`r:id` → part,
//! shared-string index → text) — a `.docx` body is *direct*: the text lives
//! inline where you read it. The only thing to learn is the **three-level
//! nesting** between "a paragraph" and "the characters in it".
//!
//! A word processor lets you bold *one word* mid-sentence. To record that,
//! WordprocessingML splits a paragraph's text into **runs** — maximal spans of
//! text sharing the same formatting. So
//!
//! > Second **paragraph**.
//!
//! is stored as *two* runs, `"Second "` and `"paragraph."`:
//!
//! ```xml
//! <w:p>
//!   <w:r><w:t xml:space="preserve">Second </w:t></w:r>
//!   <w:r><w:rPr><w:b/></w:rPr><w:t>paragraph.</w:t></w:r>
//! </w:p>
//! ```
//!
//! A paragraph is therefore **paragraph → runs → text**. We walk the runs in
//! order and concatenate each run's text; the split points are *formatting*
//! boundaries, not word boundaries, so joining the two runs above yields exactly
//! `"Second paragraph."` — the trailing space of `"Second "` survives (it is
//! guarded by `xml:space="preserve"`, which our xml-parser honours by preserving
//! text verbatim; we never trim run text).
//!
//! Two empty elements inside a run also contribute characters:
//! `<w:tab/>` → `\t` and `<w:br/>` → `\n`. They are siblings of `<w:t>` in
//! document order, so a run is really a small ordered sequence of
//! text | tab | break pieces, which we flatten into the run's string.
//!
//! ### Why not just `text_content()` on the paragraph?
//!
//! `text_content()` concatenates **all** descendant text. For a *simple*
//! paragraph that equals the run text — but a table cell contains paragraphs, so
//! `text_content()` on an outer element over-concatenates across structural
//! boundaries. We therefore walk `<w:r>` runs **explicitly** (and within each
//! run only `<w:t>`/`<w:tab>`/`<w:br>`), keeping paragraph text exact and table
//! text correctly scoped.

use coding_adventures_opc::{OpcError, Package};
use coding_adventures_xml_parser::{parse_xml, XmlElement, XmlNode};

// ===========================================================================
// Namespace constants
// ===========================================================================

/// The WordprocessingML "main" namespace. Every structural element we care
/// about (`document`, `body`, `p`, `r`, `t`, `tab`, `br`, `tbl`, `tr`, `tc`)
/// lives here, written with the `w:` prefix.
const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// The logical part name of the main document. `main_document_part()` yields
/// this for a `.docx`; we keep the constant for the (rare) fallback path where
/// a producer omitted the package relationship but the part still exists.
const DOCUMENT_PART: &str = "/word/document.xml";

// ===========================================================================
// Errors
// ===========================================================================

/// Everything that can go wrong opening a `.docx`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocxError {
    /// The bytes were not a readable OPC package (not a ZIP, no content types,
    /// …). Wraps the underlying [`OpcError`].
    Opc(OpcError),
    /// The package opened but declared no main document part — i.e. it is not a
    /// word-processing document (`/word/document.xml` is neither the declared
    /// main part nor present).
    MissingDocument,
    /// The document part was not valid UTF-8. Carries the part name.
    NotUtf8(String),
    /// The document part failed to parse as XML. Carries a human-readable
    /// message.
    MalformedXml(String),
}

impl std::fmt::Display for DocxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocxError::Opc(e) => write!(f, "package error: {e}"),
            DocxError::MissingDocument => {
                write!(f, "not a document: no /word/document.xml main part")
            }
            DocxError::NotUtf8(p) => write!(f, "part {p} is not valid UTF-8"),
            DocxError::MalformedXml(m) => write!(f, "malformed XML: {m}"),
        }
    }
}

impl std::error::Error for DocxError {}

impl From<OpcError> for DocxError {
    fn from(e: OpcError) -> Self {
        DocxError::Opc(e)
    }
}

// ===========================================================================
// The document model
// ===========================================================================

/// A single **run**: a maximal span of text with uniform formatting. At this
/// milestone we keep only its flattened [`text`](Run::text) (with `<w:tab>` →
/// `\t` and `<w:br>` → `\n` folded in); formatting is out of scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The run's text, with tabs and breaks flattened in document order.
    pub text: String,
}

/// A **paragraph**: an ordered list of runs plus their concatenation.
///
/// [`text`](Paragraph::text) is the join of every run's text (no separators —
/// run boundaries are formatting boundaries, not word boundaries), which is the
/// paragraph's plain-text content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    /// The paragraph's plain text: `runs` concatenated in order.
    pub text: String,
    /// The individual runs, in document order.
    pub runs: Vec<Run>,
}

/// One **table cell** (`<w:tc>`): its own paragraphs plus their newline-join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The cell's text: its paragraphs' text joined with `\n`.
    pub text: String,
    /// The cell's paragraphs, in document order.
    pub paragraphs: Vec<Paragraph>,
}

/// One table **row** (`<w:tr>`): an ordered list of cells.
pub type Row = Vec<Cell>;

/// A **table** (`<w:tbl>`): rows of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The rows, in document order; each row is its cells left-to-right.
    pub rows: Vec<Row>,
}

/// A **block-level** item in the body: either a paragraph or a table. The body
/// is a flat, ordered list of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A `<w:p>` paragraph.
    Paragraph(Paragraph),
    /// A `<w:tbl>` table.
    Table(Table),
}

/// A whole word-processing document: its body's blocks in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The body's block-level items, in document order.
    pub blocks: Vec<Block>,
}

impl Document {
    /// The whole document as plain text: every paragraph's text — including
    /// paragraphs nested inside table cells — in document order, joined with
    /// `\n`. This is the headline text-extraction output.
    ///
    /// Table cells within a row are also `\n`-separated: a table becomes one
    /// paragraph-per-cell run of lines, top-left to bottom-right. That is a
    /// deliberately simple, lossless-of-*text* flattening; callers wanting the
    /// grid structure should walk [`tables`](Document::tables) instead.
    pub fn text(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        for block in &self.blocks {
            match block {
                Block::Paragraph(p) => lines.push(p.text.clone()),
                Block::Table(t) => {
                    for row in &t.rows {
                        for cell in row {
                            // A cell's text may itself span multiple lines
                            // (multiple paragraphs); push it whole, it is
                            // already `\n`-joined.
                            lines.push(cell.text.clone());
                        }
                    }
                }
            }
        }
        lines.join("\n")
    }

    /// Iterate only the top-level paragraphs (skipping tables).
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        self.blocks.iter().filter_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            Block::Table(_) => None,
        })
    }

    /// Iterate only the tables (skipping paragraphs).
    pub fn tables(&self) -> impl Iterator<Item = &Table> {
        self.blocks.iter().filter_map(|b| match b {
            Block::Table(t) => Some(t),
            Block::Paragraph(_) => None,
        })
    }
}

// ===========================================================================
// Reading the document
// ===========================================================================

/// Open a `.docx` from its bytes and read it into a [`Document`].
///
/// The pipeline:
/// 1. Open the OPC package and locate the main document part.
/// 2. Parse `/word/document.xml` → `<w:document>` → `<w:body>`.
/// 3. Walk the body's block-level children into [`Block`]s.
pub fn open_docx(bytes: &[u8]) -> Result<Document, DocxError> {
    let package = Package::open(bytes)?;

    // --- locate the main document part ----------------------------------
    // main_document_part() follows the package-level /officeDocument
    // relationship. For a real .docx this is "/word/document.xml". If a
    // producer omitted that relationship but the part exists, fall back.
    let document_part = match package.main_document_part() {
        Some(p) => p,
        None if package.has_part(DOCUMENT_PART) => DOCUMENT_PART.to_string(),
        None => return Err(DocxError::MissingDocument),
    };

    let root = parse_part(&package, &document_part)?;

    // --- <w:document><w:body>…</w:body></w:document> --------------------
    // A document with no body is degenerate but not fatal: no blocks.
    let body = match root.get_child(Some(W_NS), "body") {
        Some(b) => b,
        None => return Ok(Document { blocks: Vec::new() }),
    };

    let blocks = read_blocks(body);
    Ok(Document { blocks })
}

/// Parse a package part as XML, returning its root element. Turns a missing
/// part, non-UTF-8 bytes, and parse failures into [`DocxError`].
fn parse_part(package: &Package, part: &str) -> Result<XmlElement, DocxError> {
    let bytes = package
        .read_part(part)
        .ok_or(DocxError::MissingDocument)?;
    let text = std::str::from_utf8(bytes).map_err(|_| DocxError::NotUtf8(part.to_string()))?;
    let doc = parse_xml(text).map_err(|e| DocxError::MalformedXml(format!("{part}: {e:?}")))?;
    Ok(doc.root)
}

/// Walk a `<w:body>`'s direct children into an ordered list of [`Block`]s.
///
/// Only `<w:p>` (paragraph) and `<w:tbl>` (table) carry content; every other
/// child — most importantly the trailing `<w:sectPr>` (section properties) — is
/// skipped. Unknown block-level elements are skipped too, for
/// forward-compatibility with producers that emit SDTs and the like.
fn read_blocks(body: &XmlElement) -> Vec<Block> {
    let mut blocks = Vec::new();
    for child in child_elements(body) {
        if child.namespace_uri.as_deref() != Some(W_NS) {
            continue;
        }
        match child.local_name.as_str() {
            "p" => blocks.push(Block::Paragraph(read_paragraph(child))),
            "tbl" => blocks.push(Block::Table(read_table(child))),
            _ => {} // sectPr and anything else: no content.
        }
    }
    blocks
}

/// Read one `<w:p>` into a [`Paragraph`].
///
/// We walk the paragraph's `<w:r>` runs explicitly (not `text_content()`, which
/// would over-concatenate across nested tables) and concatenate their text.
fn read_paragraph(p: &XmlElement) -> Paragraph {
    let mut runs = Vec::new();
    let mut text = String::new();
    for r in p.get_children(Some(W_NS), "r") {
        let run = read_run(r);
        text.push_str(&run.text);
        runs.push(run);
    }
    Paragraph { text, runs }
}

/// Read one `<w:r>` into a [`Run`], flattening its content in document order:
/// `<w:t>` contributes its (verbatim) text, `<w:tab>` a tab, `<w:br>` a newline.
///
/// We iterate the run's *direct children in order* so that a run like
/// `<w:t>a</w:t><w:tab/><w:t>b</w:t>` yields `"a\tb"`, not `"ab\t"`.
fn read_run(r: &XmlElement) -> Run {
    let mut text = String::new();
    for child in child_elements(r) {
        if child.namespace_uri.as_deref() != Some(W_NS) {
            continue;
        }
        match child.local_name.as_str() {
            // Text: verbatim. xml:space="preserve" needs no special handling
            // because the xml-parser preserves text nodes as-is.
            "t" => text.push_str(&child.text_content()),
            "tab" => text.push('\t'),
            "br" => text.push('\n'),
            _ => {} // rPr (run properties) and anything else: no text.
        }
    }
    Run { text }
}

/// Read one `<w:tbl>` into a [`Table`]: `<w:tr>` rows → `<w:tc>` cells.
fn read_table(tbl: &XmlElement) -> Table {
    let mut rows = Vec::new();
    for tr in tbl.get_children(Some(W_NS), "tr") {
        let mut row: Row = Vec::new();
        for tc in tr.get_children(Some(W_NS), "tc") {
            row.push(read_cell(tc));
        }
        rows.push(row);
    }
    Table { rows }
}

/// Read one `<w:tc>` cell into a [`Cell`]: its paragraphs plus their
/// newline-join. A cell may also *contain nested tables*; those are ignored for
/// the cell's `text` (we take only its direct `<w:p>` children), which keeps
/// cell text scoped to the cell's own prose.
fn read_cell(tc: &XmlElement) -> Cell {
    let mut paragraphs = Vec::new();
    for p in tc.get_children(Some(W_NS), "p") {
        paragraphs.push(read_paragraph(p));
    }
    let text = paragraphs
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Cell { text, paragraphs }
}

/// Iterator over just the child *elements* of a node (skipping text, comments,
/// processing instructions). The xml-parser exposes `children` as [`XmlNode`]s;
/// we need element order preserved, which `get_children` (name-filtered) would
/// lose across *different* tag names — hence this local helper.
fn child_elements(el: &XmlElement) -> impl Iterator<Item = &XmlElement> {
    el.children.iter().filter_map(|c| match c {
        XmlNode::Element(e) => Some(e.as_ref()),
        _ => None,
    })
}

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;
