//! Complete source/IR ownership cases. Family values are opaque allowlisted text.
use diagram_ir::{
    TimelineConfig, TimelineDiagram, TimelineDirection, TimelinePeriod, TimelineSection,
};
use mermaid_parser::parse_timeline;

const DEFAULT_FAMILY: &str = "\"trebuchet ms\", verdana, arial, sans-serif";

#[test]
fn timeline_family_owned_complete_source_ir() {
    let yaml = "---\nthemeVariables:\n  fontFamily: YAML Font\n---\n";
    let with_yaml = |directive: &str| [yaml, directive].concat();
    let rows = vec![
        ("default", "".to_string(), DEFAULT_FAMILY),
        ("init", "%%{init: {\"themeVariables\":{\"fontFamily\":\"Theme Font\"}}}%%\n".into(), "Theme Font"),
        ("yaml", yaml.into(), "YAML Font"),
        ("nested_yaml", "---\nconfig:\n  themeVariables:\n    fontFamily: Nested Font\n---\n".into(), DEFAULT_FAMILY),
        ("top_yaml_companion", "---\nconfig:\n  themeVariables:\n    fontFamily: Nested Font\nthemeVariables:\n  fontFamily: Top Font\n---\n".into(), "Top Font"),
        ("trim", "---\nthemeVariables:\n  fontFamily: '\u{2003}Georgia\t'\n---\n".into(), "Georgia"),
        ("empty_theme", with_yaml("%%{init: {\"themeVariables\":{}}}%%\n"), DEFAULT_FAMILY),
        ("invalid_theme", with_yaml("%%{init: {\"themeVariables\":false}}%%\n"), DEFAULT_FAMILY),
        ("absent_theme", with_yaml("%%{init: {\"timeline\":{}}}%%\n"), "YAML Font"),
        ("first_root", with_yaml("%%{init: {}}%%\n%%{init: {\"themeVariables\":{\"fontFamily\":\"Later Font\"}}}%%\n"), "YAML Font"),
        ("nested_theme", with_yaml("%%{init: {\"themeVariables\":{\"child\":{\"fontFamily\":\"Nested Font\"}}}}%%\n"), DEFAULT_FAMILY),
        ("timeline_member", "%%{init: {\"timeline\":{\"fontFamily\":\"Wrong Family\"}}}%%\n".into(), DEFAULT_FAMILY),
        ("root_member", "%%{init: {\"fontFamily\":\"Wrong Family\"}}%%\n".into(), DEFAULT_FAMILY),
        ("quoted_decoy", "%%{init: {\"themeVariables\":{\"note\":\"fontFamily: wrong\",\"fontFamily\":\"Right Font\"}}}%%\n".into(), "Right Font"),
        ("duplicate_first", "%%{init: {\"themeVariables\":{\"fontFamily\":\"First Font\",\"fontFamily\":\"Later Font\"}}}%%\n".into(), "First Font"),
        ("boolean_text", "%%{init: {\"themeVariables\":{\"fontFamily\":true}}}%%\n".into(), "true"),
        ("null_text", "%%{init: {\"themeVariables\":{\"fontFamily\":null}}}%%\n".into(), "null"),
        ("numeric_text", "%%{init: {\"themeVariables\":{\"fontFamily\":12}}}%%\n".into(), "12"),
        ("comma_text", "---\nthemeVariables:\n  fontFamily: ','\n---\n".into(), ","),
        ("unmatched_inner_quote", "---\nthemeVariables:\n  fontFamily: 'Font \" Open'\n---\n".into(), "Font \" Open"),
        ("punctuation", "---\nthemeVariables:\n  fontFamily: 'A_1-B.C, D'\n---\n".into(), "A_1-B.C, D"),
        ("empty", "---\nthemeVariables:\n  fontFamily: ''\n---\n".into(), DEFAULT_FAMILY),
        ("semicolon", "---\nthemeVariables:\n  fontFamily: 'A;B'\n---\n".into(), DEFAULT_FAMILY),
        ("function", "---\nthemeVariables:\n  fontFamily: 'url(font)'\n---\n".into(), DEFAULT_FAMILY),
        ("unicode", "---\nthemeVariables:\n  fontFamily: '東京'\n---\n".into(), DEFAULT_FAMILY),
        ("interior_tab", "---\nthemeVariables:\n  fontFamily: 'A\tB'\n---\n".into(), DEFAULT_FAMILY),
        ("nul", "---\nthemeVariables:\n  fontFamily: 'A\0B'\n---\n".into(), DEFAULT_FAMILY),
        ("array", "%%{init: {\"themeVariables\":{\"fontFamily\":[\"Wrong Family\"]}}}%%\n".into(), DEFAULT_FAMILY),
    ];
    assert_eq!(rows.len(), 28);
    for (name, prefix, family) in rows {
        for (header, direction) in [
            ("timeline", TimelineDirection::LeftRight),
            ("timeline LR", TimelineDirection::LeftRight),
            ("timeline TD", TimelineDirection::TopDown),
        ] {
            let body = "\ntitle T\naccTitle: Accessible\naccDescr {\nfontFamily: Poison\n%%{init: {\"themeVariables\":{\"fontFamily\":\"Owned Poison\"}}}%%\n}\nsection S\nP : E\n";
            let source = format!("{prefix}{header}{body}");
            let expected = (
                Some("T".into()),
                TimelineDiagram {
                    accessibility_title: Some("Accessible".into()),
                    accessibility_description: Some("fontFamily: Poison".into()),
                    config: TimelineConfig {
                        font_family: family.into(),
                        padding: 50.0,
                        left_margin: 50.0,
                        use_max_width: false,
                        disable_multicolor: false,
                        section_colors: [
                            "#dbeafe", "#dcfce7", "#fef3c7", "#fae8ff", "#ffe4e6", "#cffafe",
                        ]
                        .map(str::to_string)
                        .to_vec(),
                        section_label_colors: vec!["#0f172a".into(); 6],
                        section_inverse_colors: [
                            "#2563eb", "#16a34a", "#d97706", "#a21caf", "#e11d48", "#0891b2",
                        ]
                        .map(str::to_string)
                        .to_vec(),
                    },
                    direction,
                    sections: vec![TimelineSection {
                        label: Some("S".into()),
                        periods: vec![TimelinePeriod {
                            label: "P".into(),
                            events: vec!["E".into()],
                        }],
                    }],
                },
            );
            assert_eq!(
                parse_timeline(&source).unwrap(),
                expected,
                "whole IR {name}:{header}"
            );
            println!("timeline-family-source COMPLETE {name}:{header}");
        }
    }
}
