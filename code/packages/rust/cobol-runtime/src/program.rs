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
    /// The paragraph name — a `PERFORM` / `GO TO` target. Captured now; branched
    /// on once those verbs land (the next control-flow PR).
    #[allow(dead_code)]
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
    StopRun,
}

/// A statement operand: a data-name or a literal.
#[derive(Debug, Clone)]
pub enum Operand {
    Ident(String),
    Lit(Lit),
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
        other => Err(RuntimeError::Unsupported(format!("the {} verb", verb_name(other)))),
    }
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
