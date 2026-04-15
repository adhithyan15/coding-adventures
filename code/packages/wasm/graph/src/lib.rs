use graph::{
    bfs as core_bfs, connected_components as core_connected_components, dfs as core_dfs,
    has_cycle as core_has_cycle, is_connected as core_is_connected,
    minimum_spanning_tree as core_minimum_spanning_tree, shortest_path as core_shortest_path,
    Graph, GraphError, GraphRepr,
};
use wasm_bindgen::prelude::*;

fn to_js_error(error: GraphError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn parse_repr(repr: &str) -> Result<GraphRepr, JsValue> {
    match repr {
        "adjacency_list" => Ok(GraphRepr::AdjacencyList),
        "adjacency_matrix" => Ok(GraphRepr::AdjacencyMatrix),
        _ => Err(JsValue::from_str(
            "unknown graph representation; expected adjacency_list or adjacency_matrix",
        )),
    }
}

#[wasm_bindgen]
pub struct WasmGraph {
    inner: Graph,
}

#[wasm_bindgen]
impl WasmGraph {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Graph::default(),
        }
    }

    #[wasm_bindgen(js_name = "withRepresentation")]
    pub fn with_representation(repr: &str) -> Result<Self, JsValue> {
        Ok(Self {
            inner: Graph::new(parse_repr(repr)?),
        })
    }

    pub fn repr(&self) -> String {
        match self.inner.repr() {
            GraphRepr::AdjacencyList => "adjacency_list".to_string(),
            GraphRepr::AdjacencyMatrix => "adjacency_matrix".to_string(),
        }
    }

    #[wasm_bindgen(js_name = "addNode")]
    pub fn add_node(&mut self, node: &str) {
        self.inner.add_node(node);
    }

    #[wasm_bindgen(js_name = "removeNode")]
    pub fn remove_node(&mut self, node: &str) -> Result<(), JsValue> {
        self.inner.remove_node(node).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "hasNode")]
    pub fn has_node(&self, node: &str) -> bool {
        self.inner.has_node(node)
    }

    pub fn nodes(&self) -> Vec<String> {
        self.inner.nodes()
    }

    pub fn size(&self) -> usize {
        self.inner.size()
    }

    #[wasm_bindgen(js_name = "addEdge")]
    pub fn add_edge(&mut self, left: &str, right: &str, weight: f64) {
        self.inner.add_edge(left, right, weight);
    }

    #[wasm_bindgen(js_name = "removeEdge")]
    pub fn remove_edge(&mut self, left: &str, right: &str) -> Result<(), JsValue> {
        self.inner.remove_edge(left, right).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "hasEdge")]
    pub fn has_edge(&self, left: &str, right: &str) -> bool {
        self.inner.has_edge(left, right)
    }

    #[wasm_bindgen(js_name = "edgeWeight")]
    pub fn edge_weight(&self, left: &str, right: &str) -> Result<f64, JsValue> {
        self.inner.edge_weight(left, right).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "edgesJson")]
    pub fn edges_json(&self) -> String {
        serde_json::to_string(&self.inner.edges()).expect("edges are serializable")
    }

    pub fn neighbors(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner.neighbors(node).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "neighborsWeightedJson")]
    pub fn neighbors_weighted_json(&self, node: &str) -> Result<String, JsValue> {
        let neighbors = self.inner.neighbors_weighted(node).map_err(to_js_error)?;
        Ok(serde_json::to_string(&neighbors).expect("neighbors are serializable"))
    }

    pub fn degree(&self, node: &str) -> Result<usize, JsValue> {
        self.inner.degree(node).map_err(to_js_error)
    }

    pub fn bfs(&self, start: &str) -> Result<Vec<String>, JsValue> {
        core_bfs(&self.inner, start).map_err(to_js_error)
    }

    pub fn dfs(&self, start: &str) -> Result<Vec<String>, JsValue> {
        core_dfs(&self.inner, start).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "isConnected")]
    pub fn is_connected(&self) -> bool {
        core_is_connected(&self.inner)
    }

    #[wasm_bindgen(js_name = "connectedComponentsJson")]
    pub fn connected_components_json(&self) -> String {
        serde_json::to_string(&core_connected_components(&self.inner))
            .expect("components are serializable")
    }

    #[wasm_bindgen(js_name = "hasCycle")]
    pub fn has_cycle(&self) -> bool {
        core_has_cycle(&self.inner)
    }

    #[wasm_bindgen(js_name = "shortestPath")]
    pub fn shortest_path(&self, start: &str, end: &str) -> Vec<String> {
        core_shortest_path(&self.inner, start, end)
    }

    #[wasm_bindgen(js_name = "minimumSpanningTreeJson")]
    pub fn minimum_spanning_tree_json(&self) -> Result<String, JsValue> {
        let mst = core_minimum_spanning_tree(&self.inner).map_err(to_js_error)?;
        Ok(serde_json::to_string(&mst).expect("mst is serializable"))
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        self.inner.to_string()
    }
}

impl Default for WasmGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> WasmGraph {
        let mut graph = WasmGraph::with_representation("adjacency_matrix").unwrap();
        graph.add_edge("London", "Paris", 300.0);
        graph.add_edge("London", "Amsterdam", 520.0);
        graph.add_edge("Paris", "Berlin", 878.0);
        graph.add_edge("Amsterdam", "Berlin", 655.0);
        graph.add_edge("Amsterdam", "Brussels", 180.0);
        graph
    }

    #[test]
    fn wrapper_exposes_graph_operations_and_shortest_path() {
        let graph = make_graph();
        assert_eq!(graph.repr(), "adjacency_matrix");
        assert!(graph.has_edge("London", "Amsterdam"));
        assert_eq!(
            graph.shortest_path("London", "Berlin"),
            vec!["London", "Amsterdam", "Berlin"]
        );
        assert_eq!(graph.degree("Amsterdam").unwrap(), 3);
    }

    #[test]
    fn wrapper_exposes_json_helpers_and_algorithms() {
        let graph = make_graph();
        assert!(graph.is_connected());
        assert!(graph.has_cycle());
        assert!(graph.edges_json().contains("London"));
        assert!(graph.connected_components_json().contains("Brussels"));
        assert!(graph.minimum_spanning_tree_json().unwrap().contains("Paris"));
    }
}
