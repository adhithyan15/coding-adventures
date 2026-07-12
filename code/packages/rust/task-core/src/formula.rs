//! Computed fields: **formulas** and **rollups**.
//!
//! A custom field can be a *formula* — an expression over other fields, written in a
//! small `[field]` bracket syntax (`[work] / [duration] * 100`) — or a *rollup* — an
//! aggregate of a field across a task's children. Both are **computed, never stored**:
//! recomputed from their inputs on demand.
//!
//! ## Why `symbolic-vm`
//!
//! Formula fields need **named-variable** expressions, not cell coordinates. In this
//! repo's `symbolic-ir`, a variable *is* a named atom — `IRNode::Symbol("rate")` — and
//! `symbolic-vm`'s `StrictBackend` evaluates it once every name is bound. That is the
//! reuse win over the A1-only `spreadsheet-core`. We add only a thin bracket parser
//! (our own surface syntax, matching Microsoft Project's `[Field]` convention) and a
//! dependency walker; the evaluator is reused wholesale.
//!
//! Rollups are simple aggregations, so we fold them directly in Rust rather than
//! routing them through the VM — the VM is for arbitrary formula expressions.

use crate::ids::FieldId;
use crate::model::{FieldKind, ProjectState, RollupAgg};
use std::collections::{HashMap, HashSet};
use symbolic_ir::{apply, flt, int, sym, IRNode};
use symbolic_vm::{StrictBackend, VM};

/// What can go wrong turning a formula into a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaError {
    /// The source could not be parsed (with a human-readable reason).
    Parse(String),
    /// The computed-field dependency graph has a cycle (a formula that reads itself,
    /// directly or transitively).
    Cycle(Vec<FieldId>),
}

// ─────────────────────────────────────────────────────────────────────────────
// Parser: `[field]` bracket syntax → symbolic-ir
// ─────────────────────────────────────────────────────────────────────────────

/// Parse a formula source into a `symbolic-ir` expression tree.
///
/// Grammar (lowest precedence first): one optional comparison over additive terms,
/// additive over multiplicative, multiplicative over unary, unary over a primary
/// (`[field]`, a number literal, or a parenthesised expression). Operators lower to
/// the canonical `symbolic-ir` heads (`Add`/`Sub`/`Mul`/`Div`/`Neg`, comparisons).
pub fn parse(src: &str) -> Result<IRNode, FormulaError> {
    let toks = tokenize(src)?;
    let mut p = Parser { toks, pos: 0 };
    let node = p.expr()?;
    if p.pos != p.toks.len() {
        return Err(FormulaError::Parse(format!(
            "unexpected trailing input at token {}",
            p.pos
        )));
    }
    Ok(node)
}

/// The set of field names a parsed formula reads — its dependencies. Fields appear
/// only as leaf `Symbol`s in argument position (operator heads sit in `Apply.head`),
/// so we recurse into arguments and collect leaf symbols.
pub fn referenced_fields(node: &IRNode) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    collect(node, &mut out, &mut seen);
    return out;

    fn collect(node: &IRNode, out: &mut Vec<String>, seen: &mut HashSet<String>) {
        match node {
            IRNode::Symbol(name) => {
                if seen.insert(name.clone()) {
                    out.push(name.clone());
                }
            }
            IRNode::Apply(app) => {
                // Skip the head; recurse into arguments only.
                for arg in &app.args {
                    collect(arg, out, seen);
                }
            }
            _ => {}
        }
    }
}

/// Evaluate a parsed numeric formula given `bindings` (field name → value).
///
/// Returns `None` — never panics — when a referenced field is unbound (which would
/// otherwise trip `StrictBackend`'s strictness) or when the result is not a finite
/// number (e.g. division by zero, or a boolean comparison result). This is the
/// panic-safe boundary for untrusted formula strings.
pub fn eval_number(node: &IRNode, bindings: &HashMap<String, f64>) -> Option<f64> {
    // Gate: every referenced field must be bound, or StrictBackend would panic.
    for name in referenced_fields(node) {
        if !bindings.contains_key(&name) {
            return None;
        }
    }
    let mut vm = VM::new(Box::new(StrictBackend::new()));
    for (name, value) in bindings {
        vm.backend.bind(name, number_to_ir(*value));
    }
    // `StrictBackend` *panics* on some runtime errors (e.g. division by zero). Since
    // formula strings are untrusted, catch the unwind so a bad formula yields `None`
    // rather than crashing the caller.
    let node = node.clone();
    let evaluated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || vm.eval(node)));
    evaluated.ok().as_ref().and_then(ir_to_f64)
}

/// Fold a rollup aggregation over child values.
pub fn rollup(values: &[f64], agg: RollupAgg) -> f64 {
    match agg {
        RollupAgg::Sum => values.iter().sum(),
        RollupAgg::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
        RollupAgg::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        RollupAgg::Average => {
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            }
        }
        RollupAgg::Count => values.len() as f64,
    }
}

/// The order in which computed fields must be evaluated so every formula sees fresh
/// inputs — a topological sort of the field-dependency graph. Returns a `Cycle` error
/// if a formula references itself (directly or transitively). Reuses `directed-graph`.
pub fn field_eval_order(project: &ProjectState) -> Result<Vec<FieldId>, FormulaError> {
    // Map field name → id so a formula's `[name]` references resolve to field ids.
    let name_to_id: HashMap<&str, &FieldId> = project
        .fields
        .values()
        .map(|f| (f.name.as_str(), &f.id))
        .collect();

    let mut graph = directed_graph::Graph::new();
    let mut computed: Vec<FieldId> = Vec::new();
    for f in project.fields.values() {
        graph.add_node(f.id.as_str());
    }
    for f in project.fields.values() {
        match &f.kind {
            FieldKind::Formula { source } => {
                computed.push(f.id.clone());
                if let Ok(ast) = parse(source) {
                    for dep_name in referenced_fields(&ast) {
                        if let Some(dep_id) = name_to_id.get(dep_name.as_str()) {
                            // A field referencing itself is a cycle. `directed-graph`
                            // silently drops self-loops, so detect it explicitly.
                            if **dep_id == f.id {
                                return Err(FormulaError::Cycle(vec![f.id.clone()]));
                            }
                            let _ = graph.add_edge(dep_id.as_str(), f.id.as_str());
                        }
                    }
                }
            }
            FieldKind::Rollup { field, .. } => {
                computed.push(f.id.clone());
                if field == &f.id {
                    return Err(FormulaError::Cycle(vec![f.id.clone()]));
                }
                let _ = graph.add_edge(field.as_str(), f.id.as_str());
            }
            _ => {}
        }
    }

    if graph.has_cycle() {
        return Err(FormulaError::Cycle(computed));
    }
    let order = graph
        .topological_sort()
        .map_err(|_| FormulaError::Cycle(computed.clone()))?;
    // Keep only computed fields, in dependency order.
    let computed_set: HashSet<&str> = computed.iter().map(|f| f.as_str()).collect();
    Ok(order
        .into_iter()
        .filter(|id| computed_set.contains(id.as_str()))
        .map(FieldId::from_raw)
        .collect())
}

// ─────────────────────────────────────────────────────────────────────────────
// internals
// ─────────────────────────────────────────────────────────────────────────────

/// A number binds as an exact integer when it is integral (so `symbolic-vm` keeps
/// exact arithmetic), otherwise as a float.
fn number_to_ir(v: f64) -> IRNode {
    if v.fract() == 0.0 && v.abs() < 9.0e18 {
        int(v as i64)
    } else {
        flt(v)
    }
}

/// Extract a finite `f64` from an evaluated node, or `None` for non-numeric / infinite
/// results (unevaluated symbols, booleans, division by zero).
fn ir_to_f64(node: &IRNode) -> Option<f64> {
    let v = match node {
        IRNode::Integer(n) => *n as f64,
        IRNode::Rational(n, d) if *d != 0 => *n as f64 / *d as f64,
        IRNode::Float(f) => *f,
        _ => return None,
    };
    v.is_finite().then_some(v)
}

// ── tokenizer + recursive-descent parser ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64, bool), // value, is_integer
    Field(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
}

fn tokenize(src: &str) -> Result<Vec<Tok>, FormulaError> {
    let chars: Vec<char> = src.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '[' => {
                let mut name = String::new();
                i += 1;
                while i < chars.len() && chars[i] != ']' {
                    name.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(FormulaError::Parse(
                        "unclosed '[' in field reference".into(),
                    ));
                }
                i += 1; // consume ']'
                toks.push(Tok::Field(name.trim().to_string()));
            }
            '0'..='9' | '.' => {
                let start = i;
                let mut is_int = true;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    if chars[i] == '.' {
                        is_int = false;
                    }
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let v: f64 = s
                    .parse()
                    .map_err(|_| FormulaError::Parse(format!("bad number '{s}'")))?;
                toks.push(Tok::Num(v, is_int));
            }
            '+' => {
                toks.push(Tok::Plus);
                i += 1;
            }
            '-' => {
                toks.push(Tok::Minus);
                i += 1;
            }
            '*' => {
                toks.push(Tok::Star);
                i += 1;
            }
            '/' => {
                toks.push(Tok::Slash);
                i += 1;
            }
            '(' => {
                toks.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                toks.push(Tok::RParen);
                i += 1;
            }
            '=' => {
                toks.push(Tok::Eq);
                i += 1;
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    toks.push(Tok::Le);
                    i += 2;
                } else if i + 1 < chars.len() && chars[i + 1] == '>' {
                    toks.push(Tok::Ne);
                    i += 2;
                } else {
                    toks.push(Tok::Lt);
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    toks.push(Tok::Ge);
                    i += 2;
                } else {
                    toks.push(Tok::Gt);
                    i += 1;
                }
            }
            other => {
                return Err(FormulaError::Parse(format!(
                    "unexpected character '{other}'"
                )))
            }
        }
    }
    Ok(toks)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn expr(&mut self) -> Result<IRNode, FormulaError> {
        let left = self.additive()?;
        let head = match self.peek() {
            Some(Tok::Eq) => symbolic_ir::EQUAL,
            Some(Tok::Ne) => symbolic_ir::NOT_EQUAL,
            Some(Tok::Lt) => symbolic_ir::LESS,
            Some(Tok::Gt) => symbolic_ir::GREATER,
            Some(Tok::Le) => symbolic_ir::LESS_EQUAL,
            Some(Tok::Ge) => symbolic_ir::GREATER_EQUAL,
            _ => return Ok(left),
        };
        self.bump();
        let right = self.additive()?;
        Ok(apply(sym(head), vec![left, right]))
    }

    fn additive(&mut self) -> Result<IRNode, FormulaError> {
        let mut node = self.multiplicative()?;
        loop {
            let head = match self.peek() {
                Some(Tok::Plus) => symbolic_ir::ADD,
                Some(Tok::Minus) => symbolic_ir::SUB,
                _ => break,
            };
            self.bump();
            let rhs = self.multiplicative()?;
            node = apply(sym(head), vec![node, rhs]);
        }
        Ok(node)
    }

    fn multiplicative(&mut self) -> Result<IRNode, FormulaError> {
        let mut node = self.unary()?;
        loop {
            let head = match self.peek() {
                Some(Tok::Star) => symbolic_ir::MUL,
                Some(Tok::Slash) => symbolic_ir::DIV,
                _ => break,
            };
            self.bump();
            let rhs = self.unary()?;
            node = apply(sym(head), vec![node, rhs]);
        }
        Ok(node)
    }

    fn unary(&mut self) -> Result<IRNode, FormulaError> {
        if matches!(self.peek(), Some(Tok::Minus)) {
            self.bump();
            let inner = self.unary()?;
            return Ok(apply(sym(symbolic_ir::NEG), vec![inner]));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<IRNode, FormulaError> {
        match self.bump() {
            Some(Tok::Num(v, is_int)) => Ok(if is_int { int(v as i64) } else { flt(v) }),
            Some(Tok::Field(name)) => Ok(sym(name)),
            Some(Tok::LParen) => {
                let inner = self.expr()?;
                match self.bump() {
                    Some(Tok::RParen) => Ok(inner),
                    _ => Err(FormulaError::Parse("expected ')'".into())),
                }
            }
            other => Err(FormulaError::Parse(format!(
                "expected a field, number, or '(', found {other:?}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{FieldId, ProjectId};
    use crate::model::{FieldDef, FieldKind};

    fn binds(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn parses_and_evaluates_arithmetic() {
        let ast = parse("[a] * [b] + 2").unwrap();
        let v = eval_number(&ast, &binds(&[("a", 3.0), ("b", 4.0)])).unwrap();
        assert_eq!(v, 14.0);
    }

    #[test]
    fn respects_precedence_and_parens() {
        assert_eq!(
            eval_number(&parse("2 + 3 * 4").unwrap(), &binds(&[])).unwrap(),
            14.0
        );
        assert_eq!(
            eval_number(&parse("(2 + 3) * 4").unwrap(), &binds(&[])).unwrap(),
            20.0
        );
        assert_eq!(
            eval_number(&parse("-[a] + 10").unwrap(), &binds(&[("a", 3.0)])).unwrap(),
            7.0
        );
    }

    #[test]
    fn extracts_referenced_fields() {
        let ast = parse("[work] / [duration] * 100").unwrap();
        let mut fields = referenced_fields(&ast);
        fields.sort();
        assert_eq!(fields, vec!["duration".to_string(), "work".to_string()]);
    }

    #[test]
    fn unbound_field_is_none_not_panic() {
        let ast = parse("[a] + [missing]").unwrap();
        assert_eq!(eval_number(&ast, &binds(&[("a", 1.0)])), None);
    }

    #[test]
    fn division_by_zero_is_none_not_panic() {
        let ast = parse("[a] / [b]").unwrap();
        assert_eq!(
            eval_number(&ast, &binds(&[("a", 6.0), ("b", 2.0)])),
            Some(3.0)
        );
        assert_eq!(eval_number(&ast, &binds(&[("a", 6.0), ("b", 0.0)])), None);
    }

    #[test]
    fn rollups_fold_correctly() {
        let v = [1.0, 5.0, 3.0];
        assert_eq!(rollup(&v, RollupAgg::Sum), 9.0);
        assert_eq!(rollup(&v, RollupAgg::Min), 1.0);
        assert_eq!(rollup(&v, RollupAgg::Max), 5.0);
        assert_eq!(rollup(&v, RollupAgg::Average), 3.0);
        assert_eq!(rollup(&v, RollupAgg::Count), 3.0);
        assert_eq!(rollup(&[], RollupAgg::Average), 0.0);
    }

    #[test]
    fn malformed_formula_is_a_parse_error() {
        assert!(matches!(parse("[a] +"), Err(FormulaError::Parse(_))));
        assert!(matches!(parse("[a"), Err(FormulaError::Parse(_))));
        assert!(matches!(parse("2 @ 3"), Err(FormulaError::Parse(_))));
    }

    fn formula_field(id: &str, name: &str, source: &str) -> FieldDef {
        FieldDef {
            id: FieldId::from_raw(id),
            name: name.to_string(),
            kind: FieldKind::Formula {
                source: source.to_string(),
            },
        }
    }

    #[test]
    fn field_eval_order_is_topological() {
        // total = [subtotal] + 1; subtotal = [base] * 2. Order: subtotal before total.
        let mut p = ProjectState::empty(ProjectId::from_raw("p1"));
        p.fields.insert(
            FieldId::from_raw("f_total"),
            formula_field("f_total", "total", "[subtotal] + 1"),
        );
        p.fields.insert(
            FieldId::from_raw("f_sub"),
            formula_field("f_sub", "subtotal", "[base] * 2"),
        );
        let order = field_eval_order(&p).unwrap();
        let pos = |id: &str| order.iter().position(|f| f.as_str() == id).unwrap();
        assert!(pos("f_sub") < pos("f_total"));
    }

    #[test]
    fn self_referential_formula_is_a_cycle() {
        let mut p = ProjectState::empty(ProjectId::from_raw("p1"));
        p.fields.insert(
            FieldId::from_raw("f_a"),
            formula_field("f_a", "a", "[a] + 1"),
        );
        assert!(matches!(field_eval_order(&p), Err(FormulaError::Cycle(_))));
    }
}
