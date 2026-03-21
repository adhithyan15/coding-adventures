//! # Abstract Syntax Tree (AST) — the structured representation of source code.
//!
//! After the lexer breaks source code into tokens, the parser arranges those
//! tokens into a tree that reflects the **meaning** of the code. This tree is
//! called an Abstract Syntax Tree, or AST.
//!
//! # Why "abstract"?
//!
//! The AST is "abstract" because it throws away syntactic details that don't
//! affect meaning. Consider these two expressions:
//!
//! ```text
//! 1 + 2 * 3       (no parentheses)
//! 1 + (2 * 3)     (explicit parentheses)
//! ```
//!
//! Both produce the same AST:
//!
//! ```text
//!        BinaryOp(+)
//!        /         \
//!   Number(1)   BinaryOp(*)
//!               /         \
//!          Number(2)   Number(3)
//! ```
//!
//! The parentheses guided the parser's decisions, but they don't appear in the
//! tree. The tree structure itself encodes the grouping.
//!
//! # Why Rust enums are ideal for ASTs
//!
//! In Go, the AST uses interfaces with marker methods:
//!
//! ```text
//! type Expression interface { isExpression() }
//! type NumberLiteral struct { Value int }
//! func (NumberLiteral) isExpression() {}
//! ```
//!
//! This works, but has drawbacks:
//! - The compiler does not enforce exhaustive handling of all node types.
//! - Each node type needs a boilerplate marker method.
//! - There is no single place that lists all possible expressions.
//!
//! In Rust, a single `enum` captures all node types, and `match` requires
//! exhaustive handling:
//!
//! ```text
//! enum ASTNode {
//!     Number(f64),
//!     String(String),
//!     Name(String),
//!     BinaryOp { left: Box<ASTNode>, op: String, right: Box<ASTNode> },
//!     ...
//! }
//! ```
//!
//! The compiler will warn (or error) if a `match` on `ASTNode` forgets a
//! variant. This makes it much harder to accidentally ignore a node type
//! when writing a code generator, interpreter, or pretty-printer.
//!
//! # Why `Box<ASTNode>` for recursive types?
//!
//! Rust needs to know the size of every type at compile time. But an AST node
//! can contain other AST nodes, creating a recursive type of unknown size:
//!
//! ```text
//! BinaryOp contains ASTNode, which might be BinaryOp, which contains ASTNode...
//! ```
//!
//! `Box<T>` solves this by storing the inner node on the heap. A `Box` itself
//! is always pointer-sized (8 bytes on 64-bit systems), regardless of what it
//! points to. This gives the compiler a fixed size for the enum.
//!
//! Think of `Box` as "I own this data, but it lives on the heap." For an AST,
//! this is perfect — parent nodes own their children.

use std::fmt;

// ===========================================================================
// The AST node enum
// ===========================================================================

/// A node in the abstract syntax tree.
///
/// Each variant represents one kind of construct in our Python-like language:
///
/// | Variant       | Example source  | Description                              |
/// |---------------|-----------------|------------------------------------------|
/// | `Number`      | `42`            | A numeric literal                        |
/// | `String`      | `"hello"`       | A string literal                         |
/// | `Name`        | `x`             | A variable reference                     |
/// | `BinaryOp`    | `1 + 2`         | An arithmetic operation                  |
/// | `Assignment`  | `x = 42`        | Variable assignment                      |
/// | `ExpressionStmt` | `1 + 2`     | An expression used as a statement        |
/// | `Program`     | (whole file)    | A sequence of statements                 |
///
/// # Ownership and memory layout
///
/// Recursive variants (`BinaryOp`, `Assignment`, `ExpressionStmt`) use
/// `Box<ASTNode>` for their children. This heap-allocates the child nodes,
/// giving the enum a fixed stack size. Without `Box`, the compiler would
/// reject the type as infinitely sized.
#[derive(Debug, Clone, PartialEq)]
pub enum ASTNode {
    /// A numeric literal.
    ///
    /// Stores the value as `f64` to support both integers and decimals.
    /// Even though our current lexer only produces integers, using `f64`
    /// future-proofs the AST for when we add floating-point support.
    ///
    /// ```text
    /// Source: 42
    /// AST:    Number(42.0)
    /// ```
    Number(f64),

    /// A string literal.
    ///
    /// The value stored here has already been processed by the lexer:
    /// escape sequences like `\n` have been converted to actual newline
    /// characters. The surrounding quotes are not included.
    ///
    /// ```text
    /// Source: "hello\nworld"
    /// AST:    String("hello\nworld")   // actual newline, not \n
    /// ```
    String(String),

    /// A variable reference (identifier).
    ///
    /// In an expression like `x + 1`, the `x` becomes a `Name("x")` node.
    /// At this stage, we don't know what `x` refers to — that's a job for
    /// later stages (name resolution, type checking).
    ///
    /// ```text
    /// Source: x
    /// AST:    Name("x")
    /// ```
    Name(String),

    /// A binary operation: two operands connected by an operator.
    ///
    /// Binary operations are the heart of expression parsing. The `left`
    /// and `right` fields are `Box<ASTNode>` because they can be any
    /// expression — including other `BinaryOp` nodes (for chained
    /// operations like `1 + 2 + 3`).
    ///
    /// ```text
    /// Source: 1 + 2 * 3
    /// AST:    BinaryOp {
    ///             left: Number(1),
    ///             op: "+",
    ///             right: BinaryOp {
    ///                 left: Number(2),
    ///                 op: "*",
    ///                 right: Number(3),
    ///             }
    ///         }
    /// ```
    ///
    /// Note how operator precedence is encoded in the tree structure:
    /// `*` is deeper (evaluated first), `+` is at the top (evaluated last).
    BinaryOp {
        left: Box<ASTNode>,
        op: String,
        right: Box<ASTNode>,
    },

    /// A variable assignment statement.
    ///
    /// Assignments bind a value to a name. The `target` is always a simple
    /// name (we don't support destructuring or attribute assignment yet).
    ///
    /// ```text
    /// Source: x = 42
    /// AST:    Assignment {
    ///             target: "x",
    ///             value: Number(42),
    ///         }
    /// ```
    Assignment {
        target: String,
        value: Box<ASTNode>,
    },

    /// An expression used as a statement.
    ///
    /// In Python, any expression can appear as a statement on its own line.
    /// The value is computed but (usually) discarded. This is common for
    /// function calls: `print("hello")` is an expression statement.
    ///
    /// ```text
    /// Source: 1 + 2
    /// AST:    ExpressionStmt(BinaryOp { ... })
    /// ```
    ExpressionStmt(Box<ASTNode>),

    /// The top-level program: a sequence of statements.
    ///
    /// Every parsed source file produces exactly one `Program` node at
    /// the root of the AST. Its children are the statements that make
    /// up the program, in order.
    ///
    /// ```text
    /// Source: x = 1
    ///         y = 2
    /// AST:    Program([
    ///             Assignment { target: "x", value: Number(1) },
    ///             Assignment { target: "y", value: Number(2) },
    ///         ])
    /// ```
    Program(Vec<ASTNode>),
}

/// Pretty-print AST nodes for debugging and test output.
///
/// This implementation produces a compact, readable representation that
/// makes it easy to verify parse results in tests and error messages.
impl fmt::Display for ASTNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ASTNode::Number(n) => write!(f, "Number({})", n),
            ASTNode::String(s) => write!(f, "String({:?})", s),
            ASTNode::Name(n) => write!(f, "Name({})", n),
            ASTNode::BinaryOp { left, op, right } => {
                write!(f, "BinaryOp({} {} {})", left, op, right)
            }
            ASTNode::Assignment { target, value } => {
                write!(f, "Assignment({} = {})", target, value)
            }
            ASTNode::ExpressionStmt(expr) => {
                write!(f, "ExpressionStmt({})", expr)
            }
            ASTNode::Program(stmts) => {
                write!(f, "Program([")?;
                for (i, stmt) in stmts.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", stmt)?;
                }
                write!(f, "])")
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that AST nodes can be compared for equality.
    ///
    /// The `PartialEq` derive on `ASTNode` enables deep structural comparison,
    /// which is essential for testing parse results.
    #[test]
    fn test_ast_equality() {
        let a = ASTNode::Number(42.0);
        let b = ASTNode::Number(42.0);
        let c = ASTNode::Number(99.0);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// Test that BinaryOp nodes with Box children compare correctly.
    #[test]
    fn test_binary_op_equality() {
        let node = ASTNode::BinaryOp {
            left: Box::new(ASTNode::Number(1.0)),
            op: "+".to_string(),
            right: Box::new(ASTNode::Number(2.0)),
        };

        let same = ASTNode::BinaryOp {
            left: Box::new(ASTNode::Number(1.0)),
            op: "+".to_string(),
            right: Box::new(ASTNode::Number(2.0)),
        };

        assert_eq!(node, same);
    }

    /// Test Display formatting for debugging.
    #[test]
    fn test_display() {
        let node = ASTNode::BinaryOp {
            left: Box::new(ASTNode::Number(1.0)),
            op: "+".to_string(),
            right: Box::new(ASTNode::Number(2.0)),
        };
        assert_eq!(format!("{}", node), "BinaryOp(Number(1) + Number(2))");
    }

    /// Test that Program contains multiple statements.
    #[test]
    fn test_program_display() {
        let prog = ASTNode::Program(vec![
            ASTNode::Assignment {
                target: "x".to_string(),
                value: Box::new(ASTNode::Number(1.0)),
            },
            ASTNode::ExpressionStmt(Box::new(ASTNode::Name("x".to_string()))),
        ]);
        let display = format!("{}", prog);
        assert!(display.starts_with("Program(["));
        assert!(display.contains("Assignment"));
    }
}
