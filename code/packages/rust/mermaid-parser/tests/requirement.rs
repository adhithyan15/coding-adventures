use diagram_ir::{DiagramDirection, RelKind, StructuralKind, StructuralNodeKind};
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

#[test]
fn requirement_preserves_layout_direction() {
    for (keyword, expected) in [
        ("TB", DiagramDirection::Tb),
        ("BT", DiagramDirection::Bt),
        ("LR", DiagramDirection::Lr),
        ("RL", DiagramDirection::Rl),
    ] {
        let source = format!(
            "requirementDiagram\ndirection {keyword}\nrequirement a {{\nid: a\n}}"
        );
        let diagram = parse_requirement_diagram(&source).expect("requirement direction");
        assert_eq!(diagram.direction, Some(expected));
    }
}

#[test]
fn requirement_relationships_preserve_semantics_and_orientation() {
    let source = r#"requirementDiagram
requirement a {
id: a
}
requirement b {
id: b
}
a - contains -> b
a - copies -> b
a - derives -> b
a - satisfies -> b
a - verifies -> b
a - refines -> b
a - traces -> b
a <- copies - b"#;
    let diagram = parse_requirement_diagram(source).expect("relationship families");
    let kinds = diagram
        .relationships
        .iter()
        .map(|relationship| relationship.kind.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            RelKind::Composition,
            RelKind::Association,
            RelKind::Dependency,
            RelKind::Realization,
            RelKind::Dependency,
            RelKind::Inheritance,
            RelKind::Link,
            RelKind::Association,
        ]
    );
    let labels = diagram
        .relationships
        .iter()
        .map(|relationship| relationship.label.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            Some("contains"),
            Some("copies"),
            Some("derives"),
            Some("satisfies"),
            Some("verifies"),
            Some("refines"),
            Some("traces"),
            Some("copies"),
        ]
    );
    assert_eq!(diagram.relationships[7].from, "b");
    assert_eq!(diagram.relationships[7].to, "a");
}
