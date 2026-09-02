//! # FNT02 — Glyph outlines from `glyf` and `loca`
//!
//! FNT00 (`font-parser`) answers *how wide is this glyph*. This crate answers
//! *what shape is it*: the actual contours, in font design units, ready for a
//! rasteriser or an SVG/PDF path.
//!
//! ## Why this exists
//!
//! Two lanes need it and neither can proceed without it:
//!
//! * **PDF font embedding.** Engram is a language-study app, so Tamil, Hindi,
//!   Japanese and Chinese decks are the normal case rather than an edge case,
//!   and none of that renders with PDF's base-14 fonts. Embedding a whole CJK
//!   font is megabytes; a study sheet uses a few hundred glyphs. Subsetting is
//!   the difference between a usable export and an unusable one, and you cannot
//!   subset what you cannot parse.
//! * **Maths typesetting.** A formula drawn as outlines renders identically on
//!   a machine that does not have the font installed. For a flashcard that is a
//!   correctness property, not a styling preference.
//!
//! ## The shape of a TrueType glyph
//!
//! A glyph is a list of closed contours. Each contour is a loop of points, and
//! each point is either **on** the curve or **off** it:
//!
//! ```text
//!      on ●━━━━━━━━● on          Two consecutive ON points → straight line.
//!
//!      on ●╌╌╌╌○╌╌╌╌● on         An OFF point between two ON points is the
//!               ↑                control point of a quadratic Bézier.
//!            off (control)
//!
//!      on ●╌╌○╌╌╌╌╌╌○╌╌● on      Two consecutive OFF points imply an ON point
//!            ↑   ●    ↑          exactly halfway between them. TrueType omits
//!          off  implied off      it to save bytes; we must put it back.
//! ```
//!
//! That implied midpoint is the first place a reader goes quietly wrong: skip
//! it and the outline still closes, still looks like a letter, and has subtly
//! wrong curvature everywhere two off-curve points meet — which in a rounded
//! typeface is nearly every curve.
//!
//! ## Where the bytes live
//!
//! ```text
//!   loca[gid]        →  where glyph `gid` starts inside `glyf`
//!   loca[gid + 1]    →  where it ends
//!
//!   loca[gid] == loca[gid + 1]  →  the glyph is EMPTY (a space), not an error
//! ```
//!
//! `loca` comes in two formats, chosen by `head.indexToLocFormat`: `u16`
//! entries storing *half* the real offset, or `u32` entries storing it
//! directly. Reading one as the other does not fail — it produces offsets that
//! land inside `glyf` and decode into a glyph-shaped thing that is not the
//! glyph. This is why the tests compare against fontTools rather than against
//! themselves.
//!
//! ## Composite glyphs
//!
//! `á` is not drawn twice. It is stored as "glyph `a`, plus glyph `acute`
//! shifted up and right". Roughly two thirds of Inter's 2,926 glyphs are
//! composites, so this is the common case, not an extra. We resolve them
//! recursively and hand back one flat outline; the caller never learns the
//! glyph was composite.
//!
//! ## Usage
//!
//! ```no_run
//! let bytes = std::fs::read("Inter-Regular.ttf").unwrap();
//! let font = font_parser::load(&bytes).unwrap();
//! let parser = glyph_parser::GlyphParser::new(&font).unwrap();
//!
//! if let Some(outline) = parser.glyph_outline(36).unwrap() {
//!     for contour in &outline.contours {
//!         for command in &contour.commands {
//!             println!("{command:?}");
//!         }
//!     }
//! }
//! ```

use font_parser::FontFile;

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Everything that can go wrong reading an outline.
///
/// These all mean "this font file is malformed" or "this font is not the kind
/// of font we parse". None of them is a condition a caller retries out of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlyphError {
    /// The font stores outlines in `CFF `/`CFF2` (PostScript), not `glyf`.
    /// A different format, not a broken one — worth its own variant so callers
    /// can fall back rather than report corruption.
    UnsupportedFontFormat,
    /// `glyph_id >= maxp.numGlyphs`.
    GlyphIndexOutOfRange(u16),
    /// Point count disagrees with the contour end-point array.
    MalformedContour,
    /// A composite record runs off the end of the glyph's bytes.
    MalformedComposite,
    /// Composites nested deeper than [`MAX_COMPOSITE_DEPTH`].
    CompositeDepthExceeded,
    /// One glyph tried to resolve more than [`MAX_COMPOSITE_COMPONENTS`]
    /// components in total.
    CompositeBudgetExceeded,
    /// A table this crate requires is absent.
    MissingTable(&'static str),
    /// A `REPEAT_FLAG` run extends past the end of the flag array.
    InvalidFlagRun,
    /// A read ran past the end of the buffer.
    BufferTooShort,
}

impl std::fmt::Display for GlyphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFontFormat => {
                write!(
                    f,
                    "font has no `glyf` outlines (CFF fonts are not supported)"
                )
            }
            Self::GlyphIndexOutOfRange(id) => write!(f, "glyph id {id} is out of range"),
            Self::MalformedContour => write!(f, "glyph points disagree with its contour ends"),
            Self::MalformedComposite => write!(f, "composite glyph record is truncated"),
            Self::CompositeDepthExceeded => {
                write!(
                    f,
                    "composite nesting deeper than {MAX_COMPOSITE_DEPTH} levels"
                )
            }
            Self::CompositeBudgetExceeded => write!(
                f,
                "glyph resolves more than {MAX_COMPOSITE_COMPONENTS} components"
            ),
            Self::MissingTable(name) => write!(f, "font is missing the `{name}` table"),
            Self::InvalidFlagRun => write!(f, "flag repeat run extends past the flag array"),
            Self::BufferTooShort => write!(f, "read past the end of the glyph data"),
        }
    }
}

impl std::error::Error for GlyphError {}

/// How deep composite references may nest before we refuse.
///
/// Real fonts reach two or three. The limit exists because a font is untrusted
/// input: a glyph that references itself would otherwise recurse until the
/// stack runs out, turning a malformed download into a crash.
pub const MAX_COMPOSITE_DEPTH: u8 = 10;

/// How many component references one glyph may resolve in total.
///
/// The depth limit alone does not bound the work: depth caps the *height* of
/// the tree and says nothing about its width. A glyph whose ten levels each
/// fan out to eight components is a few hundred bytes of font and
/// 8^10 -- over a billion -- component resolutions, which hangs the process
/// just as effectively as unbounded recursion, without ever exceeding depth 10.
///
/// So the budget counts the whole tree, not one path through it. Real
/// composites resolve a handful of components; four thousand is far beyond any
/// legitimate glyph and still returns instantly.
pub const MAX_COMPOSITE_COMPONENTS: u32 = 4096;

// ─────────────────────────────────────────────────────────────────────────────
// The outline types
// ─────────────────────────────────────────────────────────────────────────────

/// A glyph's design-space bounding box, copied from the glyph header.
///
/// TrueType does not require this to be tight, so treat it as a bound rather
/// than a measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoundingBox {
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
}

/// One drawing command.
///
/// There is no `CubicTo`: TrueType outlines are quadratic, and inventing a
/// cubic representation would mean every consumer converting back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    MoveTo { x: i16, y: i16 },
    LineTo { x: i16, y: i16 },
    QuadTo { cx: i16, cy: i16, x: i16, y: i16 },
}

/// One closed subpath.
///
/// Begins with exactly one `MoveTo`. The closing segment back to the start is
/// implicit — TrueType contours are always closed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contour {
    pub commands: Vec<Command>,
}

/// A glyph outline, flattened: composites have already been resolved.
///
/// All coordinates are in font design units, the same space as FNT00's
/// metrics. To draw at `N` pixels with `units_per_em = U`, scale by `N / U`.
/// That scaling belongs to the rasteriser, not here.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlyphOutline {
    pub glyph_id: u16,
    pub bounds: BoundingBox,
    pub contours: Vec<Contour>,
}

/// A decoded point, before it becomes drawing commands.
///
/// This is the representation composites are built in: transforming a
/// component means transforming its points, and only the final flattened point
/// set is turned into commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Point {
    x: i32,
    y: i32,
    on_curve: bool,
}

/// Decoded points plus where each contour ends. The intermediate form shared by
/// the simple and composite paths.
#[derive(Debug, Clone, Default)]
struct Points {
    points: Vec<Point>,
    /// Index of the LAST point of each contour, as stored in the font.
    end_pts: Vec<u16>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Big-endian reads
// ─────────────────────────────────────────────────────────────────────────────

fn read_u16(buf: &[u8], offset: usize) -> Result<u16, GlyphError> {
    let bytes = buf
        .get(offset..offset + 2)
        .ok_or(GlyphError::BufferTooShort)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_i16(buf: &[u8], offset: usize) -> Result<i16, GlyphError> {
    read_u16(buf, offset).map(|v| v as i16)
}

fn read_u32(buf: &[u8], offset: usize) -> Result<u32, GlyphError> {
    let bytes = buf
        .get(offset..offset + 4)
        .ok_or(GlyphError::BufferTooShort)?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

// ─────────────────────────────────────────────────────────────────────────────
// GlyphParser
// ─────────────────────────────────────────────────────────────────────────────

/// Reads glyph outlines out of a loaded font.
///
/// Borrows the [`FontFile`] rather than copying it: a CJK font is several
/// megabytes and there is no reason to hold two of them.
#[derive(Debug)]
pub struct GlyphParser<'a> {
    glyf: &'a [u8],
    loca: &'a [u8],
    loca_is_long: bool,
    num_glyphs: u16,
}

impl<'a> GlyphParser<'a> {
    /// Bind a parser to a font, checking up front that it can work at all.
    ///
    /// Every requirement is verified here rather than on first use, so a CFF
    /// font or a bitmap-only font is rejected at the point the caller can still
    /// do something about it.
    pub fn new(font: &'a FontFile) -> Result<Self, GlyphError> {
        let glyf = font
            .table(b"glyf")
            .ok_or(GlyphError::UnsupportedFontFormat)?;
        let loca = font
            .table(b"loca")
            .ok_or(GlyphError::MissingTable("loca"))?;

        // 0 = short (u16, halved), 1 = long (u32). Anything else is nonsense,
        // and guessing would silently produce wrong offsets, so refuse.
        let loca_is_long = match font
            .index_to_loc_format()
            .map_err(|_| GlyphError::MissingTable("head"))?
        {
            0 => false,
            1 => true,
            _ => return Err(GlyphError::MissingTable("head")),
        };

        let num_glyphs = font
            .num_glyphs()
            .map_err(|_| GlyphError::MissingTable("maxp"))?;

        Ok(Self {
            glyf,
            loca,
            loca_is_long,
            num_glyphs,
        })
    }

    /// How many glyphs the font has, from `maxp`.
    pub fn num_glyphs(&self) -> u16 {
        self.num_glyphs
    }

    /// The outline for one glyph.
    ///
    /// `Ok(None)` means the glyph is empty — a space, or any glyph with no
    /// contours. That is an ordinary thing for a font to contain, so it is not
    /// an error; callers draw nothing and carry on.
    pub fn glyph_outline(&self, glyph_id: u16) -> Result<Option<GlyphOutline>, GlyphError> {
        if glyph_id >= self.num_glyphs {
            return Err(GlyphError::GlyphIndexOutOfRange(glyph_id));
        }

        let Some(glyph_data) = self.glyph_data(glyph_id)? else {
            return Ok(None);
        };

        let bounds = BoundingBox {
            x_min: read_i16(glyph_data, 2)?,
            y_min: read_i16(glyph_data, 4)?,
            x_max: read_i16(glyph_data, 6)?,
            y_max: read_i16(glyph_data, 8)?,
        };

        let mut budget = MAX_COMPOSITE_COMPONENTS;
        let points = self.decode_points(glyph_id, 0, &mut budget)?;

        Ok(Some(GlyphOutline {
            glyph_id,
            bounds,
            contours: build_contours(&points)?,
        }))
    }

    /// Outlines for several glyphs.
    ///
    /// A convenience for the subsetting case, which asks for a few hundred at
    /// once. Errors are per glyph so one bad glyph does not lose the rest.
    pub fn glyph_outlines(&self, ids: &[u16]) -> Vec<Result<Option<GlyphOutline>, GlyphError>> {
        ids.iter().map(|&id| self.glyph_outline(id)).collect()
    }

    /// The raw bytes of one glyph, or `None` if the glyph is empty.
    ///
    /// ## The `loca` format trap
    ///
    /// Short entries store *half* the offset, because glyphs are 2-byte
    /// aligned and halving lets a `u16` address 128 KiB instead of 64 KiB.
    /// Forgetting the doubling reads real bytes from the wrong place — a
    /// failure that looks like a rendering bug, not a parsing bug.
    fn glyph_data(&self, glyph_id: u16) -> Result<Option<&'a [u8]>, GlyphError> {
        let index = glyph_id as usize;

        let (start, end) = if self.loca_is_long {
            (
                read_u32(self.loca, index * 4)? as usize,
                read_u32(self.loca, index * 4 + 4)? as usize,
            )
        } else {
            (
                read_u16(self.loca, index * 2)? as usize * 2,
                read_u16(self.loca, index * 2 + 2)? as usize * 2,
            )
        };

        // Equal offsets mean "no outline". Space is the obvious example, but
        // fonts also use it for unmapped ids in a sparse glyph range.
        if start >= end {
            return Ok(None);
        }

        // A glyph header alone is 10 bytes; anything shorter is truncated.
        let data = self
            .glyf
            .get(start..end)
            .ok_or(GlyphError::BufferTooShort)?;
        if data.len() < 10 {
            return Err(GlyphError::BufferTooShort);
        }
        Ok(Some(data))
    }

    /// Decode one glyph into points, resolving composites on the way.
    ///
    /// `depth` guards against a font whose glyph references itself.
    fn decode_points(
        &self,
        glyph_id: u16,
        depth: u8,
        budget: &mut u32,
    ) -> Result<Points, GlyphError> {
        if depth > MAX_COMPOSITE_DEPTH {
            return Err(GlyphError::CompositeDepthExceeded);
        }

        let Some(data) = self.glyph_data(glyph_id)? else {
            return Ok(Points::default());
        };

        let number_of_contours = read_i16(data, 0)?;
        if number_of_contours >= 0 {
            parse_simple_glyph(data, number_of_contours as usize)
        } else {
            self.parse_composite_glyph(data, depth, budget)
        }
    }

    /// Resolve a composite into the flattened points of its components.
    ///
    /// Each record names a component glyph, how to place it, and whether
    /// another record follows. We decode the component (recursively — a
    /// component may itself be composite), transform its points, and append.
    fn parse_composite_glyph(
        &self,
        data: &[u8],
        depth: u8,
        budget: &mut u32,
    ) -> Result<Points, GlyphError> {
        // Flag bits. Named rather than inlined because `flags & 0x0008` at the
        // use site is exactly how a reader ends up testing the wrong bit.
        const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
        const ARGS_ARE_XY_VALUES: u16 = 0x0002;
        const WE_HAVE_A_SCALE: u16 = 0x0008;
        const MORE_COMPONENTS: u16 = 0x0020;
        const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
        const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;

        let mut result = Points::default();
        let mut offset = 10; // past the glyph header

        loop {
            let flags = read_u16(data, offset)?;
            let component_id = read_u16(data, offset + 2)?;
            offset += 4;

            // The two arguments are either byte- or word-sized, and mean
            // either an offset or a pair of point indices.
            let (arg1, arg2) = if flags & ARG_1_AND_2_ARE_WORDS != 0 {
                let pair = (
                    read_i16(data, offset)? as i32,
                    read_i16(data, offset + 2)? as i32,
                );
                offset += 4;
                pair
            } else {
                let bytes = data
                    .get(offset..offset + 2)
                    .ok_or(GlyphError::MalformedComposite)?;
                offset += 2;
                // Point indices are UNSIGNED bytes; offsets are SIGNED. Reading
                // an anchor index as signed would make index 200 mean -56.
                if flags & ARGS_ARE_XY_VALUES != 0 {
                    (bytes[0] as i8 as i32, bytes[1] as i8 as i32)
                } else {
                    (bytes[0] as i32, bytes[1] as i32)
                }
            };

            // The 2x2 part of the transform, in F2Dot14: a signed 16-bit value
            // holding a number in [-2, 2) with 14 fractional bits.
            let (a, b, c, d) = if flags & WE_HAVE_A_SCALE != 0 {
                let scale = f2dot14(read_i16(data, offset)?);
                offset += 2;
                (scale, 0.0, 0.0, scale)
            } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                let x = f2dot14(read_i16(data, offset)?);
                let y = f2dot14(read_i16(data, offset + 2)?);
                offset += 4;
                (x, 0.0, 0.0, y)
            } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                let a = f2dot14(read_i16(data, offset)?);
                let b = f2dot14(read_i16(data, offset + 2)?);
                let c = f2dot14(read_i16(data, offset + 4)?);
                let d = f2dot14(read_i16(data, offset + 6)?);
                offset += 8;
                (a, b, c, d)
            } else {
                (1.0, 0.0, 0.0, 1.0)
            };

            // Charged before descending, so a wide tree exhausts the budget
            // rather than the stack or the clock.
            *budget = budget
                .checked_sub(1)
                .ok_or(GlyphError::CompositeBudgetExceeded)?;
            let component = self.decode_points(component_id, depth + 1, budget)?;

            // Placement. The common case is an explicit offset; the rare case
            // aligns a point of what we have so far with a point of the
            // component, which is why it has to happen after decoding it.
            let (dx, dy) = if flags & ARGS_ARE_XY_VALUES != 0 {
                (arg1, arg2)
            } else {
                let parent = result
                    .points
                    .get(arg1 as usize)
                    .ok_or(GlyphError::MalformedComposite)?;
                let child = component
                    .points
                    .get(arg2 as usize)
                    .ok_or(GlyphError::MalformedComposite)?;
                // The transform applies to the child point too, since it is the
                // transformed child that must land on the parent's point.
                let (tx, ty) = transform(child.x, child.y, a, b, c, d);
                (parent.x - tx, parent.y - ty)
            };

            let base = result.points.len() as u16;
            for point in &component.points {
                let (x, y) = transform(point.x, point.y, a, b, c, d);
                result.points.push(Point {
                    x: x + dx,
                    y: y + dy,
                    on_curve: point.on_curve,
                });
            }
            // Contour ends are indices into the point array, so appending a
            // component shifts all of its ends by however many points came
            // before it.
            for end in &component.end_pts {
                result.end_pts.push(end.saturating_add(base));
            }

            if flags & MORE_COMPONENTS == 0 {
                break;
            }
        }

        Ok(result)
    }
}

/// F2Dot14 → f64. A signed 16-bit fixed-point value with 14 fractional bits.
fn f2dot14(raw: i16) -> f64 {
    f64::from(raw) / 16384.0
}

/// Apply the 2x2 part of a component transform.
///
/// A scale turns integer coordinates into fractional ones (150 x 0.25 = 37.5),
/// so how the halves are rounded is part of the format's meaning, not a detail.
///
/// The rule is `floor(x + 0.5)` — round half UP — which is what OpenType
/// consumers use (`otRound` in fontTools). Rust's `f64::round` rounds half
/// *away from zero* instead, so the two agree on +37.5 and disagree on -37.5,
/// giving -37 and -38. A composite scaled onto negative coordinates is exactly
/// where that shows up, and the `halved` glyph in the synthetic fixture exists
/// to keep this pinned.
fn transform(x: i32, y: i32, a: f64, b: f64, c: f64, d: f64) -> (i32, i32) {
    let fx = f64::from(x);
    let fy = f64::from(y);
    (
        round_half_up(a * fx + c * fy),
        round_half_up(b * fx + d * fy),
    )
}

fn round_half_up(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

// ─────────────────────────────────────────────────────────────────────────────
// Simple glyphs
// ─────────────────────────────────────────────────────────────────────────────

/// Decode a simple (non-composite) glyph's points.
///
/// The body after the 10-byte header is five variable-length arrays, and each
/// one's length depends on the one before it:
///
/// ```text
///   endPtsOfContours[numberOfContours]   u16 each
///   instructionLength                    u16
///   instructions[instructionLength]      u8  each   (hinting; we skip it)
///   flags[numPoints]                     RUN-LENGTH ENCODED
///   xCoordinates[]                       1 or 2 bytes each, PER FLAG
///   yCoordinates[]                       1 or 2 bytes each, PER FLAG
/// ```
///
/// `numPoints` is not stored anywhere: it is `endPtsOfContours.last() + 1`.
/// And the coordinate arrays have no length of their own — you only know how
/// many bytes `xCoordinates` occupies by having decoded every flag first. Get
/// one flag wrong and every coordinate after it is read from the wrong offset,
/// which is why this is verified against an independent implementation.
fn parse_simple_glyph(data: &[u8], number_of_contours: usize) -> Result<Points, GlyphError> {
    const ON_CURVE_POINT: u8 = 0x01;
    const X_SHORT_VECTOR: u8 = 0x02;
    const Y_SHORT_VECTOR: u8 = 0x04;
    const REPEAT_FLAG: u8 = 0x08;
    // These two bits do double duty. With the matching SHORT bit set they mean
    // "this one-byte delta is positive"; without it they mean "this coordinate
    // is unchanged from the previous point, and occupies no bytes at all".
    const X_IS_SAME_OR_POSITIVE: u8 = 0x10;
    const Y_IS_SAME_OR_POSITIVE: u8 = 0x20;

    if number_of_contours == 0 {
        return Ok(Points::default());
    }

    let mut offset = 10;
    let mut end_pts = Vec::with_capacity(number_of_contours);
    for _ in 0..number_of_contours {
        end_pts.push(read_u16(data, offset)?);
        offset += 2;
    }

    // The point count comes from the last contour's end index. Contour ends
    // must be non-decreasing; if they are not, the glyph is malformed and any
    // count we derive is meaningless.
    let num_points = *end_pts.last().expect("checked non-empty above") as usize + 1;
    if end_pts.windows(2).any(|pair| pair[0] > pair[1]) {
        return Err(GlyphError::MalformedContour);
    }

    let instruction_length = read_u16(data, offset)? as usize;
    offset += 2 + instruction_length; // hinting bytecode: not our concern

    // ── flags, run-length decoded ────────────────────────────────────────────
    //
    // A flag with REPEAT_FLAG set is followed by a count byte saying how many
    // ADDITIONAL points share it. So `[0x37, 0x03]` means four points, not two.
    let mut flags = Vec::with_capacity(num_points);
    while flags.len() < num_points {
        let flag = *data.get(offset).ok_or(GlyphError::BufferTooShort)?;
        offset += 1;
        flags.push(flag);

        if flag & REPEAT_FLAG != 0 {
            let repeat = *data.get(offset).ok_or(GlyphError::BufferTooShort)?;
            offset += 1;
            for _ in 0..repeat {
                if flags.len() >= num_points {
                    // A run that would overrun the point count means the flag
                    // array and endPtsOfContours disagree about this glyph.
                    return Err(GlyphError::InvalidFlagRun);
                }
                flags.push(flag);
            }
        }
    }

    // ── coordinates, as deltas from the previous point ───────────────────────
    //
    // Both arrays are stored the same way and X comes first in its entirety,
    // so Y cannot be read until every X has been consumed.
    let mut xs = Vec::with_capacity(num_points);
    let mut accumulator = 0_i32;
    for &flag in &flags {
        accumulator += read_delta(
            data,
            &mut offset,
            flag & X_SHORT_VECTOR != 0,
            flag & X_IS_SAME_OR_POSITIVE != 0,
        )?;
        xs.push(accumulator);
    }

    let mut ys = Vec::with_capacity(num_points);
    accumulator = 0;
    for &flag in &flags {
        accumulator += read_delta(
            data,
            &mut offset,
            flag & Y_SHORT_VECTOR != 0,
            flag & Y_IS_SAME_OR_POSITIVE != 0,
        )?;
        ys.push(accumulator);
    }

    Ok(Points {
        points: (0..num_points)
            .map(|i| Point {
                x: xs[i],
                y: ys[i],
                on_curve: flags[i] & ON_CURVE_POINT != 0,
            })
            .collect(),
        end_pts,
    })
}

/// Read one coordinate delta, whose width and sign live in the flags.
///
/// | short | same/positive | meaning                          | bytes |
/// |-------|---------------|----------------------------------|-------|
/// | yes   | yes           | +u8                              | 1     |
/// | yes   | no            | -u8                              | 1     |
/// | no    | yes           | delta is ZERO, same as previous  | 0     |
/// | no    | no            | i16                              | 2     |
///
/// The third row is the one that catches people: the bit means "positive" only
/// when the short bit is set. Alone it means the coordinate repeats and
/// consumes no bytes, so treating it as a value desynchronises everything that
/// follows.
fn read_delta(
    data: &[u8],
    offset: &mut usize,
    is_short: bool,
    same_or_positive: bool,
) -> Result<i32, GlyphError> {
    if is_short {
        let magnitude = i32::from(*data.get(*offset).ok_or(GlyphError::BufferTooShort)?);
        *offset += 1;
        Ok(if same_or_positive {
            magnitude
        } else {
            -magnitude
        })
    } else if same_or_positive {
        Ok(0)
    } else {
        let delta = i32::from(read_i16(data, *offset)?);
        *offset += 2;
        Ok(delta)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Points → drawing commands
// ─────────────────────────────────────────────────────────────────────────────

/// Turn decoded points into closed contours of drawing commands.
///
/// This is where the implied on-curve midpoints get restored. Walking one
/// contour, for each point:
///
/// * **on-curve**, previous was on-curve → `LineTo`
/// * **on-curve**, previous was a control point → `QuadTo` ending here
/// * **off-curve**, previous was off-curve → the midpoint between them is an
///   implied on-curve point, so emit `QuadTo` to it and keep the new control
///
/// A contour may also *begin* off-curve, in which case the start point is the
/// midpoint between the first and last points — or, if every point is
/// off-curve (a circle drawn with four control points, which real fonts do
/// contain), the midpoint of the first and last.
fn build_contours(points: &Points) -> Result<Vec<Contour>, GlyphError> {
    let mut contours = Vec::with_capacity(points.end_pts.len());
    let mut start = 0_usize;

    for &end in &points.end_pts {
        let end = end as usize;
        if end >= points.points.len() || end < start {
            return Err(GlyphError::MalformedContour);
        }
        let slice = &points.points[start..=end];
        start = end + 1;

        if slice.is_empty() {
            continue;
        }
        contours.push(build_contour(slice));
    }

    Ok(contours)
}

fn midpoint(a: &Point, b: &Point) -> (i32, i32) {
    // Integer division truncates toward zero; TrueType's implied point is the
    // arithmetic mean, so use a floor-consistent halving of the sum.
    ((a.x + b.x).div_euclid(2), (a.y + b.y).div_euclid(2))
}

fn build_contour(slice: &[Point]) -> Contour {
    let mut commands = Vec::new();

    // Where does the pen start? Normally the first on-curve point. If the
    // contour begins off-curve, the true start is implied.
    let (start_x, start_y, first_index) = if slice[0].on_curve {
        (slice[0].x, slice[0].y, 1)
    } else if let Some(last) = slice.last().filter(|p| p.on_curve) {
        (last.x, last.y, 0)
    } else {
        let (mx, my) = midpoint(&slice[0], &slice[slice.len() - 1]);
        (mx, my, 0)
    };

    commands.push(Command::MoveTo {
        x: start_x as i16,
        y: start_y as i16,
    });

    // The pending control point, if the previous point was off-curve.
    let mut control: Option<(i32, i32)> = None;

    // Walk the contour once, then close back to the start.
    let count = slice.len();
    for step in 0..count {
        let index = (first_index + step) % count;
        // Stop before re-consuming the point we started on.
        if step >= count - first_index && first_index == 1 {
            break;
        }
        let point = &slice[index];

        match (point.on_curve, control) {
            (true, None) => commands.push(Command::LineTo {
                x: point.x as i16,
                y: point.y as i16,
            }),
            (true, Some((cx, cy))) => {
                commands.push(Command::QuadTo {
                    cx: cx as i16,
                    cy: cy as i16,
                    x: point.x as i16,
                    y: point.y as i16,
                });
                control = None;
            }
            (false, None) => control = Some((point.x, point.y)),
            (false, Some((cx, cy))) => {
                // Two controls in a row: the on-curve point between them was
                // omitted by the font and has to be put back.
                let (mx, my) = midpoint(
                    &Point {
                        x: cx,
                        y: cy,
                        on_curve: false,
                    },
                    point,
                );
                commands.push(Command::QuadTo {
                    cx: cx as i16,
                    cy: cy as i16,
                    x: mx as i16,
                    y: my as i16,
                });
                control = Some((point.x, point.y));
            }
        }
    }

    // Close the loop back to where the pen started.
    if let Some((cx, cy)) = control {
        commands.push(Command::QuadTo {
            cx: cx as i16,
            cy: cy as i16,
            x: start_x as i16,
            y: start_y as i16,
        });
    }

    Contour { commands }
}

#[cfg(test)]
mod oracle {
    //! Checked against fontTools, over every glyph in three real fonts.
    //!
    //! Nothing in `fixtures/oracle.txt` was produced by this crate. That is the
    //! entire point: `loca` in two formats, run-length flags, and deltas whose
    //! width and sign live in *other* flag bits are all places where a wrong
    //! stride yields a glyph-shaped thing that is not the glyph. A fixture
    //! written by the same hands as the parser would agree with it perfectly
    //! and be wrong in exactly the same way -- a shape this repository has now
    //! hit with zstd, the Anki importer, an unbuilt Vite project, and a PDF
    //! writer whose `FlateDecode` was raw deflate.
    //!
    //! Two levels, because they fail differently:
    //!
    //! * a digest per glyph, for **all 4,149 glyphs** -- coverage, so no glyph
    //!   is quietly unchecked, at the cost of only saying *which* one broke;
    //! * a full point-by-point record for a sampled ~40 per font -- diagnosis,
    //!   so a failure says what actually differs.
    //!
    //! Regenerate with `python3 code/scripts/generate_glyph_oracle.py`.

    use super::*;
    use std::path::PathBuf;

    const ORACLE: &str = include_str!("../tests/fixtures/oracle.txt");

    fn repo_root() -> PathBuf {
        // <repo>/code/packages/rust/glyph-parser
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
    }

    /// FNV-1a, mirroring `fnv1a` in the generator exactly.
    fn fnv1a(text: &str) -> String {
        let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
        for byte in text.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        format!("{hash:016x}")
    }

    /// The canonical encoding both sides agree to hash.
    ///
    /// Must match `canonical()` in the generator character for character; any
    /// divergence here shows up as every glyph failing at once, which is at
    /// least an unmistakable signal.
    fn canonical(record: &Record) -> String {
        let Some(record) = record.as_ref() else {
            return "EMPTY".to_string();
        };
        let bounds = format!(
            "{},{},{},{}",
            record.bounds.x_min, record.bounds.y_min, record.bounds.x_max, record.bounds.y_max
        );
        let ends = record
            .end_pts
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let points = record
            .points
            .iter()
            .map(|p| format!("{},{},{}", p.x, p.y, u8::from(p.on_curve)))
            .collect::<Vec<_>>()
            .join(";");
        format!("{bounds}|{ends}|{points}")
    }

    struct Decoded {
        bounds: BoundingBox,
        end_pts: Vec<u16>,
        points: Vec<Point>,
    }

    /// `None` is a glyph with no outline, which fontTools reports as
    /// `numberOfContours == 0` and we report as an empty `loca` range.
    type Record = Option<Decoded>;

    /// Decode a glyph to the stage the oracle describes: points, not commands.
    ///
    /// Deliberately stops short of `build_contours`. The oracle's job is the
    /// transcription -- offsets, flags, deltas, composite transforms -- and
    /// comparing at this stage means a mismatch points at the byte-level bug
    /// rather than at curve construction. Command emission is covered by the
    /// unit tests above, where the expected shapes can be written out by hand.
    fn decode(parser: &GlyphParser<'_>, glyph_id: u16) -> Result<Record, GlyphError> {
        let Some(data) = parser.glyph_data(glyph_id)? else {
            return Ok(None);
        };
        let mut budget = MAX_COMPOSITE_COMPONENTS;
        let points = parser.decode_points(glyph_id, 0, &mut budget)?;
        if points.points.is_empty() {
            return Ok(None);
        }
        Ok(Some(Decoded {
            bounds: BoundingBox {
                x_min: read_i16(data, 2)?,
                y_min: read_i16(data, 4)?,
                x_max: read_i16(data, 6)?,
                y_max: read_i16(data, 8)?,
            },
            end_pts: points.end_pts,
            points: points.points,
        }))
    }

    /// One font's expectations, as read out of the fixture.
    struct Expected {
        key: String,
        path: String,
        num_glyphs: u16,
        loca_format: i16,
        digests: Vec<String>,
        /// (glyph id, the full expected record as raw text)
        sampled: Vec<(u16, String)>,
    }

    fn parse_oracle() -> Vec<Expected> {
        let mut fonts: Vec<Expected> = Vec::new();
        for line in ORACLE.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            match parts.next() {
                Some("FONT") => {
                    let key = parts.next().unwrap().to_string();
                    let path = parts.next().unwrap().to_string();
                    fonts.push(Expected {
                        key,
                        path,
                        num_glyphs: parts.next().unwrap().parse().unwrap(),
                        loca_format: parts.next().unwrap().parse().unwrap(),
                        digests: Vec::new(),
                        sampled: Vec::new(),
                    });
                }
                Some("D") => {
                    let font = fonts.last_mut().expect("D line before any FONT line");
                    font.digests.push(parts.next().unwrap().to_string());
                }
                Some("G") => {
                    let font = fonts.last_mut().expect("G line before any FONT line");
                    let id: u16 = parts.next().unwrap().parse().unwrap();
                    let rest = parts.collect::<Vec<_>>().join(" ");
                    font.sampled.push((id, rest));
                }
                other => panic!("unrecognised oracle line: {other:?}"),
            }
        }
        assert!(!fonts.is_empty(), "oracle fixture parsed to nothing");
        fonts
    }

    /// Re-encode a decoded record in the fixture's `G` line form, so a failure
    /// prints the two side by side rather than a pair of digests.
    fn as_g_line(record: &Record) -> String {
        let Some(record) = record.as_ref() else {
            return "EMPTY".to_string();
        };
        let bounds = format!(
            "{} {} {} {}",
            record.bounds.x_min, record.bounds.y_min, record.bounds.x_max, record.bounds.y_max
        );
        let ends = record
            .end_pts
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let points = record
            .points
            .iter()
            .map(|p| format!("{},{},{}", p.x, p.y, u8::from(p.on_curve)))
            .collect::<Vec<_>>()
            .join(";");
        format!("{bounds} {ends} {points}")
    }

    #[test]
    fn every_glyph_matches_fonttools() {
        let root = repo_root();
        let mut total = 0_usize;

        for font_spec in parse_oracle() {
            let path = root.join(&font_spec.path);
            let bytes = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            let font = font_parser::load(&bytes).expect("font should load");
            let parser = GlyphParser::new(&font).expect("font should have glyf outlines");

            // If these disagree the glyph loop is meaningless -- every offset
            // would be read from the wrong table format.
            assert_eq!(
                parser.num_glyphs(),
                font_spec.num_glyphs,
                "{}: numGlyphs",
                font_spec.key
            );
            assert_eq!(
                font.index_to_loc_format().unwrap(),
                font_spec.loca_format,
                "{}: indexToLocFormat",
                font_spec.key
            );
            assert_eq!(
                font_spec.digests.len(),
                font_spec.num_glyphs as usize,
                "{}: oracle should carry one digest per glyph",
                font_spec.key
            );

            // Diagnosis: the sampled glyphs, compared in full.
            for (id, expected) in &font_spec.sampled {
                let record = decode(&parser, *id).expect("sampled glyph should decode");
                assert_eq!(
                    &as_g_line(&record),
                    expected,
                    "{}: glyph {id} differs from fontTools",
                    font_spec.key
                );
            }
            // Coverage: every glyph, not a sample.
            for (id, expected) in font_spec.digests.iter().enumerate() {
                let id = id as u16;
                let record = decode(&parser, id).unwrap_or_else(|e| {
                    panic!("{}: glyph {id} failed to decode: {e}", font_spec.key)
                });
                assert_eq!(
                    &fnv1a(&canonical(&record)),
                    expected,
                    "{}: glyph {id} decodes differently to fontTools",
                    font_spec.key
                );
                total += 1;
            }
        }

        // Guards against the fixture silently emptying out and the whole test
        // passing by checking nothing.
        assert!(
            total > 4000,
            "expected thousands of glyphs, checked {total}"
        );
        eprintln!("verified {total} glyphs against fontTools");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── The malformed-font paths ─────────────────────────────────────────────
    //
    // These fonts are built here rather than taken from the oracle, and that is
    // the right way round: the oracle exists to stop us *decoding* wrongly,
    // which needs an independent implementation to compare against. These
    // assert we REFUSE, and no shipping font contains a self-referential
    // composite to refuse -- fontTools would not survive reading one either.

    /// Assemble a font file around a set of tables.
    ///
    /// A real table directory, so `font-parser`'s lookup is exercised rather
    /// than bypassed: a test that hands the parser a pre-sliced `glyf` would
    /// not notice if the offset arithmetic that finds it were wrong.
    fn build_font(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let num_tables = tables.len() as u16;
        let mut font = Vec::new();
        font.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // sfntVersion
        font.extend_from_slice(&num_tables.to_be_bytes());
        font.extend_from_slice(&[0; 6]); // searchRange/entrySelector/rangeShift

        let mut offset = 12 + tables.len() * 16;
        let mut directory = Vec::new();
        let mut body = Vec::new();
        for (tag, data) in tables {
            directory.extend_from_slice(*tag);
            directory.extend_from_slice(&[0; 4]); // checksum: unchecked
            directory.extend_from_slice(&(offset as u32).to_be_bytes());
            directory.extend_from_slice(&(data.len() as u32).to_be_bytes());
            offset += data.len();
            body.extend_from_slice(data);
        }
        font.extend_from_slice(&directory);
        font.extend_from_slice(&body);
        font
    }

    /// The smallest font that loads, with `glyf`/`loca` supplied by the caller.
    fn font_with_glyf(glyf: Vec<u8>, loca: Vec<u8>, num_glyphs: u16) -> Vec<u8> {
        let mut head = vec![0u8; 54];
        head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes()); // magic
        head[18..20].copy_from_slice(&1000u16.to_be_bytes()); // unitsPerEm
        head[50..52].copy_from_slice(&0i16.to_be_bytes()); // indexToLocFormat: short

        let mut maxp = vec![0u8; 6];
        maxp[4..6].copy_from_slice(&num_glyphs.to_be_bytes());

        build_font(&[
            (b"cmap", vec![0; 4]),
            (b"glyf", glyf),
            (b"head", head),
            (b"hhea", vec![0; 36]),
            (b"hmtx", vec![0; 4]),
            (b"loca", loca),
            (b"maxp", maxp),
        ])
    }

    /// One composite glyph whose only component is `component_id`.
    fn composite_glyph(component_id: u16) -> Vec<u8> {
        let mut glyph = Vec::new();
        glyph.extend_from_slice(&(-1i16).to_be_bytes()); // numberOfContours < 0
        glyph.extend_from_slice(&[0; 8]); // bounding box
                                          // ARG_1_AND_2_ARE_WORDS | ARGS_ARE_XY_VALUES, and no MORE_COMPONENTS.
        glyph.extend_from_slice(&0x0003u16.to_be_bytes());
        glyph.extend_from_slice(&component_id.to_be_bytes());
        glyph.extend_from_slice(&0i16.to_be_bytes()); // dx
        glyph.extend_from_slice(&0i16.to_be_bytes()); // dy
        glyph
    }

    #[test]
    fn a_self_referential_composite_is_refused_rather_than_recursing() {
        // Glyph 0 is a composite whose component is glyph 0. Without the depth
        // guard this recurses until the stack runs out -- turning a malformed
        // download into a crash, which is the whole reason the guard exists.
        let glyph = composite_glyph(0);
        let loca = [0u16, (glyph.len() / 2) as u16]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect::<Vec<u8>>();
        let bytes = font_with_glyf(glyph, loca, 1);

        let font = font_parser::load(&bytes).expect("font should load");
        let parser = GlyphParser::new(&font).expect("font has glyf");
        assert_eq!(
            parser.glyph_outline(0).unwrap_err(),
            GlyphError::CompositeDepthExceeded
        );
    }

    /// A glyph with N components, for building a fan-out chain.
    fn composite_glyph_with(components: &[u16]) -> Vec<u8> {
        const MORE_COMPONENTS: u16 = 0x0020;
        let mut glyph = Vec::new();
        glyph.extend_from_slice(&(-1i16).to_be_bytes());
        glyph.extend_from_slice(&[0; 8]);
        for (i, id) in components.iter().enumerate() {
            let mut flags = 0x0003u16; // words + xy values
            if i + 1 < components.len() {
                flags |= MORE_COMPONENTS;
            }
            glyph.extend_from_slice(&flags.to_be_bytes());
            glyph.extend_from_slice(&id.to_be_bytes());
            glyph.extend_from_slice(&0i16.to_be_bytes());
            glyph.extend_from_slice(&0i16.to_be_bytes());
        }
        glyph
    }

    #[test]
    fn a_wide_composite_tree_is_refused_before_it_burns_the_cpu() {
        // The depth limit bounds the HEIGHT of the composite tree and says
        // nothing about its width. This font stays comfortably INSIDE the
        // depth limit -- nine levels, one less than the cap -- and is eight
        // wide at every level: 8^9, over 134 million component resolutions,
        // from a few hundred bytes. The depth guard never fires, so on its own
        // it waves this straight through and the process hangs.
        //
        // Staying under the cap is the whole point of the test. A chain that
        // tripped the depth limit would pass while proving nothing about
        // breadth.
        //
        // Each glyph N references glyph N+1 eight times; the last is a real
        // simple glyph so nothing fails for an unrelated reason.
        const LEVELS: u16 = 9;
        let mut glyphs: Vec<Vec<u8>> = Vec::new();
        for level in 0..LEVELS {
            glyphs.push(composite_glyph_with(&[level + 1; 8]));
        }
        // The leaf: a single on-curve point, so it decodes without error.
        let mut leaf = Vec::new();
        leaf.extend_from_slice(&1i16.to_be_bytes()); // one contour
        leaf.extend_from_slice(&[0; 8]); // bbox
        leaf.extend_from_slice(&0u16.to_be_bytes()); // endPts = [0]
        leaf.extend_from_slice(&0u16.to_be_bytes()); // no instructions
        leaf.push(0x01); // one flag: on-curve, long deltas
        leaf.extend_from_slice(&0i16.to_be_bytes()); // x
        leaf.extend_from_slice(&0i16.to_be_bytes()); // y
        glyphs.push(leaf);

        let mut glyf = Vec::new();
        let mut loca = Vec::new();
        for glyph in &glyphs {
            loca.extend_from_slice(&((glyf.len() / 2) as u16).to_be_bytes());
            glyf.extend_from_slice(glyph);
            // `loca` is 2-byte aligned in the short format.
            if glyf.len() % 2 != 0 {
                glyf.push(0);
            }
        }
        loca.extend_from_slice(&((glyf.len() / 2) as u16).to_be_bytes());

        let num_glyphs = glyphs.len() as u16;
        let bytes = font_with_glyf(glyf, loca, num_glyphs);
        assert!(
            bytes.len() < 2000,
            "the whole attack is {} bytes",
            bytes.len()
        );

        let font = font_parser::load(&bytes).expect("font should load");
        let parser = GlyphParser::new(&font).expect("font has glyf");

        let start = std::time::Instant::now();
        let result = parser.glyph_outline(0);
        let elapsed = start.elapsed();

        assert_eq!(result.unwrap_err(), GlyphError::CompositeBudgetExceeded);
        // The point is not merely that it returns an error -- a guard that
        // fires after a billion iterations is not a guard. It has to be fast.
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "budget check took {elapsed:?}; it should refuse almost immediately"
        );
    }

    #[test]
    fn a_glyph_id_past_the_end_is_an_error_not_a_silent_empty() {
        let glyph = composite_glyph(0);
        let loca = [0u16, (glyph.len() / 2) as u16]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect::<Vec<u8>>();
        let bytes = font_with_glyf(glyph, loca, 1);
        let font = font_parser::load(&bytes).unwrap();
        let parser = GlyphParser::new(&font).unwrap();

        // Returning Ok(None) here would be indistinguishable from a space, and
        // a subsetter asking for a glyph that does not exist has a bug worth
        // hearing about.
        assert_eq!(
            parser.glyph_outline(7).unwrap_err(),
            GlyphError::GlyphIndexOutOfRange(7)
        );
    }

    #[test]
    fn a_font_without_glyf_reports_an_unsupported_format() {
        // An OTF with CFF outlines is a legitimate font we simply do not read.
        // It should say so, rather than claim the file is corrupt.
        let mut head = vec![0u8; 54];
        head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
        head[18..20].copy_from_slice(&1000u16.to_be_bytes());
        let bytes = build_font(&[
            (b"cmap", vec![0; 4]),
            (b"head", head),
            (b"hhea", vec![0; 36]),
            (b"hmtx", vec![0; 4]),
            (b"maxp", vec![0; 6]),
        ]);
        let font = font_parser::load(&bytes).unwrap();
        assert_eq!(
            GlyphParser::new(&font).unwrap_err(),
            GlyphError::UnsupportedFontFormat
        );
    }

    #[test]
    fn f2dot14_reads_the_documented_values() {
        // From the OpenType spec's own worked examples.
        assert_eq!(f2dot14(0x7000), 1.75);
        assert_eq!(f2dot14(0x0001), 1.0 / 16384.0);
        assert_eq!(f2dot14(0x0000), 0.0);
        assert_eq!(f2dot14(-16384), -1.0);
    }

    #[test]
    fn read_delta_covers_all_four_flag_combinations() {
        let data = [0x05, 0x00, 0x10, 0xFF, 0xFF];

        // short + positive: one byte, taken as written.
        let mut offset = 0;
        assert_eq!(read_delta(&data, &mut offset, true, true).unwrap(), 5);
        assert_eq!(offset, 1);

        // short + negative: one byte, negated.
        let mut offset = 0;
        assert_eq!(read_delta(&data, &mut offset, true, false).unwrap(), -5);

        // not short + "same": no bytes consumed at all. This is the row that
        // desynchronises every later read when it is treated as a value.
        let mut offset = 2;
        assert_eq!(read_delta(&data, &mut offset, false, true).unwrap(), 0);
        assert_eq!(offset, 2);

        // not short + not same: a signed 16-bit delta.
        let mut offset = 3;
        assert_eq!(read_delta(&data, &mut offset, false, false).unwrap(), -1);
        assert_eq!(offset, 5);
    }

    #[test]
    fn a_run_of_straight_lines_becomes_line_commands() {
        let square = Points {
            points: vec![
                Point {
                    x: 0,
                    y: 0,
                    on_curve: true,
                },
                Point {
                    x: 10,
                    y: 0,
                    on_curve: true,
                },
                Point {
                    x: 10,
                    y: 10,
                    on_curve: true,
                },
                Point {
                    x: 0,
                    y: 10,
                    on_curve: true,
                },
            ],
            end_pts: vec![3],
        };
        let contours = build_contours(&square).unwrap();
        assert_eq!(contours.len(), 1);
        assert_eq!(
            contours[0].commands,
            vec![
                Command::MoveTo { x: 0, y: 0 },
                Command::LineTo { x: 10, y: 0 },
                Command::LineTo { x: 10, y: 10 },
                Command::LineTo { x: 0, y: 10 },
            ]
        );
    }

    #[test]
    fn two_off_curve_points_get_the_implied_midpoint_back() {
        // on(0,0) → off(10,10) → off(20,0) → back to start.
        // The font omits the on-curve point at (15,5) between the two controls.
        let points = Points {
            points: vec![
                Point {
                    x: 0,
                    y: 0,
                    on_curve: true,
                },
                Point {
                    x: 10,
                    y: 10,
                    on_curve: false,
                },
                Point {
                    x: 20,
                    y: 0,
                    on_curve: false,
                },
            ],
            end_pts: vec![2],
        };
        let contours = build_contours(&points).unwrap();
        assert_eq!(
            contours[0].commands,
            vec![
                Command::MoveTo { x: 0, y: 0 },
                // The implied point, restored:
                Command::QuadTo {
                    cx: 10,
                    cy: 10,
                    x: 15,
                    y: 5
                },
                // and the closing curve back to the start.
                Command::QuadTo {
                    cx: 20,
                    cy: 0,
                    x: 0,
                    y: 0
                },
            ]
        );
    }

    #[test]
    fn contour_ends_beyond_the_point_array_are_rejected() {
        let broken = Points {
            points: vec![Point {
                x: 0,
                y: 0,
                on_curve: true,
            }],
            end_pts: vec![7],
        };
        assert_eq!(
            build_contours(&broken).unwrap_err(),
            GlyphError::MalformedContour
        );
    }
}
