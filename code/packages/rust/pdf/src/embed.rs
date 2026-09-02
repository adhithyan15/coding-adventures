//! Embedded TrueType fonts: `/Type0`, `Identity-H`, and a `/ToUnicode` CMap.
//!
//! The base-14 fonts in [`crate::StandardFont`] need no embedding and cannot
//! draw Tamil, Devanagari, or CJK. Engram is a language-study app, so those
//! scripts are the normal case — which makes this the half that decides
//! whether real documents work.
//!
//! # The three things a reader needs
//!
//! ```text
//!   /Type0 font        how character codes in the content stream map to CIDs
//!     -> /CIDFontType2   the CID-keyed font, its widths, and CIDToGIDMap
//!          -> /FontFile2   the actual sfnt bytes
//!     -> /ToUnicode      how to turn those codes back into text
//! ```
//!
//! With `Identity-H` encoding, a "character code" in the content stream **is**
//! a glyph id, written as two big-endian bytes. That is why text is shown with
//! [`crate::Content::show_glyphs`] rather than `show_text`: the bytes are glyph
//! ids, not characters, and passing a string would draw whatever glyphs
//! happened to sit at those code points.
//!
//! # `/ToUnicode` is invisible until someone copies the text
//!
//! Nothing about rendering depends on it. A PDF without it draws perfectly and
//! yields gibberish when selected, searched, or read by a screen reader — a
//! defect that survives every visual check. `pdftotext` is the oracle for
//! exactly this reason: it exercises the one path rasterising cannot see.
//!
//! # Why the widths are written twice over
//!
//! The font already knows its advances, but a PDF reader is not required to
//! parse the embedded font to lay out text — it uses `/W` from the CID font.
//! If the two disagree the glyphs are drawn at the right size in the wrong
//! places, which looks like bad kerning rather than a bug. They are taken from
//! the same `font-parser` metrics here so they cannot drift.

use std::collections::BTreeMap;

use crate::{Dict, ObjId, Object, PdfWriter};

/// PDF's text space is 1000 units to the em regardless of the font's own
/// `unitsPerEm`, so every metric is scaled into it.
const PDF_UNITS_PER_EM: f64 = 1000.0;

/// One glyph's contribution to the font's PDF representation.
#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddedGlyph {
    /// Advance width in font design units.
    pub advance: u16,
    /// The text this glyph represents, for `/ToUnicode`.
    ///
    /// A `String` rather than a `char`: one glyph can stand for several
    /// characters (a ligature), and a reader recovering "fi" from a single
    /// glyph is the whole point of the CMap.
    pub text: String,
}

/// A subsetted TrueType font, ready to embed.
///
/// Built from bytes produced by `font-subset` — this type does no subsetting
/// itself, so a caller can embed a whole font when that is what they want.
#[derive(Clone, Debug)]
pub struct EmbeddedFont {
    name: String,
    font: Vec<u8>,
    units_per_em: u16,
    ascent: i16,
    descent: i16,
    bbox: [i16; 4],
    glyphs: BTreeMap<u16, EmbeddedGlyph>,
}

impl EmbeddedFont {
    /// Describe a font for embedding.
    ///
    /// `name` becomes the `/BaseFont`, prefixed with a subset tag. `glyphs`
    /// maps glyph id to its advance and the text it stands for.
    pub fn new(
        name: impl Into<String>,
        font: Vec<u8>,
        units_per_em: u16,
        ascent: i16,
        descent: i16,
        bbox: [i16; 4],
        glyphs: BTreeMap<u16, EmbeddedGlyph>,
    ) -> Self {
        Self {
            name: name.into(),
            font,
            units_per_em,
            ascent,
            descent,
            bbox,
            glyphs,
        }
    }

    /// The glyphs this font describes.
    pub fn glyphs(&self) -> &BTreeMap<u16, EmbeddedGlyph> {
        &self.glyphs
    }

    /// Scale a design-unit value into PDF's 1000-per-em text space.
    fn to_text_space(&self, value: f64) -> f64 {
        if self.units_per_em == 0 {
            return value;
        }
        value * PDF_UNITS_PER_EM / f64::from(self.units_per_em)
    }

    /// The `/BaseFont` name, with the six-letter subset tag PDF asks for.
    ///
    /// The tag exists so two subsets of the same face, each carrying different
    /// glyphs, are not treated as the same font when documents are merged.
    /// It is derived from the retained glyph ids rather than random, so the
    /// same subset always produces the same name and output stays comparable
    /// between runs.
    fn tagged_name(&self) -> String {
        let mut hash: u32 = 0x811C_9DC5;
        for id in self.glyphs.keys() {
            for byte in id.to_be_bytes() {
                hash ^= u32::from(byte);
                hash = hash.wrapping_mul(0x0100_0193);
            }
        }
        let mut tag = String::with_capacity(7);
        for index in 0..6 {
            let letter = b'A' + ((hash >> (index * 5)) & 0x1F) as u8 % 26;
            tag.push(letter as char);
        }
        tag.push('+');
        tag.push_str(&self.name);
        tag
    }

    /// `/W`, the per-CID widths.
    ///
    /// Written in the `[ cid [w w w] ]` form, grouping consecutive ids so a
    /// few hundred glyphs do not become a few hundred array entries.
    fn widths_array(&self) -> Object {
        let mut entries: Vec<Object> = Vec::new();
        let mut run_start: Option<u16> = None;
        let mut run: Vec<Object> = Vec::new();
        let mut previous: Option<u16> = None;

        for (&id, glyph) in &self.glyphs {
            let width = self.to_text_space(f64::from(glyph.advance)).round();
            let continues = previous.is_some_and(|last| id == last + 1);
            if !continues {
                if let (Some(start), false) = (run_start, run.is_empty()) {
                    entries.push(Object::Int(i64::from(start)));
                    entries.push(Object::Array(std::mem::take(&mut run)));
                }
                run_start = Some(id);
            }
            run.push(Object::Real(width));
            previous = Some(id);
        }
        if let (Some(start), false) = (run_start, run.is_empty()) {
            entries.push(Object::Int(i64::from(start)));
            entries.push(Object::Array(run));
        }
        Object::Array(entries)
    }

    /// The `/ToUnicode` CMap: glyph id back to the text it stands for.
    fn to_unicode_cmap(&self) -> Vec<u8> {
        let mut out = String::new();
        out.push_str(
            "/CIDInit /ProcSet findresource begin\n\
             12 dict begin\n\
             begincmap\n\
             /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
             /CMapName /Adobe-Identity-UCS def\n\
             /CMapType 2 def\n\
             1 begincodespacerange\n\
             <0000> <FFFF>\n\
             endcodespacerange\n",
        );

        // `bfchar` entries come in blocks of at most 100 -- a hard limit in the
        // CMap specification, not a formatting choice. Exceeding it makes
        // readers stop at the boundary, so text recovery quietly dies partway
        // through a document with many glyphs.
        let entries: Vec<(u16, &EmbeddedGlyph)> =
            self.glyphs.iter().map(|(&id, glyph)| (id, glyph)).collect();
        for chunk in entries.chunks(100) {
            out.push_str(&format!("{} beginbfchar\n", chunk.len()));
            for (id, glyph) in chunk {
                out.push_str(&format!("<{id:04X}> <"));
                for unit in glyph.text.encode_utf16() {
                    out.push_str(&format!("{unit:04X}"));
                }
                out.push_str(">\n");
            }
            out.push_str("endbfchar\n");
        }

        out.push_str(
            "endcmap\n\
             CMapName currentdict /CMap defineresource pop\n\
             end\n\
             end\n",
        );
        out.into_bytes()
    }

    /// Write the font's object graph and return the `/Type0` font to reference.
    pub(crate) fn write(&self, writer: &mut PdfWriter) -> ObjId {
        let base_name = self.tagged_name();

        // The sfnt itself. `/Length1` is the UNCOMPRESSED length, which some
        // readers use to validate the stream; deriving it from the compressed
        // bytes would be wrong in a way that only shows on those readers.
        let (encoded, filter) = crate::flate_encode(&self.font);
        let mut stream_dict = Dict::new();
        stream_dict.set("Filter", filter);
        stream_dict.set("Length1", Object::Int(self.font.len() as i64));
        let font_file = writer.add(Object::Stream {
            dict: stream_dict,
            data: encoded,
        });

        let mut descriptor = Dict::new();
        descriptor.set("Type", Object::name("FontDescriptor"));
        descriptor.set("FontName", Object::name(base_name.clone()));
        // Flag 3 (value 4) is "symbolic": the font's own encoding governs,
        // rather than a standard Latin one. Declaring a non-symbolic font here
        // invites a reader to reinterpret the codes as Latin text.
        descriptor.set("Flags", Object::Int(4));
        descriptor.set(
            "FontBBox",
            Object::Array(
                self.bbox
                    .iter()
                    .map(|value| Object::Real(self.to_text_space(f64::from(*value)).round()))
                    .collect(),
            ),
        );
        descriptor.set("ItalicAngle", Object::Int(0));
        descriptor.set(
            "Ascent",
            Object::Real(self.to_text_space(f64::from(self.ascent)).round()),
        );
        descriptor.set(
            "Descent",
            Object::Real(self.to_text_space(f64::from(self.descent)).round()),
        );
        // Required by the specification and not derivable from the outlines
        // without measuring stems; readers use it only as a hint.
        descriptor.set("StemV", Object::Int(80));
        descriptor.set(
            "CapHeight",
            Object::Real(self.to_text_space(f64::from(self.ascent)).round()),
        );
        descriptor.set("FontFile2", Object::Ref(font_file));
        let descriptor_id = writer.add(Object::Dict(descriptor));

        let mut cid_system_info = Dict::new();
        cid_system_info.set("Registry", Object::Str(b"Adobe".to_vec()));
        cid_system_info.set("Ordering", Object::Str(b"Identity".to_vec()));
        cid_system_info.set("Supplement", Object::Int(0));

        let mut cid_font = Dict::new();
        cid_font.set("Type", Object::name("Font"));
        cid_font.set("Subtype", Object::name("CIDFontType2"));
        cid_font.set("BaseFont", Object::name(base_name.clone()));
        cid_font.set("CIDSystemInfo", Object::Dict(cid_system_info));
        cid_font.set("FontDescriptor", Object::Ref(descriptor_id));
        // The default width, used for any CID absent from /W.
        cid_font.set("DW", Object::Int(1000));
        cid_font.set("W", self.widths_array());
        // `font-subset` preserves glyph ids, so a CID *is* a glyph id and the
        // map is the identity. A renumbering subsetter would have to emit an
        // explicit stream here.
        cid_font.set("CIDToGIDMap", Object::name("Identity"));
        let cid_font_id = writer.add(Object::Dict(cid_font));

        let (cmap_data, cmap_filter) = crate::flate_encode(&self.to_unicode_cmap());
        let mut cmap_dict = Dict::new();
        cmap_dict.set("Filter", cmap_filter);
        let to_unicode = writer.add(Object::Stream {
            dict: cmap_dict,
            data: cmap_data,
        });

        let mut type0 = Dict::new();
        type0.set("Type", Object::name("Font"));
        type0.set("Subtype", Object::name("Type0"));
        type0.set("BaseFont", Object::name(base_name));
        // Two-byte big-endian codes that are glyph ids directly.
        type0.set("Encoding", Object::name("Identity-H"));
        type0.set(
            "DescendantFonts",
            Object::Array(vec![Object::Ref(cid_font_id)]),
        );
        type0.set("ToUnicode", Object::Ref(to_unicode));
        writer.add(Object::Dict(type0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font_with(glyphs: &[(u16, u16, &str)]) -> EmbeddedFont {
        EmbeddedFont::new(
            "TestFace",
            vec![0u8; 32],
            2048,
            1900,
            -500,
            [0, -500, 2048, 1900],
            glyphs
                .iter()
                .map(|(id, advance, text)| {
                    (
                        *id,
                        EmbeddedGlyph {
                            advance: *advance,
                            text: (*text).to_string(),
                        },
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn widths_are_scaled_into_pdf_text_space() {
        // 1024 design units in a 2048-unit em is half an em, which PDF spells
        // as 500 -- its text space is always 1000 to the em whatever the font
        // says.
        let font = font_with(&[(5, 1024, "a")]);
        let Object::Array(entries) = font.widths_array() else {
            panic!("expected an array");
        };
        let Object::Array(widths) = &entries[1] else {
            panic!("expected a width run");
        };
        assert_eq!(widths[0], Object::Real(500.0));
    }

    #[test]
    fn consecutive_glyphs_share_one_width_run() {
        // `[cid [w w w]]` rather than three separate entries: a few hundred
        // glyphs would otherwise be a few hundred array entries.
        let font = font_with(&[(5, 1024, "a"), (6, 1024, "b"), (7, 2048, "c")]);
        let Object::Array(entries) = font.widths_array() else {
            panic!("expected an array");
        };
        assert_eq!(entries.len(), 2, "one run: {entries:?}");
        assert_eq!(entries[0], Object::Int(5));
    }

    #[test]
    fn a_gap_in_the_glyph_ids_starts_a_new_run() {
        let font = font_with(&[(5, 1024, "a"), (9, 1024, "b")]);
        let Object::Array(entries) = font.widths_array() else {
            panic!("expected an array");
        };
        assert_eq!(entries.len(), 4, "two runs: {entries:?}");
        assert_eq!(entries[0], Object::Int(5));
        assert_eq!(entries[2], Object::Int(9));
    }

    #[test]
    fn the_cmap_maps_glyph_ids_to_utf16() {
        let font = font_with(&[(5, 1024, "a")]);
        let cmap = String::from_utf8(font.to_unicode_cmap()).unwrap();
        assert!(cmap.contains("<0005> <0061>"), "{cmap}");
        assert!(cmap.contains("beginbfchar"), "{cmap}");
        assert!(cmap.contains("endcmap"), "{cmap}");
    }

    #[test]
    fn a_glyph_standing_for_several_characters_maps_to_all_of_them() {
        // A ligature. Recovering "fi" from one glyph is the reason the CMap
        // stores text rather than a single char.
        let font = font_with(&[(5, 1024, "fi")]);
        let cmap = String::from_utf8(font.to_unicode_cmap()).unwrap();
        assert!(cmap.contains("<0005> <00660069>"), "{cmap}");
    }

    #[test]
    fn bfchar_blocks_never_exceed_the_hundred_entry_limit() {
        // A hard limit in the CMap specification. Past it, readers stop at the
        // boundary and text recovery dies partway through the document.
        let glyphs: Vec<(u16, u16, &str)> = (1u16..=250).map(|id| (id, 1000u16, "x")).collect();
        let font = font_with(&glyphs);
        let cmap = String::from_utf8(font.to_unicode_cmap()).unwrap();
        for line in cmap.lines() {
            if let Some(count) = line.strip_suffix(" beginbfchar") {
                let count: usize = count.parse().expect("a count");
                assert!(count <= 100, "block of {count} exceeds the limit");
            }
        }
        assert_eq!(
            cmap.matches("beginbfchar").count(),
            3,
            "250 glyphs -> 3 blocks"
        );
    }

    #[test]
    fn the_subset_tag_is_six_letters_and_stable() {
        let font = font_with(&[(5, 1024, "a"), (6, 1024, "b")]);
        let tagged = font.tagged_name();
        let (tag, name) = tagged.split_once('+').expect("a subset tag");
        assert_eq!(tag.len(), 6, "{tagged}");
        assert!(tag.chars().all(|c| c.is_ascii_uppercase()), "{tagged}");
        assert_eq!(name, "TestFace");
        // Derived from the glyph set, so the same subset always produces the
        // same file -- output stays comparable between runs.
        assert_eq!(
            tagged,
            font_with(&[(5, 1024, "a"), (6, 1024, "b")]).tagged_name()
        );
        assert_ne!(tagged, font_with(&[(5, 1024, "a")]).tagged_name());
    }
}
