//! HIR statement nodes — the body of a Process.
//!
//! Statements are sequential within a Process. Their semantics depend on
//! whether the enclosing Process uses a sensitivity list (Verilog `always @`)
//! or explicit `wait` suspensions (VHDL `process ... wait ...`). The
//! simulation VM implements both execution models.
//!
//! JSON uses a `"kind"` discriminator matching the Python reference impl.
//!
//! ## Statement kinds
//!
//! | Kind | Meaning |
//! |---|---|
//! | `Blocking` | `=` in Verilog, `:=` for variables in VHDL — immediate update |
//! | `Nonblocking` | `<=` — deferred to next delta cycle |
//! | `If` / `Case` | Conditional branching |
//! | `For` / `While` / `Repeat` / `Forever` | Loops |
//! | `Wait` / `Delay` / `Event` | Simulation-time suspension |
//! | `Assert` / `Report` | Verification and logging |
//! | `Null` / `Return` / `Disable` / `ExprStmt` | Misc |

use serde::{Deserialize, Serialize};

use crate::expr::Expr;
use crate::provenance::Provenance;

// ---------------------------------------------------------------------------
// CaseItem
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseItem {
    pub choices: Vec<Expr>,
    pub body: Vec<Stmt>,
}

// ---------------------------------------------------------------------------
// Event (for EventStmt)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub edge: String, // "posedge" | "negedge" | "change"
    pub expr: Expr,
}

// ---------------------------------------------------------------------------
// Stmt enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Stmt {
    Blocking {
        target: Expr,
        rhs: Expr,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay: Option<Box<Expr>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Nonblocking {
        target: Expr,
        rhs: Expr,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay: Option<Box<Expr>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    #[serde(rename = "if")]
    If {
        cond: Expr,
        then_branch: Vec<Stmt>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        else_branch: Vec<Stmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Case {
        expr: Expr,
        #[serde(default, rename = "case_kind")]
        case_kind: String,
        items: Vec<CaseItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<Vec<Stmt>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    #[serde(rename = "for")]
    For {
        init: Box<Stmt>,
        cond: Expr,
        step: Box<Stmt>,
        body: Vec<Stmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Repeat {
        count: Expr,
        body: Vec<Stmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Forever {
        body: Vec<Stmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Wait {
        #[serde(default)]
        on: Vec<Expr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        until: Option<Box<Expr>>,
        #[serde(rename = "for", skip_serializing_if = "Option::is_none")]
        for_: Option<Box<Expr>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Delay {
        amount: Expr,
        body: Vec<Stmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Event {
        events: Vec<Edge>,
        body: Vec<Stmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Assert {
        cond: Expr,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<Box<Expr>>,
        #[serde(default = "default_error")]
        severity: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Report {
        message: Expr,
        #[serde(default = "default_note")]
        severity: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Disable {
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Return {
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<Box<Expr>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    Null {
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
    ExprStmt {
        expr: Expr,
        #[serde(skip_serializing_if = "Option::is_none")]
        provenance: Option<Provenance>,
    },
}

fn default_error() -> String { "error".to_string() }
fn default_note() -> String { "note".to_string() }
