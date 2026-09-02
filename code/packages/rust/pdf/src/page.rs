//! The page tree: catalogue, `Pages` node, and the pages themselves.
//!
//! A PDF's structure is a tree with exactly three levels that matter here:
//!
//! ```text
//!   /Type /Catalog        the document root; names the page tree
//!        |
//!   /Type /Pages          /Kids  [ page page … ]   /Count  n
//!        |
//!   /Type /Page           /Parent  /MediaBox  /Contents  /Resources
//! ```
//!
//! The links run **both ways**: `Pages` lists its kids, and every page names
//! its parent. That circularity is why the `Pages` object number has to be
//! reserved before any page can be written — a page cannot reference an object
//! that does not have a number yet. [`Document`] does the reserving, which is
//! the main reason it exists rather than leaving callers to assemble
//! dictionaries by hand.
//!
//! # Units
//!
//! Everything is in **points**: 72 to the inch. A4 is 595.276 × 841.89, US
//! Letter is 612 × 792. `MediaBox` is `[llx lly urx ury]`, and this module
//! always writes it anchored at the origin, so the height in the box is the
//! same number a top-down [`Content`](crate::Content) mirrors about.

use crate::{Dict, ObjId, Object, PdfError, PdfWriter};

/// US Letter, in points.
pub const LETTER: (f64, f64) = (612.0, 792.0);

/// ISO A4, in points.
pub const A4: (f64, f64) = (595.276, 841.89);

/// One of the fourteen fonts every PDF reader is required to have.
///
/// These need no embedding, which is what makes them usable before font
/// subsetting exists. They also cannot render Tamil, Devanagari or CJK — that
/// is what embedding is for, and why it is a separate piece of work rather
/// than a nicety.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandardFont {
    Helvetica,
    HelveticaBold,
    HelveticaOblique,
    TimesRoman,
    TimesBold,
    TimesItalic,
    Courier,
    CourierBold,
    Symbol,
    ZapfDingbats,
}

impl StandardFont {
    /// The `/BaseFont` name, spelled exactly as the specification requires.
    pub fn base_font(self) -> &'static str {
        match self {
            StandardFont::Helvetica => "Helvetica",
            StandardFont::HelveticaBold => "Helvetica-Bold",
            StandardFont::HelveticaOblique => "Helvetica-Oblique",
            StandardFont::TimesRoman => "Times-Roman",
            StandardFont::TimesBold => "Times-Bold",
            StandardFont::TimesItalic => "Times-Italic",
            StandardFont::Courier => "Courier",
            StandardFont::CourierBold => "Courier-Bold",
            StandardFont::Symbol => "Symbol",
            StandardFont::ZapfDingbats => "ZapfDingbats",
        }
    }

    fn dict(self) -> Dict {
        let mut dict = Dict::new();
        dict.set("Type", Object::name("Font"));
        dict.set("Subtype", Object::name("Type1"));
        dict.set("BaseFont", Object::name(self.base_font()));
        // Symbol and ZapfDingbats carry their own built-in encodings; naming
        // WinAnsi over the top of them would remap the glyphs to Latin text.
        if !matches!(self, StandardFont::Symbol | StandardFont::ZapfDingbats) {
            dict.set("Encoding", Object::name("WinAnsiEncoding"));
        }
        dict
    }
}

/// A font a page can reference.
///
/// The two kinds are genuinely different objects in the file -- a base-14
/// `/Type1` dictionary is four entries, while an embedded font is a graph of
/// five -- so they are distinguished here rather than behind one trait.
#[derive(Clone, Debug)]
pub enum FontResource {
    /// One of the fourteen fonts every reader already has.
    Standard(StandardFont),
    /// A TrueType font carried in the file, for scripts the base-14 cannot
    /// draw.
    Embedded(Box<crate::EmbeddedFont>),
}

/// A page, before it is written.
#[derive(Clone, Debug)]
pub struct Page {
    width: f64,
    height: f64,
    content: Vec<u8>,
    fonts: Vec<(String, FontResource)>,
}

impl Page {
    /// A page of the given size in points, carrying an already-built content
    /// stream.
    ///
    /// Takes the bytes rather than a [`Content`](crate::Content) so a caller
    /// can assemble a stream any way they like; [`Page::with_content`] is the
    /// path that keeps the coordinate space honest.
    pub fn new(width: f64, height: f64, content: Vec<u8>) -> Self {
        Self {
            width,
            height,
            content,
            fonts: Vec::new(),
        }
    }

    /// A page built from a [`Content`](crate::Content), checking that a
    /// top-down stream was mirrored about **this** page's height.
    ///
    /// The check is the point. A stream built for a 792-point page and placed
    /// on an 842-point one is off by 50 points everywhere: not obviously
    /// broken, just subtly wrong, and it looks like a layout mistake rather
    /// than a units mistake. Catching it here costs nothing.
    pub fn with_content(
        width: f64,
        height: f64,
        content: crate::Content,
    ) -> Result<Self, PdfError> {
        if let crate::Space::TopDown { page_height } = content.space() {
            if (page_height - height).abs() > 1e-6 {
                return Err(PdfError::Invalid(format!(
                    "content was built top-down for a page {page_height} points tall \
                     but is being placed on one {height} points tall; every \
                     coordinate would be off by {}",
                    (page_height - height).abs()
                )));
            }
        }
        Ok(Self::new(width, height, content.as_bytes().to_vec()))
    }

    /// Make a standard font available to this page under a resource name.
    ///
    /// The name is what `Tf` refers to (`F1`, say), not the typeface — the two
    /// are deliberately separate so a document can swap a face without
    /// rewriting its content streams.
    pub fn add_font(&mut self, name: impl Into<String>, font: StandardFont) -> &mut Self {
        self.fonts.push((name.into(), FontResource::Standard(font)));
        self
    }

    /// Embed a TrueType font under a resource name.
    ///
    /// Needed for anything the base-14 faces cannot draw, which is every
    /// script outside Latin, Greek and Cyrillic. Show its text with
    /// [`Content::show_glyphs`](crate::Content::show_glyphs) -- the encoding is
    /// `Identity-H`, so the content stream carries glyph ids rather than
    /// characters.
    pub fn add_embedded_font(
        &mut self,
        name: impl Into<String>,
        font: crate::EmbeddedFont,
    ) -> &mut Self {
        self.fonts
            .push((name.into(), FontResource::Embedded(Box::new(font))));
        self
    }

    /// The page's size in points.
    pub fn size(&self) -> (f64, f64) {
        (self.width, self.height)
    }
}

/// A document under construction: the writer plus the page tree.
#[derive(Debug)]
pub struct Document {
    writer: PdfWriter,
    pages_id: ObjId,
    kids: Vec<ObjId>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// An empty document with the `Pages` node reserved.
    ///
    /// Reserved rather than added: pages must name it as their parent, so its
    /// number has to exist before its contents can.
    pub fn new() -> Self {
        let mut writer = PdfWriter::new();
        let pages_id = writer.reserve();
        Self {
            writer,
            pages_id,
            kids: Vec::new(),
        }
    }

    /// Append a page, returning its object id.
    pub fn add_page(&mut self, page: Page) -> ObjId {
        let contents = self.writer.add(compress_content(page.content));

        let mut resources = Dict::new();
        if !page.fonts.is_empty() {
            let mut fonts = Dict::new();
            for (name, font) in &page.fonts {
                let id = match font {
                    FontResource::Standard(standard) => {
                        self.writer.add(Object::Dict(standard.dict()))
                    }
                    FontResource::Embedded(embedded) => embedded.write(&mut self.writer),
                };
                fonts.set(name.clone(), Object::Ref(id));
            }
            resources.set("Font", Object::Dict(fonts));
        }

        let mut dict = Dict::new();
        dict.set("Type", Object::name("Page"));
        dict.set("Parent", Object::Ref(self.pages_id));
        dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Int(0),
                Object::Int(0),
                Object::Real(page.width),
                Object::Real(page.height),
            ]),
        );
        dict.set("Contents", Object::Ref(contents));
        // An empty /Resources is legal and required: readers may treat its
        // absence as "inherit from the parent", and the parent has none.
        dict.set("Resources", Object::Dict(resources));

        let id = self.writer.add(Object::Dict(dict));
        self.kids.push(id);
        id
    }

    /// How many pages have been added.
    pub fn page_count(&self) -> usize {
        self.kids.len()
    }

    /// Serialise the document.
    ///
    /// Fills in the reserved `Pages` node last, once every kid is known, and
    /// writes the catalogue that points at it.
    pub fn finish(mut self) -> Result<Vec<u8>, PdfError> {
        if self.kids.is_empty() {
            return Err(PdfError::Invalid(
                "a PDF must have at least one page; readers reject a Pages node \
                 with an empty /Kids"
                    .to_string(),
            ));
        }

        let mut pages = Dict::new();
        pages.set("Type", Object::name("Pages"));
        pages.set(
            "Kids",
            Object::Array(self.kids.iter().copied().map(Object::Ref).collect()),
        );
        pages.set("Count", Object::Int(self.kids.len() as i64));
        self.writer.fill(self.pages_id, Object::Dict(pages));

        let mut catalog = Dict::new();
        catalog.set("Type", Object::name("Catalog"));
        catalog.set("Pages", Object::Ref(self.pages_id));
        let root = self.writer.add(Object::Dict(catalog));

        self.writer.finish(root)
    }
}

fn compress_content(data: Vec<u8>) -> Object {
    let (encoded, filter) = crate::flate_encode(&data);
    // `flate_encode` hands back the FILTER NAME, not a dictionary. Dropping it
    // produces a stream of zlib bytes with nothing saying they are compressed,
    // so a reader parses them as operators, finds nothing it recognises, and
    // renders a blank page -- without erroring, because an unparsable content
    // stream is not a structural fault.
    let mut dict = Dict::new();
    dict.set("Filter", filter);
    Object::Stream {
        dict,
        data: encoded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorTarget, Content, Paint};

    #[test]
    fn a_document_needs_at_least_one_page() {
        let err = Document::new().finish().unwrap_err();
        assert!(matches!(err, PdfError::Invalid(_)), "{err:?}");
    }

    #[test]
    fn pages_and_their_parent_reference_each_other() {
        let mut doc = Document::new();
        let mut content = Content::pdf_space();
        content.rect(0.0, 0.0, 10.0, 10.0).paint(Paint::Fill);
        doc.add_page(Page::new(LETTER.0, LETTER.1, content.as_bytes().to_vec()));
        let bytes = doc.finish().unwrap();
        let text = String::from_utf8_lossy(&bytes);

        assert!(text.contains("/Type /Pages"), "{text}");
        assert!(
            text.contains("/Type /Page\n") || text.contains("/Type /Page "),
            "{text}"
        );
        assert!(text.contains("/Count 1"), "{text}");
        assert!(text.contains("/Parent"), "{text}");
        assert!(text.contains("/Type /Catalog"), "{text}");
    }

    #[test]
    fn a_top_down_stream_is_rejected_by_a_page_of_a_different_height() {
        let content = Content::top_down(792.0);
        let err = Page::with_content(A4.0, A4.1, content).unwrap_err();
        // 841.89 - 792 = 49.89 points of silent offset, caught instead.
        assert!(format!("{err:?}").contains("off by"), "{err:?}");
    }

    #[test]
    fn a_matching_top_down_stream_is_accepted() {
        let content = Content::top_down(LETTER.1);
        assert!(Page::with_content(LETTER.0, LETTER.1, content).is_ok());
    }

    #[test]
    fn a_pdf_space_stream_is_accepted_by_any_page() {
        // Nothing was mirrored, so there is no height to disagree about.
        let content = Content::pdf_space();
        assert!(Page::with_content(A4.0, A4.1, content).is_ok());
    }

    #[test]
    fn fonts_become_named_page_resources() {
        let mut doc = Document::new();
        let mut content = Content::pdf_space();
        content
            .begin_text()
            .font("F1", 12.0)
            .text_position(72.0, 700.0)
            .show_text(b"hello")
            .end_text();
        let mut page = Page::new(LETTER.0, LETTER.1, content.as_bytes().to_vec());
        page.add_font("F1", StandardFont::Helvetica);
        doc.add_page(page);

        let bytes = doc.finish().unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Font"), "{text}");
        assert!(text.contains("/F1"), "{text}");
        assert!(text.contains("/BaseFont /Helvetica"), "{text}");
        assert!(text.contains("/WinAnsiEncoding"), "{text}");
    }

    #[test]
    fn symbol_fonts_keep_their_builtin_encoding() {
        // Naming WinAnsi over Symbol remaps its glyphs to Latin text, so the
        // page renders letters where it should show symbols.
        let text = format!("{:?}", StandardFont::Symbol.dict());
        assert!(!text.contains("WinAnsi"), "{text}");
        let helvetica = format!("{:?}", StandardFont::Helvetica.dict());
        assert!(helvetica.contains("WinAnsi"), "{helvetica}");
    }

    #[test]
    fn multiple_pages_are_all_listed_as_kids() {
        let mut doc = Document::new();
        for _ in 0..3 {
            let mut content = Content::pdf_space();
            content
                .gray(0.0, ColorTarget::Fill)
                .rect(0.0, 0.0, 5.0, 5.0)
                .paint(Paint::Fill);
            doc.add_page(Page::new(A4.0, A4.1, content.as_bytes().to_vec()));
        }
        assert_eq!(doc.page_count(), 3);
        let bytes = doc.finish().unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Count 3"), "{text}");
    }
}
