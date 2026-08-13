use diagram_ir::{TemporalBody, TemporalKind};
use mermaid_parser::{parse_any_mermaid, parse_journey, MermaidDiagram};

const SOURCE: &str = "JoUrNeY\naccTitle: Checkout journey\naccDescr {\n  A native checkout\n  experience\n}\ntitle Checkout<br/>experience\nsection Discover<br>products\nFind<br\t/>product: 5: Alice, Bob\nsection Payment\nPay: 2: Bob";

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
