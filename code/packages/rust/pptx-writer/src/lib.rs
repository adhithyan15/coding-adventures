//! # PresentationML (`.pptx`) *writer* — milestone **C3**
//!
//! This crate turns a tiny in-memory slide-deck model into the bytes of a real
//! `.pptx` file that strict consumers (PowerPoint, LibreOffice Impress,
//! python-pptx) will open. It is the presentation-side sibling of
//! [`xlsx-writer`](https://docs.rs/coding-adventures-xlsx-writer) (SpreadsheetML)
//! and is built on the format-agnostic
//! [`opc-writer`](https://docs.rs/coding-adventures-opc-writer): this crate knows
//! the *PresentationML vocabulary*, `opc-writer` knows *ZIP + content-types +
//! relationships*. See `code/specs/PPTXW01-pptx-writer.md` for the full write-up;
//! this module summarises the model inline so the source reads on its own.
//!
//! ## The one-paragraph mental model
//!
//! ```text
//!   Presentation / Slide          (this crate's model — what you build)
//!         │  write_pptx
//!         ▼
//!   PresentationML parts          (this crate: presentation.xml, slideN.xml, …)
//!         │  opc_writer::PackageWriter
//!         ▼
//!   OPC package                   (opc-writer: [Content_Types].xml, .rels, ZIP)
//!         ▼
//!   .pptx bytes                   ("PK\x03\x04…")
//! ```
//!
//! ```
//! use coding_adventures_pptx_writer::{Presentation, write_pptx};
//!
//! let mut p = Presentation::new();
//! let s = p.add_slide();
//! s.add_text("Slide One Title");
//! s.add_text("First slide body");
//!
//! let bytes = write_pptx(&p);
//! assert_eq!(&bytes[..2], b"PK");   // valid ZIP / OPC package bytes
//! ```
//!
//! ## Why a deck is *not* just "slides in a zip"
//!
//! A conformant `.pptx` needs a whole *scaffold* of parts wired by
//! relationships — `presentation → slide master → slide layout → theme` — because
//! every slide inherits its placeholders, colour map, and fonts from that chain
//! of parents. A package with only a `presentation.xml` and a `slide1.xml` is
//! rejected. So this writer always emits **one shared master, one shared layout,
//! and one shared theme** (constant boilerplate with no user data) plus one
//! `slideN.xml` per slide (the only part that carries text). The relationship
//! graph:
//!
//! ```text
//!   _rels/.rels ─rId1─▶ ppt/presentation.xml
//!                               │ rIdM ──────────────▶ slideMasters/slideMaster1.xml
//!                               │ rId1…rIdN ─▶ slides/slideN.xml ─rId1─▶ slideLayout1
//!   slideMaster1 ─rId1─▶ slideLayout1 ─rId1─▶ slideMaster1   (well-founded cycle)
//!   slideMaster1 ─rId2─▶ theme1
//! ```
//!
//! ## The `a:` namespace gotcha
//!
//! A slide element is `<p:sld>` in the **PresentationML** namespace, but the text
//! itself — paragraphs, runs, characters — lives in the **DrawingML** namespace
//! (prefix `a:`). Inside a shape's `<p:txBody>` you switch dialects:
//! `<a:p><a:r><a:t>…text…</a:t></a:r></a:p>`. Each [`Slide::add_text`] paragraph
//! becomes one such `a:p`. The text between `<a:t>` and `</a:t>` is the *only*
//! place user data enters the XML, so every paragraph is passed through
//! `opc_writer::xml_escape`; the scaffold parts hold no user data and are
//! emitted verbatim.

#![forbid(unsafe_code)]

use coding_adventures_opc_writer::{xml_escape, PackageWriter, RelationshipsBuilder};

// Re-export the escaper so callers (and downstream crates) have one place to
// reach for XML escaping, exactly as `opc-writer` intends.
pub use coding_adventures_opc_writer::xml_escape as escape_xml_text;

// ===========================================================================
// Namespaces & content types (ECMA-376 Part 1, PresentationML + DrawingML)
// ===========================================================================

/// PresentationML main namespace — the `p:` dialect (`p:presentation`, `p:sld`,
/// `p:sp`, `p:txBody`, …).
const P_NS: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";

/// DrawingML main namespace — the `a:` dialect. Slide *text* (paragraphs `a:p`,
/// runs `a:r`, text `a:t`) lives here, not in PresentationML. See the module
/// docs' "`a:` namespace gotcha".
const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// The relationships namespace prefix (`r:id` on slide-id and master-id lists).
const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// Content type of `ppt/presentation.xml` (the deck).
const CT_PRESENTATION: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";
/// Content type of each `ppt/slides/slideN.xml`.
const CT_SLIDE: &str = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
/// Content type of `ppt/slideLayouts/slideLayout1.xml`.
const CT_SLIDE_LAYOUT: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml";
/// Content type of `ppt/slideMasters/slideMaster1.xml`.
const CT_SLIDE_MASTER: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml";
/// Content type of `ppt/theme/theme1.xml`.
const CT_THEME: &str = "application/vnd.openxmlformats-officedocument.theme+xml";

// Relationship *type* URIs. Each `.rels` entry declares what kind of link it is;
// consumers navigate by type, never by hard-coded path.
const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const REL_SLIDE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";
const REL_SLIDE_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster";
const REL_SLIDE_LAYOUT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";
const REL_THEME: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";

/// The XML declaration real OOXML producers put at the head of every part.
const XML_DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n";

// Slide-size defaults: 9144000 × 6858000 EMU = the classic 10in × 7.5in 4:3
// slide. `notesSz` is the portrait 7.5in × 10in notes page. (1 inch = 914400
// English Metric Units.) These are the same constants PowerPoint writes.
const SLIDE_CX: u64 = 9_144_000;
const SLIDE_CY: u64 = 6_858_000;
const NOTES_CX: u64 = 6_858_000;
const NOTES_CY: u64 = 9_144_000;

// Conventional id bases (see the spec, §4 "Id conventions").
const SLIDE_ID_BASE: u64 = 256; // <p:sldId id="256"/>, "257", …
const SLIDE_MASTER_ID: u64 = 2_147_483_648; // <p:sldMasterId id="2147483648"/>
const SLIDE_LAYOUT_ID: u64 = 2_147_483_649; // <p:sldLayoutId id="2147483649"/>

// ===========================================================================
// The model
// ===========================================================================

/// One slide: an ordered list of text paragraphs.
///
/// A paragraph maps to a single `<a:p><a:r><a:t>…</a:t></a:r></a:p>` in the
/// slide's shape. For C3 that is the whole model of a slide — fonts, colours,
/// positioning, images, and notes are shared scaffold (identical for every
/// slide) and out of scope.
#[derive(Debug, Default, Clone)]
pub struct Slide {
    paragraphs: Vec<String>,
}

impl Slide {
    /// A new, empty slide (no text yet).
    fn new() -> Self {
        Self {
            paragraphs: Vec::new(),
        }
    }

    /// Append one paragraph / run of text to this slide.
    ///
    /// The text is stored verbatim and only escaped at serialization time (so a
    /// caller could, in principle, read it back). An empty string is allowed and
    /// produces an empty `<a:t/>`.
    pub fn add_text(&mut self, text: &str) {
        self.paragraphs.push(text.to_string());
    }

    /// The paragraphs of this slide, in order. Exposed so callers (and tests)
    /// can inspect the model without serializing it.
    pub fn paragraphs(&self) -> &[String] {
        &self.paragraphs
    }
}

/// A slide deck: an ordered list of [`Slide`]s.
///
/// Build one with [`Presentation::new`] + [`Presentation::add_slide`], then hand
/// it to [`write_pptx`] to get `.pptx` bytes.
#[derive(Debug, Default, Clone)]
pub struct Presentation {
    slides: Vec<Slide>,
}

impl Presentation {
    /// A new, empty presentation (no slides).
    ///
    /// An empty deck is still valid: [`write_pptx`] emits a `presentation.xml`
    /// with an empty `<p:sldIdLst/>` and the full master/layout/theme scaffold,
    /// so the package remains loadable.
    pub fn new() -> Self {
        Self { slides: Vec::new() }
    }

    /// Append a new empty slide and return a mutable handle to fill it.
    ///
    /// ```
    /// # use coding_adventures_pptx_writer::Presentation;
    /// let mut p = Presentation::new();
    /// p.add_slide().add_text("hello");
    /// assert_eq!(p.slides().len(), 1);
    /// ```
    pub fn add_slide(&mut self) -> &mut Slide {
        self.slides.push(Slide::new());
        // `push` never fails and we just pushed, so `last_mut` is `Some`; but we
        // avoid `unwrap` entirely (the module forbids panics on model paths) by
        // indexing the known-last position.
        let idx = self.slides.len() - 1;
        &mut self.slides[idx]
    }

    /// The slides of this deck, in order.
    pub fn slides(&self) -> &[Slide] {
        &self.slides
    }
}

// ===========================================================================
// Relationship-id helpers
// ===========================================================================
//
// Slides take rId1…rIdN in `presentation.xml.rels`; the master takes the next
// free id rId{N+1} so nothing collides. Centralising the arithmetic here keeps
// the two call sites (the .rels part and the sldMasterIdLst) in agreement.

/// The relationship id for slide number `n` (1-based) in `presentation.xml.rels`.
fn slide_rid(n: usize) -> String {
    format!("rId{n}")
}

/// The relationship id for the slide master in `presentation.xml.rels`: the
/// first id past the last slide.
fn master_rid(slide_count: usize) -> String {
    format!("rId{}", slide_count + 1)
}

// ===========================================================================
// Serialization — the slide part (the only part with user text)
// ===========================================================================

/// Serialize one slide (`ppt/slides/slideN.xml`).
///
/// The slide is a `<p:sld>` holding a single `<p:sp>` shape whose `<p:txBody>`
/// contains one DrawingML paragraph per [`Slide::add_text`] call. Note the
/// namespace switch: `p:` for the slide/shape structure, `a:` for the text.
fn slide_xml(slide: &Slide) -> Vec<u8> {
    let mut s = String::new();
    s.push_str(XML_DECL);
    // Bind a: (DrawingML), r: (relationships), p: (PresentationML) up front.
    s.push_str("<p:sld xmlns:a=\"");
    s.push_str(A_NS);
    s.push_str("\" xmlns:r=\"");
    s.push_str(R_NS);
    s.push_str("\" xmlns:p=\"");
    s.push_str(P_NS);
    s.push_str("\">");
    s.push_str("<p:cSld><p:spTree>");
    // The mandatory group-shape properties for the shape tree.
    s.push_str("<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>");
    s.push_str("<p:grpSpPr/>");
    // A single text shape holding all the paragraphs.
    s.push_str("<p:sp>");
    s.push_str("<p:nvSpPr><p:cNvPr id=\"2\" name=\"Title 1\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>");
    s.push_str("<p:spPr/>");
    s.push_str("<p:txBody><a:bodyPr/>");
    if slide.paragraphs.is_empty() {
        // A shape with no text still needs at least one (empty) paragraph to be
        // well-formed DrawingML.
        s.push_str("<a:p/>");
    } else {
        for para in &slide.paragraphs {
            // The ONE place user data enters the XML → escape it.
            s.push_str("<a:p><a:r><a:t>");
            s.push_str(&xml_escape(para));
            s.push_str("</a:t></a:r></a:p>");
        }
    }
    s.push_str("</p:txBody>");
    s.push_str("</p:sp>");
    s.push_str("</p:spTree></p:cSld>");
    s.push_str("<p:clrMapOvr><a:overrideClrMapping/></p:clrMapOvr>");
    s.push_str("</p:sld>");
    s.into_bytes()
}

// ===========================================================================
// Serialization — the deck part (presentation.xml)
// ===========================================================================

/// Serialize `ppt/presentation.xml`: the slide-master list, the slide list, and
/// the slide/notes sizes. Carries no user text — only ids.
fn presentation_xml(slide_count: usize) -> Vec<u8> {
    let mut s = String::new();
    s.push_str(XML_DECL);
    s.push_str("<p:presentation xmlns:p=\"");
    s.push_str(P_NS);
    s.push_str("\" xmlns:r=\"");
    s.push_str(R_NS);
    s.push_str("\">");

    // Slide-master list: one master, referenced by the id past the last slide.
    s.push_str("<p:sldMasterIdLst><p:sldMasterId id=\"");
    s.push_str(&SLIDE_MASTER_ID.to_string());
    s.push_str("\" r:id=\"");
    s.push_str(&master_rid(slide_count));
    s.push_str("\"/></p:sldMasterIdLst>");

    // Slide list: <p:sldId id="256+i" r:id="rId{i+1}"/> for each slide. An
    // empty deck emits an empty (self-closing) list, which is valid.
    if slide_count == 0 {
        s.push_str("<p:sldIdLst/>");
    } else {
        s.push_str("<p:sldIdLst>");
        for i in 0..slide_count {
            s.push_str("<p:sldId id=\"");
            s.push_str(&(SLIDE_ID_BASE + i as u64).to_string());
            s.push_str("\" r:id=\"");
            s.push_str(&slide_rid(i + 1));
            s.push_str("\"/>");
        }
        s.push_str("</p:sldIdLst>");
    }

    // Slide and notes page sizes.
    s.push_str("<p:sldSz cx=\"");
    s.push_str(&SLIDE_CX.to_string());
    s.push_str("\" cy=\"");
    s.push_str(&SLIDE_CY.to_string());
    s.push_str("\"/>");
    s.push_str("<p:notesSz cx=\"");
    s.push_str(&NOTES_CX.to_string());
    s.push_str("\" cy=\"");
    s.push_str(&NOTES_CY.to_string());
    s.push_str("\"/>");

    s.push_str("</p:presentation>");
    s.into_bytes()
}

// ===========================================================================
// Serialization — the scaffold parts (constant boilerplate, no user data)
// ===========================================================================

/// Serialize `ppt/slideLayouts/slideLayout1.xml`: a minimal blank layout with an
/// empty shape tree. Shared by every slide.
fn slide_layout_xml() -> Vec<u8> {
    let mut s = String::new();
    s.push_str(XML_DECL);
    s.push_str("<p:sldLayout xmlns:a=\"");
    s.push_str(A_NS);
    s.push_str("\" xmlns:r=\"");
    s.push_str(R_NS);
    s.push_str("\" xmlns:p=\"");
    s.push_str(P_NS);
    s.push_str("\" type=\"blank\" preserve=\"1\">");
    s.push_str("<p:cSld name=\"Blank\">");
    s.push_str(&empty_sp_tree());
    s.push_str("</p:cSld>");
    s.push_str("<p:clrMapOvr><a:overrideClrMapping/></p:clrMapOvr>");
    s.push_str("</p:sldLayout>");
    s.into_bytes()
}

/// Serialize `ppt/slideMasters/slideMaster1.xml`: an empty shape tree, the
/// 12-entry `<p:clrMap>`, and a `<p:sldLayoutIdLst>` pointing at the one layout.
fn slide_master_xml() -> Vec<u8> {
    let mut s = String::new();
    s.push_str(XML_DECL);
    s.push_str("<p:sldMaster xmlns:a=\"");
    s.push_str(A_NS);
    s.push_str("\" xmlns:r=\"");
    s.push_str(R_NS);
    s.push_str("\" xmlns:p=\"");
    s.push_str(P_NS);
    s.push_str("\">");
    s.push_str("<p:cSld>");
    s.push_str(&empty_sp_tree());
    s.push_str("</p:cSld>");
    // The colour map wires the six scheme slots (bg1/tx1/bg2/tx2) plus accents
    // and hyperlinks to theme colours. All 12 attributes are required.
    s.push_str(
        "<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" \
         accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" \
         accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" \
         hlink=\"hlink\" folHlink=\"folHlink\"/>",
    );
    // Layout list: rId1 → the one shared layout.
    s.push_str("<p:sldLayoutIdLst><p:sldLayoutId id=\"");
    s.push_str(&SLIDE_LAYOUT_ID.to_string());
    s.push_str("\" r:id=\"rId1\"/></p:sldLayoutIdLst>");
    s.push_str("</p:sldMaster>");
    s.into_bytes()
}

/// The empty group-shape tree shared by the master and layout parts. Named once
/// so both scaffold parts stay identical.
fn empty_sp_tree() -> String {
    let mut s = String::new();
    s.push_str("<p:spTree>");
    s.push_str("<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>");
    s.push_str("<p:grpSpPr/>");
    s.push_str("</p:spTree>");
    s
}

/// Serialize `ppt/theme/theme1.xml`: a minimal but schema-complete Office theme
/// (colour scheme, font scheme, and format scheme). Pure boilerplate — the
/// `srgbClr` values are arbitrary valid hex; the *structure* is what python-pptx
/// requires to load the deck.
fn theme_xml() -> Vec<u8> {
    let mut s = String::new();
    s.push_str(XML_DECL);
    s.push_str("<a:theme xmlns:a=\"");
    s.push_str(A_NS);
    s.push_str("\" name=\"Office Theme\">");
    s.push_str("<a:themeElements>");

    // --- Colour scheme: the 12 named colours the clrMap refers to. ---
    // dk1/lt1 use system window/windowText; the rest are plain sRGB.
    s.push_str("<a:clrScheme name=\"Office\">");
    s.push_str("<a:dk1><a:sysClr val=\"windowText\" lastClr=\"000000\"/></a:dk1>");
    s.push_str("<a:lt1><a:sysClr val=\"window\" lastClr=\"FFFFFF\"/></a:lt1>");
    s.push_str("<a:dk2><a:srgbClr val=\"44546A\"/></a:dk2>");
    s.push_str("<a:lt2><a:srgbClr val=\"E7E6E6\"/></a:lt2>");
    s.push_str("<a:accent1><a:srgbClr val=\"4472C4\"/></a:accent1>");
    s.push_str("<a:accent2><a:srgbClr val=\"ED7D31\"/></a:accent2>");
    s.push_str("<a:accent3><a:srgbClr val=\"A5A5A5\"/></a:accent3>");
    s.push_str("<a:accent4><a:srgbClr val=\"FFC000\"/></a:accent4>");
    s.push_str("<a:accent5><a:srgbClr val=\"5B9BD5\"/></a:accent5>");
    s.push_str("<a:accent6><a:srgbClr val=\"70AD47\"/></a:accent6>");
    s.push_str("<a:hlink><a:srgbClr val=\"0563C1\"/></a:hlink>");
    s.push_str("<a:folHlink><a:srgbClr val=\"954F72\"/></a:folHlink>");
    s.push_str("</a:clrScheme>");

    // --- Font scheme: a major (headings) and minor (body) font. ---
    s.push_str("<a:fontScheme name=\"Office\">");
    s.push_str(
        "<a:majorFont><a:latin typeface=\"Calibri Light\"/><a:ea typeface=\"\"/>\
         <a:cs typeface=\"\"/></a:majorFont>",
    );
    s.push_str(
        "<a:minorFont><a:latin typeface=\"Calibri\"/><a:ea typeface=\"\"/>\
         <a:cs typeface=\"\"/></a:minorFont>",
    );
    s.push_str("</a:fontScheme>");

    // --- Format scheme: fills, lines, effects, and background fills. The schema
    // requires >=2 fills/bgFills and exactly 3 lines/effects. ---
    s.push_str("<a:fmtScheme name=\"Office\">");
    // Two fills: a solid phClr and a second solid phClr.
    s.push_str("<a:fillStyleLst>");
    s.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
    s.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
    s.push_str("</a:fillStyleLst>");
    // Three line styles of increasing width, all solid phClr.
    s.push_str("<a:lnStyleLst>");
    for w in ["6350", "12700", "19050"] {
        s.push_str("<a:ln w=\"");
        s.push_str(w);
        s.push_str("\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\">");
        s.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
        s.push_str("<a:prstDash val=\"solid\"/></a:ln>");
    }
    s.push_str("</a:lnStyleLst>");
    // Three (empty) effect styles.
    s.push_str("<a:effectStyleLst>");
    for _ in 0..3 {
        s.push_str("<a:effectStyle><a:effectLst/></a:effectStyle>");
    }
    s.push_str("</a:effectStyleLst>");
    // Three background fills.
    s.push_str("<a:bgFillStyleLst>");
    for _ in 0..3 {
        s.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
    }
    s.push_str("</a:bgFillStyleLst>");
    s.push_str("</a:fmtScheme>");

    s.push_str("</a:themeElements>");
    s.push_str("</a:theme>");
    s.into_bytes()
}

// ===========================================================================
// The top-level entry point
// ===========================================================================

/// Serialize a [`Presentation`] into the bytes of a valid `.pptx` file.
///
/// This assembles every part (content types + all relationships are synthesised
/// by `opc-writer`) and returns the ZIP/OPC package bytes. It never panics: an
/// empty deck, empty slides, and arbitrary Unicode / XML-special text are all
/// handled.
///
/// ```
/// # use coding_adventures_pptx_writer::{Presentation, write_pptx};
/// let mut p = Presentation::new();
/// p.add_slide().add_text("Hello, deck!");
/// let bytes = write_pptx(&p);
/// assert_eq!(&bytes[..2], b"PK");
/// ```
pub fn write_pptx(p: &Presentation) -> Vec<u8> {
    let n = p.slides.len();
    let mut pkg = PackageWriter::new();

    // The two Defaults every OPC package needs.
    pkg.add_default(
        "rels",
        "application/vnd.openxmlformats-package.relationships+xml",
    );
    pkg.add_default("xml", "application/xml");

    // --- Package-root relationship: package → presentation. ---
    let mut root_rels = RelationshipsBuilder::new();
    root_rels.add("rId1", REL_OFFICE_DOCUMENT, "ppt/presentation.xml");
    pkg.add_part_defaulted("/_rels/.rels", &root_rels.build());

    // --- The deck. ---
    pkg.add_part(
        "/ppt/presentation.xml",
        CT_PRESENTATION,
        &presentation_xml(n),
    );

    // presentation.xml.rels: rId1…rIdN → slides, rId{N+1} → master. Targets are
    // relative to ppt/ (the .rels file's parent directory).
    let mut pres_rels = RelationshipsBuilder::new();
    for i in 1..=n {
        pres_rels.add(&slide_rid(i), REL_SLIDE, &format!("slides/slide{i}.xml"));
    }
    pres_rels.add(
        &master_rid(n),
        REL_SLIDE_MASTER,
        "slideMasters/slideMaster1.xml",
    );
    pkg.add_part_defaulted("/ppt/_rels/presentation.xml.rels", &pres_rels.build());

    // --- The slides, plus each slide's rels → the shared layout. ---
    for (i, slide) in p.slides.iter().enumerate() {
        let n1 = i + 1;
        pkg.add_part(
            &format!("/ppt/slides/slide{n1}.xml"),
            CT_SLIDE,
            &slide_xml(slide),
        );
        let mut slide_rels = RelationshipsBuilder::new();
        slide_rels.add(
            "rId1",
            REL_SLIDE_LAYOUT,
            "../slideLayouts/slideLayout1.xml",
        );
        pkg.add_part_defaulted(
            &format!("/ppt/slides/_rels/slide{n1}.xml.rels"),
            &slide_rels.build(),
        );
    }

    // --- The shared layout, plus its rels → the master. ---
    pkg.add_part(
        "/ppt/slideLayouts/slideLayout1.xml",
        CT_SLIDE_LAYOUT,
        &slide_layout_xml(),
    );
    let mut layout_rels = RelationshipsBuilder::new();
    layout_rels.add(
        "rId1",
        REL_SLIDE_MASTER,
        "../slideMasters/slideMaster1.xml",
    );
    pkg.add_part_defaulted(
        "/ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        &layout_rels.build(),
    );

    // --- The shared master, plus its rels → the layout and the theme. ---
    pkg.add_part(
        "/ppt/slideMasters/slideMaster1.xml",
        CT_SLIDE_MASTER,
        &slide_master_xml(),
    );
    let mut master_rels = RelationshipsBuilder::new();
    master_rels.add(
        "rId1",
        REL_SLIDE_LAYOUT,
        "../slideLayouts/slideLayout1.xml",
    );
    master_rels.add("rId2", REL_THEME, "../theme/theme1.xml");
    pkg.add_part_defaulted(
        "/ppt/slideMasters/_rels/slideMaster1.xml.rels",
        &master_rels.build(),
    );

    // --- The shared theme. ---
    pkg.add_part("/ppt/theme/theme1.xml", CT_THEME, &theme_xml());

    pkg.finish()
}

#[cfg(test)]
mod tests;
