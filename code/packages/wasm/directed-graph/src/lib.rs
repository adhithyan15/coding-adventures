use std::collections::HashSet;
use std::fmt::Display;

use directed_graph::{Graph, LabeledDirectedGraph};
use wasm_bindgen::prelude::*;

fn to_js_error(error: impl Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn sort_strings(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

fn sort_set(values: HashSet<String>) -> Vec<String> {
    sort_strings(values.into_iter().collect())
}

#[wasm_bindgen]
pub struct WasmDirectedGraph {
    inner: Graph,
}

#[wasm_bindgen]
impl WasmDirectedGraph {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Graph::new(),
        }
    }

    #[wasm_bindgen(js_name = "newAllowSelfLoops")]
    pub fn new_allow_self_loops() -> Self {
        Self {
            inner: Graph::new_allow_self_loops(),
        }
    }

    #[wasm_bindgen(js_name = "allowsSelfLoops")]
    pub fn allows_self_loops(&self) -> bool {
        self.inner.allows_self_loops()
    }

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
    pub fn add_edge(&mut self, from: &str, to: &str) -> Result<(), JsValue> {
        self.inner.add_edge(from, to).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "removeEdge")]
    pub fn remove_edge(&mut self, from: &str, to: &str) -> Result<(), JsValue> {
        self.inner.remove_edge(from, to).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "hasEdge")]
    pub fn has_edge(&self, from: &str, to: &str) -> bool {
        self.inner.has_edge(from, to)
    }

    #[wasm_bindgen(js_name = "edgesJson")]
    pub fn edges_json(&self) -> String {
        serde_json::to_string(&self.inner.edges()).expect("edges are serializable")
    }

    pub fn predecessors(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner.predecessors(node).map_err(to_js_error)
    }

    pub fn successors(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner.successors(node).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "topologicalSort")]
    pub fn topological_sort(&self) -> Result<Vec<String>, JsValue> {
        self.inner.topological_sort().map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "hasCycle")]
    pub fn has_cycle(&self) -> bool {
        self.inner.has_cycle()
    }

    #[wasm_bindgen(js_name = "transitiveClosure")]
    pub fn transitive_closure(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner
            .transitive_closure(node)
            .map(sort_set)
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "affectedNodes")]
    pub fn affected_nodes(&self, changed: Vec<String>) -> Vec<String> {
        sort_set(self.inner.affected_nodes(&changed.into_iter().collect()))
    }

    #[wasm_bindgen(js_name = "independentGroupsJson")]
    pub fn independent_groups_json(&self) -> Result<String, JsValue> {
        let groups = self.inner.independent_groups().map_err(to_js_error)?;
        Ok(serde_json::to_string(&groups).expect("groups are serializable"))
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        self.inner.to_string()
    }
}

impl Default for WasmDirectedGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
pub struct WasmLabeledDirectedGraph {
    inner: LabeledDirectedGraph,
}

#[wasm_bindgen]
impl WasmLabeledDirectedGraph {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: LabeledDirectedGraph::new(),
        }
    }

    #[wasm_bindgen(js_name = "newAllowSelfLoops")]
    pub fn new_allow_self_loops() -> Self {
        Self {
            inner: LabeledDirectedGraph::new_allow_self_loops(),
        }
    }

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
    pub fn add_edge(&mut self, from: &str, to: &str, label: &str) -> Result<(), JsValue> {
        self.inner.add_edge(from, to, label).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "removeEdge")]
    pub fn remove_edge(&mut self, from: &str, to: &str, label: &str) -> Result<(), JsValue> {
        self.inner.remove_edge(from, to, label).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "hasEdge")]
    pub fn has_edge(&self, from: &str, to: &str) -> bool {
        self.inner.has_edge(from, to)
    }

    #[wasm_bindgen(js_name = "hasEdgeWithLabel")]
    pub fn has_edge_with_label(&self, from: &str, to: &str, label: &str) -> bool {
        self.inner.has_edge_with_label(from, to, label)
    }

    #[wasm_bindgen(js_name = "edgesJson")]
    pub fn edges_json(&self) -> String {
        serde_json::to_string(&self.inner.edges()).expect("edges are serializable")
    }

    pub fn labels(&self, from: &str, to: &str) -> Vec<String> {
        sort_set(self.inner.labels(from, to))
    }

    pub fn predecessors(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner.predecessors(node).map_err(to_js_error)
    }

    pub fn successors(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner.successors(node).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "topologicalSort")]
    pub fn topological_sort(&self) -> Result<Vec<String>, JsValue> {
        self.inner.topological_sort().map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "hasCycle")]
    pub fn has_cycle(&self) -> bool {
        self.inner.has_cycle()
    }

    #[wasm_bindgen(js_name = "transitiveClosure")]
    pub fn transitive_closure(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner
            .transitive_closure(node)
            .map(sort_set)
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        format!(
            "LabeledDirectedGraph(size={}, edges={})",
            self.size(),
            self.inner.edges().len()
        )
    }
}

impl Default for WasmLabeledDirectedGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> WasmDirectedGraph {
        let mut graph = WasmDirectedGraph::new();
        graph.add_edge("compile", "link").unwrap();
        graph.add_edge("link", "package").unwrap();
        graph
    }

    #[test]
    fn wasm_directed_graph_wraps_core_operations() {
        let graph = make_graph();
        assert_eq!(
            graph.topological_sort().unwrap(),
            vec!["compile", "link", "package"]
        );
        assert!(graph
            .transitive_closure("compile")
            .unwrap()
            .contains(&"package".to_string()));
        assert!(!graph.has_cycle());
        assert!(graph.edges_json().contains("compile"));
    }

    #[test]
    fn wasm_labeled_directed_graph_wraps_labels_and_traversals() {
        let mut graph = WasmLabeledDirectedGraph::new();
        graph.add_edge("A", "B", "compile").unwrap();
        graph.add_edge("A", "B", "test").unwrap();
        graph.add_edge("B", "C", "runtime").unwrap();

        assert!(graph.has_edge_with_label("A", "B", "compile"));
        assert_eq!(
            graph.labels("A", "B"),
            vec!["compile".to_string(), "test".to_string()]
        );
        assert_eq!(graph.topological_sort().unwrap(), vec!["A", "B", "C"]);
        assert!(graph.edges_json().contains("runtime"));
    }
}
