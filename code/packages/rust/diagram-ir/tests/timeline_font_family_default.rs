//! The ABI bootstrap declares every Timeline default before the field exists.
use diagram_ir::TimelineConfig;

#[test]
fn complete_timeline_defaults_include_the_opaque_font_family() {
    assert_eq!(
        TimelineConfig::default(),
        TimelineConfig {
            padding: 50.0,
            left_margin: 50.0,
            font_family: "\"trebuchet ms\", verdana, arial, sans-serif".into(),
            use_max_width: false,
            disable_multicolor: false,
            section_colors: ["#dbeafe", "#dcfce7", "#fef3c7", "#fae8ff", "#ffe4e6", "#cffafe"]
                .map(str::to_string)
                .to_vec(),
            section_label_colors: vec!["#0f172a".into(); 6],
            section_inverse_colors: [
                "#2563eb", "#16a34a", "#d97706", "#a21caf", "#e11d48", "#0891b2"
            ]
            .map(str::to_string)
            .to_vec(),
        }
    );
}
