//! A PDF writer built from scratch, with no third-party dependencies.
//!
//! This is PDF-1 of the from-scratch PDF effort: the **object model** and the
//! **file structure**. There are no pages, text, fonts, or graphics here yet —
//! those sit on top, and this layer has to be right first, because everything
//! above it is expressed in these eight object types and located by the byte
//! offsets this layer computes.
//!
//! ## Why start here, and why it is mostly counting
//!
//! A PDF is opened from the *back*. A reader seeks to the end, reads
//! `startxref` to find the cross-reference table, and then jumps directly to
//! each object's recorded byte offset. Nothing is scanned for. So the entire
//! validity of a PDF rests on a set of offsets being exactly right, and the
//! failure mode is not degradation — a reader that lands one byte off lands
//! mid-token.
//!
//! ## Example
//!
//! ```
//! use pdf::{dict, Dict, Object, PdfWriter};
//!
//! let mut w = PdfWriter::new();
//!
//! // Pages must name its kids, and each page must name its parent, so one of
//! // the two ids has to exist before its object does.
//! let pages = w.reserve();
//! let page = w.add(Object::Dict(dict! {
//!     "Type"      => Object::name("Page"),
//!     "Parent"    => Object::Ref(pages),
//!     "MediaBox"  => Object::Array(vec![
//!         Object::Int(0), Object::Int(0), Object::Int(612), Object::Int(792),
//!     ]),
//! }));
//! w.fill(pages, Object::Dict(dict! {
//!     "Type"  => Object::name("Pages"),
//!     "Kids"  => Object::Array(vec![Object::Ref(page)]),
//!     "Count" => Object::Int(1),
//! }));
//! let root = w.add(Object::Dict(dict! {
//!     "Type"  => Object::name("Catalog"),
//!     "Pages" => Object::Ref(pages),
//! }));
//!
//! let bytes = w.finish(root).unwrap();
//! assert!(bytes.starts_with(b"%PDF-1.7"));
//! ```
//!
//! ## Verification
//!
//! Correctness here cannot be established by reading our own output back —
//! that is the circularity this repository has been bitten by more than once.
//! The tests therefore run **`qpdf --check`**, an independent implementation,
//! over every PDF produced. `tests/qpdf_gate.rs` **fails** when `qpdf` is
//! absent rather than skipping, because a test that silently passes when its
//! oracle is missing is not a test.

mod content;
mod embed;
mod object;
mod page;
mod writer;

pub use content::{ColorTarget, Content, Paint, Space, TextRun};
pub use embed::{EmbeddedFont, EmbeddedGlyph};
pub use object::{format_real, Dict, ObjId, Object};
pub use page::{Document, FontResource, Page, StandardFont, A4, LETTER};
pub use writer::{flate_encode, PdfError, PdfWriter};

#[cfg(test)]
mod tests {
    use super::*;

    /// Find the first occurrence of `needle` in `hay`.
    ///
    /// These tests work on **bytes**, never on a `String`. The binary marker on
    /// line two is deliberately not valid UTF-8, so `from_utf8` fails outright
    /// — and `from_utf8_lossy` would be worse than useless here, because it
    /// substitutes a three-byte replacement character and would silently shift
    /// every offset these tests exist to verify.
    fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
        hay.windows(needle.len()).position(|w| w == needle)
    }

    fn to_string(object: &Object) -> String {
        let mut out = Vec::new();
        object.write(&mut out);
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn scalars_serialise_in_pdf_syntax() {
        assert_eq!(to_string(&Object::Null), "null");
        assert_eq!(to_string(&Object::Bool(true)), "true");
        assert_eq!(to_string(&Object::Bool(false)), "false");
        assert_eq!(to_string(&Object::Int(-42)), "-42");
        assert_eq!(to_string(&Object::name("Type")), "/Type");
        assert_eq!(
            to_string(&Object::Ref(ObjId::new(7))),
            "7 0 R",
            "an indirect reference is what turns the object list into a graph"
        );
    }

    #[test]
    fn reals_never_use_exponent_notation() {
        // PDF has no exponent syntax: `1e-7` is a syntax error, not a number.
        // Rust's default float formatting reaches for it on small magnitudes,
        // so this is a real hazard rather than a stylistic preference.
        assert_eq!(format_real(0.0000001), "0");
        assert_eq!(
            format_real(1e20),
            "100000000000000000000.0".trim_end_matches(".0")
        );
        assert!(!format_real(0.000000123).contains('e'));
        assert!(!format_real(1.5e10).contains('e'));
    }

    #[test]
    fn reals_are_trimmed_but_stay_exact_for_integers() {
        assert_eq!(format_real(1.0), "1");
        assert_eq!(format_real(-0.0), "0");
        assert_eq!(format_real(1.5), "1.5");
        assert_eq!(format_real(1.250000), "1.25");
    }

    #[test]
    fn non_finite_reals_become_zero_rather_than_breaking_the_file() {
        // Neither is expressible in PDF. Emitting `inf` would produce a file no
        // reader accepts; zero is the least surprising coordinate.
        assert_eq!(format_real(f64::INFINITY), "0");
        assert_eq!(format_real(f64::NAN), "0");
    }

    #[test]
    fn literal_strings_escape_parentheses_and_backslash() {
        // An unescaped `)` would end the string early and the rest of the
        // object would be reinterpreted as syntax.
        assert_eq!(to_string(&Object::Str(b"a(b)c".to_vec())), "(a\\(b\\)c)");
        assert_eq!(
            to_string(&Object::Str(b"back\\slash".to_vec())),
            "(back\\\\slash)"
        );
    }

    #[test]
    fn hex_strings_carry_binary() {
        assert_eq!(
            to_string(&Object::HexStr(vec![0x00, 0xFF, 0x10])),
            "<00FF10>"
        );
    }

    #[test]
    fn names_escape_delimiters() {
        // A raw `/` inside a name would start a second name; a raw space would
        // end it. Both silently change the meaning of the dictionary.
        assert_eq!(to_string(&Object::name("A B")), "/A#20B");
        assert_eq!(to_string(&Object::name("a/b")), "/a#2Fb");
        assert_eq!(to_string(&Object::name("100%")), "/100#25");
        assert_eq!(to_string(&Object::name("Plain")), "/Plain");
    }

    #[test]
    fn dictionaries_keep_insertion_order() {
        // PDF does not care, but stable output is diffable — which matters when
        // the thing being debugged is a byte offset.
        let d = dict! {
            "First"  => Object::Int(1),
            "Second" => Object::Int(2),
        };
        assert_eq!(to_string(&Object::Dict(d)), "<< /First 1 /Second 2 >>");
    }

    #[test]
    fn setting_an_existing_key_replaces_in_place() {
        let mut d = Dict::new();
        d.set("A", Object::Int(1));
        d.set("B", Object::Int(2));
        d.set("A", Object::Int(9));
        assert_eq!(d.len(), 2);
        assert_eq!(to_string(&Object::Dict(d)), "<< /A 9 /B 2 >>");
    }

    #[test]
    fn arrays_nest() {
        let a = Object::Array(vec![
            Object::Int(1),
            Object::Array(vec![Object::name("Two")]),
        ]);
        assert_eq!(to_string(&a), "[1 [/Two]]");
    }

    #[test]
    fn an_unfilled_reservation_is_refused_rather_than_written_as_null() {
        // Writing `null` would produce a structurally valid file that means
        // something different from what the caller built.
        let mut w = PdfWriter::new();
        let orphan = w.reserve();
        let root = w.add(Object::Dict(dict! { "Type" => Object::name("Catalog") }));
        assert_eq!(w.finish(root), Err(PdfError::UnfilledObject(orphan.number)));
    }

    #[test]
    fn a_root_from_another_writer_is_refused() {
        let mut w = PdfWriter::new();
        w.add(Object::Null);
        assert_eq!(w.finish(ObjId::new(99)), Err(PdfError::UnknownRoot(99)));
    }

    #[test]
    fn stream_length_is_derived_from_the_data_not_the_dictionary() {
        // A caller-supplied /Length that disagrees with the bytes yields a file
        // some readers accept and others reject.
        let mut w = PdfWriter::new();
        let stream = w.add(Object::Stream {
            dict: dict! { "Length" => Object::Int(9999) },
            data: b"hello".to_vec(),
        });
        let root = w.add(Object::Dict(dict! {
            "Type"  => Object::name("Catalog"),
            "Pages" => Object::Ref(stream),
        }));
        let bytes = w.finish(root).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Length 5"), "got: {text}");
        assert!(!text.contains("9999"));
    }

    #[test]
    fn xref_entries_are_exactly_twenty_bytes() {
        // Readers index the table by multiplying, so a shorter line silently
        // shifts every entry after it.
        let mut w = PdfWriter::new();
        let root = w.add(Object::Dict(dict! { "Type" => Object::name("Catalog") }));
        let bytes = w.finish(root).unwrap();

        let table = find(&bytes, b"xref\n").expect("xref table") + 5;
        let after_subsection = table + find(&bytes[table..], b"\n").unwrap() + 1;

        // Two entries: the free-list head, and our one object.
        for index in 0..2 {
            let start = after_subsection + index * 20;
            let entry = &bytes[start..start + 20];
            assert_eq!(
                entry[19], b'\n',
                "entry {index} must be 20 bytes: {entry:?}"
            );
            assert!(
                entry[10] == b' ' && entry[16] == b' ',
                "entry {index} must be nnnnnnnnnn ggggg t: {entry:?}"
            );
        }
    }

    #[test]
    fn startxref_points_at_the_xref_keyword() {
        // The single most important offset in the file: a reader seeks here
        // first, and nothing is scanned for.
        let mut w = PdfWriter::new();
        let root = w.add(Object::Dict(dict! { "Type" => Object::name("Catalog") }));
        let bytes = w.finish(root).unwrap();

        let marker = find(&bytes, b"startxref\n").expect("startxref") + 10;
        let end = marker + find(&bytes[marker..], b"\n").unwrap();
        let declared: usize = std::str::from_utf8(&bytes[marker..end])
            .unwrap()
            .trim()
            .parse()
            .unwrap();

        assert_eq!(
            &bytes[declared..declared + 4],
            b"xref",
            "startxref must land exactly on the xref keyword"
        );
    }

    #[test]
    fn recorded_offsets_land_on_their_object_headers() {
        // The other half of the same property, and the one that breaks when
        // someone inserts a byte into the body without updating the table.
        let mut w = PdfWriter::new();
        let a = w.add(Object::Int(1));
        let b = w.add(Object::Str(b"two".to_vec()));
        let root = w.add(Object::Dict(dict! {
            "Type" => Object::name("Catalog"),
            "A"    => Object::Ref(a),
            "B"    => Object::Ref(b),
        }));
        let bytes = w.finish(root).unwrap();

        let table = find(&bytes, b"xref\n").expect("xref table") + 5;
        let after_subsection = table + find(&bytes[table..], b"\n").unwrap() + 1;

        for index in 0..3 {
            // Skip the free-list head, then take each object's entry.
            let start = after_subsection + (index + 1) * 20;
            let offset: usize = std::str::from_utf8(&bytes[start..start + 10])
                .unwrap()
                .parse()
                .unwrap();
            let expected = format!("{} 0 obj", index + 1);
            assert!(
                bytes[offset..].starts_with(expected.as_bytes()),
                "object {} offset {offset} should start {expected:?}",
                index + 1
            );
        }
    }

    #[test]
    fn the_binary_marker_is_present() {
        // Without high bytes on line two, tools that sniff text-vs-binary can
        // translate line endings and corrupt every offset in the file.
        let mut w = PdfWriter::new();
        let root = w.add(Object::Dict(dict! { "Type" => Object::name("Catalog") }));
        let bytes = w.finish(root).unwrap();
        assert!(bytes[9..15].iter().any(|b| *b > 127));
    }

    #[test]
    fn flate_encode_emits_a_zlib_container_not_raw_deflate() {
        // The distinction that broke the first version of this crate. PDF's
        // FlateDecode is RFC 1950 zlib: two header bytes, the deflate payload,
        // and a four-byte Adler-32. ZIP method 8 is the bare deflate stream.
        // Our own reader inflated the bare form back happily; qpdf reported
        // `unknown compression method`, because it read the first deflate byte
        // as the zlib CMF.
        let data = b"the quick brown fox ".repeat(50);
        let (encoded, filter) = flate_encode(&data);
        assert_eq!(filter, Object::name("FlateDecode"));

        assert_eq!(encoded[0], 0x78, "zlib CMF: deflate, 32 KiB window");
        assert_eq!(
            (u16::from_be_bytes([encoded[0], encoded[1]])) % 31,
            0,
            "the zlib header carries its own check constraint"
        );

        // The payload between header and checksum is what our deflate produced.
        let payload = &encoded[2..encoded.len() - 4];
        let decoded = zip::raw_inflate(payload, 1 << 20).unwrap();
        assert_eq!(decoded, data);

        // And the trailer is the Adler-32 of the *original* bytes.
        let trailer = u32::from_be_bytes(encoded[encoded.len() - 4..].try_into().unwrap());
        let (mut s1, mut s2) = (1u32, 0u32);
        for &byte in data.iter() {
            s1 = (s1 + byte as u32) % 65521;
            s2 = (s2 + s1) % 65521;
        }
        assert_eq!(trailer, (s2 << 16) | s1);
    }
}
