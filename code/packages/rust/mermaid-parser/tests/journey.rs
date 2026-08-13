use diagram_ir::{TemporalBody, TemporalKind};
use mermaid_parser::{parse_any_mermaid, parse_journey, MermaidDiagram};

const SOURCE: &str = "JoUrNeY\ntitle Checkout experience\nsection Discovery\nFind product: 5: Alice, Bob\nsection Payment\nPay: 2: Bob";

#[test]
fn journey_core_grammar_lowers_to_typed_ir() {
    let (title, journey) = parse_journey(SOURCE).expect("journey should parse");
    assert_eq!(title.as_deref(), Some("Checkout experience"));
    assert_eq!(journey.sections.len(), 2);
    assert_eq!(journey.sections[0].label, "Discovery");
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
