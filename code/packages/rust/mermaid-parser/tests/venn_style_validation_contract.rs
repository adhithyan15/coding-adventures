//! Style admission is distinct from theme fallback and target resolution.
//! Compare full semantic objects for admitted sources and exact diagnostics for
//! rejected sources. In particular, a later valid directive cannot repair an
//! earlier invalid recognized value, even when its target would be ignored.
use diagram_ir::{VennConfig, VennDiagram, VennRegion, VennStyle, VennText};

const INGRESS: [(&str, usize); 3] = [
    ("", 0),
    ("%%{init: {\"venn\": {}}}%%\n", 1),
    ("---\nconfig: {}\n---\n", 3),
];

fn expected(style: VennStyle, text_target: bool) -> VennDiagram {
    VennDiagram { title: None, config: VennConfig::default(), regions: vec![
        VennRegion { sets: vec!["A".into()], size: 1.0, label: Some("A".into()),
            style: if text_target { VennStyle::default() } else { style.clone() } },
    ], texts: vec![VennText { sets: vec!["A".into()], id: "n".into(), label: Some("N".into()),
        style: if text_target { style } else { VennStyle::default() } }] }
}

fn reject(property: &str, value: &str, message: &str) {
    let valid = match property { "stroke-width" => "2", "fill-opacity" => "0.5", _ => "#112233" };
    for (prefix, offset) in INGRESS { for target in ["A", "n", "unknown"] { for sequence in 0..3 {
        let before = if sequence == 1 { format!("style {target} {property}:{valid}\n") } else { String::new() };
        let after = if sequence == 2 { format!("\nstyle {target} {property}:{valid}") } else { String::new() };
        let source = format!("{prefix}venn-beta\nset A[A]: 1\ntext n[N]\n{before}style {target} {property}:{value}{after}");
        let error = mermaid_parser::parse_venn(&source).expect_err(&source);
        assert_eq!((error.message.as_str(), error.line, error.col),
            (message, offset + 4 + usize::from(sequence == 1), 1), "{source}");
    } } }
}

const BAD_COLORS: [&str; 6] = ["url(evil)", "var(evil)", "expression(1)", "rgb(1,2,var(x))", "#12", "\"\""];

#[test]
fn venn_style_rejects_invalid_fill_at_the_eager_source_boundary() {
    for value in BAD_COLORS { reject("fill", value, "invalid Venn fill color"); }
}
#[test]
fn venn_style_rejects_invalid_stroke_at_the_eager_source_boundary() {
    for value in BAD_COLORS { reject("stroke", value, "invalid Venn stroke color"); }
}
#[test]
fn venn_style_rejects_invalid_text_color_at_the_eager_source_boundary() {
    for value in BAD_COLORS { reject("color", value, "invalid Venn color color"); }
}
#[test]
fn venn_style_rejects_invalid_width_independently_from_opacity() {
    for value in ["-0.1", "NaN", "inf", "+inf", "-inf", "1e309", "oops", "\"\""] {
        reject("stroke-width", value, "invalid Venn stroke-width");
    }
}
#[test]
fn venn_style_rejects_invalid_opacity_independently_from_width() {
    for value in ["-0.1", "1.01", "NaN", "inf", "-inf", "1e309", "oops", "\"\""] {
        reject("fill-opacity", value, "invalid Venn fill-opacity");
    }
}

#[test]
fn venn_style_admits_the_declared_lexical_color_subset_as_complete_ir() {
    // This is lexical admission, not a CSS name dictionary or arity validator.
    // None/transparent, shorthand/alpha hex and numeric functions retain spelling.
    let colors = ["black", "none", "transparent", "#123", "#1234", "#112233", "#11223344",
        "rgb(1,2,3)", "rgba(10,20,30,0.5)", "hsl(120 50% 25%)", "hwb(1 2% 3%)",
        "lab(1 2 3)", "lch(1 2 3)", "oklab(1 2 3)", "oklch(1 2 3)"];
    for (prefix, _) in INGRESS { for text_target in [false, true] { for property in ["fill", "stroke", "color"] {
        for color in colors {
            let target = if text_target { "n" } else { "A" };
            let source = format!("{prefix}venn-beta\nset A[A]: 1\ntext n[N]\nstyle {target} {property}:\"{color}\"");
            let mut style = VennStyle::default();
            match property { "fill" => style.fill = Some(color.into()), "stroke" => style.stroke = Some(color.into()), _ => style.text_color = Some(color.into()) }
            assert_eq!(mermaid_parser::parse_venn(&source).unwrap(), expected(style, text_target), "{source}");
        }
    } } }
}

#[test]
fn venn_style_numeric_endpoints_and_existing_quote_unit_conventions_keep_complete_ir() {
    for (prefix, _) in INGRESS { for text_target in [false, true] {
        let target = if text_target { "n" } else { "A" };
        for (property, spelling, number) in [
            ("stroke-width", "0", 0.0), ("stroke-width", "-0", -0.0),
            ("stroke-width", "0.5", 0.5), ("stroke-width", "2px", 2.0),
            ("stroke-width", "1e308", 1e308), ("stroke-width", "2pxpx", 2.0),
            ("fill-opacity", "0", 0.0), ("fill-opacity", "-0", -0.0),
            ("fill-opacity", "1", 1.0), ("fill-opacity", "0.5", 0.5),
            ("fill-opacity", "5e-1", 0.5),
        ] {
            let source = format!("{prefix}venn-beta\nset A[A]: 1\ntext n[N]\nstyle {target} {property}:\"{spelling}\"");
            let mut style = VennStyle::default();
            if property == "stroke-width" { style.stroke_width = Some(number); } else { style.fill_opacity = Some(number); }
            let parsed = mermaid_parser::parse_venn(&source).unwrap();
            assert_eq!(parsed, expected(style, text_target), "{source}");
            let actual = if text_target { &parsed.texts[0].style } else { &parsed.regions[0].style };
            assert_eq!(if property == "stroke-width" { actual.stroke_width } else { actual.fill_opacity }.unwrap().to_bits(), number.to_bits(), "{source}");
        }
        let source = format!("{prefix}venn-beta\nset A[A]: 1\ntext n[N]\nstyle {target} FiLl : \"#123456\",StRoKe-WiDtH:2px,fill-opacity:0.4,unrecognized:ignored\nstyle {target} fill:#abcdef,color:#112233\nstyle {target} fill-opacity:0");
        let style = VennStyle { fill: Some("#abcdef".into()), stroke_width: Some(2.0),
            text_color: Some("#112233".into()), fill_opacity: Some(0.0), ..Default::default() };
        assert_eq!(mermaid_parser::parse_venn(&source).unwrap(), expected(style, text_target), "{source}");
    } }
}
