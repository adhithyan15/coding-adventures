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
        math_leading: value_at(0)?,                      // spec 4
        axis_height: value_at(1)?,                       // spec 5
        accent_base_height: value_at(2)?,                // spec 6
        subscript_shift_down: value_at(4)?,              // spec 8
        superscript_shift_up: value_at(7)?,              // spec 11
        fraction_numerator_shift_up: value_at(28)?,      // spec 32
        fraction_denominator_shift_down: value_at(30)?,  // spec 34
        fraction_rule_thickness: value_at(34)?,          // spec 38
        radical_vertical_gap: value_at(45)?,             // spec 49
        radical_rule_thickness: value_at(47)?,           // spec 51
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
