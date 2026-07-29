use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{self, Debug, Display};
use std::hash::Hash;

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

pub type PropertyBag = HashMap<String, PropertyValue>;

#[derive(Clone, Debug, PartialEq)]
pub struct MultiDirectedEdge<T> {
    pub id: String,
    pub from: T,
    pub to: T,
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GraphError {
    NodeNotFound(String),
    EdgeNotFound(String),
    DuplicateEdgeId(String),
    SelfLoopNotAllowed(String),
    InvalidWeight(String),
    Cycle { processed: usize, total: usize },
}

impl Display for GraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NodeNotFound(node) => write!(formatter, "node not found: {node}"),
            GraphError::EdgeNotFound(edge_id) => write!(formatter, "edge not found: {edge_id}"),
            GraphError::DuplicateEdgeId(edge_id) => {
                write!(formatter, "edge ID already exists: {edge_id}")
            }
            GraphError::SelfLoopNotAllowed(edge) => {
                write!(formatter, "self-loops are not allowed: {edge}")
            }
            GraphError::InvalidWeight(message) => write!(formatter, "{message}"),
            GraphError::Cycle { processed, total } => {
                write!(
                    formatter,
                    "graph contains a cycle: processed {processed}/{total} nodes"
                )
            }
        }
    }
}

impl Error for GraphError {}

#[derive(Clone, Debug)]
pub struct MultiDirectedGraph<T>
where
    T: Clone + Debug + Eq + Hash,
{
    allow_self_loops: bool,
    nodes: Vec<T>,
    node_set: HashSet<T>,
    edges: HashMap<String, MultiDirectedEdge<T>>,
    edge_order: Vec<String>,
    outgoing: HashMap<T, Vec<String>>,
    incoming: HashMap<T, Vec<String>>,
    graph_properties: PropertyBag,
    node_properties: HashMap<T, PropertyBag>,
    edge_properties: HashMap<String, PropertyBag>,
    next_edge_id: usize,
}

impl<T> Default for MultiDirectedGraph<T>
where
    T: Clone + Debug + Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MultiDirectedGraph<T>
where
    T: Clone + Debug + Eq + Hash,
{
    pub fn new() -> Self {
        Self::with_self_loops(false)
    }

    pub fn new_allow_self_loops() -> Self {
        Self::with_self_loops(true)
    }

    pub fn with_self_loops(allow_self_loops: bool) -> Self {
        Self {
            allow_self_loops,
            nodes: Vec::new(),
            node_set: HashSet::new(),
            edges: HashMap::new(),
            edge_order: Vec::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            graph_properties: PropertyBag::new(),
            node_properties: HashMap::new(),
            edge_properties: HashMap::new(),
            next_edge_id: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn add_node(&mut self, node: T, properties: PropertyBag) {
        if !self.node_set.contains(&node) {
            self.node_set.insert(node.clone());
            self.nodes.push(node.clone());
            self.outgoing.insert(node.clone(), Vec::new());
            self.incoming.insert(node.clone(), Vec::new());
            self.node_properties
                .insert(node.clone(), PropertyBag::new());
        }
        if let Some(target) = self.node_properties.get_mut(&node) {
            target.extend(properties);
        }
    }

    pub fn remove_node(&mut self, node: &T) -> Result<(), GraphError> {
        self.assert_node(node)?;
        let outgoing = self.outgoing.get(node).cloned().unwrap_or_default();
        let incoming = self.incoming.get(node).cloned().unwrap_or_default();
        let mut edge_ids = outgoing;
        edge_ids.extend(incoming);
        edge_ids.sort();
        edge_ids.dedup();
        for edge_id in edge_ids {
            self.remove_edge(&edge_id)?;
        }
        self.node_set.remove(node);
        self.nodes.retain(|candidate| candidate != node);
        self.outgoing.remove(node);
        self.incoming.remove(node);
        self.node_properties.remove(node);
        Ok(())
    }

    pub fn has_node(&self, node: &T) -> bool {
        self.node_set.contains(node)
    }

    pub fn nodes(&self) -> Vec<T> {
        self.nodes.clone()
    }

    pub fn add_edge(
        &mut self,
        from: T,
        to: T,
        weight: f64,
        properties: PropertyBag,
        edge_id: Option<String>,
    ) -> Result<String, GraphError> {
        if from == to && !self.allow_self_loops {
            return Err(GraphError::SelfLoopNotAllowed(format!(
                "{from:?} -> {to:?}"
            )));
        }
        let weight = validate_weight(weight)?;
        let edge_id = match edge_id {
            Some(edge_id) => edge_id,
            None => self.allocate_edge_id(),
        };
        if self.edges.contains_key(&edge_id) {
            return Err(GraphError::DuplicateEdgeId(edge_id));
        }

        self.add_node(from.clone(), PropertyBag::new());
        self.add_node(to.clone(), PropertyBag::new());

        self.edges.insert(
            edge_id.clone(),
            MultiDirectedEdge {
                id: edge_id.clone(),
                from: from.clone(),
                to: to.clone(),
                weight,
            },
        );
        self.edge_order.push(edge_id.clone());
        self.outgoing.entry(from).or_default().push(edge_id.clone());
        self.incoming.entry(to).or_default().push(edge_id.clone());

        let mut merged = properties;
        merged.insert("weight".to_string(), PropertyValue::Number(weight));
        self.edge_properties.insert(edge_id.clone(), merged);
        Ok(edge_id)
    }

    pub fn remove_edge(&mut self, edge_id: &str) -> Result<(), GraphError> {
        let edge = self.edge(edge_id)?;
        if let Some(outgoing) = self.outgoing.get_mut(&edge.from) {
            outgoing.retain(|candidate| candidate != edge_id);
        }
        if let Some(incoming) = self.incoming.get_mut(&edge.to) {
            incoming.retain(|candidate| candidate != edge_id);
        }
        self.edges.remove(edge_id);
        self.edge_order.retain(|candidate| candidate != edge_id);
        self.edge_properties.remove(edge_id);
        Ok(())
    }

    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.edges.contains_key(edge_id)
    }

    pub fn edge(&self, edge_id: &str) -> Result<MultiDirectedEdge<T>, GraphError> {
        self.edges
            .get(edge_id)
            .cloned()
            .ok_or_else(|| GraphError::EdgeNotFound(edge_id.to_string()))
    }

    pub fn edges(&self) -> Vec<MultiDirectedEdge<T>> {
        self.edge_order
            .iter()
            .filter_map(|edge_id| self.edges.get(edge_id).cloned())
            .collect()
    }

    pub fn edges_between(&self, from: &T, to: &T) -> Result<Vec<MultiDirectedEdge<T>>, GraphError> {
        self.assert_node(from)?;
        self.assert_node(to)?;
        Ok(self
            .outgoing_edges(from)?
            .into_iter()
            .filter(|edge| &edge.to == to)
            .collect())
    }

    pub fn outgoing_edges(&self, node: &T) -> Result<Vec<MultiDirectedEdge<T>>, GraphError> {
        self.assert_node(node)?;
        Ok(self
            .outgoing
            .get(node)
            .into_iter()
            .flatten()
            .filter_map(|edge_id| self.edges.get(edge_id).cloned())
            .collect())
    }

    pub fn incoming_edges(&self, node: &T) -> Result<Vec<MultiDirectedEdge<T>>, GraphError> {
        self.assert_node(node)?;
        Ok(self
            .incoming
            .get(node)
            .into_iter()
            .flatten()
            .filter_map(|edge_id| self.edges.get(edge_id).cloned())
            .collect())
    }

    pub fn successors(&self, node: &T) -> Result<Vec<T>, GraphError> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for edge in self.outgoing_edges(node)? {
            if seen.insert(edge.to.clone()) {
                result.push(edge.to);
            }
        }
        Ok(result)
    }

    pub fn predecessors(&self, node: &T) -> Result<Vec<T>, GraphError> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for edge in self.incoming_edges(node)? {
            if seen.insert(edge.from.clone()) {
                result.push(edge.from);
            }
        }
        Ok(result)
    }

    pub fn edge_weight(&self, edge_id: &str) -> Result<f64, GraphError> {
        Ok(self.edge(edge_id)?.weight)
    }

    pub fn graph_properties(&self) -> PropertyBag {
        self.graph_properties.clone()
    }

    pub fn set_graph_property(&mut self, key: impl Into<String>, value: PropertyValue) {
        self.graph_properties.insert(key.into(), value);
    }

    pub fn remove_graph_property(&mut self, key: &str) {
        self.graph_properties.remove(key);
    }

    pub fn node_properties(&self, node: &T) -> Result<PropertyBag, GraphError> {
        self.assert_node(node)?;
        Ok(self.node_properties.get(node).cloned().unwrap_or_default())
    }

    pub fn set_node_property(
        &mut self,
        node: &T,
        key: impl Into<String>,
        value: PropertyValue,
    ) -> Result<(), GraphError> {
        self.assert_node(node)?;
        if let Some(properties) = self.node_properties.get_mut(node) {
            properties.insert(key.into(), value);
        }
        Ok(())
    }

    pub fn remove_node_property(&mut self, node: &T, key: &str) -> Result<(), GraphError> {
        self.assert_node(node)?;
        if let Some(properties) = self.node_properties.get_mut(node) {
            properties.remove(key);
        }
        Ok(())
    }

    pub fn edge_properties(&self, edge_id: &str) -> Result<PropertyBag, GraphError> {
        self.assert_edge(edge_id)?;
        let mut properties = self
            .edge_properties
            .get(edge_id)
            .cloned()
            .unwrap_or_default();
        properties.insert(
            "weight".to_string(),
            PropertyValue::Number(self.edge_weight(edge_id)?),
        );
        Ok(properties)
    }

    pub fn set_edge_property(
        &mut self,
        edge_id: &str,
        key: impl Into<String>,
        value: PropertyValue,
    ) -> Result<(), GraphError> {
        self.assert_edge(edge_id)?;
        let key = key.into();
        if key == "weight" {
            match value {
                PropertyValue::Number(weight) => self.set_edge_weight(edge_id, weight)?,
                _ => {
                    return Err(GraphError::InvalidWeight(
                        "edge property 'weight' must be numeric".to_string(),
                    ))
                }
            }
        }
        if let Some(properties) = self.edge_properties.get_mut(edge_id) {
            properties.insert(key, value);
        }
        Ok(())
    }

    pub fn remove_edge_property(&mut self, edge_id: &str, key: &str) -> Result<(), GraphError> {
        self.assert_edge(edge_id)?;
        if key == "weight" {
            self.set_edge_weight(edge_id, 1.0)?;
            if let Some(properties) = self.edge_properties.get_mut(edge_id) {
                properties.insert("weight".to_string(), PropertyValue::Number(1.0));
            }
            return Ok(());
        }
        if let Some(properties) = self.edge_properties.get_mut(edge_id) {
            properties.remove(key);
        }
        Ok(())
    }

    pub fn topological_sort(&self) -> Result<Vec<T>, GraphError> {
        let mut indegree: HashMap<T, usize> = self
            .nodes
            .iter()
            .map(|node| (node.clone(), self.incoming.get(node).map_or(0, Vec::len)))
            .collect();
        let mut ready: Vec<T> = self
            .nodes
            .iter()
            .filter(|node| indegree.get(*node).copied().unwrap_or(0) == 0)
            .cloned()
            .collect();
        sort_nodes(&mut ready);
        let mut order = Vec::new();

        while let Some(node) = pop_first(&mut ready) {
            order.push(node.clone());
            for edge in self.outgoing_edges(&node)? {
                if let Some(degree) = indegree.get_mut(&edge.to) {
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push(edge.to);
                        sort_nodes(&mut ready);
                    }
                }
            }
        }

        if order.len() != self.nodes.len() {
            return Err(GraphError::Cycle {
                processed: order.len(),
                total: self.nodes.len(),
            });
        }
        Ok(order)
    }

    pub fn has_cycle(&self) -> bool {
        self.topological_sort().is_err()
    }

    pub fn independent_groups(&self) -> Result<Vec<Vec<T>>, GraphError> {
        let mut indegree: HashMap<T, usize> = self
            .nodes
            .iter()
            .map(|node| (node.clone(), self.incoming.get(node).map_or(0, Vec::len)))
            .collect();
        let mut current: Vec<T> = self
            .nodes
            .iter()
            .filter(|node| indegree.get(*node).copied().unwrap_or(0) == 0)
            .cloned()
            .collect();
        sort_nodes(&mut current);
        let mut groups = Vec::new();
        let mut processed = 0;

        while !current.is_empty() {
            processed += current.len();
            groups.push(current.clone());
            let mut next = Vec::new();
            for node in current {
                for edge in self.outgoing_edges(&node)? {
                    if let Some(degree) = indegree.get_mut(&edge.to) {
                        *degree -= 1;
                        if *degree == 0 {
                            next.push(edge.to);
                        }
                    }
                }
            }
            sort_nodes(&mut next);
            current = next;
        }

        if processed != self.nodes.len() {
            return Err(GraphError::Cycle {
                processed,
                total: self.nodes.len(),
            });
        }
        Ok(groups)
    }

    fn allocate_edge_id(&mut self) -> String {
        let mut edge_id = format!("e{}", self.next_edge_id);
        while self.edges.contains_key(&edge_id) {
            self.next_edge_id += 1;
            edge_id = format!("e{}", self.next_edge_id);
        }
        self.next_edge_id += 1;
        edge_id
    }

    fn assert_node(&self, node: &T) -> Result<(), GraphError> {
        if self.node_set.contains(node) {
            Ok(())
        } else {
            Err(GraphError::NodeNotFound(format!("{node:?}")))
        }
    }

    fn assert_edge(&self, edge_id: &str) -> Result<(), GraphError> {
        if self.edges.contains_key(edge_id) {
            Ok(())
        } else {
            Err(GraphError::EdgeNotFound(edge_id.to_string()))
        }
    }

    fn set_edge_weight(&mut self, edge_id: &str, weight: f64) -> Result<(), GraphError> {
        let weight = validate_weight(weight)?;
        if let Some(edge) = self.edges.get_mut(edge_id) {
            edge.weight = weight;
        }
        Ok(())
    }
}

fn validate_weight(weight: f64) -> Result<f64, GraphError> {
    if weight.is_nan() {
        return Err(GraphError::InvalidWeight(
            "edge weight must not be NaN".to_string(),
        ));
    }
    Ok(weight)
}

fn sort_nodes<T>(nodes: &mut [T])
where
    T: Debug,
{
    nodes.sort_by_key(|node| format!("{}:{node:?}", std::any::type_name::<T>()));
}

fn pop_first<T>(items: &mut Vec<T>) -> Option<T> {
    if items.is_empty() {
        None
    } else {
        Some(items.remove(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(key: &str, value: PropertyValue) -> PropertyBag {
        let mut bag = PropertyBag::new();
        bag.insert(key.to_string(), value);
        bag
    }

    #[test]
    fn keeps_parallel_edges_with_stable_ids_and_properties() {
        let mut graph = MultiDirectedGraph::new();
        graph.add_node(
            "a".to_string(),
            prop("kind", PropertyValue::String("input".into())),
        );
        let first = graph
            .add_edge(
                "a".to_string(),
                "b".to_string(),
                0.25,
                PropertyBag::new(),
                None,
            )
            .unwrap();
        let second = graph
            .add_edge(
                "a".to_string(),
                "b".to_string(),
                0.75,
                prop("trainable", PropertyValue::Boolean(true)),
                Some("w1".to_string()),
            )
            .unwrap();

        assert_eq!(first, "e0");
        assert_eq!(second, "w1");
        assert_eq!(
            graph
                .edges_between(&"a".to_string(), &"b".to_string())
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            graph.edge_properties("w1").unwrap()["trainable"],
            PropertyValue::Boolean(true)
        );
    }

    #[test]
    fn rejects_duplicate_edge_ids() {
        let mut graph = MultiDirectedGraph::new();
        graph
            .add_edge(
                "a".to_string(),
                "b".to_string(),
                1.0,
                PropertyBag::new(),
                Some("edge".to_string()),
            )
            .unwrap();
        let error = graph
            .add_edge(
                "a".to_string(),
                "c".to_string(),
                1.0,
                PropertyBag::new(),
                Some("edge".to_string()),
            )
            .unwrap_err();
        assert_eq!(error, GraphError::DuplicateEdgeId("edge".to_string()));
    }

    #[test]
    fn synchronizes_weight_property_with_edge_weight() {
        let mut graph = MultiDirectedGraph::new();
        graph
            .add_edge(
                "a".to_string(),
                "b".to_string(),
                2.0,
                PropertyBag::new(),
                Some("w".to_string()),
            )
            .unwrap();
        graph
            .set_edge_property("w", "weight", PropertyValue::Number(3.5))
            .unwrap();
        assert_eq!(graph.edge_weight("w").unwrap(), 3.5);
        assert_eq!(
            graph.edge_properties("w").unwrap()["weight"],
            PropertyValue::Number(3.5)
        );
        graph.remove_edge_property("w", "weight").unwrap();
        assert_eq!(graph.edge_weight("w").unwrap(), 1.0);
    }

    #[test]
    fn sorts_topologically_and_groups_independent_nodes() {
        let mut graph = MultiDirectedGraph::new();
        graph
            .add_edge(
                "a".to_string(),
                "c".to_string(),
                1.0,
                PropertyBag::new(),
                None,
            )
            .unwrap();
        graph
            .add_edge(
                "b".to_string(),
                "c".to_string(),
                1.0,
                PropertyBag::new(),
                None,
            )
            .unwrap();
        graph
            .add_edge(
                "c".to_string(),
                "d".to_string(),
                1.0,
                PropertyBag::new(),
                None,
            )
            .unwrap();

        assert_eq!(
            graph.topological_sort().unwrap(),
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string()
            ]
        );
        assert_eq!(
            graph.independent_groups().unwrap(),
            vec![
                vec!["a".to_string(), "b".to_string()],
                vec!["c".to_string()],
                vec!["d".to_string()]
            ]
        );
    }

    #[test]
    fn detects_cycles_and_removes_incident_edges() {
        let mut graph = MultiDirectedGraph::new();
        graph
            .add_edge(
                "a".to_string(),
                "b".to_string(),
                1.0,
                PropertyBag::new(),
                None,
            )
            .unwrap();
        graph
            .add_edge(
                "b".to_string(),
                "a".to_string(),
                1.0,
                PropertyBag::new(),
                None,
            )
            .unwrap();
        assert!(graph.has_cycle());

        graph.remove_node(&"b".to_string()).unwrap();
        assert_eq!(graph.edges().len(), 0);
        assert!(!graph.has_cycle());
    }

    #[test]
    fn copies_and_mutates_property_bags() {
        let mut graph = MultiDirectedGraph::new();
        graph.add_node(
            "a".to_string(),
            prop("role", PropertyValue::String("input".into())),
        );
        graph.add_node(
            "a".to_string(),
            prop("shape", PropertyValue::String("[1]".into())),
        );
        graph.set_graph_property("name", PropertyValue::String("generic".into()));

        let mut graph_properties = graph.graph_properties();
        graph_properties.insert("name".to_string(), PropertyValue::String("mutated".into()));
        assert_eq!(
            graph.graph_properties()["name"],
            PropertyValue::String("generic".into())
        );

        let mut node_properties = graph.node_properties(&"a".to_string()).unwrap();
        node_properties.insert("role".to_string(), PropertyValue::String("mutated".into()));
        graph
            .set_node_property(
                &"a".to_string(),
                "kind",
                PropertyValue::String("feature".into()),
            )
            .unwrap();
        graph
            .remove_node_property(&"a".to_string(), "shape")
            .unwrap();
        let node_properties = graph.node_properties(&"a".to_string()).unwrap();
        assert_eq!(
            node_properties["role"],
            PropertyValue::String("input".into())
        );
        assert_eq!(
            node_properties["kind"],
            PropertyValue::String("feature".into())
        );
        assert!(!node_properties.contains_key("shape"));

        graph.remove_graph_property("name");
        assert!(graph.graph_properties().is_empty());
    }

    #[test]
    fn controls_self_loops_and_auto_edge_ids() {
        let mut graph = MultiDirectedGraph::new();
        assert!(matches!(
            graph.add_edge(
                "a".to_string(),
                "a".to_string(),
                1.0,
                PropertyBag::new(),
                None,
            ),
            Err(GraphError::SelfLoopNotAllowed(_))
        ));

        let mut graph = MultiDirectedGraph::new_allow_self_loops();
        graph
            .add_edge(
                "a".to_string(),
                "a".to_string(),
                1.0,
                PropertyBag::new(),
                Some("e0".to_string()),
            )
            .unwrap();
        let allocated = graph
            .add_edge(
                "a".to_string(),
                "b".to_string(),
                1.0,
                PropertyBag::new(),
                None,
            )
            .unwrap();
        assert_eq!(allocated, "e1");
        assert!(graph.has_cycle());
    }

    #[test]
    fn reports_missing_nodes_and_edges() {
        let mut graph = MultiDirectedGraph::<String>::new();
        let missing = "missing".to_string();

        assert!(matches!(
            graph.node_properties(&missing),
            Err(GraphError::NodeNotFound(_))
        ));
        assert!(matches!(
            graph.outgoing_edges(&missing),
            Err(GraphError::NodeNotFound(_))
        ));
        assert!(matches!(
            graph.incoming_edges(&missing),
            Err(GraphError::NodeNotFound(_))
        ));
        assert!(matches!(
            graph.successors(&missing),
            Err(GraphError::NodeNotFound(_))
        ));
        assert!(matches!(
            graph.predecessors(&missing),
            Err(GraphError::NodeNotFound(_))
        ));
        assert!(matches!(
            graph.set_node_property(&missing, "key", PropertyValue::Null),
            Err(GraphError::NodeNotFound(_))
        ));
        assert!(matches!(
            graph.remove_node_property(&missing, "key"),
            Err(GraphError::NodeNotFound(_))
        ));
        assert!(matches!(
            graph.remove_node(&missing),
            Err(GraphError::NodeNotFound(_))
        ));

        assert!(matches!(
            graph.edge("missing"),
            Err(GraphError::EdgeNotFound(_))
        ));
        assert!(matches!(
            graph.edge_weight("missing"),
            Err(GraphError::EdgeNotFound(_))
        ));
        assert!(matches!(
            graph.edge_properties("missing"),
            Err(GraphError::EdgeNotFound(_))
        ));
        assert!(matches!(
            graph.set_edge_property("missing", "key", PropertyValue::Null),
            Err(GraphError::EdgeNotFound(_))
        ));
        assert!(matches!(
            graph.remove_edge_property("missing", "key"),
            Err(GraphError::EdgeNotFound(_))
        ));
        assert!(matches!(
            graph.remove_edge("missing"),
            Err(GraphError::EdgeNotFound(_))
        ));
    }

    #[test]
    fn validates_and_resets_edge_weights() {
        let mut graph = MultiDirectedGraph::new();
        graph
            .add_edge(
                "a".to_string(),
                "b".to_string(),
                2.0,
                prop("channel", PropertyValue::String("left".into())),
                Some("w".to_string()),
            )
            .unwrap();
        let mut properties = graph.edge_properties("w").unwrap();
        properties.insert(
            "channel".to_string(),
            PropertyValue::String("mutated".into()),
        );
        assert_eq!(
            graph.edge_properties("w").unwrap()["channel"],
            PropertyValue::String("left".into())
        );

        graph
            .set_edge_property("w", "weight", PropertyValue::Number(3.5))
            .unwrap();
        assert_eq!(graph.edge_weight("w").unwrap(), 3.5);
        assert!(matches!(
            graph.set_edge_property("w", "weight", PropertyValue::String("heavy".into())),
            Err(GraphError::InvalidWeight(_))
        ));
        assert!(matches!(
            graph.set_edge_property("w", "weight", PropertyValue::Number(f64::NAN)),
            Err(GraphError::InvalidWeight(_))
        ));
        graph.remove_edge_property("w", "weight").unwrap();
        assert_eq!(graph.edge_weight("w").unwrap(), 1.0);
    }

    #[test]
    fn topological_algorithms_count_parallel_edges() {
        let mut graph = MultiDirectedGraph::new();
        for edge_id in ["a_to_c_0", "a_to_c_1"] {
            graph
                .add_edge(
                    "a".to_string(),
                    "c".to_string(),
                    1.0,
                    PropertyBag::new(),
                    Some(edge_id.to_string()),
                )
                .unwrap();
        }
        graph
            .add_edge(
                "b".to_string(),
                "c".to_string(),
                1.0,
                PropertyBag::new(),
                None,
            )
            .unwrap();
        graph
            .add_edge(
                "c".to_string(),
                "d".to_string(),
                1.0,
                PropertyBag::new(),
                None,
            )
            .unwrap();

        assert_eq!(
            graph.topological_sort().unwrap(),
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string()
            ]
        );
        assert_eq!(
            graph.independent_groups().unwrap(),
            vec![
                vec!["a".to_string(), "b".to_string()],
                vec!["c".to_string()],
                vec!["d".to_string()]
            ]
        );
        assert_eq!(
            graph.successors(&"a".to_string()).unwrap(),
            vec!["c".to_string()]
        );
        assert_eq!(
            graph.predecessors(&"c".to_string()).unwrap(),
            vec!["a".to_string(), "b".to_string()]
        );
    }
}
