use diagram_ir::{
    DiagramDirection, RelKind, RequirementKind, RequirementRisk, RequirementVerifyMethod,
    StructuralKind, StructuralNodeKind, StructuralNodeMetadata,
};
use mermaid_parser::{parse_any_mermaid, parse_requirement_diagram, MermaidDiagram};

const SOURCE: &str = r#"requirementDiagram
title Checkout requirements
accTitle: Checkout requirement graph
accDescr {
Payment requirements
and their implementation
}
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
    assert_eq!(
        diagram.accessibility_title.as_deref(),
        Some("Checkout requirement graph")
    );
    assert_eq!(
        diagram.accessibility_description.as_deref(),
        Some("Payment requirements\nand their implementation")
    );
    assert_eq!(diagram.nodes.len(), 2);
    assert_eq!(diagram.nodes[0].node_kind, StructuralNodeKind::Requirement);
    assert_eq!(diagram.nodes[1].node_kind, StructuralNodeKind::Element);
    assert_eq!(
        diagram.nodes[0].metadata,
        Some(StructuralNodeMetadata::Requirement(
            diagram_ir::RequirementMetadata {
                kind: RequirementKind::Functional,
                external_id: Some("PAY-1".into()),
                text: Some("Payment completes securely".into()),
                risk: Some(RequirementRisk::High),
                verify_method: Some(RequirementVerifyMethod::Test),
            }
        ))
    );
    assert_eq!(
        diagram.nodes[1].metadata,
        Some(StructuralNodeMetadata::RequirementElement(
            diagram_ir::RequirementElementMetadata {
                element_type: Some("service".into()),
                document_reference: Some("docs/checkout".into()),
            }
        ))
    );
    assert_eq!(diagram.relationships[0].from, "checkout_service");
    assert_eq!(diagram.relationships[0].to, "payment_req");
    assert_eq!(diagram.relationships[0].label.as_deref(), Some("satisfies"));
}

#[test]
fn requirement_parses_single_line_accessibility_description() {
    let diagram = parse_requirement_diagram(
        "requirementDiagram\naccDescr: A concise requirement graph\nrequirement req {}",
    )
    .expect("single-line accessibility description");
    assert_eq!(
        diagram.accessibility_description.as_deref(),
        Some("A concise requirement graph")
    );
}

#[test]
fn requirement_accepts_documented_enum_casing() {
    let source = r#"requirementDiagram
requirement test_req {
id: 1
text: Test
risk: Medium
verifyMethod: Inspection
}"#;
    let diagram = parse_requirement_diagram(source).expect("documented enum casing");
    let Some(StructuralNodeMetadata::Requirement(metadata)) = &diagram.nodes[0].metadata else {
        panic!("typed requirement metadata");
    };
    assert_eq!(metadata.risk, Some(RequirementRisk::Medium));
    assert_eq!(
        metadata.verify_method,
        Some(RequirementVerifyMethod::Inspection)
    );
}

#[test]
fn requirement_preserves_all_definition_kinds() {
    let source = r#"requirementDiagram
requirement base {}
functionalRequirement functional {}
interfaceRequirement interface {}
performanceRequirement performance {}
physicalRequirement physical {}
designConstraint constraint {}"#;
    let diagram = parse_requirement_diagram(source).expect("requirement kinds");
    let kinds = diagram
        .nodes
        .iter()
        .map(|node| match node.metadata.as_ref() {
            Some(StructuralNodeMetadata::Requirement(metadata)) => metadata.kind.clone(),
            _ => panic!("typed requirement metadata"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            RequirementKind::Requirement,
            RequirementKind::Functional,
            RequirementKind::Interface,
            RequirementKind::Performance,
            RequirementKind::Physical,
            RequirementKind::DesignConstraint,
        ]
    );
}

#[test]
fn requirement_rejects_fields_from_the_other_definition_family() {
    let requirement_with_element_field = "requirementDiagram\nrequirement req {\ntype: service\n}";
    assert!(parse_requirement_diagram(requirement_with_element_field).is_err());

    let element_with_requirement_field = "requirementDiagram\nelement system {\nrisk: high\n}";
    assert!(parse_requirement_diagram(element_with_requirement_field).is_err());
}

#[test]
fn requirement_dispatches_through_structural_pipeline() {
    assert!(matches!(
        parse_any_mermaid(SOURCE).expect("requirement dispatch"),
        MermaidDiagram::Structural(_)
    ));
}

#[test]
fn requirement_matches_the_pinned_case_insensitive_lexer() {
    let source = r#"ReQuIrEmEnTdIaGrAm
TiTlE Mixed case requirements
DiReCtIoN lr
FuNcTiOnAlReQuIrEmEnT req {
ID: REQ-1
TeXt: "Mixed case"
RiSk: HIGH
VeRiFyMeThOd: InSpEcTiOn
}

ElEmEnT system {
TyPe: service
DoCrEf: docs/system
}
ClAsSdEf important fill:#fff1a8
ClAsS req important
system - SaTiSfIeS -> req"#;
    let diagram = parse_requirement_diagram(source).expect("case-insensitive requirement parse");
    assert_eq!(diagram.title.as_deref(), Some("Mixed case requirements"));
    assert_eq!(diagram.direction, Some(DiagramDirection::Lr));
    assert_eq!(diagram.relationships[0].label.as_deref(), Some("satisfies"));
    assert_eq!(
        diagram.nodes[0].style.as_ref().unwrap().fill.as_deref(),
        Some("#fff1a8")
    );
    assert!(matches!(
        parse_any_mermaid(source).expect("case-insensitive requirement dispatch"),
        MermaidDiagram::Structural(_)
    ));
}

#[test]
fn requirement_preserves_quoted_and_unquoted_multiword_identifiers() {
    let source = r#"requirementDiagram
requirement Checkout payment requirement:::important {
id: PAY-1
}

element "Checkout service" {
type: service
}
classDef important fill:#fff1a8
"Checkout service" - satisfies -> Checkout payment requirement"#;
    let diagram = parse_requirement_diagram(source).expect("multiword requirement identifiers");
    assert_eq!(diagram.nodes[0].id, "Checkout payment requirement");
    assert_eq!(diagram.nodes[1].id, "Checkout service");
    assert_eq!(diagram.relationships[0].from, "Checkout service");
    assert_eq!(diagram.relationships[0].to, "Checkout payment requirement");
    assert_eq!(
        diagram.nodes[0].style.as_ref().unwrap().fill.as_deref(),
        Some("#fff1a8")
    );
}

#[test]
fn requirement_styles_quoted_node_and_class_identifiers() {
    let source = r##"requirementDiagram
requirement "Checkout service" {}
classDef "critical,class" fill:#fff1a8,stroke:#b45309
class "Checkout service" "critical,class"
style "Checkout service" color:#7c2d12,font-family:"Avenir, Next""##;
    let diagram = parse_requirement_diagram(source).expect("quoted requirement styles");
    let style = diagram.nodes[0]
        .style
        .as_ref()
        .expect("quoted style target");
    assert_eq!(style.fill.as_deref(), Some("#fff1a8"));
    assert_eq!(style.stroke.as_deref(), Some("#b45309"));
    assert_eq!(style.text_color.as_deref(), Some("#7c2d12"));
    assert_eq!(style.font_family.as_deref(), Some("Avenir, Next"));
}

#[test]
fn requirement_preserves_layout_direction() {
    for (keyword, expected) in [
        ("TB", DiagramDirection::Tb),
        ("BT", DiagramDirection::Bt),
        ("LR", DiagramDirection::Lr),
        ("RL", DiagramDirection::Rl),
    ] {
        let source =
            format!("requirementDiagram\ndirection {keyword}\nrequirement a {{\nid: a\n}}");
        let diagram = parse_requirement_diagram(&source).expect("requirement direction");
        assert_eq!(diagram.direction, Some(expected));
    }
}

#[test]
fn requirement_preserves_and_merges_direct_styles() {
    let source = r#"requirementDiagram
style req,system fill:#fff1a8,stroke:#b45309
requirement req {}
element system {}
style req stroke-width:4px,color:#7c2d12"#;
    let diagram = parse_requirement_diagram(source).expect("direct requirement styles");

    let requirement_style = diagram.nodes[0].style.as_ref().expect("requirement style");
    assert_eq!(requirement_style.fill.as_deref(), Some("#fff1a8"));
    assert_eq!(requirement_style.stroke.as_deref(), Some("#b45309"));
    assert_eq!(requirement_style.stroke_width, Some(4.0));
    assert_eq!(requirement_style.text_color.as_deref(), Some("#7c2d12"));

    let element_style = diagram.nodes[1].style.as_ref().expect("element style");
    assert_eq!(element_style.fill.as_deref(), Some("#fff1a8"));
    assert_eq!(element_style.stroke.as_deref(), Some("#b45309"));
    assert_eq!(element_style.stroke_width, None);
}

#[test]
fn requirement_rejects_styles_for_unknown_nodes() {
    let error = parse_requirement_diagram(
        "requirementDiagram\nstyle missing fill:#fff\nrequirement present {}",
    )
    .expect_err("unknown style target");
    assert!(error.message.contains("unknown styled node"));
}

#[test]
fn requirement_resolves_default_named_and_inline_classes_in_source_order() {
    let source = r#"requirementDiagram
classDef default fill:#f8fafc,stroke:#64748b
classDef important fill:#fff1a8,stroke:#b45309
requirement req:::important {
id: REQ-1
}
element system {}
class system important
classDef emphasized stroke-width:4px,color:#7c2d12
class req,system emphasized
style system fill:#dcfce7"#;
    let diagram = parse_requirement_diagram(source).expect("requirement style classes");

    let requirement_style = diagram.nodes[0].style.as_ref().expect("requirement style");
    assert_eq!(requirement_style.fill.as_deref(), Some("#fff1a8"));
    assert_eq!(requirement_style.stroke.as_deref(), Some("#b45309"));
    assert_eq!(requirement_style.stroke_width, Some(4.0));
    assert_eq!(requirement_style.text_color.as_deref(), Some("#7c2d12"));

    let element_style = diagram.nodes[1].style.as_ref().expect("element style");
    assert_eq!(element_style.fill.as_deref(), Some("#dcfce7"));
    assert_eq!(element_style.stroke.as_deref(), Some("#b45309"));
    assert_eq!(element_style.stroke_width, Some(4.0));
    assert_eq!(element_style.text_color.as_deref(), Some("#7c2d12"));
}

#[test]
fn requirement_applies_classes_declared_after_assignment() {
    let diagram = parse_requirement_diagram(
        "requirementDiagram\nrequirement req {}\nclass req late\nclassDef late fill:#f96,stroke:#333;",
    )
    .expect("deferred requirement class");
    let style = diagram.nodes[0]
        .style
        .as_ref()
        .expect("deferred class style");
    assert_eq!(style.fill.as_deref(), Some("#f96"));
    assert_eq!(style.stroke.as_deref(), Some("#333"));
}

#[test]
fn requirement_preserves_typography_styles() {
    let diagram = parse_requirement_diagram(
        "requirementDiagram\nrequirement req {}\nstyle req font-size:22px,font-weight:bold,font-style:italic,font-family:\"Avenir Next\"",
    )
    .expect("requirement typography");
    let style = diagram.nodes[0].style.as_ref().expect("typography style");
    assert_eq!(style.font_size, Some(22.0));
    assert_eq!(style.font_weight, Some(700));
    assert_eq!(style.font_italic, Some(true));
    assert_eq!(style.font_family.as_deref(), Some("Avenir Next"));
}

#[test]
fn requirement_resolves_standalone_inline_class_shorthand() {
    let source = r#"requirementDiagram
requirement req {}
classDef base fill:#f9f,stroke:#333
classDef emphasized stroke-width:4px,color:blue
req:::base,emphasized"#;
    let diagram = parse_requirement_diagram(source).expect("standalone class shorthand");
    let style = diagram.nodes[0].style.as_ref().expect("shorthand style");
    assert_eq!(style.fill.as_deref(), Some("#f9f"));
    assert_eq!(style.stroke.as_deref(), Some("#333"));
    assert_eq!(style.stroke_width, Some(4.0));
    assert_eq!(style.text_color.as_deref(), Some("blue"));
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
