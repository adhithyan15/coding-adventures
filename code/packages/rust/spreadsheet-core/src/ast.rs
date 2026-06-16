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

    /// The source token for this operator (`+`, `<=`, `&`, …).
    pub fn symbol(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Pow => "^",
            BinaryOp::Concat => "&",
            BinaryOp::Eq => "=",
            BinaryOp::Ne => "<>",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
        }
    }
}

impl FormulaAst {
    /// Render this AST back to a formula string (without the leading `=`).
    ///
    /// Binary operators are **fully parenthesised**, so the output always
    /// re-parses to an equivalent tree regardless of precedence — the result may
    /// carry redundant parens (`(A1+(A2*3))`) but never the wrong grouping. This
    /// is used to refresh a cell's stored source text after a structural edit
    /// (insert/delete rows or columns) rewrites its references via
    /// [`FormulaAst::adjust`](crate::edit), so the echoed formula keeps naming
    /// the cells the rewritten AST points at.
    pub fn to_formula_string(&self) -> String {
        match self {
            FormulaAst::Literal(v) => literal_source(v),
            FormulaAst::Ref(addr) => addr.to_a1(),
            FormulaAst::Range(range) => format!("{}:{}", range.start.to_a1(), range.end.to_a1()),
            FormulaAst::Unary { op, operand } => {
                let inner = operand.to_formula_string();
                match op {
                    UnaryOp::Negate => format!("-{inner}"),
                    UnaryOp::Plus => format!("+{inner}"),
                }
            }
            FormulaAst::Binary { op, lhs, rhs } => format!(
                "({}{}{})",
                lhs.to_formula_string(),
                op.symbol(),
                rhs.to_formula_string()
            ),
            FormulaAst::Percent(inner) => format!("{}%", inner.to_formula_string()),
            FormulaAst::Call { name, args } => {
                let parts: Vec<String> = args.iter().map(FormulaAst::to_formula_string).collect();
                format!("{}({})", name, parts.join(","))
            }
        }
    }
}

/// Render a literal value back to its formula-source form: numbers bare
/// (integers without a trailing `.0`), text double-quoted with `"` doubled,
/// booleans as `TRUE`/`FALSE`, errors as their `#…!` code.
fn literal_source(v: &CellValue) -> String {
    match v {
        CellValue::Empty => String::new(),
        CellValue::Boolean(true) => "TRUE".to_string(),
        CellValue::Boolean(false) => "FALSE".to_string(),
        CellValue::Number(n) => {
            if *n == n.trunc() && n.abs() < 1e16 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        CellValue::Text(s) => format!("\"{}\"", s.replace('"', "\"\"")),
        CellValue::Error(e) => e.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;

    /// `to_formula_string` must produce something that re-parses to the *same*
    /// tree (modulo the redundant parens it adds), for a spread of node kinds.
    fn round_trips(src: &str) {
        let ast = parse(src).unwrap();
        let rendered = ast.to_formula_string();
        let reparsed = parse(&format!("={rendered}")).unwrap();
        assert_eq!(ast, reparsed, "src {src:?} -> {rendered:?} -> reparsed differs");
    }

    #[test]
    fn serializer_round_trips_across_node_kinds() {
        for src in [
            "=A1",
            "=$A$1",
            "=A1+A2",
            "=A1+A2*3",       // precedence preserved by full parenthesisation
            "=(A1+A2)*3",
            "=-A1",
            "=A1%",
            "=SUM(A1:A4)",
            "=IF(A1>0,A1,-A1)",
            "=A1&\" txt\"",
            "=2^3^2",          // right-assoc
            "=A1<=B1",
        ] {
            round_trips(src);
        }
    }

    #[test]
    fn serializer_renders_literals() {
        assert_eq!(parse("=42").unwrap().to_formula_string(), "42");
        assert_eq!(parse("=3.5").unwrap().to_formula_string(), "3.5");
        assert_eq!(parse("=\"hi\"").unwrap().to_formula_string(), "\"hi\"");
        assert_eq!(parse("=SUM(A1:A2)").unwrap().to_formula_string(), "SUM(A1:A2)");
    }
}
