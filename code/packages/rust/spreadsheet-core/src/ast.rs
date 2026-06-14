//! Formula abstract syntax tree.

use crate::address::{CellAddress, CellRange};
use crate::cell::CellValue;

/// One node in a parsed formula.
#[derive(Debug, Clone, PartialEq)]
pub enum FormulaAst {
    /// A literal value (number, text, bool, error).
    Literal(CellValue),
    /// A single-cell reference.
    Ref(CellAddress),
    /// A rectangular range reference.
    Range(CellRange),
    /// Unary prefix operator.
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand.
        operand: Box<FormulaAst>,
    },
    /// Binary infix operator.
    Binary {
        /// The operator.
        op: BinaryOp,
        /// Left operand.
        lhs: Box<FormulaAst>,
        /// Right operand.
        rhs: Box<FormulaAst>,
    },
    /// Percent-suffix (`x%` = `x / 100`).
    Percent(Box<FormulaAst>),
    /// Function call: `Name(arg1, arg2, …)`.
    Call {
        /// The function name as the user typed it (case preserved
        /// for diagnostics; dispatch is case-insensitive).
        name: String,
        /// Argument expressions.
        args: Vec<FormulaAst>,
    },
}

/// Unary operators supported in formulas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-x` — negation.
    Negate,
    /// `+x` — unary plus (identity).
    Plus,
}

/// Binary operators supported in formulas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    /// `+` — addition.
    Add,
    /// `-` — subtraction.
    Sub,
    /// `*` — multiplication.
    Mul,
    /// `/` — division.
    Div,
    /// `^` — exponentiation (right-associative).
    Pow,
    /// `&` — text concatenation.
    Concat,
    /// `=`.
    Eq,
    /// `<>`.
    Ne,
    /// `<`.
    Lt,
    /// `<=`.
    Le,
    /// `>`.
    Gt,
    /// `>=`.
    Ge,
}

impl BinaryOp {
    /// Precedence rank — bigger binds tighter.
    /// Matches Excel: comparison < concat < add/sub < mul/div < pow.
    pub fn precedence(self) -> u8 {
        match self {
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => 1,
            BinaryOp::Concat => 2,
            BinaryOp::Add | BinaryOp::Sub => 3,
            BinaryOp::Mul | BinaryOp::Div => 4,
            BinaryOp::Pow => 5,
        }
    }

    /// Whether the operator is right-associative (only `^`).
    pub fn right_associative(self) -> bool {
        matches!(self, BinaryOp::Pow)
    }
}
