//! Whole semantic objects keep property overlay and target identity observable.
use diagram_ir::{VennConfig, VennDiagram, VennRegion, VennStyle, VennText};

fn style(fill: &str, stroke: &str, width: f64, color: &str, opacity: f64) -> VennStyle {
    VennStyle { fill: Some(fill.into()), stroke: Some(stroke.into()),
        stroke_width: Some(width), text_color: Some(color.into()), fill_opacity: Some(opacity) }
}

#[test]
fn venn_cascade_complete_ir_preserves_property_order_target_priority_and_absence() {
    // Three ingress forms, two directive orders and both union key orders.
    // Duplicate text/region keys expose the established first-match policy;
    // a text named A must not outrank the set named A. Original invalid numeric
    // fixtures now reject; their valid companions retain the complete oracle.
    for ingress in 0..3 { for reversed in [false, true] { for union in ["A,B", "B,A"] {
        let prefix = match ingress {
            0 => "",
            1 => "%%{init: {\"venn\": {\"width\": 400, \"height\": 300, \"padding\": 25}}}%%\n",
            _ => "---\nconfig:\n  venn:\n    width: 400\n    height: 300\n    padding: 25\n---\n",
        };
        let base = "style A fill:#112233,stroke:#445566,stroke-width:3,color:#778899,fill-opacity:0.4\n";
        let overlay = "style A stroke:#abcdef,stroke-width:0,fill-opacity:0\n";
        let ordered = if reversed { format!("{overlay}{base}") } else { format!("{base}{overlay}") };
        let source = format!("{prefix}venn-beta\nset A[A]: 1\ntext note[N]\ntext A[X]\nset B[B]: 2\nunion B,A[U]: 0.5\nunion A,B[V]: 0.25\ntext pair[P]\ntext note[Q]\n{ordered}style A stroke-width:nope,fill-opacity:nope\nstyle note fill:#123456,stroke:#234567,stroke-width:7,color:#345678,fill-opacity:0.6\nstyle note color:#456789,stroke-width:0,fill-opacity:0\nstyle note stroke-width:nope,fill-opacity:nope\nstyle B fill:#aabbcc\nstyle {union} fill:#556677,color:#667788\nstyle {union} stroke:#778899,stroke-width:0,fill-opacity:0\nstyle pair fill:#8899aa,stroke:#99aabb,stroke-width:2,color:#aabbcc,fill-opacity:0.7\nstyle pair fill:#bbccdd,color:#ccddee\nstyle unknown fill:#ffffff");
        let mut config = VennConfig::default();
        if ingress != 0 { config.width = 400.0; config.height = 300.0; config.padding = 25.0; }
        let a = if reversed { style("#112233", "#445566", 3.0, "#778899", 0.4) }
            else { style("#112233", "#abcdef", 0.0, "#778899", 0.0) };
        let expected = VennDiagram { title: None, config, regions: vec![
            VennRegion { sets: vec!["A".into()], size: 1.0, label: Some("A".into()), style: a },
            VennRegion { sets: vec!["B".into()], size: 2.0, label: Some("B".into()), style: VennStyle { fill: Some("#aabbcc".into()), ..Default::default() } },
            VennRegion { sets: vec!["A".into(), "B".into()], size: 0.5, label: Some("U".into()), style: style("#556677", "#778899", 0.0, "#667788", 0.0) },
            VennRegion { sets: vec!["A".into(), "B".into()], size: 0.25, label: Some("V".into()), style: Default::default() },
        ], texts: vec![
            VennText { sets: vec!["A".into()], id: "note".into(), label: Some("N".into()), style: style("#123456", "#234567", 0.0, "#456789", 0.0) },
            VennText { sets: vec!["A".into()], id: "A".into(), label: Some("X".into()), style: Default::default() },
            VennText { sets: vec!["A".into(), "B".into()], id: "pair".into(), label: Some("P".into()), style: style("#bbccdd", "#99aabb", 2.0, "#ccddee", 0.7) },
            VennText { sets: vec!["A".into(), "B".into()], id: "note".into(), label: Some("Q".into()), style: Default::default() },
        ] };
        let error = mermaid_parser::parse_venn(&source).expect_err(&source);
        assert_eq!((error.message.as_str(), error.line, error.col),
            ("invalid Venn stroke-width", [12, 13, 19][ingress], 1), "{source}");
        // Reverse only the two declared invalid directives. Every original
        // expected field and all other authored source remain unchanged.
        let invalid_region = "style A stroke-width:nope,fill-opacity:nope\n";
        let invalid_text = "style note stroke-width:nope,fill-opacity:nope\n";
        assert_eq!(source.matches(invalid_region).count(), 1);
        assert_eq!(source.matches(invalid_text).count(), 1);
        let source = source.replace(invalid_region, "").replace(invalid_text, "");
        assert_eq!(mermaid_parser::parse_venn(&source).unwrap(), expected, "{source}");
    } } }
}
