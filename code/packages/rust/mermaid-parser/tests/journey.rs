use diagram_ir::{TemporalBody, TemporalKind};
use mermaid_parser::{parse_any_mermaid, parse_journey, MermaidDiagram};

const SOURCE: &str = "%%{init: {\"journey\": {\"diagramMarginX\": 24, \"diagramMarginY\": 12, \"width\": 280, \"height\": 52, \"taskMargin\": 18, \"taskFontSize\": \"18px\", \"taskFontFamily\": \"Avenir Next\", \"titleFontSize\": \"22px\", \"titleFontFamily\": \"Georgia\", \"titleColor\": \"#123456\", \"actorColours\": [\"#010203\", \"#040506\"], \"sectionFills\": [\"#112233\", \"#445566\"], \"sectionColours\": [\"#fefefe\"]}}}%%\nJoUrNeY\naccTitle: Checkout journey\naccDescr {\n  A native checkout\n  experience\n}\ntitle Checkout<br/>experience\nsection Discover<br>products\nFind<br\t/>product: 5: Alice, Bob\nsection Payment\nPay: 2: Bob";

#[test]
fn journey_core_grammar_lowers_to_typed_ir() {
    let (title, journey) = parse_journey(SOURCE).expect("journey should parse");
    assert_eq!(title.as_deref(), Some("Checkout\nexperience"));
    assert_eq!(journey.accessibility_title.as_deref(), Some("Checkout journey"));
    assert_eq!(
        journey.accessibility_description.as_deref(),
        Some("A native checkout\nexperience")
    );
    assert_eq!(journey.sections.len(), 2);
    assert_eq!(journey.sections[0].label, "Discover\nproducts");
    assert_eq!(journey.sections[0].tasks[0].label, "Find\nproduct");
    assert_eq!(journey.sections[0].tasks[0].score, 5);
    assert_eq!(journey.sections[0].tasks[0].people, ["Alice", "Bob"]);
    assert_eq!(journey.config.diagram_margin_x, Some(24.0));
    assert_eq!(journey.config.diagram_margin_y, Some(12.0));
    assert_eq!(journey.config.task_width, Some(280.0));
    assert_eq!(journey.config.task_height, Some(52.0));
    assert_eq!(journey.config.task_margin, Some(18.0));
    assert_eq!(journey.config.task_font_size, Some(18.0));
    assert_eq!(journey.config.task_font_family.as_deref(), Some("Avenir Next"));
    assert_eq!(journey.config.title_font_size, Some(22.0));
    assert_eq!(journey.config.title_font_family.as_deref(), Some("Georgia"));
    assert_eq!(journey.config.title_color.as_deref(), Some("#123456"));
    assert_eq!(journey.config.actor_colors, ["#010203", "#040506"]);
    assert_eq!(journey.config.section_fills, ["#112233", "#445566"]);
    assert_eq!(journey.config.section_colors, ["#fefefe"]);
}

#[test]
fn journey_dispatches_through_temporal_ir() {
    let MermaidDiagram::Temporal(diagram) = parse_any_mermaid(SOURCE).expect("journey dispatch")
    else {
        panic!("journey must use temporal pipeline");
    };
    assert_eq!(diagram.kind, TemporalKind::Journey);
    assert!(matches!(diagram.body, TemporalBody::Journey(_)));
}

#[test]
fn journey_rejects_scores_outside_mermaids_one_to_five_domain() {
    for score in ["0", "6", "15"] {
        let source = format!("journey\nsection Work\nTask: {score}: Me");
        assert!(parse_journey(&source).is_err(), "score {score}");
    }
}

#[test]
fn journey_normalizes_css_relative_font_sizes() {
    let (_, journey) = parse_journey(
        "%%{init: {\"journey\": {\"titleFontSize\": \"4ex\", \"taskFontSize\": \"1.25rem\"}}}%%\njourney",
    )
    .expect("relative font sizes should parse");
    assert_eq!(journey.config.title_font_size, Some(32.0));
    assert_eq!(journey.config.task_font_size, Some(20.0));
}
