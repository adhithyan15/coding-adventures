//! # Tree -- A rooted tree data structure backed by a directed graph.
//!
//! A tree is one of the most fundamental data structures in computer science.
//! You encounter trees everywhere: file systems, HTML/XML, Abstract Syntax
//! Trees (ASTs), organization charts, and more.
//!
//! This crate provides a `Tree` type that wraps the `Graph` from the
//! `directed-graph` crate, enforcing tree invariants (single root, single
//! parent per node, no cycles) and providing tree-specific operations:
//!
//! - **Traversals**: preorder, postorder, level-order
//! - **Queries**: parent, children, siblings, depth, height, leaves
//! - **Utilities**: path_to, lowest common ancestor, subtree extraction
//! - **Visualization**: ASCII art rendering
//!
//! # Example
//!
//! ```
//! use tree::Tree;
//!
//! let mut t = Tree::new("root");
//! t.add_child("root", "child1").unwrap();
//! t.add_child("root", "child2").unwrap();
//! t.add_child("child1", "grandchild").unwrap();
//!
//! assert_eq!(t.size(), 4);
//! assert_eq!(t.height(), 2);
//! println!("{}", t.to_ascii());
//! ```

pub mod errors;
pub mod tree;

pub use errors::TreeError;
pub use tree::Tree;
