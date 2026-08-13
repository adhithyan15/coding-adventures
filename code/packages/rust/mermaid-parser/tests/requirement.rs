use diagram_ir::{StructuralKind, StructuralNodeKind};
use mermaid_parser::{parse_any_mermaid, parse_requirement_diagram, MermaidDiagram};

const SOURCE: &str = r#"requirementDiagram
title Checkout requirements
functionalRequirement payment_req {
id: PAY-1
text: "Payment completes securely"
risk: high
verifyMethod: test
}
element checkout_service {
type: service
docref: docs/checkout
}
checkout_service - satisfies -> payment_req"#;

#[test]
fn requirement_core_lowers_to_structural_ir() {
    let diagram = parse_requirement_diagram(SOURCE).expect("requirement diagram");
    assert_eq!(diagram.kind, StructuralKind::Requirement);
    assert_eq!(diagram.title.as_deref(), Some("Checkout requirements"));
    assert_eq!(diagram.nodes.len(), 2);
    assert_eq!(diagram.nodes[0].node_kind, StructuralNodeKind::Requirement);
    assert_eq!(diagram.nodes[1].node_kind, StructuralNodeKind::Element);
    assert_eq!(diagram.relationships[0].from, "checkout_service");
    assert_eq!(diagram.relationships[0].to, "payment_req");
    assert_eq!(diagram.relationships[0].label.as_deref(), Some("satisfies"));
}

#[test]
fn requirement_dispatches_through_structural_pipeline() {
    assert!(matches!(
        parse_any_mermaid(SOURCE).expect("requirement dispatch"),
        MermaidDiagram::Structural(_)
    ));
}
