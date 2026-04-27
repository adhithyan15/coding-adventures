use std::fmt;

/// GraphError represents all possible errors from graph operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// Node not found in the graph
    NodeNotFound(String),
    /// Edge not found in the graph
    EdgeNotFound(String, String),
    /// Self-loops are not allowed
    SelfLoop(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NodeNotFound(node) => {
                write!(f, "Node not found: {}", node)
            }
            GraphError::EdgeNotFound(from, to) => {
                write!(f, "Edge not found: {} -- {}", from, to)
            }
            GraphError::SelfLoop(node) => {
                write!(f, "Self-loop not allowed: {}", node)
            }
        }
    }
}

impl std::error::Error for GraphError {}
