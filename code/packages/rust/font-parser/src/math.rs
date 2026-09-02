//! The OpenType `MATH` table.
//!
//! Every layout decision a maths typesetter makes needs a number from here. How
//! far above the baseline a fraction bar sits, how thick it is, how far a
//! superscript rises, how much clearance a radical needs over its contents —
//! none of that is guessable, and a renderer that invents the values produces
//! output that looks *almost* right, which is the worst outcome available. This
//! is what makes TeX-quality maths possible rather than approximate.
//!
//! ## Layout of the table
//!
//! ```text
//!   MATH
//!   ├── MathConstants     ~56 values: shifts, gaps, thicknesses
//!   ├── MathGlyphInfo
//!   │   ├── italic corrections     (per glyph)
//!   │   ├── top accent attachment  (per glyph)
//!   │   ├── extended shapes        (a coverage table, no values)
//!   │   └── kern info              (per-corner kerning)
//!   └── MathVariants
//!       ├── vertical   glyph construction  (how `(` grows)
//!       └── horizontal glyph construction  (how `\overbrace` grows)
//! ```
//!
//! ## `MathValue` and why most fields are two bytes wider than they look
//!
//! Almost every value in `MathConstants` is a **`MathValue`**: an `int16` in
//! font design units followed by an `Offset16` to a device table. The device
//! table carries per-pixel-size corrections for hinting at small sizes, and is
//! usually zero. Reading a `MathValue` as a bare `int16` therefore *appears* to
//! work — the first field is genuinely the value — while silently halving the
//! stride, so every constant after the first comes out wrong.
//!
//! The first four constants are the exception: they are plain `int16` percentages
//! and integers with no device table. Treating them uniformly with the rest is
//! the other easy way to get this wrong.
//!
//! ## Verification
//!
//! These readers are checked against **fontTools**, an independent
//! implementation, reading the same font. Transcribing ~56 signed values from a
//! binary table is exactly where a transposition or an off-by-two passes every
//! self-consistent test — our own reader would agree with our own understanding
//! and both would be wrong together.

use crate::{read_i16, read_u16, FontError};

/// The subset of `MathConstants` a typesetter actually needs.
///
/// Not all ~56 values are exposed yet. The ones here are those Appendix G
/// consumes for fractions, radicals, and scripts — the three constructions that
/// make up the overwhelming majority of real mathematical notation. The rest are
/// deliberately left for when a consumer needs them, rather than transcribed
/// speculatively: an untested constant is a liability, not a feature.
///
/// All values are in font design units. Divide by `unitsPerEm` and multiply by
/// the point size to get physical units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathConstants {
    /// Percentage the font scales down to for script (superscript) size.
    /// A percentage, not a design unit — typically around 70.
    pub script_percent_scale_down: i16,
    /// Percentage for scriptscript (super-superscript) size, typically ~55.
    pub script_script_percent_scale_down: i16,
    /// Minimum height of a `\left…\right` subformula before it is treated as
    /// delimited rather than inline.
    pub delimited_sub_formula_min_height: u16,
    /// Minimum height for a display-style operator such as `\sum`.
    pub display_operator_min_height: u16,
    /// Extra leading between lines of a maths formula.
    pub math_leading: i16,
    /// **Height of the maths axis above the baseline.** The single most
    /// load-bearing constant in the table: fraction bars, radical rules, and
    /// `\left(` delimiters are all centred on it, so an error here tilts every
    /// composite construction at once.
    pub axis_height: i16,
    /// Height of the base above which accents are placed.
    pub accent_base_height: i16,
    /// Thickness of the bar in a fraction. Also used as the default rule
    /// thickness for over- and under-lines.
    pub fraction_rule_thickness: i16,
    /// How far the numerator's baseline shifts up, in text style.
    pub fraction_numerator_shift_up: i16,
    /// How far the denominator's baseline shifts down, in text style.
    pub fraction_denominator_shift_down: i16,
    /// Thickness of the radical's overbar.
    pub radical_rule_thickness: i16,
    /// Vertical clearance between the radical rule and its contents.
    pub radical_vertical_gap: i16,
    /// How far a superscript's baseline shifts up, in an uncramped style.
    pub superscript_shift_up: i16,
    /// How far a subscript's baseline shifts down.
    pub subscript_shift_down: i16,
}

/// Read the `MATH` table's constants.
///
/// `math_offset` is the table's offset within `buf`, from the table directory.
pub(crate) fn math_constants(buf: &[u8], math_offset: usize) -> Result<MathConstants, FontError> {
    // MATH header: version (u32), then three Offset16s to the subtables.
    let constants_offset = read_u16(buf, math_offset + 4)? as usize;
    if constants_offset == 0 {
        return Err(FontError::TableNotFound("MATH.MathConstants"));
    }
    let base = math_offset + constants_offset;

    // The first four are bare int16 / uint16 with NO device table. Everything
    // after them is a MathValue: int16 value + Offset16 device table, so the
    // stride doubles from four bytes in.
    let script_percent_scale_down = read_i16(buf, base)?;
    let script_script_percent_scale_down = read_i16(buf, base + 2)?;
    let delimited_sub_formula_min_height = read_u16(buf, base + 4)?;
    let display_operator_min_height = read_u16(buf, base + 6)?;

    // MathValue records begin here, each 4 bytes wide, in the order the
    // specification lists them. `value_at` indexes by record rather than by byte
    // so the stride cannot be got wrong in one place and right in another.
    let value_at = |index: usize| -> Result<i16, FontError> { read_i16(buf, base + 8 + index * 4) };

    Ok(MathConstants {
        script_percent_scale_down,
        script_script_percent_scale_down,
        delimited_sub_formula_min_height,
        display_operator_min_height,
        // Record index = the constant's position in the specification's list,
        // minus the four bare fields above. The comments carry the spec
        // position so the arithmetic is checkable without counting records.
        math_leading: value_at(0)?,                     // spec 4
        axis_height: value_at(1)?,                      // spec 5
        accent_base_height: value_at(2)?,               // spec 6
        subscript_shift_down: value_at(4)?,             // spec 8
        superscript_shift_up: value_at(7)?,             // spec 11
        fraction_numerator_shift_up: value_at(28)?,     // spec 32
        fraction_denominator_shift_down: value_at(30)?, // spec 34
        fraction_rule_thickness: value_at(34)?,         // spec 38
        radical_vertical_gap: value_at(45)?,            // spec 49
        radical_rule_thickness: value_at(47)?,          // spec 51
    })
}

/// Italic correction for one glyph, in design units.
///
/// Italic correction is the horizontal gap a slanted glyph needs after it. Set a
/// subscript directly after an italic *f* without it and the subscript collides
/// with the descender — which is why maths without this looks subtly broken in a
/// way that is hard to name but easy to see.
///
/// Returns `None` when the glyph has no entry, which means zero correction.
pub(crate) fn italic_correction(
    buf: &[u8],
    math_offset: usize,
    glyph_id: u16,
) -> Result<Option<i16>, FontError> {
    let glyph_info_offset = read_u16(buf, math_offset + 6)? as usize;
    if glyph_info_offset == 0 {
        return Ok(None);
    }
    let glyph_info = math_offset + glyph_info_offset;

    // MathGlyphInfo begins with an Offset16 to MathItalicsCorrectionInfo.
    let italics_offset = read_u16(buf, glyph_info)? as usize;
    if italics_offset == 0 {
        return Ok(None);
    }
    let italics = glyph_info + italics_offset;

    // MathItalicsCorrectionInfo: Offset16 coverage, uint16 count, then that
    // many MathValue records.
    let coverage_offset = read_u16(buf, italics)? as usize;
    let count = read_u16(buf, italics + 2)?;
    let Some(index) = coverage_index(buf, italics + coverage_offset, glyph_id)? else {
        return Ok(None);
    };
    if index >= count {
        // The coverage table and the value array disagree. That is a corrupt
        // font rather than a missing glyph, so say so instead of returning
        // `None` and letting a wrong-but-plausible zero flow onward.
        return Err(FontError::BufferTooShort);
    }
    Ok(Some(read_i16(buf, italics + 4 + index as usize * 4)?))
}

/// Find a glyph's index within an OpenType coverage table.
///
/// Coverage tables are the standard way OpenType says "these glyphs, in this
/// order". Format 1 is an explicit sorted list; format 2 is sorted ranges each
/// carrying the index its first glyph maps to.
///
/// Both are sorted, so both could binary-search. Linear is used here because the
/// tables are small and because a binary search over a format-2 range list is a
/// classic off-by-one site — and this returns an index used to offset into a
/// value array, where being one out yields a wrong number rather than an error.
/// Which way a glyph stretches.
///
/// A `MATH` table keeps two independent constructions per glyph, and asking on
/// the wrong axis silently returns nothing -- so a brace that should grow tall
/// simply does not, with no error to notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StretchAxis {
    /// How `(` grows to wrap a tall fraction.
    Vertical,
    /// How `\overbrace` grows to span its contents.
    Horizontal,
}

/// One ready-made larger glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphVariant {
    pub glyph_id: u16,
    /// Its size along the stretch axis, in design units -- height for a
    /// vertical construction, width for a horizontal one.
    pub advance: u16,
}

/// One piece of a glyph built by assembly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssemblyPart {
    pub glyph_id: u16,
    /// How much of this part may overlap the piece BEFORE it.
    pub start_connector: u16,
    /// How much may overlap the piece AFTER it.
    pub end_connector: u16,
    /// Its full size along the axis.
    pub full_advance: u16,
    /// An extender may be repeated to reach the required size; the others
    /// appear exactly once. A renderer that repeats a non-extender draws two
    /// brace tips in the middle of a brace.
    pub is_extender: bool,
}

/// How one glyph grows.
///
/// Two mechanisms, and a renderer needs both. **Variants** are complete larger
/// glyphs the designer drew -- the four sizes of `(` that cover most cases.
/// **Assembly** is for when none is big enough: pieces stacked with overlap,
/// which is how a brace spanning half a page is built from a top, a bottom,
/// and a repeated extender.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GlyphConstruction {
    /// Larger pre-drawn glyphs, in increasing size.
    pub variants: Vec<GlyphVariant>,
    /// Pieces to stack when no variant is large enough. Empty when the font
    /// offers none, which means the largest variant is the limit.
    pub assembly: Vec<AssemblyPart>,
    /// Italic correction for the assembled glyph.
    pub assembly_italics_correction: i16,
}

/// The minimum overlap between adjacent assembly parts, in design units.
///
/// Parts must overlap by at least this much or the seams show as hairline gaps
/// at some sizes -- the defect that looks like a rendering artefact rather than
/// a layout bug.
pub(crate) fn min_connector_overlap(buf: &[u8], math_offset: usize) -> Result<u16, FontError> {
    let variants_offset = read_u16(buf, math_offset + 8)? as usize;
    if variants_offset == 0 {
        return Ok(0);
    }
    read_u16(buf, math_offset + variants_offset)
}

/// The construction for one glyph on one axis.
///
/// `Ok(None)` means the font gives this glyph no construction on that axis,
/// which is the common case: only delimiters and a few operators stretch.
pub(crate) fn glyph_construction(
    buf: &[u8],
    math_offset: usize,
    glyph_id: u16,
    axis: StretchAxis,
) -> Result<Option<GlyphConstruction>, FontError> {
    // MATH header: majorVersion, minorVersion, then three Offset16 --
    // MathConstants at 4, MathGlyphInfo at 6, MathVariants at 8. Reading +6
    // here parses MathGlyphInfo as MathVariants: it does not fail, it returns
    // plausible numbers from the wrong table. The oracle reported a minimum
    // connector overlap of 8 -- itself an offset -- rather than 100.
    let variants_offset = read_u16(buf, math_offset + 8)? as usize;
    if variants_offset == 0 {
        return Ok(None);
    }
    let variants = math_offset + variants_offset;

    // MathVariants: minConnectorOverlap, vertCoverage, horizCoverage,
    // vertCount, horizCount, then the two construction-offset arrays.
    let vert_coverage = read_u16(buf, variants + 2)? as usize;
    let horiz_coverage = read_u16(buf, variants + 4)? as usize;
    let vert_count = read_u16(buf, variants + 6)? as usize;

    let (coverage, count_offset, array_offset) = match axis {
        StretchAxis::Vertical => (vert_coverage, variants + 6, variants + 10),
        // The horizontal array follows the vertical one, so its start depends
        // on how many vertical constructions there are.
        StretchAxis::Horizontal => (horiz_coverage, variants + 8, variants + 10 + vert_count * 2),
    };
    if coverage == 0 {
        return Ok(None);
    }
    let count = read_u16(buf, count_offset)? as usize;

    let Some(index) = coverage_index(buf, variants + coverage, glyph_id)? else {
        return Ok(None);
    };
    let index = index as usize;
    if index >= count {
        // Coverage and the construction array disagree, which means the table
        // is malformed; reporting "no construction" is safer than reading an
        // offset from beyond the array.
        return Ok(None);
    }

    let construction_offset = read_u16(buf, array_offset + index * 2)? as usize;
    if construction_offset == 0 {
        return Ok(None);
    }
    let construction = variants + construction_offset;

    // MathGlyphConstruction: Offset16 glyphAssembly, uint16 variantCount,
    // then the variant records.
    let assembly_offset = read_u16(buf, construction)? as usize;
    let variant_count = read_u16(buf, construction + 2)? as usize;

    let mut records = Vec::with_capacity(variant_count);
    for i in 0..variant_count {
        let record = construction + 4 + i * 4;
        records.push(GlyphVariant {
            glyph_id: read_u16(buf, record)?,
            advance: read_u16(buf, record + 2)?,
        });
    }

    let mut assembly = Vec::new();
    let mut assembly_italics_correction = 0;
    if assembly_offset != 0 {
        let table = construction + assembly_offset;
        // GlyphAssembly: a MathValueRecord (int16 + Offset16 device table),
        // then partCount, then the part records. Reading the italics
        // correction as a bare int16 would put every part record two bytes
        // early -- the same trap `MathConstants` has.
        assembly_italics_correction = read_i16(buf, table)?;
        let part_count = read_u16(buf, table + 4)? as usize;
        for i in 0..part_count {
            let record = table + 6 + i * 10;
            assembly.push(AssemblyPart {
                glyph_id: read_u16(buf, record)?,
                start_connector: read_u16(buf, record + 2)?,
                end_connector: read_u16(buf, record + 4)?,
                full_advance: read_u16(buf, record + 6)?,
                // Bit 0 of partFlags. The other bits are reserved.
                is_extender: read_u16(buf, record + 8)? & 0x0001 != 0,
            });
        }
    }

    Ok(Some(GlyphConstruction {
        variants: records,
        assembly,
        assembly_italics_correction,
    }))
}

fn coverage_index(buf: &[u8], offset: usize, glyph_id: u16) -> Result<Option<u16>, FontError> {
    match read_u16(buf, offset)? {
        1 => {
            let count = read_u16(buf, offset + 2)?;
            for i in 0..count as usize {
                if read_u16(buf, offset + 4 + i * 2)? == glyph_id {
                    return Ok(Some(i as u16));
                }
            }
            Ok(None)
        }
        2 => {
            let range_count = read_u16(buf, offset + 2)?;
            for i in 0..range_count as usize {
                let rec = offset + 4 + i * 6;
                let start = read_u16(buf, rec)?;
                let end = read_u16(buf, rec + 2)?;
                let start_index = read_u16(buf, rec + 4)?;
                if glyph_id >= start && glyph_id <= end {
                    return Ok(Some(start_index + (glyph_id - start)));
                }
            }
            Ok(None)
        }
        // An unknown coverage format is corruption, not absence.
        _ => Err(FontError::BufferTooShort),
    }
}
