// tree.rs -- A Rooted Tree Backed by a Directed Graph
// ====================================================
//
// What Is a Tree?
// ---------------
//
// A **tree** is one of the most fundamental data structures in computer
// science. You encounter trees everywhere:
//
// - File systems: directories contain files and subdirectories
// - HTML/XML: elements contain child elements
// - Programming languages: Abstract Syntax Trees (ASTs) represent code
// - Organization charts: managers have direct reports
//
// Formally, a tree is a connected, acyclic graph where:
//
// 1. There is exactly **one root** node (a node with no parent).
// 2. Every other node has exactly **one parent**.
// 3. There are **no cycles**.
//
// These constraints mean a tree with N nodes always has exactly N-1 edges.
//
// Tree vs. Graph
// ~~~~~~~~~~~~~~
//
// A tree IS a graph (specifically, a directed acyclic graph with the
// single-parent constraint). We leverage this by building our Tree on top
// of the `Graph` type from the directed-graph crate. The `Graph` handles
// all the low-level node/edge storage, while this `Tree` type enforces
// the tree invariants and provides tree-specific operations like
// traversals, depth calculation, and lowest common ancestor.
//
// Edges point from parent to child:
//
//     Program
//     +-- Assignment    (edge: Program -> Assignment)
//     |   +-- Name      (edge: Assignment -> Name)
//     |   +-- BinaryOp  (edge: Assignment -> BinaryOp)
//     +-- Print         (edge: Program -> Print)
//
//
// Implementation Strategy
// -----------------------
//
// We store the tree as a `Graph` with edges pointing parent -> child.
// This means:
//
// - `graph.successors(node)` returns the children
// - `graph.predecessors(node)` returns a Vec with 0 or 1 element
//   (the parent, or empty for the root)
//
// We maintain the tree invariants by checking them in `add_child`:
//
// - The parent must already exist in the tree
// - The child must NOT already exist (no duplicate nodes)
// - Since we only add one parent edge per child, cycles are impossible

use std::collections::VecDeque;
use std::fmt;

use directed_graph::Graph;

use crate::errors::TreeError;

/// A rooted tree backed by a [`Graph`] from the directed-graph crate.
///
/// A tree is a directed graph with three constraints:
///
/// 1. Exactly one root (no predecessors)
/// 2. Every non-root node has exactly one parent
/// 3. No cycles
///
/// Edges point parent -> child. Build the tree by specifying a root node,
/// then adding children one at a time with [`add_child`].
///
/// # Example
///
/// ```
/// use tree::Tree;
///
/// let mut t = Tree::new("Program");
/// t.add_child("Program", "Assignment").unwrap();
/// t.add_child("Program", "Print").unwrap();
/// t.add_child("Assignment", "Name").unwrap();
/// t.add_child("Assignment", "BinaryOp").unwrap();
///
/// println!("{}", t.to_ascii());
/// ```
pub struct Tree {
    /// The underlying directed graph that stores nodes and edges.
    graph: Graph,
    /// The root node. Set at construction time, never changes.
    root: String,
}

impl Tree {
    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------
    // A tree always starts with a root. You can't have an empty tree.

    /// Create a new tree with the given root node.
    ///
    /// The root will be the ancestor of every other node in the tree.
    pub fn new(root: &str) -> Self {
        let mut graph = Graph::new();
        graph.add_node(root);
        Tree {
            graph,
            root: root.to_string(),
        }
    }

    // ------------------------------------------------------------------
    // Mutation
    // ------------------------------------------------------------------

    /// Add a child node under the given parent.
    ///
    /// This is the primary way to build up a tree. Each call adds one
    /// new node and one edge (parent -> child).
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if `parent` is not in the tree.
    /// Returns `Err(TreeError::DuplicateNode)` if `child` is already in the tree.
    ///
    /// Why not allow adding a node that already exists? Because in a tree,
    /// every node has exactly one parent. If we allowed adding "X" under
    /// both "A" and "B", node "X" would have two parents -- violating the
    /// tree invariant.
    pub fn add_child(&mut self, parent: &str, child: &str) -> Result<(), TreeError> {
        if !self.graph.has_node(parent) {
            return Err(TreeError::NodeNotFound(parent.to_string()));
        }
        if self.graph.has_node(child) {
            return Err(TreeError::DuplicateNode(child.to_string()));
        }

        // add_edge implicitly creates the child node and adds the edge.
        // It returns Result for self-loop detection, but we know parent != child
        // (because child doesn't exist yet), so unwrap is safe.
        self.graph
            .add_edge(parent, child)
            .expect("add_edge should not fail: parent != child");
        Ok(())
    }

    /// Remove a node and all its descendants from the tree.
    ///
    /// This is a "prune" operation -- it cuts off an entire branch.
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if `node` is not in the tree.
    /// Returns `Err(TreeError::RootRemoval)` if `node` is the root.
    pub fn remove_subtree(&mut self, node: &str) -> Result<(), TreeError> {
        if !self.graph.has_node(node) {
            return Err(TreeError::NodeNotFound(node.to_string()));
        }
        if node == self.root {
            return Err(TreeError::RootRemoval);
        }

        // Collect subtree via BFS, then remove in reverse (children first)
        let to_remove = self.collect_subtree_nodes(node);

        for n in to_remove.iter().rev() {
            let _ = self.graph.remove_node(n);
        }
        Ok(())
    }

    /// Collect all nodes in the subtree rooted at `node` using BFS.
    fn collect_subtree_nodes(&self, node: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(node.to_string());

        while let Some(current) = queue.pop_front() {
            result.push(current.clone());
            if let Ok(children) = self.graph.successors(&current) {
                let mut sorted_children = children;
                sorted_children.sort();
                for child in sorted_children {
                    queue.push_back(child);
                }
            }
        }

        result
    }

    // ------------------------------------------------------------------
    // Queries
    // ------------------------------------------------------------------

    /// The root node of the tree. Set at construction time, never changes.
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Return the parent of a node, or None if the node is the root.
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if the node doesn't exist.
    pub fn parent(&self, node: &str) -> Result<Option<String>, TreeError> {
        if !self.graph.has_node(node) {
            return Err(TreeError::NodeNotFound(node.to_string()));
        }

        let preds = self
            .graph
            .predecessors(node)
            .expect("node exists, predecessors should work");
        if preds.is_empty() {
            Ok(None)
        } else {
            Ok(Some(preds[0].clone()))
        }
    }

    /// Return the children of a node (sorted alphabetically).
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if the node doesn't exist.
    pub fn children(&self, node: &str) -> Result<Vec<String>, TreeError> {
        if !self.graph.has_node(node) {
            return Err(TreeError::NodeNotFound(node.to_string()));
        }

        let mut children = self
            .graph
            .successors(node)
            .expect("node exists, successors should work");
        children.sort();
        Ok(children)
    }

    /// Return the siblings of a node (other children of the same parent).
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if the node doesn't exist.
    pub fn siblings(&self, node: &str) -> Result<Vec<String>, TreeError> {
        if !self.graph.has_node(node) {
            return Err(TreeError::NodeNotFound(node.to_string()));
        }

        let parent_node = self.parent(node)?;
        match parent_node {
            None => Ok(Vec::new()),
            Some(p) => {
                let all_children = self.children(&p)?;
                Ok(all_children
                    .into_iter()
                    .filter(|c| c != node)
                    .collect())
            }
        }
    }

    /// Return true if the node has no children (a leaf).
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if the node doesn't exist.
    pub fn is_leaf(&self, node: &str) -> Result<bool, TreeError> {
        if !self.graph.has_node(node) {
            return Err(TreeError::NodeNotFound(node.to_string()));
        }

        let children = self
            .graph
            .successors(node)
            .expect("node exists");
        Ok(children.is_empty())
    }

    /// Return true if the node is the root of the tree.
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if the node doesn't exist.
    pub fn is_root(&self, node: &str) -> Result<bool, TreeError> {
        if !self.graph.has_node(node) {
            return Err(TreeError::NodeNotFound(node.to_string()));
        }

        Ok(node == self.root)
    }

    /// Return the depth of a node (distance from root).
    ///
    /// Root = 0, its children = 1, grandchildren = 2, etc.
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if the node doesn't exist.
    pub fn depth(&self, node: &str) -> Result<usize, TreeError> {
        if !self.graph.has_node(node) {
            return Err(TreeError::NodeNotFound(node.to_string()));
        }

        let mut d = 0;
        let mut current = node.to_string();
        while current != self.root {
            let preds = self
                .graph
                .predecessors(&current)
                .expect("node exists");
            current = preds[0].clone();
            d += 1;
        }

        Ok(d)
    }

    /// Return the height of the tree (maximum depth of any node).
    ///
    /// A single-node tree has height 0.
    pub fn height(&self) -> usize {
        let mut max_depth = 0;
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        queue.push_back((self.root.clone(), 0));

        while let Some((current, d)) = queue.pop_front() {
            if d > max_depth {
                max_depth = d;
            }
            if let Ok(children) = self.graph.successors(&current) {
                for child in children {
                    queue.push_back((child, d + 1));
                }
            }
        }

        max_depth
    }

    /// Return the total number of nodes in the tree.
    pub fn size(&self) -> usize {
        self.graph.size()
    }

    /// Return all nodes in the tree (sorted alphabetically).
    pub fn nodes(&self) -> Vec<String> {
        let mut nodes = self.graph.nodes();
        nodes.sort();
        nodes
    }

    /// Return all leaf nodes (sorted alphabetically).
    pub fn leaves(&self) -> Vec<String> {
        let mut leaves: Vec<String> = self
            .graph
            .nodes()
            .into_iter()
            .filter(|n| {
                self.graph
                    .successors(n)
                    .map(|s| s.is_empty())
                    .unwrap_or(false)
            })
            .collect();
        leaves.sort();
        leaves
    }

    /// Return true if the node exists in the tree.
    pub fn has_node(&self, node: &str) -> bool {
        self.graph.has_node(node)
    }

    // ------------------------------------------------------------------
    // Traversals
    // ------------------------------------------------------------------
    //
    // Tree traversals visit every node exactly once, in different orders.
    //
    // 1. **Preorder** (root first): Visit a node, then visit all its
    //    children. Top-down. Good for: copying a tree, prefix notation.
    //
    // 2. **Postorder** (root last): Visit all children, then the node.
    //    Bottom-up. Good for: computing sizes, deleting trees.
    //
    // 3. **Level-order** (BFS): Visit all nodes at depth 0, then 1,
    //    then 2, etc.
    //
    // For a tree:
    //       A
    //      / \
    //     B   C
    //    / \
    //   D   E
    //
    // Preorder:    A, B, D, E, C
    // Postorder:   D, E, B, C, A
    // Level-order: A, B, C, D, E

    /// Return nodes in preorder (parent before children).
    ///
    /// Uses an explicit stack. Children are pushed in reverse sorted order
    /// so the smallest pops first.
    pub fn preorder(&self) -> Vec<String> {
        let mut result = Vec::new();
        let mut stack = vec![self.root.clone()];

        while let Some(node) = stack.pop() {
            result.push(node.clone());
            if let Ok(mut children) = self.graph.successors(&node) {
                children.sort();
                children.reverse();
                stack.extend(children);
            }
        }

        result
    }

    /// Return nodes in postorder (children before parent).
    ///
    /// Uses a recursive helper. Children visited in sorted order.
    pub fn postorder(&self) -> Vec<String> {
        let mut result = Vec::new();
        self.postorder_recursive(&self.root, &mut result);
        result
    }

    fn postorder_recursive(&self, node: &str, result: &mut Vec<String>) {
        if let Ok(mut children) = self.graph.successors(node) {
            children.sort();
            for child in children {
                self.postorder_recursive(&child, result);
            }
        }
        result.push(node.to_string());
    }

    /// Return nodes in level-order (breadth-first).
    ///
    /// Classic BFS using a queue. Children visited in sorted order.
    pub fn level_order(&self) -> Vec<String> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(self.root.clone());

        while let Some(node) = queue.pop_front() {
            result.push(node.clone());
            if let Ok(mut children) = self.graph.successors(&node) {
                children.sort();
                for child in children {
                    queue.push_back(child);
                }
            }
        }

        result
    }

    // ------------------------------------------------------------------
    // Utilities
    // ------------------------------------------------------------------

    /// Return the path from the root to the given node.
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if the node doesn't exist.
    pub fn path_to(&self, node: &str) -> Result<Vec<String>, TreeError> {
        if !self.graph.has_node(node) {
            return Err(TreeError::NodeNotFound(node.to_string()));
        }

        let mut path = Vec::new();
        let mut current = Some(node.to_string());

        while let Some(ref c) = current {
            path.push(c.clone());
            let p = self.parent(c)?;
            current = p;
        }

        path.reverse();
        Ok(path)
    }

    /// Return the lowest common ancestor (LCA) of nodes a and b.
    ///
    /// The LCA is the deepest node that is an ancestor of both a and b.
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if a or b doesn't exist.
    pub fn lca(&self, a: &str, b: &str) -> Result<String, TreeError> {
        if !self.graph.has_node(a) {
            return Err(TreeError::NodeNotFound(a.to_string()));
        }
        if !self.graph.has_node(b) {
            return Err(TreeError::NodeNotFound(b.to_string()));
        }

        let path_a = self.path_to(a)?;
        let path_b = self.path_to(b)?;

        let mut lca_node = self.root.clone();
        let min_len = path_a.len().min(path_b.len());
        for i in 0..min_len {
            if path_a[i] == path_b[i] {
                lca_node = path_a[i].clone();
            } else {
                break;
            }
        }

        Ok(lca_node)
    }

    /// Extract the subtree rooted at the given node.
    ///
    /// Returns a NEW Tree. The original tree is not modified.
    ///
    /// Returns `Err(TreeError::NodeNotFound)` if the node doesn't exist.
    pub fn subtree(&self, node: &str) -> Result<Tree, TreeError> {
        if !self.graph.has_node(node) {
            return Err(TreeError::NodeNotFound(node.to_string()));
        }

        let mut new_tree = Tree::new(node);
        let mut queue = VecDeque::new();
        queue.push_back(node.to_string());

        while let Some(current) = queue.pop_front() {
            if let Ok(mut children) = self.graph.successors(&current) {
                children.sort();
                for child in children {
                    new_tree
                        .add_child(&current, &child)
                        .expect("adding child to subtree should not fail");
                    queue.push_back(child);
                }
            }
        }

        Ok(new_tree)
    }

    // ------------------------------------------------------------------
    // Visualization
    // ------------------------------------------------------------------

    /// Render the tree as an ASCII art diagram.
    ///
    /// Produces output like:
    ///
    /// ```text
    /// Program
    /// ├── Assignment
    /// │   ├── BinaryOp
    /// │   └── Name
    /// └── Print
    /// ```
    pub fn to_ascii(&self) -> String {
        let mut lines = Vec::new();
        self.ascii_recursive(&self.root, "", "", &mut lines);
        lines.join("\n")
    }

    fn ascii_recursive(
        &self,
        node: &str,
        prefix: &str,
        child_prefix: &str,
        lines: &mut Vec<String>,
    ) {
        lines.push(format!("{}{}", prefix, node));
        if let Ok(mut children) = self.graph.successors(node) {
            children.sort();
            let len = children.len();
            for (i, child) in children.iter().enumerate() {
                if i < len - 1 {
                    self.ascii_recursive(
                        child,
                        &format!("{}\u{251C}\u{2500}\u{2500} ", child_prefix),
                        &format!("{}\u{2502}   ", child_prefix),
                        lines,
                    );
                } else {
                    self.ascii_recursive(
                        child,
                        &format!("{}\u{2514}\u{2500}\u{2500} ", child_prefix),
                        &format!("{}    ", child_prefix),
                        lines,
                    );
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Graph access
    // ------------------------------------------------------------------

    /// Access the underlying directed graph.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tree(root={:?}, size={})", self.root, self.size())
    }
}
