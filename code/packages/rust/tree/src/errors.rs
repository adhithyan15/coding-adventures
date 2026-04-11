// errors.rs -- Custom Error Types for the Tree Library
// ====================================================
//
// Trees impose strict structural constraints on top of directed graphs.
// When those constraints are violated, we need clear, specific errors
// rather than generic String messages. Each variant of TreeError
// corresponds to one particular kind of violation:
//
// - NodeNotFound -- raised when you reference a node that doesn't exist
//   in the tree.
//
// - DuplicateNode -- raised when you try to add a node that already
//   exists. In a tree, every node name must be unique.
//
// - RootRemoval -- raised when you try to remove the root node. The
//   root is the anchor of the entire tree.
//
// We use a single enum rather than separate types because Rust's
// Result<T, E> works best with a unified error type. We implement
// Display and std::error::Error manually to keep this package
// dependency-free.

use std::fmt;

/// All errors that can occur when operating on a [`Tree`].
///
/// Each variant carries enough context for callers to produce useful
/// error messages.
///
/// # Variants
///
/// - **NodeNotFound(String)**: an operation referenced a node that
///   doesn't exist in the tree (e.g., `parent("X")` when X was
///   never added).
///
/// - **DuplicateNode(String)**: `add_child` was called with a child
///   name that already exists in the tree. In a tree, every node has
///   exactly one parent, so duplicates violate the tree invariant.
///
/// - **RootRemoval**: `remove_subtree` was called on the root node.
///   The root is the anchor of the tree; removing it would destroy
///   the connected structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    /// The specified node does not exist in the tree.
    NodeNotFound(String),
    /// The specified node already exists in the tree.
    DuplicateNode(String),
    /// Cannot remove the root node.
    RootRemoval,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeError::NodeNotFound(node) => {
                write!(f, "node not found in tree: {:?}", node)
            }
            TreeError::DuplicateNode(node) => {
                write!(f, "node already exists in tree: {:?}", node)
            }
            TreeError::RootRemoval => write!(f, "cannot remove the root node"),
        }
    }
}

impl std::error::Error for TreeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_are_specific() {
        assert_eq!(
            TreeError::NodeNotFound("child".into()).to_string(),
            r#"node not found in tree: "child""#
        );
        assert_eq!(
            TreeError::DuplicateNode("child".into()).to_string(),
            r#"node already exists in tree: "child""#
        );
        assert_eq!(
            TreeError::RootRemoval.to_string(),
            "cannot remove the root node"
        );
    }
}
