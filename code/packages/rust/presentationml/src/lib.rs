//! # `coding-adventures-presentationml` — read a `.pptx` as per-slide text
//!
//! This is milestone **PML01** of the OOXML effort (see `code/specs/PML01`). It
//! takes the raw bytes of a `.pptx` (a PowerPoint presentation) and produces an
//! ordered [`Presentation`] → [`Slide`] → [`Shape`] model where each shape
//! carries its text and each slide can hand you its full text.
//!
//! It sits on the two lower OOXML layers — the exact same stack the sibling
//! [`spreadsheetml`](https://docs.rs/coding-adventures-spreadsheetml) crate uses:
//!
//! ```text
//! bytes → zip (M0) → xml-parser (M1) → opc (M2) → presentationml (HERE)
//! ```
//!
//! * The **`opc`** crate opens the ZIP, exposes named parts, and — crucially —
//!   resolves relationship ids (`r:id="rId1"`) to part names.
//! * The **`xml-parser`** crate parses a part's UTF-8 XML into a namespaced
//!   element tree with entity decoding already done.
//!
//! ## Two gotchas this crate resolves for you
//!
//! ### 1. `r:id` → part (which file *is* this slide?)
//!
//! `ppt/presentation.xml` lists slides by a relationship **id**, not a path:
//!
//! ```xml
//! <p:sldIdLst>
//!   <p:sldId id="256" r:id="rId1"/>
//!   <p:sldId id="257" r:id="rId2"/>
//! </p:sldIdLst>
//! ```
//!
//! `rId1` is dereferenced through a *separate* `.rels` file. The OPC layer does
//! that for us: [`Package::resolve`](coding_adventures_opc::Package::resolve)
//! turns `("/ppt/presentation.xml", "rId1")` into `"/ppt/slides/slide1.xml"`.
//! We read the `<p:sldId>` elements **in document order**, so the slide order in
//! our [`Presentation`] is exactly the order of the show.
//!
//! Note the namespace asymmetry on `<p:sldId>`: `id="256"` is unprefixed
//! (namespace `None`) while `r:id="rId1"` is in the **relationships** namespace
//! because it is written `r:id`. We read them with different namespaces.
//!
//! ### 2. Slide text lives in the **DrawingML** namespace, not PresentationML
//!
//! This is the subtle one. A slide's *structure* — the shape tree, the shapes —
//! is in the **PresentationML** namespace (prefix `p:`). But the actual **text**
//! is in the **DrawingML** namespace (prefix `a:`), a completely different URI.
//!
//! ```xml
//! <p:sp>                              <!-- p: shape           (PresentationML) -->
//!   <p:txBody>                        <!-- p: text body       (PresentationML) -->
//!     <a:p>                           <!-- a: PARAGRAPH       (DrawingML!)     -->
//!       <a:r><a:t>Slide One Title</a:t></a:r>  <!-- a: run + text (DrawingML!) -->
//!     </a:p>
//!   </p:txBody>
//! </p:sp>
//! ```
//!
//! If you look for `<a:t>` under the `p:` namespace you find **nothing** and
//! conclude the slide is empty. The boundary is exactly at `<p:txBody>`: it and
//! everything above is PresentationML; `<a:p>`, `<a:r>`, `<a:t>` are DrawingML.
//! We switch namespaces at that boundary.

use coding_adventures_opc::{OpcError, Package};
use coding_adventures_xml_parser::{parse_xml, XmlElement};

// ===========================================================================
// Namespace constants
// ===========================================================================

/// The PresentationML "main" namespace — the `p:` prefix. Every *structural*
/// element we care about (`presentation`, `sldIdLst`, `sldId`, `sld`, `cSld`,
/// `spTree`, `sp`, `txBody`) lives here.
const PML_NS: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";

/// The DrawingML "main" namespace — the `a:` prefix. This is where slide **text**
/// lives: paragraphs (`p`), runs (`r`), and text (`t`) are all DrawingML, *not*
/// PresentationML. This asymmetry is the crate's headline gotcha (see the module
/// docs). Note the `p` local name collides with PresentationML's `presentation`
/// prefix `p:` — but the *namespace URI* is different, and that is what we match.
const DML_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// The relationships namespace — the `r:` prefix on `<p:sldId r:id="rId1">`.
/// Note the asymmetry with `id` (unprefixed, namespace `None`): the numeric
/// `id="256"` is not namespaced, but `r:id` is.
const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// The logical part name of the presentation. `main_document_part()` yields this
/// for a real `.pptx`; we keep the constant for the (rare) fallback path where a
/// producer omitted the package-level relationship but the part still exists.
const PRESENTATION_PART: &str = "/ppt/presentation.xml";

// ===========================================================================
// Errors
// ===========================================================================

/// Everything that can go wrong opening a presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PptxError {
    /// The bytes were not a readable OPC package (not a ZIP, no content types,
    /// …). Wraps the underlying [`OpcError`].
    Opc(OpcError),
    /// The package opened but declared no main document part — i.e. it is not a
    /// presentation (`/ppt/presentation.xml` is neither the declared main part
    /// nor present).
    MissingPresentation,
    /// A part that had to be XML was not valid UTF-8. Carries the part name.
    NotUtf8(String),
    /// A part failed to parse as XML. Carries a human-readable message.
    MalformedXml(String),
    /// A `<p:sldId r:id="…">` did not resolve to any part, or the resolved part
    /// is absent from the package. Carries the r:id (or part name).
    MissingSlidePart(String),
}

impl std::fmt::Display for PptxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PptxError::Opc(e) => write!(f, "package error: {e}"),
            PptxError::MissingPresentation => {
                write!(f, "not a presentation: no /ppt/presentation.xml main part")
            }
            PptxError::NotUtf8(p) => write!(f, "part {p} is not valid UTF-8"),
            PptxError::MalformedXml(m) => write!(f, "malformed XML: {m}"),
            PptxError::MissingSlidePart(id) => {
                write!(f, "slide relationship {id} did not resolve to a part")
            }
        }
    }
}

impl std::error::Error for PptxError {}

impl From<OpcError> for PptxError {
    fn from(e: OpcError) -> Self {
        PptxError::Opc(e)
    }
}

// ===========================================================================
// The model
// ===========================================================================

/// One shape's text. A shape (`<p:sp>`) whose text body has several runs has
/// them joined here into a single string, run order preserved. A shape with no
/// text body, or an empty one, yields an empty [`text`](Shape::text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// The shape's text, its runs (`<a:r><a:t>`) concatenated in document order.
    pub text: String,
}

/// One slide: the shapes on it, in document (shape-tree) order.
#[derive(Debug, Clone)]
pub struct Slide {
    shapes: Vec<Shape>,
}

impl Slide {
    /// The shapes on this slide, in document order.
    pub fn shapes(&self) -> &[Shape] {
        &self.shapes
    }

    /// How many shapes this slide has (including text-less ones).
    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }

    /// The slide's full text: every shape's text joined by a newline.
    ///
    /// Shapes with empty text are skipped in the join, so the result never has a
    /// stray blank line for a decorative (text-less) shape. A slide with no text
    /// at all yields an empty string.
    pub fn text(&self) -> String {
        self.shapes
            .iter()
            .filter(|s| !s.text.is_empty())
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A whole presentation: its slides in show (`sldIdLst`) order.
#[derive(Debug, Clone)]
pub struct Presentation {
    slides: Vec<Slide>,
}

impl Presentation {
    /// The slides, in show order.
    pub fn slides(&self) -> &[Slide] {
        &self.slides
    }

    /// How many slides the presentation has.
    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }
}

// ===========================================================================
// Reading the presentation
// ===========================================================================

/// Open a `.pptx` from its bytes and read it into a [`Presentation`].
///
/// The pipeline (see the module docs for the *why*):
/// 1. Open the OPC package and locate the presentation part.
/// 2. Parse `<p:presentation>` → `<p:sldIdLst>` → each `<p:sldId>`, in order.
/// 3. For each `<p:sldId>`, resolve its `r:id` to a slide part, parse it, and
///    pull the text out of every shape's DrawingML text body.
pub fn open_pptx(bytes: &[u8]) -> Result<Presentation, PptxError> {
    let package = Package::open(bytes)?;

    // --- locate the presentation part -----------------------------------
    // main_document_part() follows the package-level /officeDocument
    // relationship. For a real .pptx this is "/ppt/presentation.xml". If a
    // producer omitted that relationship but the part exists, fall back.
    let presentation_part = match package.main_document_part() {
        Some(p) => p,
        None if package.has_part(PRESENTATION_PART) => PRESENTATION_PART.to_string(),
        None => return Err(PptxError::MissingPresentation),
    };

    let presentation_root = parse_part(&package, &presentation_part)?;

    // --- slide id list ---------------------------------------------------
    // <p:presentation><p:sldIdLst><p:sldId id="256" r:id="rId1"/>…
    // A presentation with no <p:sldIdLst> (or an empty one) is legal — it just
    // has no slides.
    let mut slides = Vec::new();
    if let Some(sld_id_lst) = presentation_root.get_child(Some(PML_NS), "sldIdLst") {
        for sld_id in sld_id_lst.get_children(Some(PML_NS), "sldId") {
            // r:id is prefixed → REL_NS. Without it we cannot find the bytes.
            let rid = sld_id
                .get_attr(Some(REL_NS), "id")
                .ok_or_else(|| PptxError::MissingSlidePart(String::new()))?;
            let slide_part = package
                .resolve(&presentation_part, rid)
                .ok_or_else(|| PptxError::MissingSlidePart(rid.to_string()))?;

            let shapes = read_slide(&package, &slide_part)?;
            slides.push(Slide { shapes });
        }
    }

    Ok(Presentation { slides })
}

/// Parse a package part as XML, returning its root element. Turns absent-part,
/// UTF-8, and parse failures into [`PptxError`].
fn parse_part(package: &Package, part: &str) -> Result<XmlElement, PptxError> {
    let bytes = package
        .read_part(part)
        .ok_or_else(|| PptxError::MissingSlidePart(part.to_string()))?;
    let text =
        std::str::from_utf8(bytes).map_err(|_| PptxError::NotUtf8(part.to_string()))?;
    let doc = parse_xml(text).map_err(|e| PptxError::MalformedXml(format!("{part}: {e:?}")))?;
    Ok(doc.root)
}

/// Read one slide part into its shapes.
///
/// Walk `<p:sld>` → `<p:cSld>` → `<p:spTree>` → each `<p:sp>`, and pull each
/// shape's text out of its DrawingML text body. All three container elements are
/// PresentationML; the text underneath is DrawingML (see [`shape_text`]).
fn read_slide(package: &Package, slide_part: &str) -> Result<Vec<Shape>, PptxError> {
    let root = parse_part(package, slide_part)?;
    let mut shapes = Vec::new();

    // <p:sld><p:cSld><p:spTree><p:sp>…</p:sp></p:spTree></p:cSld></p:sld>
    if let Some(c_sld) = root.get_child(Some(PML_NS), "cSld") {
        if let Some(sp_tree) = c_sld.get_child(Some(PML_NS), "spTree") {
            for sp in sp_tree.get_children(Some(PML_NS), "sp") {
                shapes.push(Shape {
                    text: shape_text(sp),
                });
            }
        }
    }

    Ok(shapes)
}

/// Extract a shape's text by concatenating its runs.
///
/// The namespace switch is the whole point here: `<p:txBody>` is PresentationML,
/// but its `<a:p>` paragraphs, `<a:r>` runs, and `<a:t>` text are **DrawingML**.
/// We join every `<a:t>` under the shape's text body, paragraph then run order,
/// with a paragraph break (`\n`) between paragraphs.
///
/// A shape with no `<p:txBody>` (e.g. a decorative rectangle) yields `""`.
fn shape_text(sp: &XmlElement) -> String {
    let tx_body = match sp.get_child(Some(PML_NS), "txBody") {
        Some(t) => t,
        None => return String::new(),
    };

    // Each <a:p> is a paragraph (DrawingML). Its text is the concatenation of
    // its runs' <a:t> text — which is exactly text_content() over the runs.
    let mut paragraphs = Vec::new();
    for para in tx_body.get_children(Some(DML_NS), "p") {
        let mut runs = String::new();
        for run in para.get_children(Some(DML_NS), "r") {
            if let Some(t) = run.get_child(Some(DML_NS), "t") {
                runs.push_str(&t.text_content());
            }
        }
        paragraphs.push(runs);
    }

    // Join paragraphs with a newline. We keep empty paragraphs (a blank line in
    // a text box is meaningful), but a text body with a single empty paragraph
    // collapses to "" so a truly empty box contributes nothing.
    let joined = paragraphs.join("\n");
    if joined.trim().is_empty() {
        String::new()
    } else {
        joined
    }
}

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;
