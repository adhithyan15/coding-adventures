//! # Undirected Graph — graph data structure and algorithms.
//!
//! An undirected graph is a set of nodes connected by edges, where each edge
//! has no direction — it connects two nodes symmetrically. Think of it like a
//! two-way street map: if you can travel from A to B, you can also travel from
//! B to A.
//!
//! Undirected graphs are useful for:
//! - Social networks (friendships are mutual)
//! - Road networks (two-way streets)
//! - Game maps (can move between adjacent cells both ways)
//! - Peer-to-peer networks (symmetric connections)
//!
//! # Features
//!
//! - **Graph construction** -- add and remove nodes and edges
//! - **Traversal** -- BFS and DFS to explore the graph
//! - **Connectivity** -- find connected components, check if graph is connected
//! - **Shortest path** -- find shortest path between two nodes (BFS)
//! - **Cycle detection** -- detect if graph contains cycles
//! - **Degree queries** -- find neighbors and degree of nodes
//!
//! # Example
//!
//! ```
//! use graph::Graph;
//!
//! let mut g = Graph::new();
//! g.add_edge("A", "B").unwrap();
//! g.add_edge("B", "C").unwrap();
//! g.add_edge("A", "C").unwrap();
//!
//! // Check neighbors
//! let neighbors = g.neighbors("A").unwrap();
//! // neighbors contains "B" and "C"
//! ```

pub mod graph;
pub mod errors;
pub mod traversal;

pub use graph::Graph;
pub use errors::GraphError;
pub use traversal::{bfs, TraversalGraph};
