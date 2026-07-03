//! Tests for the PresentationML reader.
//!
//! The headline is [`end_to_end_two_slides`], which opens the real
//! DEFLATE-compressed two-slide fixture and checks slide order + text. The rest
//! cover the error variants, the DrawingML namespace switch, and the shape/slide
//! text-joining logic in isolation.

use super::fixture::MINIMAL_PPTX;
use super::*;

// ===========================================================================
// End-to-end: the real two-slide fixture
// ===========================================================================

#[test]
fn end_to_end_two_slides() {
    let pres = open_pptx(MINIMAL_PPTX).expect("fixture must open");

    // Exactly two slides.
    assert_eq!(pres.slide_count(), 2, "expected two slides");
    assert_eq!(pres.slides().len(), 2);

    // Slide order is preserved: slide 0 is slide one, slide 1 is slide two.
    let s0 = pres.slides()[0].text();
    let s1 = pres.slides()[1].text();

    assert!(
        s0.contains("Slide One Title"),
        "slide 0 text should contain the slide-one title, got: {s0:?}"
    );
    assert!(
        s0.contains("First slide body"),
        "slide 0 text should contain the slide-one body, got: {s0:?}"
    );

    assert!(
        s1.contains("Slide Two Title"),
        "slide 1 text should contain the slide-two title, got: {s1:?}"
    );
    assert!(
        s1.contains("Second slide body"),
        "slide 1 text should contain the slide-two body, got: {s1:?}"
    );

    // Order sanity: slide-two strings must NOT appear on slide one and vice
    // versa — this catches an out-of-order resolution bug.
    assert!(!s0.contains("Slide Two Title"));
    assert!(!s1.contains("Slide One Title"));
}

#[test]
fn each_slide_has_titled_and_body_shapes() {
    let pres = open_pptx(MINIMAL_PPTX).unwrap();
    // Each slide should have at least the two text-bearing shapes.
    let slide0 = &pres.slides()[0];
    let texts: Vec<&str> = slide0
        .shapes()
        .iter()
        .map(|s| s.text.as_str())
        .filter(|t| !t.is_empty())
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("Slide One Title")),
        "a shape should carry the title, got shapes: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("First slide body")),
        "a shape should carry the body, got shapes: {texts:?}"
    );
    assert!(slide0.shape_count() >= 2);
}

// ===========================================================================
// Errors
// ===========================================================================

#[test]
fn non_pptx_bytes_are_a_package_error() {
    // Random bytes are not a ZIP → OPC rejects them → we wrap it.
    let err = open_pptx(b"this is definitely not a pptx file").unwrap_err();
    assert!(
        matches!(err, PptxError::Opc(_)),
        "expected an Opc error, got {err:?}"
    );
    // Display should mention it is a package error.
    assert!(format!("{err}").contains("package error"));
}

#[test]
fn empty_bytes_are_a_package_error() {
    let err = open_pptx(b"").unwrap_err();
    assert!(matches!(err, PptxError::Opc(_)));
}

#[test]
fn error_display_strings_are_readable() {
    // Exercise each Display arm so a caller logging errors gets sensible text.
    let cases = [
        (
            PptxError::MissingPresentation,
            "no /ppt/presentation.xml",
        ),
        (PptxError::NotUtf8("/ppt/x.xml".into()), "not valid UTF-8"),
        (PptxError::MalformedXml("boom".into()), "malformed XML"),
        (
            PptxError::MissingSlidePart("rId9".into()),
            "did not resolve",
        ),
    ];
    for (err, needle) in cases {
        let shown = format!("{err}");
        assert!(
            shown.contains(needle),
            "Display of {err:?} = {shown:?} should contain {needle:?}"
        );
    }
}

#[test]
fn error_is_std_error_and_clonable() {
    let err = PptxError::MissingPresentation;
    // std::error::Error object-safety + Clone + PartialEq.
    let _boxed: Box<dyn std::error::Error> = Box::new(err.clone());
    assert_eq!(err.clone(), err);
    assert_eq!(err, PptxError::MissingPresentation);
    assert_ne!(err, PptxError::MalformedXml("x".into()));
}

#[test]
fn opc_error_converts_via_from() {
    // The `?` operator relies on this From impl; exercise it directly.
    let opc_err = coding_adventures_opc::Package::open(b"nope").unwrap_err();
    let pptx_err: PptxError = opc_err.clone().into();
    assert_eq!(pptx_err, PptxError::Opc(opc_err));
}

// ===========================================================================
// The DrawingML namespace switch and text joining, tested on hand-built XML
// ===========================================================================

use coding_adventures_xml_parser::parse_xml;

/// Parse a `<p:sp>` snippet with both namespaces declared and return its shape
/// text via the private [`shape_text`] helper.
fn shape_text_of(sp_xml: &str) -> String {
    let doc = parse_xml(sp_xml).expect("snippet must parse");
    shape_text(&doc.root)
}

const NS_DECL: &str = concat!(
    r#" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#,
    r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
);

#[test]
fn shape_text_reads_from_drawingml_namespace() {
    // The runs are in the a: (DrawingML) namespace. If we mistakenly looked in
    // p:, this would come back empty.
    let sp = format!(
        r#"<p:sp{ns}>
             <p:txBody>
               <a:p><a:r><a:t>Hello</a:t></a:r><a:r><a:t>World</a:t></a:r></a:p>
             </p:txBody>
           </p:sp>"#,
        ns = NS_DECL
    );
    assert_eq!(shape_text_of(&sp), "HelloWorld");
}

#[test]
fn shape_text_joins_paragraphs_with_newline() {
    let sp = format!(
        r#"<p:sp{ns}>
             <p:txBody>
               <a:p><a:r><a:t>Line one</a:t></a:r></a:p>
               <a:p><a:r><a:t>Line two</a:t></a:r></a:p>
             </p:txBody>
           </p:sp>"#,
        ns = NS_DECL
    );
    assert_eq!(shape_text_of(&sp), "Line one\nLine two");
}

#[test]
fn shape_with_no_txbody_is_empty() {
    // A decorative shape (no text body) contributes no text.
    let sp = format!(r#"<p:sp{ns}><p:spPr/></p:sp>"#, ns = NS_DECL);
    assert_eq!(shape_text_of(&sp), "");
}

#[test]
fn shape_with_empty_txbody_is_empty() {
    // A text body with a single empty paragraph collapses to "".
    let sp = format!(
        r#"<p:sp{ns}><p:txBody><a:p/></p:txBody></p:sp>"#,
        ns = NS_DECL
    );
    assert_eq!(shape_text_of(&sp), "");
}

#[test]
fn run_without_t_child_is_skipped() {
    // A run element with no <a:t> (e.g. a line-break run) contributes nothing.
    let sp = format!(
        r#"<p:sp{ns}><p:txBody><a:p><a:r/><a:r><a:t>ok</a:t></a:r></a:p></p:txBody></p:sp>"#,
        ns = NS_DECL
    );
    assert_eq!(shape_text_of(&sp), "ok");
}

// ===========================================================================
// Slide::text joining
// ===========================================================================

#[test]
fn slide_text_joins_nonempty_shapes_only() {
    let slide = Slide {
        shapes: vec![
            Shape {
                text: "Title".into(),
            },
            Shape { text: String::new() }, // decorative, skipped
            Shape {
                text: "Body".into(),
            },
        ],
    };
    // The empty shape must not introduce a blank line.
    assert_eq!(slide.text(), "Title\nBody");
    assert_eq!(slide.shape_count(), 3);
    assert_eq!(slide.shapes().len(), 3);
}

#[test]
fn slide_with_no_text_is_empty_string() {
    let slide = Slide {
        shapes: vec![Shape { text: String::new() }],
    };
    assert_eq!(slide.text(), "");
}

#[test]
fn empty_slide_has_no_shapes() {
    let slide = Slide { shapes: vec![] };
    assert_eq!(slide.shape_count(), 0);
    assert_eq!(slide.text(), "");
}

// ===========================================================================
// Model accessors on the real fixture
// ===========================================================================

#[test]
fn presentation_accessors_agree() {
    let pres = open_pptx(MINIMAL_PPTX).unwrap();
    assert_eq!(pres.slide_count(), pres.slides().len());
    // Debug + Clone derive coverage.
    let cloned = pres.clone();
    assert_eq!(cloned.slide_count(), 2);
    let _ = format!("{cloned:?}");
}
