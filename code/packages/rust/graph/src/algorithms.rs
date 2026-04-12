use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::graph::{Graph, GraphError, WeightedEdge};

pub trait TraversalGraph {
    type Error;

    fn has_node(&self, node: &str) -> bool;
    fn neighbors(&self, node: &str) -> Result<Vec<String>, Self::Error>;
}

pub fn bfs<G>(graph: &G, start: &str) -> Result<Vec<String>, G::Error>
where
    G: TraversalGraph,
{
    let _ = graph.neighbors(start)?;

    let mut visited = BTreeSet::new();
    visited.insert(start.to_string());
    let mut queue = VecDeque::from([start.to_string()]);
    let mut result = Vec::new();

    while let Some(node) = queue.pop_front() {
        result.push(node.clone());
        for neighbor in graph.neighbors(&node)? {
            if visited.insert(neighbor.clone()) {
                queue.push_back(neighbor);
            }
        }
    }

    Ok(result)
}

pub fn dfs<G>(graph: &G, start: &str) -> Result<Vec<String>, G::Error>
where
    G: TraversalGraph,
{
    let _ = graph.neighbors(start)?;

    let mut visited = BTreeSet::new();
    let mut stack = vec![start.to_string()];
    let mut result = Vec::new();

    while let Some(node) = stack.pop() {
        if !visited.insert(node.clone()) {
            continue;
        }

        result.push(node.clone());
        let mut neighbors = graph.neighbors(&node)?;
        neighbors.reverse();
        for neighbor in neighbors {
            if !visited.contains(&neighbor) {
                stack.push(neighbor);
            }
        }
    }

    Ok(result)
}

pub fn is_connected(graph: &Graph) -> bool {
    if graph.size() == 0 {
        return true;
    }
    let nodes = graph.nodes();
    bfs(graph, &nodes[0])
        .map(|visited| visited.len() == graph.size())
        .unwrap_or(false)
}

pub fn connected_components(graph: &Graph) -> Vec<Vec<String>> {
    let mut remaining: BTreeSet<String> = graph.nodes().into_iter().collect();
    let mut result = Vec::new();

    while let Some(start) = remaining.iter().next().cloned() {
        let component = bfs(graph, &start).expect("start node must exist");
        for node in &component {
            remaining.remove(node);
        }
        result.push(component);
    }

    result
}

pub fn has_cycle(graph: &Graph) -> bool {
    fn visit(
        graph: &Graph,
        node: &str,
        parent: Option<&str>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        for neighbor in graph.neighbors(node).expect("node must exist") {
            if !visited.contains(&neighbor) {
                if visit(graph, &neighbor, Some(node), visited) {
                    return true;
                }
            } else if Some(neighbor.as_str()) != parent {
                return true;
            }
        }
        false
    }

    let mut visited = BTreeSet::new();
    for node in graph.nodes() {
        if !visited.contains(&node) && visit(graph, &node, None, &mut visited) {
            return true;
        }
    }
    false
}

pub fn shortest_path(graph: &Graph, start: &str, end: &str) -> Vec<String> {
    if !graph.has_node(start) || !graph.has_node(end) {
        return Vec::new();
    }
    if start == end {
        return vec![start.to_string()];
    }

    let all_unit = graph.edges().iter().all(|edge| edge.2 == 1.0);
    if all_unit {
        bfs_shortest_path(graph, start, end)
    } else {
        dijkstra_shortest_path(graph, start, end)
    }
}

fn bfs_shortest_path(graph: &Graph, start: &str, end: &str) -> Vec<String> {
    let mut parent = BTreeMap::new();
    parent.insert(start.to_string(), None::<String>);
    let mut queue = VecDeque::from([start.to_string()]);

    while let Some(node) = queue.pop_front() {
        if node == end {
            break;
        }
        for neighbor in graph.neighbors(&node).expect("node must exist") {
            if !parent.contains_key(&neighbor) {
                parent.insert(neighbor.clone(), Some(node.clone()));
                queue.push_back(neighbor);
            }
        }
    }

    if !parent.contains_key(end) {
        return Vec::new();
    }

    let mut path = Vec::new();
    let mut current = Some(end.to_string());
    while let Some(node) = current {
        current = parent.get(&node).cloned().unwrap_or(None);
        path.push(node);
    }
    path.reverse();
    path
}

fn dijkstra_shortest_path(graph: &Graph, start: &str, end: &str) -> Vec<String> {
    let mut distances = BTreeMap::new();
    let mut parent = BTreeMap::new();
    for node in graph.nodes() {
        distances.insert(node, f64::INFINITY);
    }
    distances.insert(start.to_string(), 0.0);

    let mut sequence = 0usize;
    let mut queue: Vec<(f64, usize, String)> = vec![(0.0, sequence, start.to_string())];

    while !queue.is_empty() {
        queue.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        });
        let (distance, _, node) = queue.remove(0);
        if distance > *distances.get(&node).unwrap_or(&f64::INFINITY) {
            continue;
        }
        if node == end {
            break;
        }

        for (neighbor, weight) in graph.neighbors_weighted(&node).expect("node must exist") {
            let next_distance = distance + weight;
            if next_distance < *distances.get(&neighbor).unwrap_or(&f64::INFINITY) {
                distances.insert(neighbor.clone(), next_distance);
                parent.insert(neighbor.clone(), node.clone());
                sequence += 1;
                queue.push((next_distance, sequence, neighbor));
            }
        }
    }

    if distances.get(end).copied().unwrap_or(f64::INFINITY) == f64::INFINITY {
        return Vec::new();
    }

    let mut path = Vec::new();
    let mut current = end.to_string();
    loop {
        path.push(current.clone());
        if current == start {
            break;
        }
        let Some(previous) = parent.get(&current).cloned() else {
            return Vec::new();
        };
        current = previous;
    }
    path.reverse();
    path
}

pub fn minimum_spanning_tree(graph: &Graph) -> Result<Vec<WeightedEdge>, GraphError> {
    let nodes = graph.nodes();
    if nodes.len() <= 1 || graph.edges().is_empty() {
        return Ok(Vec::new());
    }
    if !is_connected(graph) {
        return Err(GraphError::NotConnected);
    }

    let mut result = Vec::new();
    let mut union_find = UnionFind::new(nodes);
    for edge in graph.edges() {
        if union_find.find(&edge.0) != union_find.find(&edge.1) {
            union_find.union(&edge.0, &edge.1);
            result.push(edge);
            if result.len() == union_find.size() - 1 {
                break;
            }
        }
    }
    Ok(result)
}

struct UnionFind {
    parent: BTreeMap<String, String>,
    rank: BTreeMap<String, usize>,
}

impl UnionFind {
    fn new(nodes: Vec<String>) -> Self {
        let mut parent = BTreeMap::new();
        let mut rank = BTreeMap::new();
        for node in nodes {
            parent.insert(node.clone(), node.clone());
            rank.insert(node, 0);
        }
        Self { parent, rank }
    }

    fn size(&self) -> usize {
        self.parent.len()
    }

    fn find(&mut self, node: &str) -> String {
        let parent = self.parent.get(node).cloned().expect("node must exist");
        if parent != node {
            let root = self.find(&parent);
            self.parent.insert(node.to_string(), root.clone());
            root
        } else {
            parent
        }
    }

    fn union(&mut self, left: &str, right: &str) {
        let mut left_root = self.find(left);
        let mut right_root = self.find(right);
        if left_root == right_root {
            return;
        }

        let left_rank = *self.rank.get(&left_root).unwrap_or(&0);
        let right_rank = *self.rank.get(&right_root).unwrap_or(&0);
        if left_rank < right_rank {
            std::mem::swap(&mut left_root, &mut right_root);
        }

        self.parent.insert(right_root.clone(), left_root.clone());
        if left_rank == right_rank {
            self.rank.insert(left_root, left_rank + 1);
        }
    }
}
