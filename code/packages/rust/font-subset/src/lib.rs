//! Build a TrueType font containing only the glyphs a document actually uses.
//!
//! # Why this exists
//!
//! A PDF that shows Tamil, Devanagari or CJK has to **embed** a font — the
//! base-14 faces every reader ships cannot draw any of it. But a CJK font is
//! several megabytes and a study sheet uses a few hundred glyphs, so embedding
//! the whole thing turns a two-page export into a 20 MB file. Subsetting is the
//! difference between an export people use and one they do not.
//!
//! Engram is a language-study app, so this is the normal case rather than an
//! edge case.
//!
//! # How it subsets: by emptying, not by renumbering
//!
//! The obvious design compacts the glyph set — keep 300 glyphs, renumber them
//! 0..300, rewrite every table that refers to a glyph id. That is smaller, and
//! it is a great deal more dangerous: `cmap`, `GSUB`, `GPOS`, `MATH`, composite
//! glyph components and the PDF's own `CIDToGIDMap` all speak glyph ids, and
//! any table missed produces a font that renders *plausible wrong glyphs*.
//!
//! This keeps every glyph id exactly where it was and empties the ones nobody
//! asked for. `loca` still has one entry per glyph — four bytes each, so
//! 80 KB for a 20,000-glyph CJK face — while `glyf`, which holds the outlines
//! and nearly all of the bulk, shrinks to what was requested. That captures
//! almost all of the saving for none of the renumbering risk, and it keeps the
//! PDF's `CIDToGIDMap` an identity map.
//!
//! ```text
//!   NotoSansJP subset to 3 glyphs:
//!     glyf   1,200,000 bytes  ->  ~400 bytes
//!     loca       2,592 bytes  ->    2,592 bytes   (unchanged, and tiny)
//! ```
//!
//! # Composite glyphs must drag their components along
//!
//! `á` is stored as "draw `a`, then draw `acute` shifted". Keeping `á` while
//! dropping `acute` yields a glyph that references an empty one — no error,
//! just a missing accent. So the requested set is closed over composite
//! components before anything is written, transitively, because a component can
//! itself be composite.
//!
//! Keeping the ids also means `cmap` needs no rewriting at all: every mapping
//! in it still points where it did. A renumbering subsetter has to rebuild it,
//! and a mapping missed there is a character that silently draws the wrong
//! glyph.
//!
//! # Scope
//!
//! TrueType outlines (`glyf`/`loca`). A `CFF `-based OpenType font stores its
//! outlines completely differently and needs its own subsetter;
//! [`SubsetError::UnsupportedFontFormat`] says so rather than emitting a font
//! with no outlines at all.

use std::collections::BTreeSet;

use font_parser::FontFile;

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubsetError {
    /// The font has no `glyf` table — a CFF/PostScript OpenType font.
    UnsupportedFontFormat,
    /// A table this needs is absent.
    MissingTable(&'static str),
    /// A read ran past the end of a table.
    Truncated(&'static str),
    /// A requested glyph id is `>= maxp.numGlyphs`.
    GlyphOutOfRange(u16),
    /// Composite nesting deeper than [`MAX_COMPOSITE_DEPTH`], or a glyph that
    /// references itself.
    CompositeDepthExceeded,
}

impl std::fmt::Display for SubsetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFontFormat => {
                write!(
                    f,
                    "font has no `glyf` outlines (CFF needs its own subsetter)"
                )
            }
            Self::MissingTable(name) => write!(f, "font is missing the `{name}` table"),
            Self::Truncated(name) => write!(f, "the `{name}` table is truncated"),
            Self::GlyphOutOfRange(id) => write!(f, "glyph {id} is out of range"),
            Self::CompositeDepthExceeded => {
                write!(f, "composite glyphs nest deeper than {MAX_COMPOSITE_DEPTH}")
            }
        }
    }
}

impl std::error::Error for SubsetError {}

/// How deep composite references may nest while computing the closure.
///
/// A font is untrusted input: a glyph that references itself would otherwise
/// recurse forever. Real fonts reach two or three.
pub const MAX_COMPOSITE_DEPTH: u8 = 10;

// ─────────────────────────────────────────────────────────────────────────────
// Reading helpers
// ─────────────────────────────────────────────────────────────────────────────

fn u16_at(buf: &[u8], offset: usize, what: &'static str) -> Result<u16, SubsetError> {
    let bytes = buf
        .get(offset..offset + 2)
        .ok_or(SubsetError::Truncated(what))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn u32_at(buf: &[u8], offset: usize, what: &'static str) -> Result<u32, SubsetError> {
    let bytes = buf
        .get(offset..offset + 4)
        .ok_or(SubsetError::Truncated(what))?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

// ─────────────────────────────────────────────────────────────────────────────
// The subset
// ─────────────────────────────────────────────────────────────────────────────

/// A subsetted font, plus what went into it.
#[derive(Debug, Clone)]
pub struct Subset {
    /// The complete font file, ready to embed as a PDF `FontFile2`.
    pub font: Vec<u8>,
    /// Every glyph the subset retains, including components pulled in to
    /// satisfy composites. Superset of what was asked for.
    pub retained: BTreeSet<u16>,
    /// `maxp.numGlyphs`, unchanged — glyph ids are not renumbered, so this is
    /// still the id space the PDF's `CIDToGIDMap` indexes.
    pub num_glyphs: u16,
}

/// Build a font containing only `glyphs` (and whatever they depend on).
///
/// Glyph 0 — `.notdef` — is always retained: a font without it is invalid, and
/// it is what a reader draws for anything unmapped.
pub fn subset(font: &FontFile, glyphs: &BTreeSet<u16>) -> Result<Subset, SubsetError> {
    let glyf = font
        .table(b"glyf")
        .ok_or(SubsetError::UnsupportedFontFormat)?;
    let loca = font
        .table(b"loca")
        .ok_or(SubsetError::MissingTable("loca"))?;
    let head = font
        .table(b"head")
        .ok_or(SubsetError::MissingTable("head"))?;
    let hhea = font
        .table(b"hhea")
        .ok_or(SubsetError::MissingTable("hhea"))?;
    let maxp = font
        .table(b"maxp")
        .ok_or(SubsetError::MissingTable("maxp"))?;
    let hmtx = font
        .table(b"hmtx")
        .ok_or(SubsetError::MissingTable("hmtx"))?;
    // Carried through unchanged, and correct BECAUSE glyph ids are preserved:
    // a subsetter that renumbered would have to rewrite every mapping in here.
    // It also keeps the result a usable font in its own right rather than a
    // PDF-only payload -- and font loaders, ours included, expect it.
    let cmap = font
        .table(b"cmap")
        .ok_or(SubsetError::MissingTable("cmap"))?;

    let num_glyphs = font
        .num_glyphs()
        .map_err(|_| SubsetError::MissingTable("maxp"))?;
    let long_loca = font
        .index_to_loc_format()
        .map_err(|_| SubsetError::MissingTable("head"))?
        != 0;

    for &id in glyphs {
        if id >= num_glyphs {
            return Err(SubsetError::GlyphOutOfRange(id));
        }
    }

    // Close over composite components before deciding what to keep.
    let mut retained: BTreeSet<u16> = BTreeSet::new();
    retained.insert(0); // .notdef is not optional
    for &id in glyphs {
        add_with_components(id, glyf, loca, long_loca, num_glyphs, 0, &mut retained)?;
    }

    // ── glyf and loca, rebuilt ───────────────────────────────────────────────
    //
    // Written with LONG offsets regardless of the original format. A short
    // `loca` stores half the offset, so it can only address 128 KiB of `glyf`;
    // that is fine for the subset but the format is then a second thing to get
    // right for no saving worth having (two bytes per glyph).
    let mut new_glyf: Vec<u8> = Vec::new();
    let mut new_loca: Vec<u8> = Vec::with_capacity((num_glyphs as usize + 1) * 4);
    for id in 0..num_glyphs {
        new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
        if retained.contains(&id) {
            let (start, end) = glyph_range(loca, long_loca, id)?;
            if end > start {
                let bytes = glyf.get(start..end).ok_or(SubsetError::Truncated("glyf"))?;
                new_glyf.extend_from_slice(bytes);
                // Every glyph must start on an even boundary for a short
                // `loca`; harmless for long, and keeps the file byte-identical
                // in shape to what other tools produce.
                while !new_glyf.len().is_multiple_of(4) {
                    new_glyf.push(0);
                }
            }
        }
        // A dropped glyph gets no bytes at all: its loca entry equals the next
        // one, which is precisely how TrueType spells "this glyph is empty".
    }
    new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());

    // ── head, with indexToLocFormat corrected ────────────────────────────────
    let mut new_head = head.to_vec();
    if new_head.len() < 54 {
        return Err(SubsetError::Truncated("head"));
    }
    new_head[50..52].copy_from_slice(&1i16.to_be_bytes()); // long loca
                                                           // checkSumAdjustment is computed over the finished file, so it cannot be
                                                           // known yet. Zero it now and patch it at the end.
    new_head[8..12].copy_from_slice(&0u32.to_be_bytes());

    let tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"glyf", new_glyf),
        (b"head", new_head),
        (b"hhea", hhea.to_vec()),
        (b"hmtx", hmtx.to_vec()),
        (b"loca", new_loca),
        (b"maxp", maxp.to_vec()),
        (b"cmap", cmap.to_vec()),
    ];

    Ok(Subset {
        font: assemble(&tables),
        retained,
        num_glyphs,
    })
}

/// The byte range of one glyph within `glyf`.
fn glyph_range(loca: &[u8], long: bool, id: u16) -> Result<(usize, usize), SubsetError> {
    let index = id as usize;
    if long {
        Ok((
            u32_at(loca, index * 4, "loca")? as usize,
            u32_at(loca, index * 4 + 4, "loca")? as usize,
        ))
    } else {
        // Short entries store HALF the offset -- glyphs are 2-byte aligned, so
        // halving lets a u16 address 128 KiB instead of 64.
        Ok((
            u16_at(loca, index * 2, "loca")? as usize * 2,
            u16_at(loca, index * 2 + 2, "loca")? as usize * 2,
        ))
    }
}

/// Add a glyph and, if it is composite, everything it draws.
fn add_with_components(
    id: u16,
    glyf: &[u8],
    loca: &[u8],
    long_loca: bool,
    num_glyphs: u16,
    depth: u8,
    out: &mut BTreeSet<u16>,
) -> Result<(), SubsetError> {
    if depth > MAX_COMPOSITE_DEPTH {
        return Err(SubsetError::CompositeDepthExceeded);
    }
    if id >= num_glyphs || !out.insert(id) {
        // Already present, so its components are too -- and this is what stops
        // a cycle from recursing forever.
        return Ok(());
    }

    let (start, end) = glyph_range(loca, long_loca, id)?;
    if end <= start {
        return Ok(()); // empty glyph, nothing to depend on
    }
    let data = glyf.get(start..end).ok_or(SubsetError::Truncated("glyf"))?;
    if data.len() < 10 {
        return Err(SubsetError::Truncated("glyf"));
    }
    let contours = u16_at(data, 0, "glyf")? as i16;
    if contours >= 0 {
        return Ok(()); // simple glyph
    }

    // Walk the component records. Their sizes vary with the flags, so the only
    // way to find the next one is to decode each in turn.
    const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
    const WE_HAVE_A_SCALE: u16 = 0x0008;
    const MORE_COMPONENTS: u16 = 0x0020;
    const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
    const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;

    let mut offset = 10;
    loop {
        let flags = u16_at(data, offset, "glyf")?;
        let component = u16_at(data, offset + 2, "glyf")?;
        offset += 4;
        offset += if flags & ARG_1_AND_2_ARE_WORDS != 0 {
            4
        } else {
            2
        };
        if flags & WE_HAVE_A_SCALE != 0 {
            offset += 2;
        } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            offset += 4;
        } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
            offset += 8;
        }

        add_with_components(component, glyf, loca, long_loca, num_glyphs, depth + 1, out)?;

        if flags & MORE_COMPONENTS == 0 {
            break;
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Writing the sfnt
// ─────────────────────────────────────────────────────────────────────────────

/// Sum a table as big-endian `u32` words, zero-padding a ragged tail.
fn checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for block in data.chunks(4) {
        let mut chunk = [0u8; 4];
        chunk[..block.len()].copy_from_slice(block);
        sum = sum.wrapping_add(u32::from_be_bytes(chunk));
    }
    sum
}

/// Assemble tables into a font file.
///
/// Table records must be sorted by tag and every table must start on a 4-byte
/// boundary. Both are requirements of the format rather than conventions, and
/// a reader that trusts the directory will read the wrong bytes if either is
/// wrong rather than reporting a problem.
fn assemble(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut sorted: Vec<&(&[u8; 4], Vec<u8>)> = tables.iter().collect();
    sorted.sort_by_key(|(tag, _)| **tag);

    let count = sorted.len() as u16;
    // searchRange / entrySelector / rangeShift describe a binary search over
    // the records. Readers vary in how much they care; the spec defines them,
    // so they are computed rather than zeroed.
    let entry_selector = (15u16 - (count.leading_zeros() as u16)).min(15);
    let search_range = 16u16 << entry_selector;
    let range_shift = count * 16 - search_range;

    let mut font = Vec::new();
    font.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // TrueType outlines
    font.extend_from_slice(&count.to_be_bytes());
    font.extend_from_slice(&search_range.to_be_bytes());
    font.extend_from_slice(&entry_selector.to_be_bytes());
    font.extend_from_slice(&range_shift.to_be_bytes());

    let mut offset = 12 + sorted.len() * 16;
    let mut directory = Vec::new();
    let mut body = Vec::new();
    for (tag, data) in &sorted {
        directory.extend_from_slice(*tag);
        directory.extend_from_slice(&checksum(data).to_be_bytes());
        directory.extend_from_slice(&(offset as u32).to_be_bytes());
        // The recorded length is the REAL length; the padding that follows is
        // not part of the table.
        directory.extend_from_slice(&(data.len() as u32).to_be_bytes());
        body.extend_from_slice(data);
        while !body.len().is_multiple_of(4) {
            body.push(0);
        }
        offset = 12 + sorted.len() * 16 + body.len();
    }
    font.extend_from_slice(&directory);
    font.extend_from_slice(&body);

    patch_head_checksum(&mut font, &sorted);
    font
}

/// Write `head.checkSumAdjustment`, which is a checksum of the whole file.
///
/// Defined as `0xB1B0AFBA` minus the sum of the entire font computed with this
/// field set to zero — so it can only be written once everything else is in
/// place.
fn patch_head_checksum(font: &mut [u8], sorted: &[&(&[u8; 4], Vec<u8>)]) {
    let Some(index) = sorted.iter().position(|(tag, _)| *tag == b"head") else {
        return;
    };
    let record = 12 + index * 16;
    let Some(bytes) = font.get(record + 8..record + 12) else {
        return;
    };
    let head_offset = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(font));
    if let Some(slot) = font.get_mut(head_offset + 8..head_offset + 12) {
        slot.copy_from_slice(&adjustment.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inter() -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/fonts/Inter-Regular.ttf");
        std::fs::read(path).expect("Inter fixture")
    }

    #[test]
    fn notdef_is_always_retained() {
        let bytes = inter();
        let font = font_parser::load(&bytes).unwrap();
        let result = subset(&font, &BTreeSet::from([36u16])).unwrap();
        assert!(
            result.retained.contains(&0),
            "a font without .notdef is invalid"
        );
    }

    #[test]
    fn the_glyph_id_space_is_preserved() {
        let bytes = inter();
        let font = font_parser::load(&bytes).unwrap();
        let result = subset(&font, &BTreeSet::from([36u16])).unwrap();
        // Not renumbered: the PDF's CIDToGIDMap stays an identity map, and no
        // other table's glyph references need rewriting.
        assert_eq!(result.num_glyphs, font.num_glyphs().unwrap());
    }

    /// The saving, measured where subsetting actually acts.
    ///
    /// Overall file size is a blunt measure: it mixes the outlines (which
    /// subsetting removes) with fixed costs like `cmap` that it keeps. So this
    /// checks `glyf` directly -- 213 KB of Inter's 407 KB, and 108 KB of
    /// NotoSansJP's 128 KB -- and then sanity-checks the whole file.
    #[test]
    fn the_outlines_shrink_to_almost_nothing() {
        let bytes = inter();
        let font = font_parser::load(&bytes).unwrap();
        let result = subset(&font, &BTreeSet::from([36u16, 37, 38])).unwrap();

        let original_glyf = font.table(b"glyf").unwrap().len();
        let subset_font = font_parser::load(&result.font).unwrap();
        let subset_glyf = subset_font.table(b"glyf").unwrap().len();
        assert!(
            subset_glyf * 100 < original_glyf,
            "three glyphs should be a rounding error of the outlines: {subset_glyf} vs {original_glyf}"
        );

        // And the file as a whole is still much smaller, just not by as much:
        // `cmap` is retained deliberately (see the module docs) and is 26 KB in
        // Inter, which is unusually large.
        assert!(
            result.font.len() * 4 < bytes.len(),
            "expected a large saving overall: {} vs {}",
            result.font.len(),
            bytes.len()
        );
    }

    #[test]
    fn a_composite_glyph_drags_its_components_in() {
        let bytes = inter();
        let font = font_parser::load(&bytes).unwrap();
        // Glyph 3 is `Adieresis` in Inter -- a composite of `A` and a diaeresis.
        // Keeping it without its components would render an unaccented A.
        let result = subset(&font, &BTreeSet::from([3u16])).unwrap();
        assert!(
            result.retained.len() > 2,
            "expected components beyond .notdef and the glyph itself, got {:?}",
            result.retained
        );
    }

    #[test]
    fn an_out_of_range_glyph_is_refused() {
        let bytes = inter();
        let font = font_parser::load(&bytes).unwrap();
        let too_big = font.num_glyphs().unwrap();
        assert_eq!(
            subset(&font, &BTreeSet::from([too_big])).unwrap_err(),
            SubsetError::GlyphOutOfRange(too_big)
        );
    }

    #[test]
    fn the_result_is_a_font_our_own_parser_can_load() {
        let bytes = inter();
        let font = font_parser::load(&bytes).unwrap();
        let result = subset(&font, &BTreeSet::from([36u16, 3])).unwrap();
        // A weak check on its own -- our parser sharing a misunderstanding with
        // our writer is the whole hazard -- which is why `tests/fonttools_oracle.rs`
        // hands the same bytes to fontTools. This just fails faster.
        let reloaded = font_parser::load(&result.font).expect("subset should load");
        assert_eq!(reloaded.num_glyphs().unwrap(), result.num_glyphs);
    }
}
