use graph::{
    bfs as core_bfs, connected_components as core_connected_components, dfs as core_dfs,
    has_cycle as core_has_cycle, is_connected as core_is_connected,
    minimum_spanning_tree as core_minimum_spanning_tree, shortest_path as core_shortest_path,
    Graph, GraphError, GraphPropertyBag, GraphPropertyValue, GraphRepr,
};
use serde_json::{Map, Value};
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

fn parse_property_value_json(value_json: &str) -> Result<GraphPropertyValue, JsValue> {
    let value: Value = serde_json::from_str(value_json)
        .map_err(|error| JsValue::from_str(&format!("invalid property JSON: {error}")))?;
    property_value_from_json(value)
}

fn parse_property_bag_json(properties_json: &str) -> Result<GraphPropertyBag, JsValue> {
    let value: Value = serde_json::from_str(properties_json)
        .map_err(|error| JsValue::from_str(&format!("invalid property bag JSON: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| JsValue::from_str("property bag JSON must be an object"))?;
    let mut properties = GraphPropertyBag::new();
    for (key, value) in object {
        properties.insert(key.clone(), property_value_from_json(value.clone())?);
    }
    Ok(properties)
}

fn property_value_from_json(value: Value) -> Result<GraphPropertyValue, JsValue> {
    match value {
        Value::Null => Ok(GraphPropertyValue::Null),
        Value::Bool(value) => Ok(GraphPropertyValue::Bool(value)),
        Value::Number(value) => value
            .as_f64()
            .map(GraphPropertyValue::Number)
            .ok_or_else(|| JsValue::from_str("numeric property is out of range")),
        Value::String(value) => Ok(GraphPropertyValue::String(value)),
        Value::Array(_) | Value::Object(_) => Err(JsValue::from_str(
            "property values must be string, number, boolean, or null",
        )),
    }
}

fn property_value_to_json(value: GraphPropertyValue) -> Value {
    match value {
        GraphPropertyValue::String(value) => Value::String(value),
        GraphPropertyValue::Number(value) => Value::from(value),
        GraphPropertyValue::Bool(value) => Value::Bool(value),
        GraphPropertyValue::Null => Value::Null,
    }
}

fn property_bag_to_json(properties: GraphPropertyBag) -> String {
    let mut object = Map::new();
    for (key, value) in properties {
        object.insert(key, property_value_to_json(value));
    }
    Value::Object(object).to_string()
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

    #[wasm_bindgen(js_name = "addNodeWithProperties")]
    pub fn add_node_with_properties(
        &mut self,
        node: &str,
        properties_json: &str,
    ) -> Result<(), JsValue> {
        self.inner
            .add_node_with_properties(node, parse_property_bag_json(properties_json)?);
        Ok(())
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

    #[wasm_bindgen(js_name = "addEdgeWithProperties")]
    pub fn add_edge_with_properties(
        &mut self,
        left: &str,
        right: &str,
        weight: f64,
        properties_json: &str,
    ) -> Result<(), JsValue> {
        self.inner.add_edge_with_properties(
            left,
            right,
            weight,
            parse_property_bag_json(properties_json)?,
        );
        Ok(())
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

    #[wasm_bindgen(js_name = "graphPropertiesJson")]
    pub fn graph_properties_json(&self) -> String {
        property_bag_to_json(self.inner.graph_properties())
    }

    #[wasm_bindgen(js_name = "setGraphProperty")]
    pub fn set_graph_property(&mut self, key: &str, value_json: &str) -> Result<(), JsValue> {
        self.inner
            .set_graph_property(key, parse_property_value_json(value_json)?);
        Ok(())
    }

    #[wasm_bindgen(js_name = "removeGraphProperty")]
    pub fn remove_graph_property(&mut self, key: &str) {
        self.inner.remove_graph_property(key);
    }

    #[wasm_bindgen(js_name = "nodePropertiesJson")]
    pub fn node_properties_json(&self, node: &str) -> Result<String, JsValue> {
        let properties = self.inner.node_properties(node).map_err(to_js_error)?;
        Ok(property_bag_to_json(properties))
    }

    #[wasm_bindgen(js_name = "setNodeProperty")]
    pub fn set_node_property(
        &mut self,
        node: &str,
        key: &str,
        value_json: &str,
    ) -> Result<(), JsValue> {
        self.inner
            .set_node_property(node, key, parse_property_value_json(value_json)?)
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "removeNodeProperty")]
    pub fn remove_node_property(&mut self, node: &str, key: &str) -> Result<(), JsValue> {
        self.inner
            .remove_node_property(node, key)
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "edgePropertiesJson")]
    pub fn edge_properties_json(&self, left: &str, right: &str) -> Result<String, JsValue> {
        let properties = self
            .inner
            .edge_properties(left, right)
            .map_err(to_js_error)?;
        Ok(property_bag_to_json(properties))
    }

    #[wasm_bindgen(js_name = "setEdgeProperty")]
    pub fn set_edge_property(
        &mut self,
        left: &str,
        right: &str,
        key: &str,
        value_json: &str,
    ) -> Result<(), JsValue> {
        self.inner
            .set_edge_property(left, right, key, parse_property_value_json(value_json)?)
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "removeEdgeProperty")]
    pub fn remove_edge_property(
        &mut self,
        left: &str,
        right: &str,
        key: &str,
    ) -> Result<(), JsValue> {
        self.inner
            .remove_edge_property(left, right, key)
            .map_err(to_js_error)
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
        assert!(graph
            .minimum_spanning_tree_json()
            .unwrap()
            .contains("Paris"));
    }

    #[test]
    fn wrapper_exposes_property_bags_as_json() {
        let mut graph = WasmGraph::with_representation("adjacency_list").unwrap();
        graph.set_graph_property("name", "\"city-map\"").unwrap();
        assert!(graph.graph_properties_json().contains("city-map"));

        graph
            .add_node_with_properties("A", r#"{"kind":"input"}"#)
            .unwrap();
        graph.set_node_property("A", "slot", "0").unwrap();
        assert!(graph.node_properties_json("A").unwrap().contains("slot"));

        graph
            .add_edge_with_properties("A", "B", 2.5, r#"{"role":"distance"}"#)
            .unwrap();
        assert!(graph
            .edge_properties_json("B", "A")
            .unwrap()
            .contains("distance"));
        graph.set_edge_property("B", "A", "weight", "7.0").unwrap();
        assert_eq!(graph.edge_weight("A", "B").unwrap(), 7.0);
        graph.remove_edge_property("A", "B", "role").unwrap();
        assert!(!graph
            .edge_properties_json("A", "B")
            .unwrap()
            .contains("role"));
    }
}
