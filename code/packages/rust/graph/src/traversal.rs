// traversal.rs -- Shared Graph Traversal Traits and Algorithms
// ============================================================
//
// This module defines the `TraversalGraph` trait and the generic `bfs`
// function. Both are designed to be *reusable across graph types*: the
// undirected `Graph` in this crate implements `TraversalGraph`, and so
// does the directed `Graph` in the `directed-graph` crate.
//
// Why a trait instead of a concrete function?
// --------------------------------------------
//
// If we wrote `bfs(graph: &Graph, ...)` we would be locked to the
// undirected graph type. The directed graph crate would have to
// duplicate the BFS loop. By abstracting over any graph that exposes
// `has_node` + `neighbors`, we get one correct BFS implementation
// shared across the whole project.
//
// The trait is deliberately minimal — it only requires the two
// operations that BFS actually needs.

use std::collections::{BTreeSet, VecDeque};

// ---------------------------------------------------------------------------
// TraversalGraph — the minimal interface required for BFS/DFS
// ---------------------------------------------------------------------------
//
// Any graph type that wants to participate in the shared traversal
// algorithms must implement this trait.
//
//   `has_node(node)` -- returns true if the node exists
//   `neighbors(node)` -- returns the neighbors (or successors) of the node
//
// The associated `Error` type lets each graph use its own error type.
// For the undirected `Graph` that's `GraphError`; for `directed_graph::Graph`
// it's that crate's `GraphError`.

/// A minimal interface for graph types that support BFS/DFS traversal.
pub trait TraversalGraph {
    /// The error type returned when a node is not found.
    type Error;

    /// Returns `true` if the node exists in the graph.
    fn has_node(&self, node: &str) -> bool;

    /// Returns the neighbors (for undirected graphs) or successors (for
    /// directed graphs) of the given node.
    fn neighbors(&self, node: &str) -> Result<Vec<String>, Self::Error>;
}

// ---------------------------------------------------------------------------
// bfs -- Breadth-First Search
// ---------------------------------------------------------------------------
//
// BFS explores the graph level by level: first the start node, then all
// its immediate neighbors, then all their neighbors, and so on.
//
// Internally we use a `BTreeSet` (sorted set) for the visited set so that
// the traversal order is deterministic -- important for tests and for
// reproducible build plans.
//
// The function returns `Err` if the start node does not exist.
//
// # Example
//
//   Graph: A -- B -- C
//   bfs(&g, "A") => Ok(["A", "B", "C"])

/// Perform a breadth-first traversal from `start`, returning nodes in
/// visit order.
///
/// Returns `Err` if `start` does not exist in the graph.
pub fn bfs<G>(graph: &G, start: &str) -> Result<Vec<String>, G::Error>
where
    G: TraversalGraph,
{
    // Validate start node by attempting to fetch its neighbors.
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
