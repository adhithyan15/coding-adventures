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
    let error = parse_swimlane(&unknown).expect_err("unknown target remains refused");
    assert_eq!(error.message, "Swimlane style references unknown node, edge, or subgraph \"missing\"");
    assert_eq!((error.line, error.col), (7, 1));
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
    let expected = parse_swimlane("swimlane-beta LR\nsubgraph L\nb[B]\na[a] --> b\nend")
        .expect("same authored node/lane/edge order without metadata syntax");
    let actual = parse_swimlane("swimlane-beta LR\nsubgraph L\nb[B]\na@{ shape: rect } --> b\nend")
        .expect("node shape metadata on an edge endpoint remains valid");
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

#[test]
fn curve_configurations_do_not_rescan_unrelated_edges() {
    for count in [32, 128] {
        let mut source = source_with_edges(count);
        source.push_str("a chosen@--> b\nchosen@{animation: slow}\n");
        source.push_str(&"chosen@{curve: basis}\n".repeat(count));
        let mut expected = parse_swimlane(&source_with_edges(count)).expect("unnamed-edge control");
        let mut chosen = expected.edges[0].clone();
        chosen.id = Some("chosen".into());
        chosen.animate = true;
        chosen.animation = Some(SwimlaneEdgeAnimation::Slow);
        chosen.curve = Some(SwimlaneEdgeCurve::Basis);
        expected.edges.push(chosen);
        reset_visits();
        let actual = parse_swimlane(&source).expect("repeated curve-only directives");
        assert_eq!(actual, expected, "all authored IR and earlier animation settings survive");
        assert_eq!(visits(), count + 1);
        assert_eq!(SWIMLANE_TARGET_LOOKUPS.with(std::cell::Cell::get), count + 1);
        assert_eq!(SWIMLANE_TARGET_MATCH_VISITS.with(std::cell::Cell::get), count + 1);
    }
}

#[test]
fn curve_duplicate_ids_preserve_partial_update_order_and_style() {
    let base = "swimlane-beta LR\nsubgraph L\na[A]\nb[B]\nend\na a@--> b\na a@==> b\na --> b\n";
    let source = format!("{base}a@{{animation: slow}}\na@{{curve: basis}}\na@{{animate: false}}\na@{{animation: fast}}\na@{{animate: false, curve: linear}}\nclassDef marked stroke:#123456\nclass a marked\nstyle a stroke-width:7");
    let mut expected = parse_swimlane(base).expect("same authored order without directives");
    let style = DiagramStyle { stroke: Some("#123456".into()), stroke_width: Some(7.0), ..DiagramStyle::default() };
    expected.nodes[0].classes = vec!["marked".into()];
    expected.nodes[0].style = style.clone();
    for edge in &mut expected.edges[..2] {
        edge.animate = false;
        edge.animation = Some(SwimlaneEdgeAnimation::Fast);
        edge.curve = Some(SwimlaneEdgeCurve::Linear);
        edge.classes = vec!["marked".into()];
        edge.style = style.clone();
    }
    reset_visits();
    let actual = parse_swimlane(&source).expect("duplicate edge curves and partial updates");
    assert_eq!(actual, expected, "complete authored diagram, including unnamed edge and node collision");
    assert_eq!(visits(), 3);
    assert_eq!(SWIMLANE_TARGET_LOOKUPS.with(std::cell::Cell::get), 7);
    assert_eq!(SWIMLANE_TARGET_MATCH_VISITS.with(std::cell::Cell::get), 14);
    let unknown = source_with_edges(1) + "a@{curve: basis}\n";
    assert!(parse_swimlane(&unknown).expect_err("a node alone is not a curve edge").message.contains("unknown edge"));
}
