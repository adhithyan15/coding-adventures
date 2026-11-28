use graph::{
    minimum_spanning_tree, shortest_path, try_shortest_path, Graph, GraphPropertyBag,
    GraphPropertyValue, GraphRepr,
};

fn representations() -> [GraphRepr; 2] {
    [GraphRepr::AdjacencyList, GraphRepr::AdjacencyMatrix]
}

#[test]
fn stored_invalid_weights_are_rejected_before_query_shortcuts() {
    let invalid = [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    for repr in representations() {
        for weight in invalid {
            let mut graph = Graph::new(repr);
            graph.add_edge("A", "B", 1.0);
            graph.add_edge("X", "Y", weight);

            let before = graph.edge_weight("X", "Y").unwrap();
            let error = try_shortest_path(&graph, "A", "B").unwrap_err();
            assert_eq!((error.left.as_str(), error.right.as_str()), ("X", "Y"));
            assert!(try_shortest_path(&graph, "A", "A").is_err());
            assert!(try_shortest_path(&graph, "missing", "missing").is_err());
            assert!(shortest_path(&graph, "A", "B").is_empty());
            assert_eq!(
                graph.edge_weight("X", "Y").unwrap().to_bits(),
                before.to_bits()
            );
        }
    }
}

#[test]
fn invalid_self_loop_weights_are_checked_before_same_node_shortcut() {
    for repr in representations() {
        let mut graph = Graph::new(repr);
        graph.add_edge("A", "A", f64::NAN);
        assert!(try_shortest_path(&graph, "A", "A").is_err());
        assert!(shortest_path(&graph, "A", "A").is_empty());
    }
}

#[test]
fn invalid_weight_error_endpoints_are_lexical_in_both_representations() {
    for repr in representations() {
        let mut graph = Graph::new(repr);
        // Insert the lexically larger endpoint first so matrix storage visits
        // the edge in reverse lexical order.
        graph.add_edge("Z", "A", -1.0);

        let error = try_shortest_path(&graph, "Z", "A").unwrap_err();
        assert_eq!((error.left.as_str(), error.right.as_str()), ("A", "Z"));
    }
}

#[test]
fn property_bearing_add_rejects_invalid_weight_at_query_time() {
    for repr in representations() {
        let mut graph = Graph::new(repr);
        graph.add_edge_with_properties("A", "B", f64::NAN, GraphPropertyBag::new());
        assert!(try_shortest_path(&graph, "A", "B").is_err());
        assert!(shortest_path(&graph, "A", "B").is_empty());
    }
}

#[test]
fn negative_edge_with_disconnected_destination_terminates_safely() {
    for repr in representations() {
        let mut graph = Graph::new(repr);
        graph.add_edge("A", "B", -1.0);
        graph.add_node("C");
        assert!(try_shortest_path(&graph, "A", "C").is_err());
        assert!(shortest_path(&graph, "A", "C").is_empty());
    }
}

#[test]
fn canonical_weight_updates_are_checked_at_query_time() {
    for repr in representations() {
        let mut graph = Graph::new(repr);
        graph.add_edge("A", "B", 1.0);
        for weight in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            graph
                .set_edge_property("B", "A", "weight", GraphPropertyValue::Number(weight))
                .unwrap();
            assert!(try_shortest_path(&graph, "A", "B").is_err());
        }
        graph
            .set_edge_property("A", "B", "weight", GraphPropertyValue::Number(-0.0))
            .unwrap();
        assert_eq!(try_shortest_path(&graph, "A", "B").unwrap(), vec!["A", "B"]);
        assert_eq!(
            graph.edge_weight("B", "A").unwrap().to_bits(),
            (-0.0f64).to_bits()
        );
    }
}

#[test]
fn valid_missing_and_unreachable_query_shapes_are_preserved() {
    for repr in representations() {
        let mut graph = Graph::new(repr);
        graph.add_edge("A", "B", 1.0);
        graph.add_node("C");
        assert_eq!(try_shortest_path(&graph, "A", "A").unwrap(), vec!["A"]);
        assert!(try_shortest_path(&graph, "missing", "A")
            .unwrap()
            .is_empty());
        assert!(try_shortest_path(&graph, "A", "C").unwrap().is_empty());
        assert!(shortest_path(&graph, "A", "C").is_empty());
    }
}

#[test]
fn negative_finite_edges_remain_valid_for_mst() {
    for repr in representations() {
        let mut graph = Graph::new(repr);
        graph.add_edge("A", "B", -2.0);
        assert_eq!(
            minimum_spanning_tree(&graph).unwrap(),
            vec![("A".into(), "B".into(), -2.0)]
        );
    }
}
