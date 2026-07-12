//! Lowering the generic parse tree into a typed program model.
//!
//! The `cobol-parser` CST is a uniform [`GrammarASTNode`] tree (see the parser's
//! probe output). This module walks it once and produces the small typed model
//! the interpreter runs — data definitions and procedure statements — returning
//! a descriptive [`RuntimeError`] for anything v0.1 does not yet handle.

use crate::error::RuntimeError;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ---------------------------------------------------------------------------
// Typed model
// ---------------------------------------------------------------------------

/// A whole program: its WORKING-STORAGE definitions and its paragraphs.
#[derive(Debug, Clone)]
pub struct Program {
    pub data: Vec<DataDef>,
    pub paragraphs: Vec<Paragraph>,
}

/// One WORKING-STORAGE entry, as written (the interpreter turns these into the
/// item tree).
#[derive(Debug, Clone)]
pub struct DataDef {
    pub level: u32,
    /// The data-name, or `None` for `FILLER`.
    pub name: Option<String>,
    /// The raw picture string (`"9(3)V99"`), if the entry has a PICTURE clause.
    pub picture: Option<String>,
    /// The VALUE literal, if present.
    pub value: Option<Lit>,
}

/// A literal or figurative constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lit {
    Num(String),
    Str(String),
    Fig(Fig),
}

/// A figurative constant (v0.1 subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fig {
    Zero,
    Space,
}

/// A named paragraph of statements.
#[derive(Debug, Clone)]
pub struct Paragraph {
    /// The paragraph name — a `PERFORM` (and, later, `GO TO`) target.
    pub name: String,
    pub stmts: Vec<Stmt>,
}

/// An executable statement (v0.1 subset).
#[derive(Debug, Clone)]
pub enum Stmt {
    Display(Vec<Operand>),
    Move { src: Operand, dsts: Vec<String> },
    /// `ADD op… TO name [GIVING g]` — result = op1+…+name, stored in g or name.
    Add { operands: Vec<Operand>, to: String, giving: Option<String> },
    /// `SUBTRACT op… FROM name [GIVING g]` — result = name-(op1+…), in g or name.
    Subtract { operands: Vec<Operand>, from: String, giving: Option<String> },
    /// `MULTIPLY a BY b [GIVING g]` — result = a*b, stored in g or b.
    Multiply { a: Operand, by: Operand, giving: Option<String> },
    /// `DIVIDE a INTO b [GIVING g]` — result = b/a, stored in g or b.
    Divide { divisor: Operand, dividend: Operand, giving: Option<String> },
    /// `COMPUTE target [ROUNDED] = expr [ON SIZE ERROR stmts…]` — evaluate an
    /// arithmetic expression and store it in `target`, rounding instead of
    /// truncating when `rounded`, running `on_size_error` when the result
    /// overflows the receiver (or a division by zero occurs).
    Compute {
        target: String,
        rounded: bool,
        expr: Expr,
        on_size_error: Vec<Stmt>,
    },
    /// `PERFORM para [THRU para2] <mode>` — run a paragraph (or the range
    /// `para`…`para2` in source order) out of line, then return. The
    /// [`PerformMode`] is the repeat form.
    Perform { target: String, thru: Option<String>, mode: PerformMode },
    /// `GO TO para` — transfer control unconditionally to a paragraph (no return).
    GoTo { target: String },
    /// `IF cond then… [ELSE else…]`.
    If { cond: Cond, then_branch: Vec<Stmt>, else_branch: Vec<Stmt> },
    StopRun,
}

/// How a [`Stmt::Perform`] repeats its paragraph.
#[derive(Debug, Clone)]
pub enum PerformMode {
    /// Bare `PERFORM para` — run it once.
    Once,
    /// `PERFORM para n TIMES` — run it a fixed number of times.
    Times(Operand),
    /// `PERFORM para UNTIL cond` — run it while `cond` is false (test before).
    Until(Cond),
    /// `PERFORM para VARYING id FROM start BY step UNTIL cond` — set `id` to
    /// `start`, then run while `cond` is false, stepping `id` by `step` after
    /// each iteration (test before).
    Varying {
        var: String,
        from: Operand,
        by: Operand,
        until: Cond,
    },
}

/// An arithmetic expression tree (the operand of `COMPUTE`). Operator precedence
/// and grouping are already resolved by the grammar's rule cascade, so this is a
/// plain binary tree — no precedence logic lives here.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A numeric literal (its source text, parsed to a value at evaluation).
    Num(String),
    /// A data-name reference (must resolve to a numeric item).
    Var(String),
    /// A unary minus (`neg == true`); unary plus is folded away by the reader.
    Unary { neg: bool, operand: Box<Expr> },
    /// A binary operation `left <op> right`.
    Binary { op: ArithOp, left: Box<Expr>, right: Box<Expr> },
}

/// The binary arithmetic operators COMPUTE understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    /// Exponentiation (`**`), right-associative.
    Pow,
}

/// A statement operand: a data-name or a literal.
#[derive(Debug, Clone)]
pub enum Operand {
    Ident(String),
    Lit(Lit),
}

/// A simple relational condition: `left <relop> right`, optionally negated.
#[derive(Debug, Clone)]
pub struct Cond {
    pub left: Operand,
    pub op: RelOp,
    pub negated: bool,
    pub right: Operand,
}

/// The relational operator of a condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelOp {
    Greater,
    Less,
    Equal,
}

// ---------------------------------------------------------------------------
// CST navigation helpers
// ---------------------------------------------------------------------------

/// Direct child nodes with the given rule name.
fn child_nodes<'a>(n: &'a GrammarASTNode, rule: &str) -> Vec<&'a GrammarASTNode> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(x) if x.rule_name == rule => Some(x),
            _ => None,
        })
        .collect()
}

/// First direct child node with the given rule name.
fn child_node<'a>(n: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    child_nodes(n, rule).into_iter().next()
}

/// First descendant node with the given rule name (depth-first) — for locating
/// divisions/sections anywhere under the program root.
fn find<'a>(n: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    if n.rule_name == rule {
        return Some(n);
    }
    for c in &n.children {
        if let ASTNodeOrToken::Node(x) = c {
            if let Some(f) = find(x, rule) {
                return Some(f);
            }
        }
    }
    None
}

/// Direct child tokens' (effective type name, value) pairs.
fn child_tokens(n: &GrammarASTNode) -> Vec<(String, String)> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some((t.effective_type_name().to_string(), t.value.clone())),
            _ => None,
        })
        .collect()
}

/// The value of the first direct child token of the given effective type.
fn first_token(n: &GrammarASTNode, type_name: &str) -> Option<String> {
    child_tokens(n).into_iter().find(|(k, _)| k == type_name).map(|(_, v)| v)
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// Lower a parsed program CST into the typed [`Program`] model.
pub fn read_program(root: &GrammarASTNode) -> Result<Program, RuntimeError> {
    let data = match find(root, "data_division") {
        Some(dd) => read_working_storage(dd)?,
        None => Vec::new(),
    };
    let paragraphs = match find(root, "procedure_division") {
        Some(pd) => read_procedure(pd)?,
        None => Vec::new(),
    };
    Ok(Program { data, paragraphs })
}

fn read_working_storage(dd: &GrammarASTNode) -> Result<Vec<DataDef>, RuntimeError> {
    let ws = match find(dd, "working_storage_section") {
        Some(ws) => ws,
        None => return Ok(Vec::new()),
    };
    child_nodes(ws, "data_entry").into_iter().map(read_data_entry).collect()
}

fn read_data_entry(e: &GrammarASTNode) -> Result<DataDef, RuntimeError> {
    let level = first_token(e, "NUMBER")
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| RuntimeError::Unsupported("data entry without a level number".into()))?;

    // Name: the NAME token, or FILLER (a KEYWORD).
    let name = first_token(e, "NAME");
    let name = if name.is_none() {
        // FILLER entries are unnamed.
        None
    } else {
        name
    };

    // Clauses: each `data_clause` wraps a `picture_clause` or `value_clause`.
    let mut picture = None;
    let mut value = None;
    for clause in child_nodes(e, "data_clause") {
        if let Some(pc) = child_node(clause, "picture_clause") {
            picture = first_token(pc, "PIC_STRING");
        } else if let Some(vc) = child_node(clause, "value_clause") {
            let lit = child_node(vc, "literal")
                .ok_or_else(|| RuntimeError::Unsupported("VALUE without a literal".into()))?;
            value = Some(read_literal(lit)?);
        }
    }

    Ok(DataDef { level, name, picture, value })
}

fn read_literal(lit: &GrammarASTNode) -> Result<Lit, RuntimeError> {
    if let Some(fig) = child_node(lit, "figurative") {
        // The figurative's token value is the uppercased word.
        let word = child_tokens(fig).into_iter().map(|(_, v)| v).next().unwrap_or_default();
        return match word.as_str() {
            "ZERO" | "ZEROS" | "ZEROES" => Ok(Lit::Fig(Fig::Zero)),
            "SPACE" | "SPACES" => Ok(Lit::Fig(Fig::Space)),
            other => Err(RuntimeError::Unsupported(format!("figurative constant {other}"))),
        };
    }
    for (kind, val) in child_tokens(lit) {
        match kind.as_str() {
            "NUMBER" => return Ok(Lit::Num(val)),
            "STRING" => return Ok(Lit::Str(val)),
            _ => {}
        }
    }
    Err(RuntimeError::Unsupported("unrecognised literal".into()))
}

fn read_operand(op: &GrammarASTNode) -> Result<Operand, RuntimeError> {
    if let Some(lit) = child_node(op, "literal") {
        return Ok(Operand::Lit(read_literal(lit)?));
    }
    if let Some(name) = first_token(op, "NAME") {
        return Ok(Operand::Ident(name));
    }
    Err(RuntimeError::Unsupported("unrecognised operand".into()))
}

fn read_procedure(pd: &GrammarASTNode) -> Result<Vec<Paragraph>, RuntimeError> {
    let mut paragraphs = Vec::new();
    for para in child_nodes(pd, "paragraph") {
        let name = first_token(para, "NAME").unwrap_or_default();
        let mut stmts = Vec::new();
        for sentence in child_nodes(para, "sentence") {
            for stmt in child_nodes(sentence, "statement") {
                stmts.push(read_statement(stmt)?);
            }
        }
        paragraphs.push(Paragraph { name, stmts });
    }
    Ok(paragraphs)
}

fn read_statement(stmt: &GrammarASTNode) -> Result<Stmt, RuntimeError> {
    // A `statement` wraps exactly one verb node.
    let verb = stmt
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(x) => Some(x),
            _ => None,
        })
        .ok_or_else(|| RuntimeError::Unsupported("empty statement".into()))?;

    match verb.rule_name.as_str() {
        "display_stmt" => {
            let ops = child_nodes(verb, "operand")
                .into_iter()
                .map(read_operand)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Stmt::Display(ops))
        }
        "move_stmt" => {
            let src_node = child_node(verb, "operand")
                .ok_or_else(|| RuntimeError::Unsupported("MOVE without a source".into()))?;
            let src = read_operand(src_node)?;
            // Receiving fields are the NAME tokens after TO.
            let dsts: Vec<String> = child_tokens(verb)
                .into_iter()
                .filter(|(k, _)| k == "NAME")
                .map(|(_, v)| v)
                .collect();
            if dsts.is_empty() {
                return Err(RuntimeError::Unsupported("MOVE without a receiver".into()));
            }
            Ok(Stmt::Move { src, dsts })
        }
        "stop_stmt" => {
            // STOP RUN vs STOP <literal>.
            let has_run = child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "RUN");
            if has_run {
                Ok(Stmt::StopRun)
            } else {
                Err(RuntimeError::Unsupported("STOP <literal> (only STOP RUN in v0.1)".into()))
            }
        }
        "add_stmt" => {
            // ADD op… TO name [GIVING g]: operands are `operand` nodes; the
            // direct NAME tokens are [to] or [to, giving].
            let operands = read_operands(verb)?;
            let (to, giving) = read_target_and_giving(verb)?;
            Ok(Stmt::Add { operands, to, giving })
        }
        "subtract_stmt" => {
            let operands = read_operands(verb)?;
            let (from, giving) = read_target_and_giving(verb)?;
            Ok(Stmt::Subtract { operands, from, giving })
        }
        "multiply_stmt" => {
            // MULTIPLY a BY b [GIVING g]: two operand nodes; a direct NAME token
            // only when GIVING is present.
            let ops = read_operands(verb)?;
            if ops.len() != 2 {
                return Err(RuntimeError::Unsupported("MULTIPLY needs exactly two operands".into()));
            }
            let has_giving = child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "GIVING");
            let names: Vec<String> = child_tokens(verb)
                .into_iter()
                .filter(|(k, _)| k == "NAME")
                .map(|(_, v)| v)
                .collect();
            let giving = if has_giving { names.into_iter().next() } else { None };
            let mut it = ops.into_iter();
            Ok(Stmt::Multiply { a: it.next().unwrap(), by: it.next().unwrap(), giving })
        }
        "divide_stmt" => {
            // DIVIDE a INTO b [GIVING g]: first operand is the divisor, second
            // the dividend; result = b / a.
            let ops = read_operands(verb)?;
            if ops.len() != 2 {
                return Err(RuntimeError::Unsupported("DIVIDE needs exactly two operands".into()));
            }
            let has_giving = child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "GIVING");
            let names: Vec<String> = child_tokens(verb)
                .into_iter()
                .filter(|(k, _)| k == "NAME")
                .map(|(_, v)| v)
                .collect();
            let giving = if has_giving { names.into_iter().next() } else { None };
            let mut it = ops.into_iter();
            Ok(Stmt::Divide { divisor: it.next().unwrap(), dividend: it.next().unwrap(), giving })
        }
        "compute_stmt" => {
            // COMPUTE target [ROUNDED] = <expr> [ON SIZE ERROR stmts…].
            // The one direct NAME token is the receiver; expression names live
            // deeper, inside the arith_* nodes.
            let target = first_token(verb, "NAME")
                .ok_or_else(|| RuntimeError::Unsupported("COMPUTE without a receiver".into()))?;
            let rounded = child_tokens(verb)
                .iter()
                .any(|(k, v)| k == "KEYWORD" && v == "ROUNDED");
            let expr_node = child_node(verb, "arith_expr")
                .ok_or_else(|| RuntimeError::Unsupported("COMPUTE without an expression".into()))?;
            let expr = read_arith_expr_bounded(expr_node)?;
            let on_size_error = match child_node(verb, "size_error") {
                Some(se) => child_nodes(se, "statement")
                    .into_iter()
                    .map(read_statement)
                    .collect::<Result<Vec<_>, _>>()?,
                None => Vec::new(),
            };
            Ok(Stmt::Compute { target, rounded, expr, on_size_error })
        }
        "perform_stmt" => {
            // PERFORM target [THROUGH/THRU target2] [ operand TIMES | UNTIL … |
            // VARYING … ]. The direct NAME tokens are [target] or, with THRU,
            // [target, target2]; the induction/TIMES/UNTIL operands live inside
            // their own child nodes.
            let names: Vec<String> = child_tokens(verb)
                .into_iter()
                .filter(|(k, _)| k == "NAME")
                .map(|(_, v)| v)
                .collect();
            let target = names
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::Unsupported("PERFORM without a target paragraph".into()))?;
            let has_thru = child_tokens(verb)
                .iter()
                .any(|(k, v)| k == "KEYWORD" && (v == "THRU" || v == "THROUGH"));
            let thru = if has_thru { names.get(1).cloned() } else { None };
            // The repeat mode: VARYING (its own node), else TIMES (a direct
            // operand), else UNTIL (a direct condition), else bare/once.
            let mode = if let Some(v) = child_node(verb, "perform_varying") {
                read_perform_varying(v)?
            } else if let Some(op) = child_node(verb, "operand") {
                PerformMode::Times(read_operand(op)?)
            } else if let Some(cond) = child_node(verb, "condition") {
                PerformMode::Until(read_condition(cond)?)
            } else {
                PerformMode::Once
            };
            Ok(Stmt::Perform { target, thru, mode })
        }
        "goto_stmt" => {
            // GO [TO] target. The DEPENDING ON form is not in the grammar yet.
            let target = first_token(verb, "NAME")
                .ok_or_else(|| RuntimeError::Unsupported("GO TO without a target paragraph".into()))?;
            Ok(Stmt::GoTo { target })
        }
        "if_stmt" => {
            // Children in order: IF, condition, then-statements…, [ELSE,
            // else-statements…]. Split the statement nodes at the ELSE keyword.
            let cond_node = child_node(verb, "condition")
                .ok_or_else(|| RuntimeError::Unsupported("IF without a condition".into()))?;
            let cond = read_condition(cond_node)?;
            let mut then_branch = Vec::new();
            let mut else_branch = Vec::new();
            let mut seen_else = false;
            for child in &verb.children {
                match child {
                    ASTNodeOrToken::Token(t) if t.value == "ELSE" && t.effective_type_name() == "KEYWORD" => {
                        seen_else = true;
                    }
                    ASTNodeOrToken::Node(n) if n.rule_name == "statement" => {
                        let stmt = read_statement(n)?;
                        if seen_else {
                            else_branch.push(stmt);
                        } else {
                            then_branch.push(stmt);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Stmt::If { cond, then_branch, else_branch })
        }
        other => Err(RuntimeError::Unsupported(format!("the {} verb", verb_name(other)))),
    }
}

/// Read a `condition` node (`operand relop operand`).
fn read_condition(cond: &GrammarASTNode) -> Result<Cond, RuntimeError> {
    let operands = child_nodes(cond, "operand");
    if operands.len() != 2 {
        return Err(RuntimeError::Unsupported("condition must be `operand relop operand`".into()));
    }
    let left = read_operand(operands[0])?;
    let right = read_operand(operands[1])?;
    let relop = child_node(cond, "relop")
        .ok_or_else(|| RuntimeError::Unsupported("condition without a relational operator".into()))?;
    let toks = child_tokens(relop);
    let negated = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "NOT");
    let op = toks
        .iter()
        .find_map(|(k, v)| {
            if k != "KEYWORD" {
                return None;
            }
            match v.as_str() {
                "GREATER" => Some(RelOp::Greater),
                "LESS" => Some(RelOp::Less),
                "EQUAL" => Some(RelOp::Equal),
                _ => None,
            }
        })
        .ok_or_else(|| RuntimeError::Unsupported("unrecognised relational operator".into()))?;
    Ok(Cond { left, op, negated, right })
}

// ---------------------------------------------------------------------------
// Arithmetic expressions (COMPUTE)
// ---------------------------------------------------------------------------
//
// The grammar's rule cascade already encodes precedence, so each reader here
// just folds one level's operands into a binary tree. `+ - * /` fold
// left-to-right (left-associative); `**` folds right-to-left (COBOL's
// right-associative exponentiation). A single operand with no operator collapses
// to the operand itself, so `COMPUTE X = A` carries no spurious tree nodes.
//
// DoS bound: the grammar's `{ … }` repetition is *flat* (no rule recursion), so
// the parser's recursion-depth cap does NOT limit how *wide* a chain can be —
// `A + A + … + A` with N terms is one node with 2N−1 children at bounded parse
// depth. Folding that yields an N-deep `Expr` tree, and both `eval_expr` and the
// recursive `Drop` of `Box<Expr>` would then overflow the native stack. So a
// single [`MAX_EXPR_OPERANDS`] budget is threaded through the whole expression
// (counting every primary, across parenthesised levels too); exhausting it is a
// clean `RuntimeError`, which keeps the folded tree — hence the eval/Drop
// recursion depth — bounded.

/// Largest number of primaries (`NUMBER`/`NAME`/parenthesised group) a single
/// `COMPUTE` expression may contain. Real expressions have a handful; the cap is
/// only a native-stack backstop against a hostile flat chain.
const MAX_EXPR_OPERANDS: usize = 1024;

/// Read a `COMPUTE` expression, bounding its size against a stack-overflow DoS.
fn read_arith_expr_bounded(node: &GrammarASTNode) -> Result<Expr, RuntimeError> {
    let mut budget = MAX_EXPR_OPERANDS;
    read_arith_expr(node, &mut budget)
}

/// `arith_expr = arith_term { ( "+" | "-" ) arith_term }` — additive, left-assoc.
fn read_arith_expr(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    read_binary_chain(node, read_arith_term, |t| match t {
        "PLUS" => Some(ArithOp::Add),
        "MINUS" => Some(ArithOp::Sub),
        _ => None,
    }, budget)
}

/// `arith_term = arith_factor { ( "*" | "/" ) arith_factor }` — multiplicative.
fn read_arith_term(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    read_binary_chain(node, read_arith_factor, |t| match t {
        "STAR" => Some(ArithOp::Mul),
        "SLASH" => Some(ArithOp::Div),
        _ => None,
    }, budget)
}

/// `arith_factor = arith_unary { "**" arith_unary }` — exponentiation, folded
/// right-associatively so `A ** B ** C` = `A ** (B ** C)`.
fn read_arith_factor(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    let units = child_nodes(node, "arith_unary");
    let mut rev = units.iter().rev();
    let last = rev
        .next()
        .ok_or_else(|| RuntimeError::Unsupported("empty arithmetic factor".into()))?;
    let mut expr = read_arith_unary(last, budget)?;
    for u in rev {
        expr = Expr::Binary {
            op: ArithOp::Pow,
            left: Box::new(read_arith_unary(u, budget)?),
            right: Box::new(expr),
        };
    }
    Ok(expr)
}

/// `arith_unary = [ "+" | "-" ] arith_primary` — a leading minus negates; a
/// leading plus is a no-op.
fn read_arith_unary(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    let neg = child_tokens(node).iter().any(|(k, _)| k == "MINUS");
    let prim = child_node(node, "arith_primary")
        .ok_or_else(|| RuntimeError::Unsupported("unary operator without an operand".into()))?;
    let e = read_arith_primary(prim, budget)?;
    Ok(if neg { Expr::Unary { neg: true, operand: Box::new(e) } } else { e })
}

/// `arith_primary = NUMBER | NAME | "(" arith_expr ")"`. Charges one unit of the
/// expression's operand budget.
fn read_arith_primary(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    *budget = budget
        .checked_sub(1)
        .ok_or_else(|| RuntimeError::Unsupported("COMPUTE expression too large".into()))?;
    // A parenthesised sub-expression recurses back to the top of the cascade.
    if let Some(inner) = child_node(node, "arith_expr") {
        return read_arith_expr(inner, budget);
    }
    for (k, v) in child_tokens(node) {
        match k.as_str() {
            "NUMBER" => return Ok(Expr::Num(v)),
            "NAME" => return Ok(Expr::Var(v)),
            _ => {}
        }
    }
    Err(RuntimeError::Unsupported("empty arithmetic primary".into()))
}

/// Fold a `head { op tail }` node into a left-associative binary tree. `sub`
/// reads each operand node; `map_op` maps an operator token's type name to an
/// [`ArithOp`] (returning `None` for tokens that are not operators). `budget`
/// bounds the total operand count (see [`MAX_EXPR_OPERANDS`]).
fn read_binary_chain(
    node: &GrammarASTNode,
    sub: fn(&GrammarASTNode, &mut usize) -> Result<Expr, RuntimeError>,
    map_op: fn(&str) -> Option<ArithOp>,
    budget: &mut usize,
) -> Result<Expr, RuntimeError> {
    let mut expr: Option<Expr> = None;
    let mut pending: Option<ArithOp> = None;
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(n) => {
                let operand = sub(n, budget)?;
                expr = Some(match (expr.take(), pending.take()) {
                    (Some(left), Some(op)) => Expr::Binary {
                        op,
                        left: Box::new(left),
                        right: Box::new(operand),
                    },
                    // First operand (or a malformed chain missing its operator):
                    // take the operand as the running expression.
                    (_, _) => operand,
                });
            }
            ASTNodeOrToken::Token(t) => {
                if let Some(op) = map_op(t.effective_type_name()) {
                    pending = Some(op);
                }
            }
        }
    }
    expr.ok_or_else(|| RuntimeError::Unsupported("empty arithmetic expression".into()))
}

/// Read a `perform_varying` node
/// (`VARYING NAME FROM operand BY operand UNTIL condition`).
fn read_perform_varying(v: &GrammarASTNode) -> Result<PerformMode, RuntimeError> {
    let var = first_token(v, "NAME")
        .ok_or_else(|| RuntimeError::Unsupported("PERFORM VARYING without a variable".into()))?;
    let operands = child_nodes(v, "operand");
    if operands.len() != 2 {
        return Err(RuntimeError::Unsupported(
            "PERFORM VARYING needs FROM and BY operands".into(),
        ));
    }
    let from = read_operand(operands[0])?;
    let by = read_operand(operands[1])?;
    let cond = child_node(v, "condition")
        .ok_or_else(|| RuntimeError::Unsupported("PERFORM VARYING without an UNTIL".into()))?;
    let until = read_condition(cond)?;
    Ok(PerformMode::Varying { var, from, by, until })
}

/// All `operand` child nodes of a verb, read to typed [`Operand`]s.
fn read_operands(verb: &GrammarASTNode) -> Result<Vec<Operand>, RuntimeError> {
    child_nodes(verb, "operand").into_iter().map(read_operand).collect()
}

/// The target NAME and optional GIVING NAME of an `ADD … TO`/`SUBTRACT … FROM`.
/// The direct NAME tokens are `[target]` or `[target, giving]`.
fn read_target_and_giving(verb: &GrammarASTNode) -> Result<(String, Option<String>), RuntimeError> {
    let names: Vec<String> = child_tokens(verb)
        .into_iter()
        .filter(|(k, _)| k == "NAME")
        .map(|(_, v)| v)
        .collect();
    let mut it = names.into_iter();
    let target = it
        .next()
        .ok_or_else(|| RuntimeError::Unsupported("arithmetic statement without a target".into()))?;
    Ok((target, it.next()))
}

/// Human-friendly verb name from a grammar rule name (`move_stmt` → `MOVE`).
fn verb_name(rule: &str) -> String {
    rule.trim_end_matches("_stmt").replace('_', " ").to_uppercase()
}
