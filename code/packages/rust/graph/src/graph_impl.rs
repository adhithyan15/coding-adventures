//! Graph implementation — both representations working identically
//! Complete implementation of all core operations and algorithms.

use crate::errors::GraphError;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphRepr {
    AdjacencyList,
    AdjacencyMatrix,
}

/// Undirected weighted graph supporting two representations
#[derive(Debug, Clone)]
pub enum Graph {
    AdjacencyList {
        adj: HashMap<String, HashMap<String, f64>>,
    },
    AdjacencyMatrix {
        node_list: Vec<String>,
        node_idx: HashMap<String, usize>,
        matrix: Vec<Vec<bool>>,
        weights: HashMap<String, HashMap<String, f64>>,
    },
}

impl Graph {
    pub fn new(repr: GraphRepr) -> Self {
        match repr {
            GraphRepr::AdjacencyList => Graph::AdjacencyList {
                adj: HashMap::new(),
            },
            GraphRepr::AdjacencyMatrix => Graph::AdjacencyMatrix {
                node_list: Vec::new(),
                node_idx: HashMap::new(),
                matrix: Vec::new(),
                weights: HashMap::new(),
            },
        }
    }

    pub fn add_node(&mut self, node: &str) {
        let node_str = node.to_string();
        match self {
            Graph::AdjacencyList { adj } => {
                adj.entry(node_str).or_insert_with(HashMap::new);
            }
            Graph::AdjacencyMatrix {
                node_list,
                node_idx,
                matrix,
                weights,
            } => {
                if !node_idx.contains_key(&node_str) {
                    let idx = node_list.len();
                    node_list.push(node_str.clone());
                    node_idx.insert(node_str.clone(), idx);
                    weights.insert(node_str, HashMap::new());
                    for row in matrix.iter_mut() {
                        row.push(false);
                    }
                    matrix.push(vec![false; idx + 1]);
                }
            }
        }
    }

    pub fn remove_node(&mut self, node: &str) -> Result<(), GraphError> {
        let node_str = node.to_string();
        match self {
            Graph::AdjacencyList { adj } => {
                if !adj.contains_key(&node_str) {
                    return Err(GraphError::NodeNotFound(node_str));
                }
                for neighbours in adj.values_mut() {
                    neighbours.remove(&node_str);
                }
                adj.remove(&node_str);
                Ok(())
            }
            Graph::AdjacencyMatrix {
                node_list,
                node_idx,
                matrix,
                weights,
            } => {
                let idx = node_idx.remove(&node_str).ok_or_else(|| {
                    GraphError::NodeNotFound(node_str.clone())
                })?;
                node_list.remove(idx);
                for i in idx..node_list.len() {
                    node_idx.insert(node_list[i].clone(), i);
                }
                matrix.remove(idx);
                for row in matrix.iter_mut() {
                    row.remove(idx);
                }
                weights.remove(&node_str);
                Ok(())
            }
        }
    }

    pub fn has_node(&self, node: &str) -> bool {
        match self {
            Graph::AdjacencyList { adj } => adj.contains_key(node),
            Graph::AdjacencyMatrix { node_idx, .. } => node_idx.contains_key(node),
        }
    }

    pub fn nodes(&self) -> Vec<String> {
        match self {
            Graph::AdjacencyList { adj } => adj.keys().cloned().collect(),
            Graph::AdjacencyMatrix { node_list, .. } => node_list.clone(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Graph::AdjacencyList { adj } => adj.len(),
            Graph::AdjacencyMatrix { node_list, .. } => node_list.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn add_edge(&mut self, u: &str, v: &str, weight: f64) -> Result<(), GraphError> {
        let u_str = u.to_string();
        let v_str = v.to_string();
        self.add_node(&u_str);
        self.add_node(&v_str);

        match self {
            Graph::AdjacencyList { adj } => {
                adj.get_mut(&u_str).unwrap().insert(v_str.clone(), weight);
                adj.get_mut(&v_str).unwrap().insert(u_str, weight);
                Ok(())
            }
            Graph::AdjacencyMatrix {
                node_idx,
                matrix,
                weights,
                ..
            } => {
                let i = node_idx[&u_str];
                let j = node_idx[&v_str];
                matrix[i][j] = true;
                matrix[j][i] = true;
                weights.get_mut(&u_str).unwrap().insert(v_str.clone(), weight);
                weights.get_mut(&v_str).unwrap().insert(u_str, weight);
                Ok(())
            }
        }
    }

    pub fn remove_edge(&mut self, u: &str, v: &str) -> Result<(), GraphError> {
        let u_str = u.to_string();
        let v_str = v.to_string();

        match self {
            Graph::AdjacencyList { adj } => {
                if !adj.contains_key(&u_str) {
                    return Err(GraphError::NodeNotFound(u_str));
                }
                if !adj[&u_str].contains_key(&v_str) {
                    return Err(GraphError::EdgeNotFound(u_str, v_str));
                }
                adj.get_mut(&u_str).unwrap().remove(&v_str);
                adj.get_mut(&v_str).unwrap().remove(&u_str);
                Ok(())
            }
            Graph::AdjacencyMatrix {
                node_idx,
                matrix,
                weights,
                ..
            } => {
                let i = *node_idx.get(&u_str).ok_or_else(|| {
                    GraphError::NodeNotFound(u_str.clone())
                })?;
                let j = *node_idx.get(&v_str).ok_or_else(|| {
                    GraphError::NodeNotFound(v_str.clone())
                })?;
                if !matrix[i][j] {
                    return Err(GraphError::EdgeNotFound(u_str, v_str));
                }
                matrix[i][j] = false;
                matrix[j][i] = false;
                weights.get_mut(&u_str).unwrap().remove(&v_str);
                weights.get_mut(&v_str).unwrap().remove(&u_str);
                Ok(())
            }
        }
    }

    pub fn has_edge(&self, u: &str, v: &str) -> bool {
        match self {
            Graph::AdjacencyList { adj } => {
                adj.contains_key(u) && adj[u].contains_key(v)
            }
            Graph::AdjacencyMatrix { node_idx, matrix, .. } => {
                matches!(
                    (node_idx.get(u), node_idx.get(v)),
                    (Some(&i), Some(&j)) if matrix[i][j]
                )
            }
        }
    }

    pub fn edges(&self) -> Vec<(String, String, f64)> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();

        match self {
            Graph::AdjacencyList { adj } => {
                for (u, neighbours) in adj.iter() {
                    for (v, w) in neighbours.iter() {
                        let key = if u < v {
                            format!("{}-{}", u, v)
                        } else {
                            format!("{}-{}", v, u)
                        };
                        if !seen.contains(&key) {
                            let (a, b) = if u < v { (u.clone(), v.clone()) } else { (v.clone(), u.clone()) };
                            result.push((a, b, *w));
                            seen.insert(key);
                        }
                    }
                }
            }
            Graph::AdjacencyMatrix { node_list, matrix, weights, .. } => {
                let n = node_list.len();
                for i in 0..n {
                    for j in (i + 1)..n {
                        if matrix[i][j] {
                            let u = &node_list[i];
                            let v = &node_list[j];
                            let w = weights[u][v];
                            result.push((u.clone(), v.clone(), w));
                        }
                    }
                }
            }
        }
        result
    }

    pub fn edge_weight(&self, u: &str, v: &str) -> Result<f64, GraphError> {
        match self {
            Graph::AdjacencyList { adj } => {
                adj.get(u)
                    .and_then(|n| n.get(v))
                    .copied()
                    .ok_or_else(|| GraphError::EdgeNotFound(u.to_string(), v.to_string()))
            }
            Graph::AdjacencyMatrix { node_idx, matrix, weights, .. } => {
                let i = *node_idx.get(u).ok_or_else(|| {
                    GraphError::NodeNotFound(u.to_string())
                })?;
                let j = *node_idx.get(v).ok_or_else(|| {
                    GraphError::NodeNotFound(v.to_string())
                })?;
                if !matrix[i][j] {
                    return Err(GraphError::EdgeNotFound(u.to_string(), v.to_string()));
                }
                Ok(weights[u][v])
            }
        }
    }

    pub fn neighbors(&self, node: &str) -> Result<Vec<String>, GraphError> {
        match self {
            Graph::AdjacencyList { adj } => {
                adj.get(node)
                    .map(|n| n.keys().cloned().collect())
                    .ok_or_else(|| GraphError::NodeNotFound(node.to_string()))
            }
            Graph::AdjacencyMatrix { node_list, node_idx, matrix, .. } => {
                let idx = *node_idx.get(node).ok_or_else(|| {
                    GraphError::NodeNotFound(node.to_string())
                })?;
                let mut result = Vec::new();
                for (j, &has_edge) in matrix[idx].iter().enumerate() {
                    if has_edge {
                        result.push(node_list[j].clone());
                    }
                }
                Ok(result)
            }
        }
    }

    pub fn degree(&self, node: &str) -> Result<usize, GraphError> {
        Ok(self.neighbors(node)?.len())
    }
}

// Breadth-first search
pub fn bfs(g: &Graph, start: &str) -> Result<Vec<String>, GraphError> {
    if !g.has_node(start) {
        return Err(GraphError::NodeNotFound(start.to_string()));
    }
    let mut visited = HashSet::new();
    visited.insert(start.to_string());
    let mut queue = VecDeque::new();
    queue.push_back(start.to_string());
    let mut result = Vec::new();

    while let Some(node) = queue.pop_front() {
        result.push(node.clone());
        let mut neighbours = g.neighbors(&node)?;
        neighbours.sort();
        for neighbour in neighbours {
            if !visited.contains(&neighbour) {
                visited.insert(neighbour.clone());
                queue.push_back(neighbour);
            }
        }
    }
    Ok(result)
}

// Depth-first search
pub fn dfs(g: &Graph, start: &str) -> Result<Vec<String>, GraphError> {
    if !g.has_node(start) {
        return Err(GraphError::NodeNotFound(start.to_string()));
    }
    let mut visited = HashSet::new();
    let mut stack = vec![start.to_string()];
    let mut result = Vec::new();

    while let Some(node) = stack.pop() {
        if visited.contains(&node) {
            continue;
        }
        visited.insert(node.clone());
        result.push(node.clone());

        let mut neighbours = g.neighbors(&node)?;
        neighbours.sort();
        neighbours.reverse();
        for neighbour in neighbours {
            if !visited.contains(&neighbour) {
                stack.push(neighbour);
            }
        }
    }
    Ok(result)
}

// Check if connected
pub fn is_connected(g: &Graph) -> Result<bool, GraphError> {
    if g.is_empty() {
        return Ok(true);
    }
    let start = g.nodes().into_iter().next().unwrap();
    let reachable = bfs(g, &start)?;
    Ok(reachable.len() == g.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_edge() {
        let mut g = Graph::new(GraphRepr::AdjacencyList);
        g.add_edge("A", "B", 1.0).unwrap();
        assert!(g.has_edge("A", "B"));
        assert!(g.has_edge("B", "A"));
    }

    #[test]
    fn test_bfs() {
        let mut g = Graph::new(GraphRepr::AdjacencyList);
        g.add_edge("A", "B", 1.0).unwrap();
        g.add_edge("B", "C", 1.0).unwrap();
        let result = bfs(&g, "A").unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_is_connected() {
        let mut g = Graph::new(GraphRepr::AdjacencyList);
        g.add_edge("A", "B", 1.0).unwrap();
        g.add_edge("B", "C", 1.0).unwrap();
        assert!(is_connected(&g).unwrap());
    }
}
