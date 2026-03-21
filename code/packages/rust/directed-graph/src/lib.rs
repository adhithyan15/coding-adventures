//! # Directed Graph — dependency tracking for build systems and beyond.
//!
//! A directed graph (or "digraph") is a set of nodes connected by edges,
//! where each edge has a direction — it goes FROM one node TO another.
//! Think of it like a one-way street map: you can travel from A to B,
//! but that doesn't mean you can travel from B to A.
//!
//! In this build system, nodes are packages and edges are dependencies:
//! if package A depends on package B, there's an edge from B to A
//! (B must be built before A).
//!
//! # Why a directed graph?
//!
//! The dependency relationships between packages form a DAG (Directed
//! Acyclic Graph). A DAG has no cycles — you can't have A depend on B
//! depend on C depend on A. The key algorithms on a DAG are:
//!
//! - **Topological sort**: order nodes so every dependency comes before
//!   the things that depend on it. This gives you a valid build order.
//!
//! - **Independent groups**: partition nodes into "levels" where everything
//!   at the same level can run in parallel. Level 0 has no dependencies.
//!   Level 1 depends only on level 0. And so on.
//!
//! - **Affected nodes**: given a set of changed nodes, find everything that
//!   transitively depends on them. These are the packages that need
//!   rebuilding when something changes.
//!
//! # Example
//!
//! ```
//! use directed_graph::Graph;
//!
//! let mut g = Graph::new();
//! g.add_edge("compile", "link").unwrap();
//! g.add_edge("link", "package").unwrap();
//!
//! let order = g.topological_sort().unwrap();
//! assert_eq!(order, vec!["compile", "link", "package"]);
//! ```

pub mod graph;

pub use graph::{Graph, GraphError};
