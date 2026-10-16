//! Incoming accessibility/link flows must coexist with delivered active quotes.
//! These strengthened controls intentionally require incoming-main model fields.
//! The earlier old-field baseline is retained separately, not a complete proof.
use diagram_ir::{IshikawaConfig, IshikawaLook, WardleyLinkFlow};
use mermaid_parser::parse_wardley;

#[test]
fn current_main_accessibility_and_flow_keep_pipeline_quote_owned_delimiters() {
    let diagram = parse_wardley(
        "wardley-beta\naccTitle: Map\naccDescr: Visible map\ncomponent P [0.5,0.1]\npipeline P {\ncomponent \"α [quoted] \\\"C\\\"\" [0.25]\n}\ncomponent B [0.5,0.75]\nP +> B\nannotation 3,[0.25,0.5] 'A%%[]'\n",
    ).unwrap();
    assert_eq!(diagram.accessibility_title.as_deref(), Some("Map"));
    assert_eq!(
        diagram.accessibility_description.as_deref(),
        Some("Visible map")
    );
    assert_eq!(diagram.nodes.len(), 3);
    assert_eq!(diagram.nodes[1].label, "α [quoted] \"C\"");
    assert_eq!(diagram.pipelines.len(), 1);
    assert_eq!(
        diagram.pipelines[0].component_ids,
        vec![diagram.nodes[1].id.clone()]
    );
    assert_eq!(diagram.links.len(), 1);
    assert_eq!(diagram.links[0].source, "P");
    assert_eq!(diagram.links[0].target, "B");
    assert_eq!(diagram.links[0].flow, Some(WardleyLinkFlow::Forward));
    assert!(!diagram.links[0].dashed);
    assert_eq!(diagram.links[0].label, None);
    assert_eq!(diagram.annotations[0].text, "A%%[]");
    assert_eq!(diagram.annotations[0].coordinates, vec![(0.25, 0.5)]);
    println!("CV04_MAIN_WARDLEY_MIXED accepted");
}

#[test]
fn current_main_accessibility_does_not_hide_malformed_quoted_pipeline_suffix() {
    let error = parse_wardley("wardley-beta\naccTitle: Map\ncomponent P [0.5,0.1]\npipeline P {\ncomponent 'C' junk [0.25]\n}\n").unwrap_err();
    assert_eq!(
        (error.message.as_str(), error.line, error.col),
        ("invalid Wardley pipeline component suffix", 5, 1)
    );
    println!("CV04_MAIN_WARDLEY_SUFFIX refused");
}

#[test]
fn unquoted_unicode_endpoints_and_unsupported_statements_never_slice_inside_utf8() {
    for (operator, flow, dashed, label) in [
        ("+>", Some(WardleyLinkFlow::Forward), false, None),
        ("+<", Some(WardleyLinkFlow::Backward), false, None),
        ("+<>", Some(WardleyLinkFlow::Bidirectional), false, None),
        ("-.->", None, true, None),
        (
            "+'flow'>",
            Some(WardleyLinkFlow::Forward),
            false,
            Some("flow"),
        ),
    ] {
        let diagram = parse_wardley(&format!(
            "wardley-beta\ncomponent α [0.5,0.1]\ncomponent B [0.5,0.75]\nα {operator} B\n"
        ))
        .unwrap();
        assert_eq!(diagram.links.len(), 1);
        let link = &diagram.links[0];
        assert_eq!(
            (
                link.source.as_str(),
                link.target.as_str(),
                link.flow,
                link.dashed,
                link.label.as_deref()
            ),
            ("α", "B", flow, dashed, label)
        );
    }
    let error = parse_wardley("wardley-beta\nα\n").unwrap_err();
    assert_eq!(
        (error.message.as_str(), error.line, error.col),
        ("unsupported Wardley statement: α", 2, 1)
    );
}

#[test]
fn quote_owned_braces_blank_pipeline_lines_and_multiline_accessibility_survive() {
    let source = "wardley-beta\naccDescr {\n\n  first\n  second\n\n}\ncomponent 'P { root' [0.5,0.1]\npipeline 'P { root' {\n\n%% a comment\ncomponent 'C [child]' [0.25]\n\n}\n";
    let diagram = parse_wardley(source).unwrap();
    assert_eq!(
        diagram.accessibility_description.as_deref(),
        Some("first\nsecond")
    );
    assert_eq!(
        diagram
            .nodes
            .iter()
            .map(|node| node.label.as_str())
            .collect::<Vec<_>>(),
        ["P { root", "C [child]"]
    );
    assert_eq!(
        diagram.pipelines[0].component_ids,
        [diagram.nodes[1].id.clone()]
    );
    let source = format!(
        "wardley-beta\naccDescr {{\n{}body\n{}}}\n",
        "\n".repeat(40_000),
        "\n".repeat(40_000)
    );
    assert_eq!(
        parse_wardley(&source)
            .unwrap()
            .accessibility_description
            .as_deref(),
        Some("body")
    );
}

#[test]
fn annotation_theme_and_ishikawa_root_settings_bind_actual_incoming_fields() {
    let wardley = parse_wardley("%%{init: {\"themeVariables\": {\"wardley\": {\"annotationStroke\":\"#123456\",\"annotationTextColor\":\"#654321\",\"annotationFill\":\"#abcdef\"}}}}%%\nwardley-beta\n").unwrap();
    assert_eq!(
        (
            wardley.config.theme.annotation_stroke.as_str(),
            wardley.config.theme.annotation_text_color.as_str(),
            wardley.config.theme.annotation_fill.as_str()
        ),
        ("#123456", "#654321", "#abcdef")
    );
    let diagram = mermaid_parser::parse_ishikawa("%%{init: {\"fontSize\":22,\"look\":\"handdrawn\",\"handDrawnSeed\":7,\"themeVariables\":{\"fontFamily\":\"Georgia\"}}}%%\nishikawa-beta\nE\n  A\n").unwrap();
    assert_eq!(
        diagram.config,
        IshikawaConfig {
            diagram_padding: 20.0,
            use_max_width: false,
            font_size: 22.0,
            font_family: "Georgia".into(),
            line_color: "#333333".into(),
            fill_color: "#ffffff".into(),
            text_color: "#333333".into(),
            look: IshikawaLook::HandDrawn,
            hand_drawn_seed: 7
        }
    );
    assert_eq!(diagram.effect, "E");
    assert_eq!(
        (
            diagram.causes[0].id.as_str(),
            diagram.causes[0].label.as_str(),
            diagram.causes[0].parent_id.as_ref(),
            diagram.causes[0].depth
        ),
        ("cause-1", "A", None, 1)
    );
}
