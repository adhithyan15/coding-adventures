//! # prolog-parser — grammar-driven ISO/Core Prolog parser.
//!
//! Thin glue around [`parser::GrammarParser`] sourced from the
//! canonical `code/grammars/prolog/iso.grammar`. Mirrors the Python
//! `iso-prolog-parser` pipeline exactly so the two implementations
//! produce structurally identical ASTs.
//!
//! ## Architecture
//!
//! ```text
//!    Prolog source text
//!         │
//!         ▼
//!    prolog-lexer (tokens)
//!         │
//!         ▼
//!    parser::GrammarParser + iso.grammar
//!         │
//!         ▼
//!    GrammarASTNode (rule-name + children tree)
//!         │
//!         ▼  ast_to_term / collect_clauses_and_queries
//!         ▼
//!    logic_core::Term  +  AdjudicationProgram (Clauses + Queries)
//! ```
//!
//! ## What This Slice Defines
//!
//! - `create_iso_prolog_parser(source)` and `parse_iso_prolog(source)`
//!   matching the convention of every other `*-parser` crate in this
//!   workspace.
//! - `ast_to_term(node, var_map)` — lowers a `term`-rooted
//!   `GrammarASTNode` into a `logic_core::Term`. Encodes lists in the
//!   canonical `'.'/2 + []` cons-cell form. Shares variable identity
//!   across uses of the same variable name within a clause; anonymous
//!   variables get fresh `LogicVar`s on each occurrence.
//! - `collect_clauses_and_queries(ast)` — walks the top-level
//!   `program → statement*` shape and returns a vector of
//!   `Clause` / `Query` items ready for the logic engine. Each clause
//!   gets its own `var_map` so variable identity is local to one
//!   clause (as Prolog requires).
//!
//! ## Not in this slice
//!
//! - **Operator-precedence resolution** (e.g., `X + Y * Z`,
//!   `X = 1 + 2`). The current grammar parses the canonical functional
//!   form. Operator-precedence parsing is a separate concern and is
//!   handled by `prolog-operator-parser` in the Python ecosystem; the
//!   Rust mirror is a follow-up.
//! - User-defined operator directives.
//! - Negation-as-failure `\+ G` parses as a compound term
//!   `'\+'(G)`; the downstream `prolog-loader` translates this into
//!   the engine's `BodyLiteral::Neg`.

use std::collections::HashMap;

use grammar_tools::parser_grammar::ParserGrammar;
use lexer::token::Token;
use logic_core::{atom, compound, LogicVar, Term};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParseError, GrammarParser};
use prolog_lexer::tokenize_iso_prolog;

mod _grammar;

// ---------------------------------------------------------------------------
// Public entry points — matching the algol-parser / csharp-parser pattern
// ---------------------------------------------------------------------------

/// Build a `GrammarParser` configured with the ISO Prolog parser
/// grammar and the tokens of `source`. Use this for fine-grained
/// control; otherwise prefer [`parse_iso_prolog`].
pub fn create_iso_prolog_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_iso_prolog(source);
    let grammar: ParserGrammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
}

/// Tokenize, parse, and return the top-level `program` AST node.
/// Panics on parse errors (matching the convention of other parsers
/// in this workspace). Use `create_iso_prolog_parser` and
/// `parser.parse()` for recoverable errors.
pub fn parse_iso_prolog(source: &str) -> GrammarASTNode {
    let mut p = create_iso_prolog_parser(source);
    p.parse()
        .unwrap_or_else(|e| panic!("ISO Prolog parse failed: {}", e.message))
}

/// Tokenize, parse, and return the AST as a `Result`. Use this when
/// you need to handle parse errors rather than panic.
pub fn try_parse_iso_prolog(source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    let mut p = create_iso_prolog_parser(source);
    p.parse()
}

// ---------------------------------------------------------------------------
// AST → Term lowering
// ---------------------------------------------------------------------------

/// Lower a grammar AST node that represents a Prolog term into a
/// [`logic_core::Term`].
///
/// `var_map` keeps named variables consistent within a single clause:
/// two `X`s in the same clause map to the same `LogicVar`. Anonymous
/// variables (`_`) are always fresh.
///
/// Accepts any of the term-rooted rule names produced by the grammar:
/// `term`, `callable_term`, `compound_term`, `atom_term`,
/// `variable_term`, `anonymous_term`, `number_term`, `string_term`,
/// `list_term`, plus the goal-shaped wrappers `callable_goal`,
/// `equality_goal`, `grouped_goal`. Unknown rule names produce a
/// best-effort lowering of the first term-shaped child.
pub fn ast_to_term(node: &GrammarASTNode, var_map: &mut HashMap<String, LogicVar>) -> Term {
    match node.rule_name.as_str() {
        // Direct passthroughs: unwrap and recurse on the term child.
        "term"
        | "callable_term"
        | "atom_term"
        | "number_term"
        | "string_term"
        | "callable_goal"
        | "goal"
        | "goal_primary" => {
            // Single-child wrappers; recurse into the inner node.
            first_term_child(node, var_map)
        }

        "grouped_goal" => {
            // grouped_goal = LPAREN goal RPAREN — return the inner goal
            // term; the parentheses are syntactic noise at this layer.
            inner_goal(node, var_map)
        }

        "naf_goal" => lower_naf(node, var_map),

        "equality_goal" => lower_equality(node, var_map),

        "variable_term" => {
            // variable_term = VARIABLE
            let text = first_token_value(node).unwrap_or_default();
            lookup_or_create_var(&text, var_map)
        }

        "anonymous_term" => Term::Var(LogicVar::fresh(Some("_"))),

        "compound_term" => lower_compound(node, var_map),

        "list_term" => lower_list(node, var_map),

        // Fallback for rules we don't explicitly enumerate: try to
        // descend into the first child if it's a node; otherwise
        // interpret the first token directly.
        _ => fallback_lower(node, var_map),
    }
}

/// Public alias for [`ast_to_term`] kept for documentation parity.
pub fn lower_term(node: &GrammarASTNode, var_map: &mut HashMap<String, LogicVar>) -> Term {
    ast_to_term(node, var_map)
}

/// Pull the first node child (skipping tokens), lowering it as a term.
fn first_term_child(
    node: &GrammarASTNode,
    var_map: &mut HashMap<String, LogicVar>,
) -> Term {
    for child in &node.children {
        if let ASTNodeOrToken::Node(inner) = child {
            return ast_to_term(inner, var_map);
        }
    }
    // No node children — interpret the first token (e.g. a leaf-only
    // chain that bottomed out at a token via the grammar).
    fallback_lower(node, var_map)
}

fn inner_goal(node: &GrammarASTNode, var_map: &mut HashMap<String, LogicVar>) -> Term {
    // Skip LPAREN / RPAREN tokens; lower the first node child.
    first_term_child(node, var_map)
}

/// Lower `naf_goal = NAF goal_primary` into the canonical Prolog
/// negation-as-failure compound term `'\+'(G)`. Downstream consumers
/// (notably `prolog_loader::naf_or_pos`) pattern-match on this shape
/// to turn the compound into a `BodyLiteral::Neg`.
fn lower_naf(node: &GrammarASTNode, var_map: &mut HashMap<String, LogicVar>) -> Term {
    // Skip the NAF token; lower the inner goal_primary child.
    let inner = first_term_child(node, var_map);
    compound("\\+", vec![inner])
}

fn lower_equality(
    node: &GrammarASTNode,
    var_map: &mut HashMap<String, LogicVar>,
) -> Term {
    // equality_goal = term equality_operator term
    let mut term_children = node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        _ => None,
    });
    let lhs_node = term_children.next();
    let _op_or_term = term_children.next();
    // The middle "node" might be the operator wrapped, or actually the
    // RHS depending on grammar shape. Let's collect all node children
    // and use first + last as the two operands.
    let nodes: Vec<&GrammarASTNode> = node
        .children
        .iter()
        .filter_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                Some(n)
            } else {
                None
            }
        })
        .collect();
    let lhs = match nodes.first() {
        Some(n) => ast_to_term(n, var_map),
        None => atom("?"),
    };
    let rhs = match nodes.last() {
        Some(n) if nodes.len() >= 2 => ast_to_term(n, var_map),
        _ => atom("?"),
    };
    // Find the operator token to distinguish `=` vs `\=`.
    let op_text: String = node
        .children
        .iter()
        .find_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                if n.rule_name == "equality_operator" {
                    return n.token().map(|t| t.value.clone());
                }
            }
            None
        })
        .unwrap_or_else(|| "=".to_string());
    let _ = lhs_node; // silence the warning if grammar shape varies
    compound(op_text, vec![lhs, rhs])
}

fn lower_compound(
    node: &GrammarASTNode,
    var_map: &mut HashMap<String, LogicVar>,
) -> Term {
    // compound_term = atom_token LPAREN [ term_arguments ] RPAREN
    let mut functor: Option<String> = None;
    let mut args: Vec<Term> = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(inner) => match inner.rule_name.as_str() {
                "atom_token" => {
                    functor = inner.token().map(|t| t.value.clone()).or_else(|| {
                        // atom_token might wrap an ATOM token in a deeper layer.
                        first_token_value(inner)
                    });
                }
                "term_arguments" => {
                    for arg_child in &inner.children {
                        if let ASTNodeOrToken::Node(arg_node) = arg_child {
                            if arg_node.rule_name == "term" {
                                args.push(ast_to_term(arg_node, var_map));
                            }
                        }
                    }
                }
                _ => {}
            },
            ASTNodeOrToken::Token(_) => {}
        }
    }
    let functor = functor.unwrap_or_else(|| "?".to_string());
    compound(functor, args)
}

fn lower_list(node: &GrammarASTNode, var_map: &mut HashMap<String, LogicVar>) -> Term {
    // list_term = LBRACKET [ list_body ] RBRACKET
    // list_body = term { COMMA term } [ BAR term ]
    let mut items: Vec<Term> = Vec::new();
    let mut tail: Option<Term> = None;
    let mut seen_bar = false;

    if let Some(body) = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "list_body" => Some(n),
        _ => None,
    }) {
        for child in &body.children {
            match child {
                ASTNodeOrToken::Node(inner) if inner.rule_name == "term" => {
                    let lowered = ast_to_term(inner, var_map);
                    if seen_bar {
                        tail = Some(lowered);
                    } else {
                        items.push(lowered);
                    }
                }
                ASTNodeOrToken::Token(t) => {
                    if t.value == "|" {
                        seen_bar = true;
                    }
                }
                _ => {}
            }
        }
    }

    // Build right-to-left: .(item_0, .(item_1, ..., tail))
    let tail_term = tail.unwrap_or_else(|| atom("[]"));
    items
        .into_iter()
        .rev()
        .fold(tail_term, |acc, item| compound(".", vec![item, acc]))
}

fn fallback_lower(node: &GrammarASTNode, var_map: &mut HashMap<String, LogicVar>) -> Term {
    // Try first node child; if none, decode the first token.
    if let Some(inner) = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        _ => None,
    }) {
        return ast_to_term(inner, var_map);
    }
    if let Some(tok) = node.children.first().and_then(|c| match c {
        ASTNodeOrToken::Token(t) => Some(t),
        _ => None,
    }) {
        return token_to_term(tok, var_map);
    }
    atom("?")
}

fn token_to_term(tok: &Token, var_map: &mut HashMap<String, LogicVar>) -> Term {
    let name = tok.effective_type_name();
    match name {
        "ATOM" => atom(&tok.value),
        "VARIABLE" => lookup_or_create_var(&tok.value, var_map),
        "ANON_VAR" => Term::Var(LogicVar::fresh(Some("_"))),
        "INTEGER" => {
            let n = tok.value.parse::<i64>().unwrap_or(0);
            logic_core::int(n)
        }
        "FLOAT" => {
            let x = tok.value.parse::<f64>().unwrap_or(0.0);
            logic_core::float(x)
        }
        "STRING" => {
            // Token value includes the surrounding quotes; strip them.
            let raw = &tok.value;
            let stripped = raw
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(raw);
            logic_core::string(stripped)
        }
        _ => atom(&tok.value),
    }
}

fn lookup_or_create_var(name: &str, var_map: &mut HashMap<String, LogicVar>) -> Term {
    let v = var_map
        .entry(name.to_string())
        .or_insert_with(|| LogicVar::fresh(Some(name)))
        .clone();
    Term::Var(v)
}

fn first_token_value(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) => Some(t.value.clone()),
        ASTNodeOrToken::Node(n) => first_token_value(n),
    })
}

// ---------------------------------------------------------------------------
// Top-level program collection
// ---------------------------------------------------------------------------

/// A single Prolog top-level item, ready to be loaded into a
/// `logic_engine::KnowledgeBase` (via the future `prolog-loader`).
#[derive(Debug, Clone, PartialEq)]
pub enum ProgramItem {
    /// A fact: a head term with no body.
    Fact(Term),
    /// A rule: head :- body, where body is a conjunction list of goals.
    Rule { head: Term, body: Vec<Term> },
    /// A top-level query: ?- body.
    Query(Vec<Term>),
}

/// Walk a parsed `program` AST and return one `ProgramItem` per
/// top-level statement, in source order.
///
/// Each item gets its own variable map (variable identity is
/// clause-local in Prolog).
pub fn collect_clauses_and_queries(program: &GrammarASTNode) -> Vec<ProgramItem> {
    let mut items = Vec::new();
    for child in &program.children {
        if let ASTNodeOrToken::Node(stmt) = child {
            if stmt.rule_name == "statement" {
                if let Some(inner) = stmt.children.iter().find_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    _ => None,
                }) {
                    items.push(lower_statement(inner));
                }
            }
        }
    }
    items
}

fn lower_statement(stmt: &GrammarASTNode) -> ProgramItem {
    let mut var_map: HashMap<String, LogicVar> = HashMap::new();
    match stmt.rule_name.as_str() {
        "fact_statement" => {
            let head = first_term_child(stmt, &mut var_map);
            ProgramItem::Fact(head)
        }
        "rule_statement" => {
            // callable_term RULE goal DOT
            let nodes: Vec<&GrammarASTNode> = stmt
                .children
                .iter()
                .filter_map(|c| {
                    if let ASTNodeOrToken::Node(n) = c {
                        Some(n)
                    } else {
                        None
                    }
                })
                .collect();
            let head = nodes
                .first()
                .map(|n| ast_to_term(n, &mut var_map))
                .unwrap_or_else(|| atom("?"));
            let body_node = nodes.get(1);
            let body = match body_node {
                Some(g) => goal_to_conjunction(g, &mut var_map),
                None => Vec::new(),
            };
            ProgramItem::Rule { head, body }
        }
        "query_statement" => {
            // QUERY goal DOT
            let body = stmt
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Node(n) if n.rule_name == "goal" => Some(n),
                    _ => None,
                })
                .map(|g| goal_to_conjunction(g, &mut var_map))
                .unwrap_or_default();
            ProgramItem::Query(body)
        }
        // dcg_statement and any future top-level shapes: fall through
        // to a Fact representation for now (the head is the callable
        // and the body is the DCG transformation, which lives in a
        // later spec). For this slice we treat unknown statements as
        // their head term.
        _ => ProgramItem::Fact(first_term_child(stmt, &mut var_map)),
    }
}

fn goal_to_conjunction(
    node: &GrammarASTNode,
    var_map: &mut HashMap<String, LogicVar>,
) -> Vec<Term> {
    // goal -> disjunction -> conjunction { COMMA conjunction } -> goal_primary*
    // We flatten the conjunction chain. Disjunction (;) is not handled
    // structurally here; if a disjunction is encountered the lowering
    // produces one Term per branch sequentially, which the caller may
    // then re-interpret. For this slice we expect canonical
    // conjunction-only bodies.
    let mut goals = Vec::new();
    collect_conjunction(node, &mut goals, var_map);
    goals
}

fn collect_conjunction(
    node: &GrammarASTNode,
    out: &mut Vec<Term>,
    var_map: &mut HashMap<String, LogicVar>,
) {
    match node.rule_name.as_str() {
        // Pass-through wrappers.
        "goal" | "disjunction" | "conjunction" => {
            for child in &node.children {
                if let ASTNodeOrToken::Node(n) = child {
                    collect_conjunction(n, out, var_map);
                }
            }
        }
        // Atomic goal shapes — lower into a single term.
        "goal_primary"
        | "callable_goal"
        | "callable_term"
        | "compound_term"
        | "atom_term"
        | "equality_goal"
        | "grouped_goal" => {
            out.push(ast_to_term(node, var_map));
        }
        // Cut appears as a child token in goal_primary; handle that
        // case by inspecting for the CUT token directly.
        _ => {
            // Fall back: if there's a CUT token, emit `!`; otherwise
            // recurse into nodes.
            if node.children.iter().any(|c| match c {
                ASTNodeOrToken::Token(t) => t.effective_type_name() == "CUT",
                _ => false,
            }) {
                out.push(atom("!"));
                return;
            }
            for child in &node.children {
                if let ASTNodeOrToken::Node(n) = child {
                    collect_conjunction(n, out, var_map);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<ProgramItem> {
        let ast = parse_iso_prolog(src);
        collect_clauses_and_queries(&ast)
    }

    #[test]
    fn bare_atom_fact() {
        let items = parse("homer.");
        assert_eq!(items.len(), 1);
        match &items[0] {
            ProgramItem::Fact(t) => assert_eq!(t, &atom("homer")),
            other => panic!("expected Fact, got {:?}", other),
        }
    }

    #[test]
    fn compound_fact() {
        let items = parse("father(homer, bart).");
        assert_eq!(items.len(), 1);
        match &items[0] {
            ProgramItem::Fact(Term::Compound { functor, args }) => {
                assert_eq!(functor, "father");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], atom("homer"));
                assert_eq!(args[1], atom("bart"));
            }
            other => panic!("expected compound fact, got {:?}", other),
        }
    }

    #[test]
    fn rule_with_single_body_goal() {
        let items = parse("parent(X, Y) :- father(X, Y).");
        assert_eq!(items.len(), 1);
        match &items[0] {
            ProgramItem::Rule { body, .. } => assert_eq!(body.len(), 1),
            other => panic!("expected Rule, got {:?}", other),
        }
    }

    #[test]
    fn rule_with_conjunction_body() {
        let items = parse("gp(X, Z) :- parent(X, Y), parent(Y, Z).");
        match &items[0] {
            ProgramItem::Rule { body, .. } => assert_eq!(body.len(), 2),
            other => panic!("expected Rule, got {:?}", other),
        }
    }

    #[test]
    fn query_with_conjunction() {
        let items = parse("?- a, b, c.");
        match &items[0] {
            ProgramItem::Query(goals) => assert_eq!(goals.len(), 3),
            other => panic!("expected Query, got {:?}", other),
        }
    }

    #[test]
    fn variable_identity_shared_within_clause() {
        // In `p(X, X)`, both args should be the same LogicVar.
        let items = parse("p(X, X).");
        match &items[0] {
            ProgramItem::Fact(Term::Compound { args, .. }) => {
                assert_eq!(args.len(), 2);
                match (&args[0], &args[1]) {
                    (Term::Var(v1), Term::Var(v2)) => assert_eq!(v1.id, v2.id),
                    _ => panic!("expected both args to be vars"),
                }
            }
            other => panic!("got {:?}", other),
        }
    }

    #[test]
    fn list_term_lowers_to_cons_cells() {
        let items = parse("p([a, b, c]).");
        match &items[0] {
            ProgramItem::Fact(Term::Compound { args, .. }) => {
                assert_eq!(args.len(), 1);
                // Pretty-print of the list should equal the cons form.
                assert_eq!(args[0].to_string(), ".(a, .(b, .(c, [])))");
            }
            other => panic!("got {:?}", other),
        }
    }

    #[test]
    fn list_with_tail_variable() {
        let items = parse("p([H | T]).");
        match &items[0] {
            ProgramItem::Fact(Term::Compound { args, .. }) => {
                // .(H, T) — the tail is the variable T directly, not [] cons.
                match &args[0] {
                    Term::Compound { functor, args: inner } => {
                        assert_eq!(functor, ".");
                        assert_eq!(inner.len(), 2);
                        assert!(matches!(inner[0], Term::Var(_)));
                        assert!(matches!(inner[1], Term::Var(_)));
                    }
                    _ => panic!("expected cons compound"),
                }
            }
            other => panic!("got {:?}", other),
        }
    }

    #[test]
    fn integer_and_float_lower_to_their_term_kinds() {
        let items = parse("p(42, 3.14).");
        match &items[0] {
            ProgramItem::Fact(Term::Compound { args, .. }) => {
                assert_eq!(args[0], logic_core::int(42));
                assert_eq!(args[1], logic_core::float(3.14));
            }
            other => panic!("got {:?}", other),
        }
    }

    #[test]
    fn full_small_program_returns_three_items() {
        let src = "\
            father(homer, bart).\n\
            parent(X, Y) :- father(X, Y).\n\
            ?- parent(homer, Who).\n\
        ";
        let items = parse(src);
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], ProgramItem::Fact(_)));
        assert!(matches!(items[1], ProgramItem::Rule { .. }));
        assert!(matches!(items[2], ProgramItem::Query(_)));
    }

    #[test]
    fn try_parse_returns_err_on_syntax_error() {
        // Missing closing paren.
        let res = try_parse_iso_prolog("father(homer, bart.");
        assert!(res.is_err());
    }
}
