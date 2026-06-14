pub mod algorithms;
pub mod graph;

pub use algorithms::{
    bfs, connected_components, dfs, has_cycle, is_connected, minimum_spanning_tree, shortest_path,
    TraversalGraph,
};
pub use graph::{Graph, GraphError, GraphPropertyBag, GraphPropertyValue, GraphRepr, WeightedEdge};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn representations() -> [GraphRepr; 2] {
        [GraphRepr::AdjacencyList, GraphRepr::AdjacencyMatrix]
    }

    fn make_graph(repr: GraphRepr) -> Graph {
        let mut g = Graph::new(repr);
        g.add_edge("London", "Paris", 300.0);
        g.add_edge("London", "Amsterdam", 520.0);
        g.add_edge("Paris", "Berlin", 878.0);
        g.add_edge("Amsterdam", "Berlin", 655.0);
        g.add_edge("Amsterdam", "Brussels", 180.0);
        g
    }

    fn make_triangle(repr: GraphRepr) -> Graph {
        let mut g = Graph::new(repr);
        g.add_edge("A", "B", 1.0);
        g.add_edge("B", "C", 1.0);
        g.add_edge("C", "A", 1.0);
        g
    }

    fn make_path(repr: GraphRepr) -> Graph {
        let mut g = Graph::new(repr);
        g.add_edge("A", "B", 1.0);
        g.add_edge("B", "C", 1.0);
        g
    }

    #[test]
    fn construction_and_nodes_work_in_both_representations() {
        for repr in representations() {
            let mut g = Graph::new(repr);
            assert_eq!(g.size(), 0);
            g.add_node("A");
            g.add_node("B");
            assert!(g.has_node("A"));
            assert!(g.has_node("B"));
            g.remove_node("A").unwrap();
            assert!(!g.has_node("A"));
            assert_eq!(g.size(), 1);
        }
    }

    #[test]
    fn edge_operations_and_neighbors_are_undirected() {
        for repr in representations() {
            let mut g = Graph::new(repr);
            g.add_edge("A", "B", 2.5);
            assert!(g.has_edge("A", "B"));
            assert!(g.has_edge("B", "A"));
            assert_eq!(g.edge_weight("A", "B").unwrap(), 2.5);
            assert_eq!(g.neighbors("A").unwrap(), vec!["B".to_string()]);
        }
    }

    #[test]
    fn property_bags_work_in_both_representations() {
        for repr in representations() {
            let mut g = Graph::new(repr);
            g.set_graph_property("name", GraphPropertyValue::String("city-map".to_string()));
            g.set_graph_property("version", GraphPropertyValue::Number(1.0));
            assert_eq!(
                g.graph_properties().get("name"),
                Some(&GraphPropertyValue::String("city-map".to_string()))
            );
            g.remove_graph_property("version");
            assert!(!g.graph_properties().contains_key("version"));

            let mut node_properties = BTreeMap::new();
            node_properties.insert(
                "kind".to_string(),
                GraphPropertyValue::String("input".to_string()),
            );
            g.add_node_with_properties("A", node_properties);
            let mut extra_node_properties = BTreeMap::new();
            extra_node_properties.insert("trainable".to_string(), GraphPropertyValue::Bool(false));
            g.add_node_with_properties("A", extra_node_properties);
            g.set_node_property("A", "slot", GraphPropertyValue::Number(0.0))
                .unwrap();
            assert_eq!(
                g.node_properties("A").unwrap().get("kind"),
                Some(&GraphPropertyValue::String("input".to_string()))
            );

            let mut edge_properties = BTreeMap::new();
            edge_properties.insert(
                "role".to_string(),
                GraphPropertyValue::String("distance".to_string()),
            );
            g.add_edge_with_properties("A", "B", 2.5, edge_properties);
            assert_eq!(
                g.edge_properties("B", "A").unwrap().get("weight"),
                Some(&GraphPropertyValue::Number(2.5))
            );

            g.set_edge_property("B", "A", "weight", GraphPropertyValue::Number(7.0))
                .unwrap();
            assert_eq!(g.edge_weight("A", "B").unwrap(), 7.0);
            g.set_edge_property("A", "B", "trainable", GraphPropertyValue::Bool(true))
                .unwrap();
            g.remove_edge_property("A", "B", "role").unwrap();
            let properties = g.edge_properties("A", "B").unwrap();
            assert_eq!(
                properties.get("trainable"),
                Some(&GraphPropertyValue::Bool(true))
            );
            assert_eq!(
                properties.get("weight"),
                Some(&GraphPropertyValue::Number(7.0))
            );

            g.remove_edge("A", "B").unwrap();
            assert!(g.edge_properties("A", "B").is_err());
        }
    }

    #[test]
    fn self_loops_and_zero_weight_edges_are_supported() {
        for repr in representations() {
            let mut g = Graph::new(repr);
            g.add_edge("A", "A", 0.0);
            assert!(g.has_edge("A", "A"));
            assert_eq!(g.edge_weight("A", "A").unwrap(), 0.0);
            assert_eq!(g.neighbors("A").unwrap(), vec!["A".to_string()]);
        }
    }

    #[test]
    fn traversals_connectivity_and_cycles_match_expectations() {
        for repr in representations() {
            assert_eq!(bfs(&make_path(repr), "A").unwrap(), vec!["A", "B", "C"]);
            assert_eq!(dfs(&make_path(repr), "A").unwrap(), vec!["A", "B", "C"]);
            assert_eq!(
                bfs(&make_graph(repr), "London").unwrap(),
                vec!["London", "Amsterdam", "Paris", "Berlin", "Brussels"]
            );
            assert!(is_connected(&make_graph(repr)));
            assert!(has_cycle(&make_triangle(repr)));
            assert!(!has_cycle(&make_path(repr)));
        }
    }

    #[test]
    fn connected_components_split_disconnected_graphs() {
        for repr in representations() {
            let mut g = Graph::new(repr);
            g.add_edge("A", "B", 1.0);
            g.add_edge("B", "C", 1.0);
            g.add_edge("D", "E", 1.0);
            g.add_node("F");
            let components = connected_components(&g);
            assert_eq!(components.len(), 3);
            assert!(components.contains(&vec!["A".to_string(), "B".to_string(), "C".to_string()]));
            assert!(components.contains(&vec!["D".to_string(), "E".to_string()]));
            assert!(components.contains(&vec!["F".to_string()]));
        }
    }

    #[test]
    fn shortest_path_and_mst_follow_the_weighted_spec() {
        for repr in representations() {
            let path = shortest_path(&make_graph(repr), "London", "Berlin");
            assert_eq!(path, vec!["London", "Amsterdam", "Berlin"]);

            let mst = minimum_spanning_tree(&make_graph(repr)).unwrap();
            assert_eq!(mst.len(), 4);
        }
    }

    #[test]
    fn disconnected_graph_has_no_spanning_tree() {
        for repr in representations() {
            let mut g = Graph::new(repr);
            g.add_edge("A", "B", 1.0);
            g.add_node("C");
            assert!(matches!(
                minimum_spanning_tree(&g),
                Err(GraphError::NotConnected)
            ));
        }
    }
}
