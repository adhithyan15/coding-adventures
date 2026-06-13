//! HIR expression nodes.
//!
//! Expressions appear on the right-hand side of assignments, as conditions
//! in `if`/`case`, as loop bounds, and as parameter values. Every expression
//! carries an optional provenance so diagnostics can cite the source line.
//!
//! The expression tree is a recursive enum (`Box<Expr>` for sub-expressions).
//! JSON uses a `"kind"` discriminator, matching the Python reference impl.
//!
//! ## Expression kinds
//!
//! ```text
//! Atomic:     Lit, NetRef, VarRef, PortRef
//! Composite:  Slice, Concat, Replication
//! Operators:  UnaryOp (NOT, NEG, AND_RED, …), BinaryOp (+, -, AND, OR, …)
//! Control:    Ternary (cond ? then : else)
//! Calls:      FunCall, SystemCall ($display, $time, …), Attribute ('event)
//! ```

use serde::{Deserialize, Serialize};

use crate::provenance::Provenance;
use crate::types::Ty;

// ---------------------------------------------------------------------------
// Valid operator sets (validated at construction)
// ---------------------------------------------------------------------------

pub const UNARY_OPS: &[&str] = &[
    "NOT", "NEG", "POS", "AND_RED", "OR_RED", "XOR_RED",
    "NAND_RED", "NOR_RED", "XNOR_RED", "LOGIC_NOT",
];

pub const BINARY_OPS: &[&str] = &[
    "+", "-", "*", "/", "%", "**",
    "AND", "OR", "XOR", "NAND", "NOR", "XNOR",
    "<<", ">>", "<<<", ">>>",
    "<", "<=", ">", ">=", "==", "!=", "===", "!==",
    "&&", "||", "&", "|", "^",
];

// ---------------------------------------------------------------------------
// Literal value (scalar or bit-vector)
// ---------------------------------------------------------------------------

/// A literal value: integer, boolean, float, string, or bit-vector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LitValue {
    Int(i64),
    Bool(bool),
    Float(f64),
    Str(String),
    Bits(Vec<u8>),
}

// ---------------------------------------------------------------------------
// Expr enum — the complete expression tree
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expr {
    // --- Atomic ---
    Lit {
        value: LitValue,
        #[serde(rename = "type")]
        ty: Ty,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    NetRef {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    VarRef {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    PortRef {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },

    // --- Composite ---
    Slice {
        base: Box<Expr>,
        msb: u32,
        lsb: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Concat {
        parts: Vec<Expr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Replication {
        count: Box<Expr>,
        body: Box<Expr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },

    // --- Operators ---
    Unary {
        op: String,
        operand: Box<Expr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Binary {
        op: String,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },

    // --- Calls ---
    FunCall {
        name: String,
        #[serde(default)]
        args: Vec<Expr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    SystemCall {
        name: String,  // must start with '$'
        #[serde(default)]
        args: Vec<Expr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Attr {
        base: Box<Expr>,
        name: String,
        #[serde(default)]
        args: Vec<Expr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

impl Expr {
    pub fn int_lit(value: i64, width: u32) -> Self {
        Expr::Lit {
            value: LitValue::Int(value),
            ty: crate::types::Ty::vec(width),
            provenance: None,
        }
    }

    pub fn net_ref(name: impl Into<String>) -> Self {
        Expr::NetRef { name: name.into(), provenance: None }
    }

    pub fn port_ref(name: impl Into<String>) -> Self {
        Expr::PortRef { name: name.into(), provenance: None }
    }

    pub fn binary(op: impl Into<String>, lhs: Expr, rhs: Expr) -> Self {
        let op = op.into();
        debug_assert!(BINARY_OPS.contains(&op.as_str()), "unknown binary op: {op}");
        Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs), provenance: None }
    }

    pub fn unary(op: impl Into<String>, operand: Expr) -> Self {
        let op = op.into();
        debug_assert!(UNARY_OPS.contains(&op.as_str()), "unknown unary op: {op}");
        Expr::Unary { op, operand: Box::new(operand), provenance: None }
    }
}
