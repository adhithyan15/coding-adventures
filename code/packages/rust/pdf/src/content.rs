//! Content streams: the operators that actually draw something.
//!
//! A page's dictionary says where the page is and how big; its **content
//! stream** says what is on it. The stream is a tiny postfix language —
//! operands first, then the operator:
//!
//! ```text
//!   1 0 0 RG          set the stroke colour to red
//!   72 720 m          move to (72, 720)
//!   300 720 l         line to (300, 720)
//!   S                 stroke the path
//! ```
//!
//! # The coordinate trap
//!
//! **PDF's origin is the bottom-left corner and y grows upward.** Screens,
//! box-layout trees and SVG all put the origin top-left with y growing
//! downward. Every renderer that emits PDF has to reconcile those, and doing it
//! ad hoc is how you get output that is perfectly plausible *upside down* —
//! text appears, paths appear, `qpdf --check` is happy, and the page is
//! mirrored. Structural validation cannot see it, because nothing is
//! structurally wrong.
//!
//! So the conversion happens **once**, in [`Content::point`], and every
//! coordinate in this module goes through it. A [`Content`] is built in one of
//! two spaces, chosen once at construction:
//!
//! ```text
//!   Space::Pdf                        Space::TopDown { page_height }
//!   y ^                               (0,0) ---> x
//!     |                                 |
//!     |                                 v
//!   (0,0) ---> x                        y
//! ```
//!
//! ## Why the flip is not a matrix
//!
//! The obvious implementation is to emit `1 0 0 -1 0 h cm` once and then use
//! top-down coordinates throughout. Do not: that matrix mirrors *everything*
//! drawn under it, so glyphs come out reversed. The text would be in the right
//! place and unreadable.
//!
//! Converting the points instead leaves the text matrix upright, which is why
//! [`Content::point`] returns a coordinate rather than installing a transform.
//! The one case that needs more than a point conversion is a rectangle, whose
//! PDF operand is its **bottom-left** corner — see [`Content::rect`].

use crate::{Dict, Object};

/// Which way is up.
///
/// Chosen once when the [`Content`] is created, so no drawing call has to
/// remember, and no call site can disagree with another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Space {
    /// PDF's own convention: origin bottom-left, y upward. Coordinates pass
    /// through untouched.
    Pdf,
    /// Origin top-left, y downward — the convention of screens, box trees and
    /// SVG. `page_height` is in points, and must match the page's `MediaBox`,
    /// since it is the mirror line.
    TopDown { page_height: f64 },
}

/// How a path should be painted once it has been built.
///
/// PDF distinguishes these with separate operators rather than a parameter,
/// and `n` (paint nothing) is not a no-op: it is how a path is used purely to
/// set a clip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    /// `S` — stroke the outline.
    Stroke,
    /// `f` — fill the interior (nonzero winding).
    Fill,
    /// `B` — fill and then stroke.
    FillStroke,
    /// `n` — paint nothing. Used with [`Content::clip`].
    None,
}

impl Paint {
    fn operator(self) -> &'static str {
        match self {
            Paint::Stroke => "S",
            Paint::Fill => "f",
            Paint::FillStroke => "B",
            Paint::None => "n",
        }
    }
}

/// A content stream under construction.
///
/// Operators are appended in order; nothing is reordered or optimised, so what
/// you call is what the page gets.
#[derive(Clone, Debug)]
pub struct Content {
    ops: Vec<u8>,
    space: Space,
}

impl Content {
    /// A stream in PDF's own coordinate space.
    pub fn pdf_space() -> Self {
        Self {
            ops: Vec::new(),
            space: Space::Pdf,
        }
    }

    /// A stream in top-down space, mirrored about `page_height`.
    ///
    /// `page_height` must be the page's `MediaBox` height in points, or every
    /// coordinate is offset by the difference — output that looks like a
    /// layout bug rather than a units bug.
    pub fn top_down(page_height: f64) -> Self {
        Self {
            ops: Vec::new(),
            space: Space::TopDown { page_height },
        }
    }

    /// The coordinate space this stream was built in.
    pub fn space(&self) -> Space {
        self.space
    }

    /// **The** conversion. Every coordinate in this module goes through here.
    ///
    /// Keeping it in one function is the point of the module: a sign error
    /// here fails every rendering test at once, where the same error spread
    /// across twenty call sites fails whichever ones nobody wrote a test for.
    fn point(&self, x: f64, y: f64) -> (f64, f64) {
        match self.space {
            Space::Pdf => (x, y),
            Space::TopDown { page_height } => (x, page_height - y),
        }
    }

    fn op(&mut self, operands: &[f64], operator: &str) -> &mut Self {
        for value in operands {
            self.ops
                .extend_from_slice(crate::format_real(*value).as_bytes());
            self.ops.push(b' ');
        }
        self.ops.extend_from_slice(operator.as_bytes());
        self.ops.push(b'\n');
        self
    }

    fn raw(&mut self, text: &str) -> &mut Self {
        self.ops.extend_from_slice(text.as_bytes());
        self.ops.push(b'\n');
        self
    }

    // ── Graphics state ───────────────────────────────────────────────────────

    /// `q` — push the graphics state.
    ///
    /// Colour, line width, transform and clip are all one stack. Anything set
    /// after a `q` is undone by the matching [`restore`](Self::restore), which
    /// is the only way to narrow a clip and then widen it again.
    pub fn save(&mut self) -> &mut Self {
        self.raw("q")
    }

    /// `Q` — pop the graphics state.
    pub fn restore(&mut self) -> &mut Self {
        self.raw("Q")
    }

    /// `cm` — concatenate a transformation matrix `[a b c d e f]`.
    ///
    /// Given in PDF's own terms deliberately: a matrix is not a point, so
    /// passing it through the top-down conversion would be meaningless. A
    /// caller wanting a top-down transform should compose it themselves, aware
    /// that a negative `d` mirrors any text drawn under it.
    pub fn transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> &mut Self {
        self.op(&[a, b, c, d, e, f], "cm")
    }

    /// `w` — line width, in points.
    pub fn line_width(&mut self, width: f64) -> &mut Self {
        self.op(&[width], "w")
    }

    /// `g` / `G` — greyscale fill or stroke colour, 0.0 black to 1.0 white.
    pub fn gray(&mut self, level: f64, target: ColorTarget) -> &mut Self {
        self.op(&[level], target.pick("g", "G"))
    }

    /// `rg` / `RG` — RGB colour, each component 0.0 to 1.0.
    pub fn rgb(&mut self, r: f64, g: f64, b: f64, target: ColorTarget) -> &mut Self {
        self.op(&[r, g, b], target.pick("rg", "RG"))
    }

    /// `k` / `K` — CMYK colour, each component 0.0 to 1.0.
    pub fn cmyk(&mut self, c: f64, m: f64, y: f64, k: f64, target: ColorTarget) -> &mut Self {
        self.op(&[c, m, y, k], target.pick("k", "K"))
    }

    // ── Paths ────────────────────────────────────────────────────────────────

    /// `m` — begin a new subpath at a point.
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        let (x, y) = self.point(x, y);
        self.op(&[x, y], "m")
    }

    /// `l` — straight line from the current point.
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        let (x, y) = self.point(x, y);
        self.op(&[x, y], "l")
    }

    /// `c` — cubic Bézier with two control points.
    pub fn curve_to(
        &mut self,
        c1x: f64,
        c1y: f64,
        c2x: f64,
        c2y: f64,
        x: f64,
        y: f64,
    ) -> &mut Self {
        let (c1x, c1y) = self.point(c1x, c1y);
        let (c2x, c2y) = self.point(c2x, c2y);
        let (x, y) = self.point(x, y);
        self.op(&[c1x, c1y, c2x, c2y, x, y], "c")
    }

    /// `h` — close the current subpath back to its start.
    pub fn close(&mut self) -> &mut Self {
        self.raw("h")
    }

    /// `re` — a rectangle, given here by its **top-left** corner in top-down
    /// space and its bottom-left corner in PDF space.
    ///
    /// This is the one shape a point conversion alone gets wrong. PDF's `re`
    /// takes the corner with the lowest x and y; mirroring the corner you were
    /// given lands on the corner *diagonally opposite* the one PDF wants, so
    /// the rectangle is drawn `height` points too high. Subtracting the height
    /// after the flip is what puts it back.
    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        // Routed through `point` like everything else, then moved down by the
        // height. Re-deriving the mirror here instead would make this the one
        // coordinate the module's single conversion does not cover -- and it
        // would keep working when that conversion was broken, which is exactly
        // the blind spot a second copy creates.
        let (x, y) = self.point(x, y);
        let y = match self.space {
            Space::Pdf => y,
            Space::TopDown { .. } => y - height,
        };
        self.op(&[x, y, width, height], "re")
    }

    /// Paint the path built so far.
    pub fn paint(&mut self, paint: Paint) -> &mut Self {
        self.raw(paint.operator())
    }

    /// `W` — use the current path as a clip.
    ///
    /// Must be followed by a paint operator, usually [`Paint::None`], because
    /// `W` marks the path for clipping and the paint operator is what ends it.
    pub fn clip(&mut self) -> &mut Self {
        self.raw("W")
    }

    // ── Text ─────────────────────────────────────────────────────────────────

    /// `BT` — begin a text object.
    ///
    /// Text position lives in its own matrix, reset by every `BT`, which is
    /// why positioning calls only make sense between `BT` and `ET`.
    pub fn begin_text(&mut self) -> &mut Self {
        self.raw("BT")
    }

    /// `ET` — end a text object.
    pub fn end_text(&mut self) -> &mut Self {
        self.raw("ET")
    }

    /// `Tf` — select a font by its resource name and a size in points.
    ///
    /// `name` is the key under `/Resources /Font`, not the typeface's name.
    pub fn font(&mut self, name: &str, size: f64) -> &mut Self {
        let escaped = crate::Object::name(name);
        let mut out = Vec::new();
        escaped.write(&mut out);
        self.ops.extend_from_slice(&out);
        self.ops.push(b' ');
        self.op(&[size], "Tf")
    }

    /// `Td` — move to the start of the next line, offset from the current line.
    ///
    /// A *relative* move, so the offset is a vector rather than a position and
    /// only its direction is mirrored, not its origin.
    pub fn text_offset(&mut self, dx: f64, dy: f64) -> &mut Self {
        let dy = match self.space {
            Space::Pdf => dy,
            Space::TopDown { .. } => -dy,
        };
        self.op(&[dx, dy], "Td")
    }

    /// `Tm` — set the text matrix, placing the next glyph's baseline origin.
    ///
    /// The identity scale is used, so this is a pure placement: the text is
    /// drawn upright at `(x, y)`, which is the whole reason the top-down space
    /// converts points rather than installing a mirroring `cm`.
    pub fn text_position(&mut self, x: f64, y: f64) -> &mut Self {
        let (x, y) = self.point(x, y);
        self.op(&[1.0, 0.0, 0.0, 1.0, x, y], "Tm")
    }

    /// `TL` — set the leading (baseline-to-baseline distance) for `T*`.
    pub fn leading(&mut self, leading: f64) -> &mut Self {
        self.op(&[leading], "TL")
    }

    /// `T*` — advance to the next line using the current leading.
    ///
    /// Always downward on the page in both spaces: PDF subtracts the leading
    /// from y, and in top-down terms that is "further down", so no conversion
    /// is needed and applying one would send text upward.
    pub fn next_line(&mut self) -> &mut Self {
        self.raw("T*")
    }

    /// `Tj` — show a string.
    ///
    /// The bytes are interpreted through the font's encoding, so for the
    /// base-14 fonts this is Latin-1 rather than UTF-8. Passing UTF-8 for
    /// anything above U+007F produces mojibake, not an error.
    pub fn show_text(&mut self, text: &[u8]) -> &mut Self {
        let mut out = Vec::new();
        Object::Str(text.to_vec()).write(&mut out);
        self.ops.extend_from_slice(&out);
        self.raw(" Tj")
    }

    /// `Tj` — show glyphs **by id**, for a font with `Identity-H` encoding.
    ///
    /// Under `Identity-H` a character code IS a glyph id, written as two
    /// big-endian bytes. So an embedded font must be shown this way and never
    /// with [`Content::show_text`]: passing a string would draw whatever
    /// glyphs happen to sit at those code points, which for a Tamil or CJK
    /// font is arbitrary.
    ///
    /// Written as a hex string, because glyph ids routinely contain bytes that
    /// a literal string would have to escape -- including the zero byte, which
    /// half of every id below 256 begins with.
    pub fn show_glyphs(&mut self, glyphs: &[u16]) -> &mut Self {
        let mut hex = Vec::with_capacity(glyphs.len() * 2);
        for glyph in glyphs {
            hex.extend_from_slice(&glyph.to_be_bytes());
        }
        let mut out = Vec::new();
        Object::HexStr(hex).write(&mut out);
        self.ops.extend_from_slice(&out);
        self.raw(" Tj")
    }

    /// `TJ` — show strings with kerning adjustments between them.
    ///
    /// Each adjustment is in thousandths of an em and moves the pen **back**,
    /// so a positive number tightens the gap. Getting that sign backwards
    /// spreads text out instead of kerning it.
    pub fn show_text_kerned(&mut self, runs: &[TextRun<'_>]) -> &mut Self {
        self.ops.push(b'[');
        for run in runs {
            match run {
                TextRun::Text(bytes) => {
                    let mut out = Vec::new();
                    Object::Str(bytes.to_vec()).write(&mut out);
                    self.ops.extend_from_slice(&out);
                }
                TextRun::Adjust(amount) => {
                    self.ops
                        .extend_from_slice(crate::format_real(*amount).as_bytes());
                }
            }
            self.ops.push(b' ');
        }
        self.raw("] TJ")
    }

    // ── Output ───────────────────────────────────────────────────────────────

    /// The operators as written, without a stream dictionary.
    pub fn as_bytes(&self) -> &[u8] {
        &self.ops
    }

    /// The stream object for this content, Flate-compressed.
    ///
    /// Content streams are mostly digits and spaces, so they compress well —
    /// a page of text is routinely a third of its plain size.
    pub fn into_stream(self) -> Object {
        let (data, filter) = crate::flate_encode(&self.ops);
        // The filter NAME, not a dictionary -- see `page::compress_content`.
        // Without `/Filter` the compressed bytes are read as operators and the
        // page renders blank, silently.
        let mut dict = Dict::new();
        dict.set("Filter", filter);
        Object::Stream { dict, data }
    }

    /// The stream object with no compression, for debugging.
    ///
    /// A `qpdf --check` failure is far easier to read when the operators are
    /// visible in the file.
    pub fn into_stream_uncompressed(self) -> Object {
        Object::Stream {
            dict: Dict::new(),
            data: self.ops,
        }
    }
}

/// Whether a colour applies to fills or strokes.
///
/// PDF spells the difference with letter case — `rg` fills, `RG` strokes —
/// which is easy to mistype and produces a shape coloured on the wrong side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorTarget {
    Fill,
    Stroke,
}

impl ColorTarget {
    fn pick(self, fill: &'static str, stroke: &'static str) -> &'static str {
        match self {
            ColorTarget::Fill => fill,
            ColorTarget::Stroke => stroke,
        }
    }
}

/// One element of a `TJ` array: literal text, or a kerning adjustment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextRun<'a> {
    Text(&'a [u8]),
    /// Thousandths of an em, moving the pen **backwards** when positive.
    Adjust(f64),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(content: &Content) -> String {
        String::from_utf8(content.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn pdf_space_passes_coordinates_through() {
        let mut c = Content::pdf_space();
        c.move_to(10.0, 20.0).line_to(30.0, 40.0);
        assert_eq!(rendered(&c), "10 20 m\n30 40 l\n");
    }

    #[test]
    fn top_down_mirrors_y_about_the_page_height() {
        let mut c = Content::top_down(800.0);
        // A point 20 from the top is 780 from the bottom.
        c.move_to(10.0, 20.0);
        assert_eq!(rendered(&c), "10 780 m\n");
    }

    #[test]
    fn a_top_down_rect_is_anchored_by_its_top_left_corner() {
        // The trap: mirroring the given corner alone lands on the corner
        // diagonally opposite the one `re` wants, drawing the rectangle
        // `height` points too high.
        let mut c = Content::top_down(800.0);
        c.rect(10.0, 100.0, 50.0, 30.0);
        // Top edge 100 from the top => bottom edge is 130 from the top,
        // i.e. 670 up from the bottom.
        assert_eq!(rendered(&c), "10 670 50 30 re\n");

        let mut plain = Content::pdf_space();
        plain.rect(10.0, 670.0, 50.0, 30.0);
        assert_eq!(rendered(&plain), "10 670 50 30 re\n");
    }

    #[test]
    fn text_position_places_an_upright_baseline() {
        let mut c = Content::top_down(800.0);
        c.text_position(72.0, 100.0);
        // Identity scale: no mirroring, so glyphs stay the right way up.
        assert_eq!(rendered(&c), "1 0 0 1 72 700 Tm\n");
    }

    #[test]
    fn a_relative_text_offset_flips_direction_but_not_origin() {
        let mut c = Content::top_down(800.0);
        // "20 further down the page" is -20 in PDF's upward y.
        c.text_offset(0.0, 20.0);
        assert_eq!(rendered(&c), "0 -20 Td\n");
    }

    #[test]
    fn next_line_needs_no_conversion_in_either_space() {
        // T* subtracts the leading from y, which is downward on the page in
        // both conventions. Converting it would send text upward.
        let mut top = Content::top_down(800.0);
        top.leading(14.0).next_line();
        let mut pdf = Content::pdf_space();
        pdf.leading(14.0).next_line();
        assert_eq!(rendered(&top), rendered(&pdf));
        assert_eq!(rendered(&top), "14 TL\nT*\n");
    }

    #[test]
    fn colour_operators_differ_by_case_for_fill_and_stroke() {
        let mut c = Content::pdf_space();
        c.rgb(1.0, 0.0, 0.0, ColorTarget::Fill)
            .rgb(0.0, 0.0, 1.0, ColorTarget::Stroke)
            .gray(0.5, ColorTarget::Fill)
            .cmyk(0.0, 0.0, 0.0, 1.0, ColorTarget::Stroke);
        assert_eq!(rendered(&c), "1 0 0 rg\n0 0 1 RG\n0.5 g\n0 0 0 1 K\n");
    }

    #[test]
    fn text_is_escaped_the_way_a_pdf_string_must_be() {
        let mut c = Content::pdf_space();
        c.show_text(b"a(b)c\\d");
        assert_eq!(rendered(&c), "(a\\(b\\)c\\\\d) Tj\n");
    }

    #[test]
    fn kerned_runs_write_a_tj_array() {
        let mut c = Content::pdf_space();
        c.show_text_kerned(&[
            TextRun::Text(b"A"),
            TextRun::Adjust(120.0),
            TextRun::Text(b"V"),
        ]);
        assert_eq!(rendered(&c), "[(A) 120 (V) ] TJ\n");
    }

    #[test]
    fn clipping_marks_the_path_and_paints_nothing() {
        let mut c = Content::pdf_space();
        c.save()
            .rect(0.0, 0.0, 100.0, 100.0)
            .clip()
            .paint(Paint::None)
            .restore();
        assert_eq!(rendered(&c), "q\n0 0 100 100 re\nW\nn\nQ\n");
    }
}
