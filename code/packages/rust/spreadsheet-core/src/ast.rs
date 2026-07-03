//! Formula abstract syntax tree.

use crate::address::{CellAddress, CellRange};
use crate::cell::CellValue;
use crate::errors::SpreadsheetError;

/// One node in a parsed formula.
#[derive(Debug, Clone, PartialEq)]
pub enum FormulaAst {
    /// A literal value (number, text, bool, error).
    Literal(CellValue),
    /// A single-cell reference, optionally qualified by a sheet name.
    ///
    /// `sheet` is `None` for the common same-sheet reference (`A1`) — that case
    /// resolves against the formula's own sheet and is byte-identical to the
    /// pre-multi-sheet behaviour. `Some(name)` is a cross-sheet reference
    /// (`Summary!A1`); the name is the sheet *as written* (resolved to a
    /// `SheetId` later, at evaluation/dependency time, by a workbook that knows
    /// its sheets). See [`FormulaAst::cell`] / [`FormulaAst::sheet_cell`].
    Ref {
        /// The qualifying sheet name, or `None` for the formula's own sheet.
        sheet: Option<String>,
        /// The cell address (column/row, with `$`-absolute flags).
        addr: CellAddress,
    },
    /// A rectangular range reference, optionally qualified by a sheet name.
    ///
    /// Both endpoints share the one qualifier (`Summary!A1:B2`); split-sheet
    /// 3-D ranges (`Sheet1:Sheet3!A1`) are intentionally out of scope. See
    /// [`FormulaAst::cell_range`] / [`FormulaAst::sheet_range`].
    Range {
        /// The qualifying sheet name, or `None` for the formula's own sheet.
        sheet: Option<String>,
        /// The rectangular range (start/end addresses).
        range: CellRange,
    },
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
    /// A same-sheet single-cell reference (`A1`) — the common case.
    pub fn cell(addr: CellAddress) -> FormulaAst {
        FormulaAst::Ref { sheet: None, addr }
    }

    /// A sheet-qualified single-cell reference (`Summary!A1`).
    pub fn sheet_cell(sheet: impl Into<String>, addr: CellAddress) -> FormulaAst {
        FormulaAst::Ref {
            sheet: Some(sheet.into()),
            addr,
        }
    }

    /// A same-sheet range reference (`A1:B2`).
    pub fn cell_range(range: CellRange) -> FormulaAst {
        FormulaAst::Range { sheet: None, range }
    }

    /// A sheet-qualified range reference (`Summary!A1:B2`).
    pub fn sheet_range(sheet: impl Into<String>, range: CellRange) -> FormulaAst {
        FormulaAst::Range {
            sheet: Some(sheet.into()),
            range,
        }
    }

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
            FormulaAst::Ref { sheet, addr } => format!("{}{}", sheet_prefix(sheet), addr.to_a1()),
            FormulaAst::Range { sheet, range } => format!(
                "{}{}:{}",
                sheet_prefix(sheet),
                range.start.to_a1(),
                range.end.to_a1()
            ),
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

    /// Rewrite every reference in this formula by `(d_row, d_col)` — the
    /// **copy/paste (fill)** transform, the sibling of
    /// [`adjust`](crate::edit) (structural edits).
    ///
    /// The two differ in how they treat absolute references:
    ///
    /// - `adjust` (insert/delete rows/cols) shifts **both** relative and
    ///   absolute refs — a cell physically moves, so `$A$1` becomes `$A$2` when a
    ///   row is inserted above it.
    /// - `shift` (this) is what fill/drag-copy does: a **relative** ref tracks
    ///   the offset (so `=A1` filled one row down becomes `=A2`), while an
    ///   **absolute** ref is pinned (`$A$1` stays `$A$1`). The absolute-flag logic
    ///   lives in [`CellAddress::shift`]; this just recurses it over the tree.
    ///
    /// A reference shifted off the top/left edge of the grid (row or column < 1)
    /// collapses to the `#REF!` error literal — the same failure mode as `adjust`,
    /// and it propagates through evaluation like any error. Pure: returns a new
    /// tree, leaving `self` untouched.
    pub fn shift(&self, d_row: i32, d_col: i32) -> FormulaAst {
        match self {
            FormulaAst::Literal(v) => FormulaAst::Literal(v.clone()),
            // The sheet qualifier rides along unchanged — filling `=Detail!A1`
            // down a column gives `=Detail!A2` (same sheet, shifted address).
            FormulaAst::Ref { sheet, addr } => match addr.shift(d_row, d_col) {
                Ok(a) => FormulaAst::Ref {
                    sheet: sheet.clone(),
                    addr: a,
                },
                Err(_) => ref_error(),
            },
            // A range shifts both corners; if either falls off-grid the whole
            // range is a dangling reference → `#REF!`.
            FormulaAst::Range { sheet, range } => {
                match (range.start.shift(d_row, d_col), range.end.shift(d_row, d_col)) {
                    (Ok(start), Ok(end)) => FormulaAst::Range {
                        sheet: sheet.clone(),
                        range: CellRange::new(start, end),
                    },
                    _ => ref_error(),
                }
            }
            FormulaAst::Unary { op, operand } => FormulaAst::Unary {
                op: *op,
                operand: Box::new(operand.shift(d_row, d_col)),
            },
            FormulaAst::Binary { op, lhs, rhs } => FormulaAst::Binary {
                op: *op,
                lhs: Box::new(lhs.shift(d_row, d_col)),
                rhs: Box::new(rhs.shift(d_row, d_col)),
            },
            FormulaAst::Percent(inner) => FormulaAst::Percent(Box::new(inner.shift(d_row, d_col))),
            FormulaAst::Call { name, args } => FormulaAst::Call {
                name: name.clone(),
                args: args.iter().map(|a| a.shift(d_row, d_col)).collect(),
            },
        }
    }

    /// Like [`shift`](FormulaAst::shift), but **only same-sheet references move** —
    /// a cross-sheet (qualified) reference is left exactly as-is.
    ///
    /// This is the transform a **range sort** needs: sorting rows *within* a sheet
    /// relocates each moved formula's references to cells on *that* sheet (so the
    /// record keeps naming its own data), but a reference into *another* sheet
    /// (`Summary!A1`) names a fixed cell elsewhere and must not move just because a
    /// row was reordered here. (Drag-fill, by contrast, replicates the formula and
    /// *does* shift qualified relative refs — use [`shift`] there.)
    ///
    /// Pure: returns a new tree.
    pub fn shift_local(&self, d_row: i32, d_col: i32) -> FormulaAst {
        match self {
            FormulaAst::Literal(v) => FormulaAst::Literal(v.clone()),
            // Same-sheet refs move exactly as in `shift`.
            FormulaAst::Ref { sheet: None, addr } => match addr.shift(d_row, d_col) {
                Ok(a) => FormulaAst::cell(a),
                Err(_) => ref_error(),
            },
            FormulaAst::Range { sheet: None, range } => {
                match (range.start.shift(d_row, d_col), range.end.shift(d_row, d_col)) {
                    (Ok(start), Ok(end)) => FormulaAst::cell_range(CellRange::new(start, end)),
                    _ => ref_error(),
                }
            }
            // Cross-sheet refs name cells on another sheet — a sort here leaves them.
            FormulaAst::Ref { sheet: Some(_), .. } | FormulaAst::Range { sheet: Some(_), .. } => {
                self.clone()
            }
            FormulaAst::Unary { op, operand } => FormulaAst::Unary {
                op: *op,
                operand: Box::new(operand.shift_local(d_row, d_col)),
            },
            FormulaAst::Binary { op, lhs, rhs } => FormulaAst::Binary {
                op: *op,
                lhs: Box::new(lhs.shift_local(d_row, d_col)),
                rhs: Box::new(rhs.shift_local(d_row, d_col)),
            },
            FormulaAst::Percent(inner) => {
                FormulaAst::Percent(Box::new(inner.shift_local(d_row, d_col)))
            }
            FormulaAst::Call { name, args } => FormulaAst::Call {
                name: name.clone(),
                args: args.iter().map(|a| a.shift_local(d_row, d_col)).collect(),
            },
        }
    }

    /// Rewrite the **sheet qualifier** of every reference named `old` to `new`,
    /// leaving the address untouched — the source-text side of renaming a sheet.
    /// References to other sheets and unqualified references are unchanged. Pure.
    pub fn rename_qualifier(&self, old: &str, new: &str) -> FormulaAst {
        match self {
            FormulaAst::Ref {
                sheet: Some(name),
                addr,
            } if name == old => FormulaAst::sheet_cell(new, *addr),
            FormulaAst::Range {
                sheet: Some(name),
                range,
            } if name == old => FormulaAst::sheet_range(new, *range),
            FormulaAst::Literal(_) | FormulaAst::Ref { .. } | FormulaAst::Range { .. } => {
                self.clone()
            }
            FormulaAst::Unary { op, operand } => FormulaAst::Unary {
                op: *op,
                operand: Box::new(operand.rename_qualifier(old, new)),
            },
            FormulaAst::Binary { op, lhs, rhs } => FormulaAst::Binary {
                op: *op,
                lhs: Box::new(lhs.rename_qualifier(old, new)),
                rhs: Box::new(rhs.rename_qualifier(old, new)),
            },
            FormulaAst::Percent(inner) => {
                FormulaAst::Percent(Box::new(inner.rename_qualifier(old, new)))
            }
            FormulaAst::Call { name, args } => FormulaAst::Call {
                name: name.clone(),
                args: args.iter().map(|a| a.rename_qualifier(old, new)).collect(),
            },
        }
    }

    /// Replace every reference qualified with the sheet `name` by the `#REF!`
    /// error literal — what deleting a sheet does to the references that pointed
    /// into it (permanent, so re-adding a same-named sheet doesn't resurrect them,
    /// matching Excel). Other references are unchanged. Pure.
    pub fn sheet_refs_to_error(&self, name: &str) -> FormulaAst {
        match self {
            FormulaAst::Ref { sheet: Some(n), .. } if n == name => ref_error(),
            FormulaAst::Range { sheet: Some(n), .. } if n == name => ref_error(),
            FormulaAst::Literal(_) | FormulaAst::Ref { .. } | FormulaAst::Range { .. } => {
                self.clone()
            }
            FormulaAst::Unary { op, operand } => FormulaAst::Unary {
                op: *op,
                operand: Box::new(operand.sheet_refs_to_error(name)),
            },
            FormulaAst::Binary { op, lhs, rhs } => FormulaAst::Binary {
                op: *op,
                lhs: Box::new(lhs.sheet_refs_to_error(name)),
                rhs: Box::new(rhs.sheet_refs_to_error(name)),
            },
            FormulaAst::Percent(inner) => {
                FormulaAst::Percent(Box::new(inner.sheet_refs_to_error(name)))
            }
            FormulaAst::Call { name: fname, args } => FormulaAst::Call {
                name: fname.clone(),
                args: args.iter().map(|a| a.sheet_refs_to_error(name)).collect(),
            },
        }
    }
}

/// The `#REF!` error as a formula literal — what a reference shifted off the grid
/// collapses to. (Mirrors `edit::ref_error`; kept local so `shift` and `adjust`
/// stay in their own modules.)
fn ref_error() -> FormulaAst {
    FormulaAst::Literal(CellValue::Error(SpreadsheetError::Ref))
}

/// Render a reference's sheet qualifier for source re-emission: `""` for an
/// unqualified (`None`) reference, otherwise `Name!`, single-quoting the name
/// when it isn't a bare token. The quoting rule matches Excel/Sheets: a name is
/// safe to write bare only when it is all letters/digits/underscore, does not
/// start with a digit, and does not itself spell a cell address (`A1`) — anything
/// else (spaces, punctuation, leading digit, an A1-looking name, or an empty
/// name) is wrapped in single quotes with internal quotes doubled (`'O''Brien'`).
fn sheet_prefix(sheet: &Option<String>) -> String {
    match sheet {
        None => String::new(),
        Some(name) => {
            if needs_quoting(name) {
                format!("'{}'!", name.replace('\'', "''"))
            } else {
                format!("{name}!")
            }
        }
    }
}

/// Whether a sheet name must be single-quoted when written in a formula.
fn needs_quoting(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    // Any character outside the bare set forces quoting.
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return true;
    }
    // A leading digit would read as a number, not a name.
    if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return true;
    }
    // A name that itself spells a cell address (`A1`, `BZ400`) must be quoted so
    // `A1!B2` can't be misread — the parser treats the token before `!` as the
    // sheet, but quoting keeps the round-trip unambiguous.
    CellAddress::parse(name).is_ok()
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
            "=Summary!A1",       // cross-sheet ref
            "=Summary!A1:B10",   // cross-sheet range
            "=Summary!B2+Detail!B2", // two cross-sheet refs in one formula
            "=SUM(Detail!A1:A4)",    // qualified range as a function arg
        ] {
            round_trips(src);
        }
    }

    #[test]
    fn cross_sheet_qualifier_quotes_only_when_needed() {
        use crate::parser::parse;
        // A bare alphanumeric name needs no quoting.
        assert_eq!(parse("=Summary!A1").unwrap().to_formula_string(), "Summary!A1");
        // A name with a space must be single-quoted.
        assert_eq!(
            parse("='Q1 Budget'!A1").unwrap().to_formula_string(),
            "'Q1 Budget'!A1"
        );
        // An apostrophe inside the name is doubled.
        assert_eq!(
            parse("='O''Brien'!A1").unwrap().to_formula_string(),
            "'O''Brien'!A1"
        );
        // A name that itself spells a cell address is quoted to stay unambiguous.
        assert_eq!(parse("='A1'!B2").unwrap().to_formula_string(), "'A1'!B2");
        // A leading-digit name is quoted.
        assert_eq!(parse("='2024'!A1").unwrap().to_formula_string(), "'2024'!A1");
    }

    #[test]
    fn shift_local_moves_same_sheet_refs_but_pins_cross_sheet() {
        use crate::parser::parse;
        // Same-sheet (unqualified) ref tracks the offset, exactly like `shift`.
        assert_eq!(
            parse("=A1").unwrap().shift_local(1, 0).to_formula_string(),
            "A2"
        );
        // A cross-sheet ref names a fixed cell elsewhere → unchanged by a local move.
        assert_eq!(
            parse("=Summary!A1").unwrap().shift_local(5, 5).to_formula_string(),
            "Summary!A1"
        );
        // Mixed: only the same-sheet side of the sum moves.
        assert_eq!(
            parse("=A1+Summary!A1")
                .unwrap()
                .shift_local(1, 0)
                .to_formula_string(),
            "(A2+Summary!A1)"
        );
    }

    #[test]
    fn shift_preserves_the_sheet_qualifier() {
        use crate::parser::parse;
        // Filling =Detail!A1 one row down → =Detail!A2 (same sheet, shifted addr).
        assert_eq!(
            parse("=Detail!A1").unwrap().shift(1, 0).to_formula_string(),
            "Detail!A2"
        );
        // A quoted qualifier rides along too, and a range shifts both corners.
        assert_eq!(
            parse("='Q1 Budget'!A1:B2")
                .unwrap()
                .shift(0, 1)
                .to_formula_string(),
            "'Q1 Budget'!B1:C2"
        );
    }

    #[test]
    fn serializer_renders_literals() {
        assert_eq!(parse("=42").unwrap().to_formula_string(), "42");
        assert_eq!(parse("=3.5").unwrap().to_formula_string(), "3.5");
        assert_eq!(parse("=\"hi\"").unwrap().to_formula_string(), "\"hi\"");
        assert_eq!(parse("=SUM(A1:A2)").unwrap().to_formula_string(), "SUM(A1:A2)");
    }

    // ── shift (copy/paste / fill) ────────────────────────────────────

    /// Shift `src`'s formula by `(d_row, d_col)` and render it back, for compact
    /// assertions.
    fn shifted(src: &str, d_row: i32, d_col: i32) -> String {
        parse(src).unwrap().shift(d_row, d_col).to_formula_string()
    }

    #[test]
    fn shift_tracks_relative_refs_and_pins_absolute() {
        // Filling =A1 one row down → =A2 (relative ref tracks the offset).
        assert_eq!(shifted("=A1", 1, 0), "A2");
        // …and one column right → =B1.
        assert_eq!(shifted("=A1", 0, 1), "B1");
        // A fully-absolute ref is pinned: $A$1 stays $A$1.
        assert_eq!(shifted("=$A$1", 5, 5), "$A$1");
        // Mixed: $A1 (abs col, rel row) shifted down+right → only the row moves.
        assert_eq!(shifted("=$A1", 2, 3), "$A3");
        // A$1 (rel col, abs row) shifted → only the column moves.
        assert_eq!(shifted("=A$1", 2, 3), "D$1");
    }

    #[test]
    fn shift_recurses_ranges_ops_and_calls() {
        // A range shifts both corners.
        assert_eq!(shifted("=SUM(A1:A4)", 0, 1), "SUM(B1:B4)");
        // Recurses through nested binary ops (fully parenthesised) and percent.
        assert_eq!(shifted("=A1+B1*2", 1, 0), "(A2+(B2*2))");
        assert_eq!(shifted("=A1%", 0, 1), "B1%");
    }

    #[test]
    fn shift_binary_and_call_structure() {
        // Binary ops are fully parenthesised by the serializer; each ref tracked.
        assert_eq!(shifted("=A1+B1", 1, 0), "(A2+B2)");
        assert_eq!(shifted("=IF(A1>0,A1,$Z$9)", 0, 1), "IF((B1>0),B1,$Z$9)");
    }

    #[test]
    fn shift_off_grid_becomes_ref_error() {
        // Shifting A1 up (row would be 0) → the ref collapses to #REF!.
        assert_eq!(shifted("=A1", -1, 0), "#REF!");
        // Left off column 1 → #REF!.
        assert_eq!(shifted("=A1", 0, -1), "#REF!");
        // A range with a corner off-grid is a dangling reference → the range
        // collapses to #REF! (here inside the SUM call → `SUM(#REF!)`).
        assert_eq!(shifted("=SUM(A1:B2)", -5, 0), "SUM(#REF!)");
        // But an absolute ref can't go off-grid by shifting — it's pinned.
        assert_eq!(shifted("=$A$1", -10, -10), "$A$1");
    }

    #[test]
    fn shift_by_zero_is_identity() {
        for src in ["=A1", "=$A$1", "=SUM(A1:A4)", "=A1+B2*C3"] {
            assert_eq!(parse(src).unwrap().shift(0, 0), parse(src).unwrap());
        }
    }
}
