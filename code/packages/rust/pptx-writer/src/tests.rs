//! Structural tests for the `.pptx` writer.
//!
//! The read side of `.pptx` is not on this branch, so we verify the generated
//! bytes *structurally*: unzip with this repo's `zip::ZipReader` and assert the
//! expected members exist, then parse a slide part with
//! `coding_adventures_xml_parser::parse_xml` and assert its `<a:t>` text nodes
//! carry the right (and only the right) text.

use super::*;
use coding_adventures_xml_parser::{parse_xml, XmlElement, XmlNode};
use zip::ZipReader;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect the member names of a produced `.pptx` archive.
fn member_names(bytes: &[u8]) -> Vec<String> {
    let reader = ZipReader::new(bytes).expect("valid zip");
    reader
        .entries()
        .iter()
        .map(|e| e.name.clone())
        .collect()
}

/// Read a single member's decompressed bytes by name.
fn read_member(bytes: &[u8], name: &str) -> Vec<u8> {
    let reader = ZipReader::new(bytes).expect("valid zip");
    reader
        .read_by_name(name)
        .unwrap_or_else(|e| panic!("member {name} should exist and decode: {e}"))
}

/// Recursively gather the text of every `<a:t>` element (DrawingML namespace).
fn collect_a_t_text(el: &XmlElement, out: &mut Vec<String>) {
    if el.local_name == "t" && el.namespace_uri.as_deref() == Some(A_NS) {
        out.push(el.text_content());
    }
    for child in &el.children {
        if let XmlNode::Element(e) = child {
            collect_a_t_text(e, out);
        }
    }
}

/// The `<a:t>` texts of a slide part, in document order.
fn slide_texts(bytes: &[u8], slide_no: usize) -> Vec<String> {
    let part = read_member(bytes, &format!("ppt/slides/slide{slide_no}.xml"));
    let src = String::from_utf8(part).expect("utf-8 slide xml");
    let doc = parse_xml(&src).expect("slide xml parses");
    let mut texts = Vec::new();
    collect_a_t_text(&doc.root, &mut texts);
    texts
}

// ---------------------------------------------------------------------------
// Model API
// ---------------------------------------------------------------------------

#[test]
fn model_add_slide_and_text() {
    let mut p = Presentation::new();
    let s = p.add_slide();
    s.add_text("one");
    s.add_text("two");
    assert_eq!(p.slides().len(), 1);
    assert_eq!(p.slides()[0].paragraphs(), &["one", "two"]);
}

#[test]
fn model_default_is_empty() {
    let p = Presentation::default();
    assert!(p.slides().is_empty());
}

// ---------------------------------------------------------------------------
// The full scaffold is present
// ---------------------------------------------------------------------------

#[test]
fn produces_valid_zip_signature() {
    let mut p = Presentation::new();
    p.add_slide().add_text("hi");
    let bytes = write_pptx(&p);
    assert_eq!(&bytes[..2], b"PK", "must be a ZIP");
}

#[test]
fn contains_full_scaffold_for_two_slides() {
    let mut p = Presentation::new();
    p.add_slide().add_text("Slide One Title");
    p.add_slide().add_text("Slide Two Title");
    let bytes = write_pptx(&p);
    let names = member_names(&bytes);

    // Every part a strict consumer needs.
    let expected = [
        "[Content_Types].xml",
        "_rels/.rels",
        "ppt/presentation.xml",
        "ppt/_rels/presentation.xml.rels",
        "ppt/slides/slide1.xml",
        "ppt/slides/slide2.xml",
        "ppt/slides/_rels/slide1.xml.rels",
        "ppt/slides/_rels/slide2.xml.rels",
        "ppt/slideLayouts/slideLayout1.xml",
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        "ppt/slideMasters/slideMaster1.xml",
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        "ppt/theme/theme1.xml",
    ];
    for e in expected {
        assert!(names.contains(&e.to_string()), "missing member: {e}");
    }
}

#[test]
fn content_types_declares_every_override() {
    let mut p = Presentation::new();
    p.add_slide();
    p.add_slide();
    let bytes = write_pptx(&p);
    let ct = String::from_utf8(read_member(&bytes, "[Content_Types].xml")).unwrap();

    assert!(ct.contains("/ppt/presentation.xml"));
    assert!(ct.contains("/ppt/slides/slide1.xml"));
    assert!(ct.contains("/ppt/slides/slide2.xml"));
    assert!(ct.contains("/ppt/slideLayouts/slideLayout1.xml"));
    assert!(ct.contains("/ppt/slideMasters/slideMaster1.xml"));
    assert!(ct.contains("/ppt/theme/theme1.xml"));
    // Defaults for rels + xml must be present.
    assert!(ct.contains("Extension=\"rels\""));
    assert!(ct.contains("Extension=\"xml\""));
}

// ---------------------------------------------------------------------------
// Slide text lives in the a: namespace and in the right slide
// ---------------------------------------------------------------------------

#[test]
fn slide_text_is_in_a_namespace_and_correct() {
    let mut p = Presentation::new();
    let s1 = p.add_slide();
    s1.add_text("Slide One Title");
    s1.add_text("First slide body");
    let s2 = p.add_slide();
    s2.add_text("Slide Two Title");
    let bytes = write_pptx(&p);

    let t1 = slide_texts(&bytes, 1);
    assert_eq!(t1, vec!["Slide One Title", "First slide body"]);

    let t2 = slide_texts(&bytes, 2);
    assert_eq!(t2, vec!["Slide Two Title"]);

    // Order guard: slide-2 text must NOT appear in slide-1.
    assert!(
        !t1.iter().any(|s| s.contains("Slide Two")),
        "slide 2 text leaked into slide 1"
    );
}

// ---------------------------------------------------------------------------
// XML escaping of special characters
// ---------------------------------------------------------------------------

#[test]
fn special_characters_are_escaped_in_raw_bytes() {
    // The five XML specials must appear as entity references in the serialized
    // part, never as their literal characters in text position — otherwise the
    // part is not well-formed and no consumer can open it.
    let mut p = Presentation::new();
    p.add_slide()
        .add_text("a & b < c > d \"quote\" 'apos'");
    let bytes = write_pptx(&p);

    let raw = String::from_utf8(read_member(&bytes, "ppt/slides/slide1.xml")).unwrap();
    assert!(raw.contains("&amp;"), "ampersand not escaped");
    assert!(raw.contains("&lt;"), "less-than not escaped");
    assert!(raw.contains("&gt;"), "greater-than not escaped");
    // No BARE special left inside the <a:t> text (the only literal '<'/'>' in the
    // whole part are the element delimiters, so a bare " & " would be a bug).
    assert!(!raw.contains(" & "), "found an unescaped bare ampersand");
}

#[test]
fn escaped_entities_round_trip_through_parser() {
    // A conforming parser decodes the entity references back to the original
    // characters. (We avoid whitespace immediately adjacent to an entity here:
    // this repo's xml-parser collapses a space that borders an entity boundary,
    // which is a parser-lexer quirk unrelated to the writer's escaping.)
    let mut p = Presentation::new();
    p.add_slide().add_text("A&B<C>D\"E\"F");
    let bytes = write_pptx(&p);

    // Escaping happened...
    let raw = String::from_utf8(read_member(&bytes, "ppt/slides/slide1.xml")).unwrap();
    assert!(raw.contains("A&amp;B&lt;C&gt;D"));

    // ...and decodes back to the exact original.
    let texts = slide_texts(&bytes, 1);
    assert_eq!(texts, vec!["A&B<C>D\"E\"F"]);
}

#[test]
fn unicode_text_survives() {
    let mut p = Presentation::new();
    p.add_slide().add_text("日本語 — résumé 🎉");
    let bytes = write_pptx(&p);
    let texts = slide_texts(&bytes, 1);
    assert_eq!(texts, vec!["日本語 — résumé 🎉"]);
}

#[test]
fn illegal_control_chars_are_dropped_not_panicked() {
    // A NUL and other C0 controls are illegal in XML 1.0; xml_escape drops them
    // so the package stays parseable. Tab/newline are legal and kept.
    let mut p = Presentation::new();
    p.add_slide().add_text("a\u{0}b\u{1}c\tok");
    let bytes = write_pptx(&p);
    let texts = slide_texts(&bytes, 1);
    assert_eq!(texts, vec!["abc\tok"]);
}

// ---------------------------------------------------------------------------
// Multiple slides: id / rels alignment
// ---------------------------------------------------------------------------

#[test]
fn multiple_slides_align_sldid_and_rels() {
    let mut p = Presentation::new();
    for _ in 0..3 {
        p.add_slide();
    }
    let bytes = write_pptx(&p);

    let pres = String::from_utf8(read_member(&bytes, "ppt/presentation.xml")).unwrap();
    // sldId ids start at 256 and count up.
    assert!(pres.contains("id=\"256\""));
    assert!(pres.contains("id=\"257\""));
    assert!(pres.contains("id=\"258\""));
    // Each slide has a relationship id rId1..rId3.
    for rid in ["rId1", "rId2", "rId3"] {
        assert!(pres.contains(rid), "presentation.xml missing {rid}");
    }
    // The master takes the next id past the last slide (rId4 here).
    assert!(pres.contains("rId4"), "master rel id rId4 missing");

    // presentation.xml.rels must map rId1..rId3 to slides and rId4 to master.
    let rels =
        String::from_utf8(read_member(&bytes, "ppt/_rels/presentation.xml.rels")).unwrap();
    assert!(rels.contains("slides/slide1.xml"));
    assert!(rels.contains("slides/slide2.xml"));
    assert!(rels.contains("slides/slide3.xml"));
    assert!(rels.contains("slideMasters/slideMaster1.xml"));

    // Exactly 3 slide parts exist (no slide4).
    let names = member_names(&bytes);
    assert!(names.contains(&"ppt/slides/slide3.xml".to_string()));
    assert!(!names.contains(&"ppt/slides/slide4.xml".to_string()));
}

// ---------------------------------------------------------------------------
// Empty deck / empty slide edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_deck_still_valid() {
    let p = Presentation::new();
    let bytes = write_pptx(&p);
    assert_eq!(&bytes[..2], b"PK");

    let names = member_names(&bytes);
    // Scaffold present even with zero slides.
    assert!(names.contains(&"ppt/presentation.xml".to_string()));
    assert!(names.contains(&"ppt/slideMasters/slideMaster1.xml".to_string()));
    assert!(names.contains(&"ppt/theme/theme1.xml".to_string()));
    // No slide parts.
    assert!(!names.contains(&"ppt/slides/slide1.xml".to_string()));

    // presentation.xml parses and has an empty slide-id list.
    let pres = String::from_utf8(read_member(&bytes, "ppt/presentation.xml")).unwrap();
    let doc = parse_xml(&pres).expect("presentation.xml parses");
    let lst = doc
        .root
        .get_child(Some(P_NS), "sldIdLst")
        .expect("sldIdLst present");
    // No <p:sldId> children.
    assert!(lst.get_children(Some(P_NS), "sldId").is_empty());
    // The master id list is still present (deck must reference its master).
    assert!(doc.root.get_child(Some(P_NS), "sldMasterIdLst").is_some());
}

#[test]
fn empty_slide_produces_well_formed_part() {
    let mut p = Presentation::new();
    p.add_slide(); // no text
    let bytes = write_pptx(&p);
    // The slide part still parses (an empty <a:p/> keeps txBody well-formed).
    let part = read_member(&bytes, "ppt/slides/slide1.xml");
    let src = String::from_utf8(part).unwrap();
    let doc = parse_xml(&src).expect("empty slide xml parses");
    assert_eq!(doc.root.local_name, "sld");
    // No <a:t> text nodes.
    let texts = slide_texts(&bytes, 1);
    assert!(texts.is_empty());
}

// ---------------------------------------------------------------------------
// Every part decompresses and parses as XML (well-formedness sweep)
// ---------------------------------------------------------------------------

#[test]
fn every_part_is_well_formed_xml() {
    let mut p = Presentation::new();
    p.add_slide().add_text("content");
    let bytes = write_pptx(&p);
    let reader = ZipReader::new(&bytes).unwrap();
    for entry in reader.entries() {
        let data = reader.read(entry).expect("member decodes");
        let src = String::from_utf8(data).expect("utf-8 part");
        parse_xml(&src).unwrap_or_else(|e| panic!("part {} not well-formed: {:?}", entry.name, e));
    }
}

#[test]
fn theme_has_required_scheme_blocks() {
    let mut p = Presentation::new();
    p.add_slide();
    let bytes = write_pptx(&p);
    let theme = String::from_utf8(read_member(&bytes, "ppt/theme/theme1.xml")).unwrap();
    let doc = parse_xml(&theme).expect("theme parses");
    let elems = doc
        .root
        .get_child(Some(A_NS), "themeElements")
        .expect("themeElements present");
    assert!(elems.get_child(Some(A_NS), "clrScheme").is_some());
    assert!(elems.get_child(Some(A_NS), "fontScheme").is_some());
    assert!(elems.get_child(Some(A_NS), "fmtScheme").is_some());
}

#[test]
fn master_has_clrmap_and_layout_list() {
    let mut p = Presentation::new();
    p.add_slide();
    let bytes = write_pptx(&p);
    let master = String::from_utf8(read_member(&bytes, "ppt/slideMasters/slideMaster1.xml")).unwrap();
    let doc = parse_xml(&master).expect("master parses");
    let clr_map = doc.root.get_child(Some(P_NS), "clrMap").expect("clrMap present");
    // All 12 colour-map attributes must be present.
    for attr in [
        "bg1", "tx1", "bg2", "tx2", "accent1", "accent2", "accent3", "accent4", "accent5",
        "accent6", "hlink", "folHlink",
    ] {
        assert!(
            clr_map.get_attr(None, attr).is_some(),
            "clrMap missing attribute {attr}"
        );
    }
    assert!(doc.root.get_child(Some(P_NS), "sldLayoutIdLst").is_some());
}
