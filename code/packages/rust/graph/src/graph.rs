use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::algorithms::TraversalGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphRepr {
    AdjacencyList,
    AdjacencyMatrix,
}

pub type WeightedEdge = (String, String, f64);

#[derive(Debug, Clone, PartialEq)]
pub enum GraphPropertyValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

pub type GraphPropertyBag = BTreeMap<String, GraphPropertyValue>;

#[derive(Debug, Clone, PartialEq)]
pub enum GraphError {
    NodeNotFound(String),
    EdgeNotFound(String, String),
    NotConnected,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NodeNotFound(node) => write!(f, "node not found: {}", node),
            GraphError::EdgeNotFound(left, right) => {
                write!(f, "edge not found: {} -- {}", left, right)
            }
            GraphError::NotConnected => write!(f, "graph is not connected"),
        }
    }
}

impl std::error::Error for GraphError {}

#[derive(Debug, Clone)]
pub struct Graph {
    repr: GraphRepr,
    adj: BTreeMap<String, BTreeMap<String, f64>>,
    node_list: Vec<String>,
    node_index: BTreeMap<String, usize>,
    matrix: Vec<Vec<Option<f64>>>,
    graph_properties: GraphPropertyBag,
    node_properties: BTreeMap<String, GraphPropertyBag>,
    edge_properties: BTreeMap<(String, String), GraphPropertyBag>,
}

impl Graph {
    pub fn new(repr: GraphRepr) -> Self {
        Self {
            repr,
            adj: BTreeMap::new(),
            node_list: Vec::new(),
            node_index: BTreeMap::new(),
            matrix: Vec::new(),
            graph_properties: BTreeMap::new(),
            node_properties: BTreeMap::new(),
            edge_properties: BTreeMap::new(),
        }
    }

    pub fn repr(&self) -> GraphRepr {
        self.repr
    }

    pub fn add_node(&mut self, node: impl Into<String>) {
        let node = node.into();
        self.add_node_with_properties(node, BTreeMap::new());
    }

    pub fn add_node_with_properties(
        &mut self,
        node: impl Into<String>,
        properties: GraphPropertyBag,
    ) {
        let node = node.into();
        match self.repr {
            GraphRepr::AdjacencyList => {
                self.adj.entry(node.clone()).or_default();
            }
            GraphRepr::AdjacencyMatrix => {
                if !self.node_index.contains_key(&node) {
                    let index = self.node_list.len();
                    self.node_list.push(node.clone());
                    self.node_index.insert(node.clone(), index);
                    for row in &mut self.matrix {
                        row.push(None);
                    }
                    self.matrix.push(vec![None; index + 1]);
                }
            }
        }
        self.node_properties
            .entry(node)
            .or_default()
            .extend(properties);
    }

    pub fn remove_node(&mut self, node: &str) -> Result<(), GraphError> {
        match self.repr {
            GraphRepr::AdjacencyList => {
                let neighbors = self
                    .adj
                    .get(node)
                    .cloned()
                    .ok_or_else(|| GraphError::NodeNotFound(node.to_string()))?;
                for neighbor in neighbors.keys() {
                    if let Some(entry) = self.adj.get_mut(neighbor) {
                        entry.remove(node);
                    }
                    self.edge_properties
                        .remove(&canonical_endpoints(node, neighbor));
                }
                self.adj.remove(node);
                self.node_properties.remove(node);
                Ok(())
            }
            GraphRepr::AdjacencyMatrix => {
                let index = self
                    .node_index
                    .remove(node)
                    .ok_or_else(|| GraphError::NodeNotFound(node.to_string()))?;
                for other in &self.node_list {
                    self.edge_properties
                        .remove(&canonical_endpoints(node, other));
                }
                self.node_list.remove(index);
                self.node_properties.remove(node);
                self.matrix.remove(index);
                for row in &mut self.matrix {
                    row.remove(index);
                }
                for (offset, name) in self.node_list[index..].iter().enumerate() {
                    self.node_index.insert(name.clone(), index + offset);
                }
                Ok(())
            }
        }
    }

    pub fn has_node(&self, node: &str) -> bool {
        match self.repr {
            GraphRepr::AdjacencyList => self.adj.contains_key(node),
            GraphRepr::AdjacencyMatrix => self.node_index.contains_key(node),
        }
    }

    pub fn nodes(&self) -> Vec<String> {
        let mut nodes = match self.repr {
            GraphRepr::AdjacencyList => self.adj.keys().cloned().collect(),
            GraphRepr::AdjacencyMatrix => self.node_list.clone(),
        };
        nodes.sort();
        nodes
    }

    pub fn add_edge(&mut self, left: impl Into<String>, right: impl Into<String>, weight: f64) {
        self.add_edge_with_properties(left, right, weight, BTreeMap::new());
    }

    pub fn add_edge_with_properties(
        &mut self,
        left: impl Into<String>,
        right: impl Into<String>,
        weight: f64,
        properties: GraphPropertyBag,
    ) {
        let left = left.into();
        let right = right.into();
        self.add_node(left.clone());
        self.add_node(right.clone());

        match self.repr {
            GraphRepr::AdjacencyList => {
                self.adj
                    .get_mut(&left)
                    .expect("left node must exist")
                    .insert(right.clone(), weight);
                self.adj
                    .get_mut(&right)
                    .expect("right node must exist")
                    .insert(left.clone(), weight);
            }
            GraphRepr::AdjacencyMatrix => {
                let left_index = self.node_index[&left];
                let right_index = self.node_index[&right];
                self.matrix[left_index][right_index] = Some(weight);
                self.matrix[right_index][left_index] = Some(weight);
            }
        }
        let edge_key = canonical_endpoints(&left, &right);
        let edge_properties = self.edge_properties.entry(edge_key).or_default();
        edge_properties.extend(properties);
        edge_properties.insert("weight".to_string(), GraphPropertyValue::Number(weight));
    }

    pub fn remove_edge(&mut self, left: &str, right: &str) -> Result<(), GraphError> {
        match self.repr {
            GraphRepr::AdjacencyList => {
                if !self
                    .adj
                    .get(left)
                    .map(|neighbors| neighbors.contains_key(right))
                    .unwrap_or(false)
                {
                    return Err(GraphError::EdgeNotFound(
                        left.to_string(),
                        right.to_string(),
                    ));
                }
                self.adj.get_mut(left).unwrap().remove(right);
                self.adj.get_mut(right).unwrap().remove(left);
                self.edge_properties
                    .remove(&canonical_endpoints(left, right));
                Ok(())
            }
            GraphRepr::AdjacencyMatrix => {
                let left_index =
                    self.node_index.get(left).copied().ok_or_else(|| {
                        GraphError::EdgeNotFound(left.to_string(), right.to_string())
                    })?;
                let right_index =
                    self.node_index.get(right).copied().ok_or_else(|| {
                        GraphError::EdgeNotFound(left.to_string(), right.to_string())
                    })?;
                if self.matrix[left_index][right_index].is_none() {
                    return Err(GraphError::EdgeNotFound(
                        left.to_string(),
                        right.to_string(),
                    ));
                }
                self.matrix[left_index][right_index] = None;
                self.matrix[right_index][left_index] = None;
                self.edge_properties
                    .remove(&canonical_endpoints(left, right));
                Ok(())
            }
        }
    }

    pub fn has_edge(&self, left: &str, right: &str) -> bool {
        match self.repr {
            GraphRepr::AdjacencyList => self
                .adj
                .get(left)
                .map(|neighbors| neighbors.contains_key(right))
                .unwrap_or(false),
            GraphRepr::AdjacencyMatrix => self
                .node_index
                .get(left)
                .zip(self.node_index.get(right))
                .and_then(|(left_index, right_index)| self.matrix[*left_index][*right_index])
                .is_some(),
        }
    }

    pub fn edge_weight(&self, left: &str, right: &str) -> Result<f64, GraphError> {
        match self.repr {
            GraphRepr::AdjacencyList => self
                .adj
                .get(left)
                .and_then(|neighbors| neighbors.get(right))
                .copied()
                .ok_or_else(|| GraphError::EdgeNotFound(left.to_string(), right.to_string())),
            GraphRepr::AdjacencyMatrix => {
                let left_index =
                    self.node_index.get(left).copied().ok_or_else(|| {
                        GraphError::EdgeNotFound(left.to_string(), right.to_string())
                    })?;
                let right_index =
                    self.node_index.get(right).copied().ok_or_else(|| {
                        GraphError::EdgeNotFound(left.to_string(), right.to_string())
                    })?;
                self.matrix[left_index][right_index]
                    .ok_or_else(|| GraphError::EdgeNotFound(left.to_string(), right.to_string()))
            }
        }
    }

    pub fn graph_properties(&self) -> GraphPropertyBag {
        self.graph_properties.clone()
    }

    pub fn set_graph_property(&mut self, key: impl Into<String>, value: GraphPropertyValue) {
        self.graph_properties.insert(key.into(), value);
    }

    pub fn remove_graph_property(&mut self, key: &str) {
        self.graph_properties.remove(key);
    }

    pub fn node_properties(&self, node: &str) -> Result<GraphPropertyBag, GraphError> {
        if !self.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }
        Ok(self.node_properties.get(node).cloned().unwrap_or_default())
    }

    pub fn set_node_property(
        &mut self,
        node: &str,
        key: impl Into<String>,
        value: GraphPropertyValue,
    ) -> Result<(), GraphError> {
        if !self.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }
        self.node_properties
            .entry(node.to_string())
            .or_default()
            .insert(key.into(), value);
        Ok(())
    }

    pub fn remove_node_property(&mut self, node: &str, key: &str) -> Result<(), GraphError> {
        if !self.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }
        if let Some(properties) = self.node_properties.get_mut(node) {
            properties.remove(key);
        }
        Ok(())
    }

    pub fn edge_properties(&self, left: &str, right: &str) -> Result<GraphPropertyBag, GraphError> {
        if !self.has_edge(left, right) {
            return Err(GraphError::EdgeNotFound(
                left.to_string(),
                right.to_string(),
            ));
        }
        let mut properties = self
            .edge_properties
            .get(&canonical_endpoints(left, right))
            .cloned()
            .unwrap_or_default();
        properties.insert(
            "weight".to_string(),
            GraphPropertyValue::Number(self.edge_weight(left, right)?),
        );
        Ok(properties)
    }

    pub fn set_edge_property(
        &mut self,
        left: &str,
        right: &str,
        key: impl Into<String>,
        value: GraphPropertyValue,
    ) -> Result<(), GraphError> {
        if !self.has_edge(left, right) {
            return Err(GraphError::EdgeNotFound(
                left.to_string(),
                right.to_string(),
            ));
        }
        let key = key.into();
        if key == "weight" {
            match value {
                GraphPropertyValue::Number(weight) => {
                    self.set_edge_weight(left, right, weight)?;
                    self.edge_properties
                        .entry(canonical_endpoints(left, right))
                        .or_default()
                        .insert(key, GraphPropertyValue::Number(weight));
                    return Ok(());
                }
                _ => {
                    return Err(GraphError::EdgeNotFound(
                        "weight".to_string(),
                        "numeric property".to_string(),
                    ));
                }
            }
        }
        self.edge_properties
            .entry(canonical_endpoints(left, right))
            .or_default()
            .insert(key, value);
        Ok(())
    }

    pub fn remove_edge_property(
        &mut self,
        left: &str,
        right: &str,
        key: &str,
    ) -> Result<(), GraphError> {
        if !self.has_edge(left, right) {
            return Err(GraphError::EdgeNotFound(
                left.to_string(),
                right.to_string(),
            ));
        }
        if key == "weight" {
            self.set_edge_weight(left, right, 1.0)?;
            self.edge_properties
                .entry(canonical_endpoints(left, right))
                .or_default()
                .insert("weight".to_string(), GraphPropertyValue::Number(1.0));
            return Ok(());
        }
        if let Some(properties) = self
            .edge_properties
            .get_mut(&canonical_endpoints(left, right))
        {
            properties.remove(key);
        }
        Ok(())
    }

    pub fn edges(&self) -> Vec<WeightedEdge> {
        let mut result = Vec::new();
        match self.repr {
            GraphRepr::AdjacencyList => {
                let mut seen = BTreeSet::new();
                for (left, neighbors) in &self.adj {
                    for (right, weight) in neighbors {
                        let (first, second) = canonical_endpoints(left, right);
                        if seen.insert((first.clone(), second.clone())) {
                            result.push((first, second, *weight));
                        }
                    }
                }
            }
            GraphRepr::AdjacencyMatrix => {
                for row in 0..self.node_list.len() {
                    for col in row..self.node_list.len() {
                        if let Some(weight) = self.matrix[row][col] {
                            result.push((
                                self.node_list[row].clone(),
                                self.node_list[col].clone(),
                                weight,
                            ));
                        }
                    }
                }
            }
        }
        result.sort_by(|left, right| {
            left.2
                .total_cmp(&right.2)
                .then_with(|| left.0.cmp(&right.0))
                .then_with(|| left.1.cmp(&right.1))
        });
        result
    }

    pub fn neighbors(&self, node: &str) -> Result<Vec<String>, GraphError> {
        match self.repr {
            GraphRepr::AdjacencyList => self
                .adj
                .get(node)
                .map(|neighbors| neighbors.keys().cloned().collect())
                .ok_or_else(|| GraphError::NodeNotFound(node.to_string())),
            GraphRepr::AdjacencyMatrix => {
                let index = self
                    .node_index
                    .get(node)
                    .copied()
                    .ok_or_else(|| GraphError::NodeNotFound(node.to_string()))?;
                let mut neighbors: Vec<String> = self.matrix[index]
                    .iter()
                    .enumerate()
                    .filter_map(|(col, weight)| weight.map(|_| self.node_list[col].clone()))
                    .collect();
                neighbors.sort();
                Ok(neighbors)
            }
        }
    }

    pub fn neighbors_weighted(&self, node: &str) -> Result<BTreeMap<String, f64>, GraphError> {
        match self.repr {
            GraphRepr::AdjacencyList => self
                .adj
                .get(node)
                .cloned()
                .ok_or_else(|| GraphError::NodeNotFound(node.to_string())),
            GraphRepr::AdjacencyMatrix => {
                let index = self
                    .node_index
                    .get(node)
                    .copied()
                    .ok_or_else(|| GraphError::NodeNotFound(node.to_string()))?;
                let mut result = BTreeMap::new();
                for (col, weight) in self.matrix[index].iter().enumerate() {
                    if let Some(weight) = weight {
                        result.insert(self.node_list[col].clone(), *weight);
                    }
                }
                Ok(result)
            }
        }
    }

    pub fn degree(&self, node: &str) -> Result<usize, GraphError> {
        Ok(self.neighbors(node)?.len())
    }

    pub fn size(&self) -> usize {
        match self.repr {
            GraphRepr::AdjacencyList => self.adj.len(),
            GraphRepr::AdjacencyMatrix => self.node_list.len(),
        }
    }

    fn set_edge_weight(&mut self, left: &str, right: &str, weight: f64) -> Result<(), GraphError> {
        match self.repr {
            GraphRepr::AdjacencyList => {
                if !self.has_edge(left, right) {
                    return Err(GraphError::EdgeNotFound(
                        left.to_string(),
                        right.to_string(),
                    ));
                }
                self.adj
                    .get_mut(left)
                    .expect("left node must exist")
                    .insert(right.to_string(), weight);
                self.adj
                    .get_mut(right)
                    .expect("right node must exist")
                    .insert(left.to_string(), weight);
            }
            GraphRepr::AdjacencyMatrix => {
                let left_index =
                    self.node_index.get(left).copied().ok_or_else(|| {
                        GraphError::EdgeNotFound(left.to_string(), right.to_string())
                    })?;
                let right_index =
                    self.node_index.get(right).copied().ok_or_else(|| {
                        GraphError::EdgeNotFound(left.to_string(), right.to_string())
                    })?;
                if self.matrix[left_index][right_index].is_none() {
                    return Err(GraphError::EdgeNotFound(
                        left.to_string(),
                        right.to_string(),
                    ));
                }
                self.matrix[left_index][right_index] = Some(weight);
                self.matrix[right_index][left_index] = Some(weight);
            }
        }
        Ok(())
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new(GraphRepr::AdjacencyList)
    }
}

impl fmt::Display for Graph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Graph(nodes={}, edges={}, repr={:?})",
            self.size(),
            self.edges().len(),
            self.repr
        )
    }
}

impl TraversalGraph for Graph {
    type Error = GraphError;

    fn has_node(&self, node: &str) -> bool {
        Graph::has_node(self, node)
    }

    fn neighbors(&self, node: &str) -> Result<Vec<String>, Self::Error> {
        Graph::neighbors(self, node)
    }
}

fn canonical_endpoints(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}
