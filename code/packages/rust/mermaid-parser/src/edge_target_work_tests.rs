//! Public authored-input work and exact metadata controls for edge targets.
use super::*;

fn reset_visits() {
    SWIMLANE_TARGET_EDGE_VISITS.with(|count| count.set(0));
    SWIMLANE_TARGET_LOOKUPS.with(|count| count.set(0));
    SWIMLANE_TARGET_MATCH_VISITS.with(|count| count.set(0));
}

fn visits() -> usize {
    SWIMLANE_TARGET_EDGE_VISITS.with(std::cell::Cell::get)
}

fn source_with_edges(count: usize) -> String {
    format!("swimlane-beta LR\nsubgraph L\na[A]\nb[B]\nend\n{}", "a --> b\n".repeat(count))
}

#[test]
fn node_only_class_targets_do_not_rescan_unidentified_edges() {
    for count in [32, 128] {
        let mut source = source_with_edges(count);
        source.push_str("classDef marked stroke:#123456\nclass ");
        source.push_str(&vec!["a"; count].join(","));
        source.push_str(" marked\n");
        reset_visits();
        let diagram = parse_swimlane(&source).expect("legacy node-only class targets");
        assert_eq!(diagram.edges.len(), count);
        assert!(diagram.edges.iter().all(|edge| edge.id.is_none() && edge.classes.is_empty() && edge.style == DiagramStyle::default()));
        assert_eq!(diagram.nodes[0].classes, ["marked"]);
        assert_eq!(diagram.nodes[0].style.stroke.as_deref(), Some("#123456"));
        assert_eq!(visits(), count, "count={count}; actual full-edge discovery visits={}", visits());
        assert_eq!(SWIMLANE_TARGET_LOOKUPS.with(std::cell::Cell::get), count);
        assert_eq!(SWIMLANE_TARGET_MATCH_VISITS.with(std::cell::Cell::get), 0);
    }
}

#[test]
fn node_only_style_targets_do_not_rescan_unidentified_edges() {
    for count in [32, 128] {
        let mut source = source_with_edges(count);
        source.push_str(&"style a stroke:#123456\n".repeat(count));
        reset_visits();
        let diagram = parse_swimlane(&source).expect("legacy node-only inline styles");
        assert_eq!(diagram.edges.len(), count);
        assert!(diagram.edges.iter().all(|edge| edge.id.is_none() && edge.classes.is_empty() && edge.style == DiagramStyle::default()));
        assert_eq!(diagram.nodes[0].style.stroke.as_deref(), Some("#123456"));
        assert_eq!(visits(), count, "count={count}; actual full-edge discovery visits={}", visits());
        assert_eq!(SWIMLANE_TARGET_LOOKUPS.with(std::cell::Cell::get), count);
        assert_eq!(SWIMLANE_TARGET_MATCH_VISITS.with(std::cell::Cell::get), 0);
    }
}

#[test]
fn duplicate_ids_node_collisions_and_precedence_preserve_exact_metadata() {
    let source = "swimlane-beta LR\nsubgraph L\na[A]\nb[B]\nend\na a@--> b\na a@==> b\na --> b\nclassDef first stroke:#123456,stroke-width:2\nclassDef second stroke:#654321,stroke-width:3\nclass a,a first,second\nstyle a stroke:#abcdef\nstyle a stroke-width:7";
    reset_visits();
    let diagram = parse_swimlane(source).expect("duplicate edge IDs and existing node ID");
    assert_eq!(visits(), 3);
    assert_eq!(SWIMLANE_TARGET_LOOKUPS.with(std::cell::Cell::get), 4);
    assert_eq!(SWIMLANE_TARGET_MATCH_VISITS.with(std::cell::Cell::get), 8);
    assert_eq!(diagram.edges.len(), 3);
    let expected = DiagramStyle { stroke: Some("#abcdef".into()), stroke_width: Some(7.0), ..DiagramStyle::default() };
    assert_eq!(diagram.nodes[0].classes, ["first", "second"]);
    assert_eq!(diagram.nodes[0].style, expected);
    for (edge, kind) in diagram.edges[..2].iter().zip([SwimlaneEdgeKind::Directed, SwimlaneEdgeKind::Thick]) {
        assert_eq!(edge.id.as_deref(), Some("a"));
        assert_eq!((edge.from.as_str(), edge.to.as_str()), ("a", "b"));
        assert_eq!(edge.classes, ["first", "second"]);
        assert_eq!(edge.style, expected);
        assert_eq!(edge.kind, kind);
        assert_eq!((edge.start_marker, edge.end_marker, edge.min_length), (EdgeMarker::None, EdgeMarker::Point, 1));
    }
    assert!(diagram.edges[2].id.is_none());
    assert!(diagram.edges[2].classes.is_empty());
    assert_eq!(diagram.edges[2].style, DiagramStyle::default());
    let unknown = source_with_edges(1) + "style missing stroke:#123456\n";
    assert!(parse_swimlane(&unknown).expect_err("unknown target remains refused").message.contains("unknown node or edge"));
}


#[test]
fn animation_configurations_do_not_rescan_unrelated_edges() {
    for count in [32, 128] {
        let mut source = source_with_edges(count);
        source.push_str("a chosen@--> b\n");
        source.push_str(&"chosen@{animation: slow}\n".repeat(count));
        reset_visits();
        let diagram = parse_swimlane(&source).expect("repeated existing-edge animation directives");
        assert_eq!(diagram.edges.len(), count + 1);
        assert!(diagram.edges[..count].iter().all(|edge| !edge.animate && edge.animation.is_none()));
        assert!(diagram.edges[count].animate);
        assert_eq!(diagram.edges[count].animation, Some(SwimlaneEdgeAnimation::Slow));
        assert_eq!(visits(), count + 1, "actual animation discovery visits={}", visits());
        assert_eq!(SWIMLANE_TARGET_LOOKUPS.with(std::cell::Cell::get), count);
        assert_eq!(SWIMLANE_TARGET_MATCH_VISITS.with(std::cell::Cell::get), count);
    }
}

#[test]
fn animation_duplicate_ids_preserve_order_and_style_passes() {
    let source = "swimlane-beta LR\nsubgraph L\na[A]\nb[B]\nend\na a@--> b\na a@==> b\na --> b\na@{animation: slow}\na@{animate: false, animation: fast}\nclassDef marked stroke:#123456\nclass a marked\nstyle a stroke-width:7";
    reset_visits();
    let diagram = parse_swimlane(source).expect("animation duplicate IDs and node collision");
    assert_eq!(visits(), 3);
    assert_eq!(SWIMLANE_TARGET_LOOKUPS.with(std::cell::Cell::get), 4);
    assert_eq!(SWIMLANE_TARGET_MATCH_VISITS.with(std::cell::Cell::get), 8);
    let expected = DiagramStyle { stroke: Some("#123456".into()), stroke_width: Some(7.0), ..DiagramStyle::default() };
    assert_eq!(diagram.nodes[0].style, expected);
    for edge in &diagram.edges[..2] {
        assert_eq!(edge.id.as_deref(), Some("a"));
        assert!(!edge.animate);
        assert_eq!(edge.animation, Some(SwimlaneEdgeAnimation::Fast));
        assert_eq!(edge.classes, ["marked"]);
        assert_eq!(edge.style, expected);
    }
    assert!(!diagram.edges[2].animate);
    assert_eq!(diagram.edges[2].animation, None);
    assert_eq!(diagram.edges[2].style, DiagramStyle::default());
    let unknown = source_with_edges(1) + "a@{animate: true}\n";
    assert!(parse_swimlane(&unknown).expect_err("a node alone is not an animation edge").message.contains("unknown edge"));
}


#[test]
fn animation_context_preserves_node_metadata_on_edge_endpoints() {
    let ordinary = "swimlane-beta LR\nsubgraph L\na[A]\nb[B]\nend\na --> b";
    let mut expected = parse_swimlane(ordinary).expect("ordinary edge");
    expected.nodes[0].label = "a".into();
    let actual = parse_swimlane("swimlane-beta LR\nsubgraph L\nb[B]\na@{ shape: rect } --> b\nend")
        .expect("node shape metadata on an edge endpoint remains valid");
    // The authored node order differs, so compare in ID order, then the whole IR.
    expected.nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut actual = actual;
    actual.nodes.sort_by(|left, right| left.id.cmp(&right.id));
    expected.lanes[0].node_ids.sort();
    actual.lanes[0].node_ids.sort();
    assert_eq!(actual, expected);
}

#[test]
fn animation_context_preserves_literal_metadata_text_in_node_labels() {
    let mut expected = parse_swimlane("swimlane-beta LR\nsubgraph L\na[A]\nb[B]\nend\na --> b")
        .expect("ordinary label control");
    expected.nodes[0].label = "literal @{animate: true}".into();
    let actual = parse_swimlane("swimlane-beta LR\nsubgraph L\na[\"literal @{animate: true}\"]\nb[B]\nend\na --> b")
        .expect("a literal node label is not an edge configuration");
    assert_eq!(actual, expected);
}
