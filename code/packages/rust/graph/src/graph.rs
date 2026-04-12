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
}

impl Graph {
    pub fn new(repr: GraphRepr) -> Self {
        Self {
            repr,
            adj: BTreeMap::new(),
            node_list: Vec::new(),
            node_index: BTreeMap::new(),
            matrix: Vec::new(),
        }
    }

    pub fn repr(&self) -> GraphRepr {
        self.repr
    }

    pub fn add_node(&mut self, node: impl Into<String>) {
        let node = node.into();
        match self.repr {
            GraphRepr::AdjacencyList => {
                self.adj.entry(node).or_default();
            }
            GraphRepr::AdjacencyMatrix => {
                if self.node_index.contains_key(&node) {
                    return;
                }
                let index = self.node_list.len();
                self.node_list.push(node.clone());
                self.node_index.insert(node, index);
                for row in &mut self.matrix {
                    row.push(None);
                }
                self.matrix.push(vec![None; index + 1]);
            }
        }
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
                }
                self.adj.remove(node);
                Ok(())
            }
            GraphRepr::AdjacencyMatrix => {
                let index = self
                    .node_index
                    .remove(node)
                    .ok_or_else(|| GraphError::NodeNotFound(node.to_string()))?;
                self.node_list.remove(index);
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
                    .insert(left, weight);
            }
            GraphRepr::AdjacencyMatrix => {
                let left_index = self.node_index[&left];
                let right_index = self.node_index[&right];
                self.matrix[left_index][right_index] = Some(weight);
                self.matrix[right_index][left_index] = Some(weight);
            }
        }
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
