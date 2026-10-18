//! Whole source IR at the direct root-size boundary; no renderer-size claim.
use diagram_ir::{
    TimelineConfig, TimelineDiagram, TimelineDirection, TimelinePeriod, TimelineSection,
};
use mermaid_parser::parse_timeline;

#[test]
fn timeline_size_owned_complete_source_ir() {
    let yaml = "---\nconfig:\n  fontSize: 20px\n---\n";
    let with_yaml = |directive: &str| [yaml, directive].concat();
    let rows = vec![
        ("default", "".to_string(), 16.0),
        (
            "init_number",
            "%%{init: {\"fontSize\":24}}%%\n".into(),
            24.0,
        ),
        (
            "init_quoted_px",
            "%%{init: {\"fontSize\":\"20px\"}}%%\n".into(),
            20.0,
        ),
        ("yaml", yaml.into(), 20.0),
        (
            "root_absent_member",
            with_yaml("%%{init: {\"timeline\":{}}}%%\n"),
            20.0,
        ),
        ("empty_root", with_yaml("%%{init: {}}%%\n"), 20.0),
        (
            "present_invalid",
            with_yaml("%%{init: {\"fontSize\":\"wrong\"}}%%\n"),
            16.0,
        ),
        (
            "present_empty",
            with_yaml("%%{init: {\"fontSize\":\"\"}}%%\n"),
            16.0,
        ),
        (
            "first_closed_root",
            "%%{init: {}}%%\n%%{init: {\"fontSize\":24}}%%\n".into(),
            16.0,
        ),
        (
            "malformed_first_root",
            "%%{init: {\"fontSize\":99} trailing}%%\n%%{init: {\"fontSize\":24}}%%\n".into(),
            24.0,
        ),
        (
            "nested_theme",
            "%%{init: {\"themeVariables\":{\"fontSize\":24}}}%%\n".into(),
            16.0,
        ),
        (
            "nested_timeline",
            "%%{init: {\"timeline\":{\"fontSize\":24}}}%%\n".into(),
            16.0,
        ),
        (
            "array_decoy",
            "%%{init: {\"extras\":[{\"fontSize\":99}],\"fontSize\":22}}%%\n".into(),
            22.0,
        ),
        (
            "quoted_decoy",
            "%%{init: {\"note\":\"fontSize: 99\",\"fontSize\":22}}%%\n".into(),
            22.0,
        ),
        (
            "duplicate_first",
            "%%{init: {\"fontSize\":12,\"fontSize\":24}}%%\n".into(),
            12.0,
        ),
        ("negative", "%%{init: {\"fontSize\":-1}}%%\n".into(), 16.0),
        ("zero", "%%{init: {\"fontSize\":0}}%%\n".into(), 16.0),
        (
            "negative_zero",
            "%%{init: {\"fontSize\":-0.0}}%%\n".into(),
            16.0,
        ),
        ("nan", "%%{init: {\"fontSize\":\"NaN\"}}%%\n".into(), 16.0),
        (
            "infinity",
            "%%{init: {\"fontSize\":\"inf\"}}%%\n".into(),
            16.0,
        ),
        (
            "negative_infinity",
            "%%{init: {\"fontSize\":\"-inf\"}}%%\n".into(),
            16.0,
        ),
        (
            "overflow",
            "%%{init: {\"fontSize\":1e309}}%%\n".into(),
            16.0,
        ),
        (
            "scaled_overflow",
            "%%{init: {\"fontSize\":\"1.7976931348623157e308rem\"}}%%\n".into(),
            16.0,
        ),
        (
            "smallest_subnormal",
            "%%{init: {\"fontSize\":5e-324}}%%\n".into(),
            f64::from_bits(1),
        ),
        (
            "largest_finite",
            "%%{init: {\"fontSize\":1.7976931348623157e308}}%%\n".into(),
            f64::MAX,
        ),
        (
            "rem",
            "%%{init: {\"fontSize\":\"1.5rem\"}}%%\n".into(),
            24.0,
        ),
        ("em", "%%{init: {\"fontSize\":\"2em\"}}%%\n".into(), 32.0),
        ("ex", "%%{init: {\"fontSize\":\"3ex\"}}%%\n".into(), 24.0),
        ("px", "%%{init: {\"fontSize\":\"12px\"}}%%\n".into(), 12.0),
        ("exponent", "%%{init: {\"fontSize\":1e1}}%%\n".into(), 10.0),
        ("plus", "%%{init: {\"fontSize\":+18}}%%\n".into(), 18.0),
        (
            "upper_unit",
            "%%{init: {\"fontSize\":\"12PX\"}}%%\n".into(),
            16.0,
        ),
        (
            "unknown_unit",
            "%%{init: {\"fontSize\":\"12pt\"}}%%\n".into(),
            16.0,
        ),
        (
            "boolean",
            with_yaml("%%{init: {\"fontSize\":true}}%%\n"),
            16.0,
        ),
        ("null", with_yaml("%%{init: {\"fontSize\":null}}%%\n"), 16.0),
        (
            "array",
            with_yaml("%%{init: {\"fontSize\":[24]}}%%\n"),
            16.0,
        ),
        (
            "object",
            with_yaml("%%{init: {\"fontSize\":{\"value\":24}}}%%\n"),
            16.0,
        ),
        (
            "unsupported_escape",
            "%%{init: {\"fontSize\":\"2\\u0034px\"}}%%\n".into(),
            16.0,
        ),
        (
            "quoted_tail",
            "%%{init: {\"fontSize\":\"24\"invalid}}%%\n".into(),
            16.0,
        ),
        (
            "yaml_top_decoy",
            "---\nfontSize: 99\nconfig:\n  fontSize: 20px\n---\n".into(),
            20.0,
        ),
        (
            "yaml_timeline_decoy",
            "---\nconfig:\n  timeline:\n    fontSize: 99\n---\n".into(),
            16.0,
        ),
        (
            "yaml_theme_decoy",
            "---\nconfig:\n  themeVariables:\n    fontSize: 99\n---\n".into(),
            16.0,
        ),
        (
            "yaml_sequence_decoy",
            "---\nconfig:\n  - fontSize: 99\n---\n".into(),
            16.0,
        ),
        (
            "trimmed",
            "---\nconfig:\n  fontSize: '\u{2003}24 px\t'\n---\n".into(),
            24.0,
        ),
    ];
    assert_eq!(rows.len(), 44);
    for (name, prefix, size) in rows {
        for (header, direction) in [
            ("timeline", TimelineDirection::LeftRight),
            ("timeline LR", TimelineDirection::LeftRight),
            ("timeline TD", TimelineDirection::TopDown),
        ] {
            let source = format!("{prefix}{header}\ntitle T\naccTitle: Accessible\naccDescr {{\nfontSize: Poison\n%%{{init: {{\"fontSize\":88}}}}%%\n}}\nsection S\nP : E\n");
            let expected = (
                Some("T".into()),
                TimelineDiagram {
                    accessibility_title: Some("Accessible".into()),
                    accessibility_description: Some("fontSize: Poison".into()),
                    config: TimelineConfig {
                        font_family: "\"trebuchet ms\", verdana, arial, sans-serif".into(),
                        font_size: size,
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
                "whole size IR {name}:{header}"
            );
            println!("timeline-size-source COMPLETE {name}:{header}");
        }
    }
}
