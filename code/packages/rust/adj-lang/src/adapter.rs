//! # Adapter — generic GrammarASTNode → typed `ast::Program`.
//!
//! The grammar-driven parser produces a generic tree of
//! [`GrammarASTNode`]s and [`Token`]s. The lowerer (`lower.rs`)
//! consumes a typed [`crate::ast::Program`] with semantically rich
//! variants. This module bridges the two.
//!
//! The mapping is mechanical: each rule in `adj_lang.grammar` has
//! one matching variant in the typed AST. The adapter walks the
//! parse tree by rule name, extracting tokens and child rule nodes
//! by structural position.
//!
//! ## Why a typed AST at all
//!
//! Keeping the lowerer's input typed has two benefits:
//!
//! 1. **Lowering stays small.** Walking a generic AST by
//!    `node.rule_name == "prior_decl"` works but is fragile;
//!    matching on `Statement::Prior { … }` is checked by the
//!    compiler.
//! 2. **The typed AST is reusable.** Other Rust consumers (an
//!    LSP, a formatter, a documentation generator) can build on
//!    `Statement` / `Term` without re-implementing the adapter.
//!
//! The adapter is small (~250 LOC) and shares the failure modes of
//! the generic parser: if the grammar accepts an input but the
//! adapter can't map it, that's an adapter bug, not a malformed
//! program.

use lexer::token::TokenType;
use math_frontend::{BinOp, MathExpr, Number, RelOp as MathRelOp, UnaryOp};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::ast::{
    AggOp, Annotation, ArithOp, CmpOp, Define, DefineKind, Evidence, ExprAst, OptDir, Program,
    RelOp, RuleLiteral, Statement, Term, TrustTierName,
};

/// Errors raised while adapting a generic AST to the typed AST.
///
/// These are *structural* — they fire when the parse tree doesn't
/// match the shape the adapter expects from the grammar. In
/// practice, a `Mismatch` indicates a divergence between
/// `adj_lang.grammar` and this adapter; a `MissingChild` indicates
/// an empty production where the grammar guarantees at least one
/// element.
#[derive(Debug, Clone, PartialEq)]
pub enum AdapterError {
    /// The parse tree's root rule was not `program`.
    NotProgram { actual: String },
    /// A node had a rule name the adapter did not expect at this
    /// position. Carries the expected name(s) for debugging.
    UnexpectedRule {
        expected: &'static str,
        actual: String,
    },
    /// A node was missing a required child element. `position` is a
    /// human-readable description of the slot.
    MissingChild {
        rule: String,
        position: &'static str,
    },
    /// A token had an unexpected type or shape (e.g. a NUMBER token
    /// whose `value` does not parse as `f64`).
    BadToken {
        rule: String,
        kind: TokenType,
        value: String,
        reason: &'static str,
    },
    /// An unknown trust tier keyword reached the adapter; should be
    /// impossible if the grammar's `trust_tier` rule stays in sync
    /// with [`TrustTierName`].
    UnknownTrustTier { actual: String },
    /// A `latex "<math>"` expression or `constrain latex "<equation>"`
    /// could not be parsed by the repo's LaTeX MathFrontend.
    LatexParse { source: String, detail: String },
    /// LaTeX parsed successfully, but used math outside the ADJ arithmetic /
    /// constraint subset this surface can lower faithfully.
    UnsupportedLatexMath { source: String, detail: String },
}

/// Adapt the root `program` node.
pub fn adapt_program(root: &GrammarASTNode) -> Result<Program, AdapterError> {
    if root.rule_name != "program" {
        return Err(AdapterError::NotProgram {
            actual: root.rule_name.clone(),
        });
    }
    let mut statements = Vec::new();
    for child in &root.children {
        if let ASTNodeOrToken::Node(node) = child {
            // The `program` rule produces `statement` children plus
            // a trailing EOF token (which is `ASTNodeOrToken::Token`,
            // not a Node, so it skips this branch).
            if node.rule_name == "statement" {
                statements.push(adapt_statement(node)?);
            }
        }
    }
    Ok(Program { statements })
}

fn adapt_statement(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    if node.rule_name != "statement" {
        return Err(AdapterError::UnexpectedRule {
            expected: "statement",
            actual: node.rule_name.clone(),
        });
    }
    let child = first_child_node(node, "statement", "decl")?;
    match child.rule_name.as_str() {
        "prior_decl" => adapt_prior(child),
        "contributes_decl" => adapt_contributes(child),
        "interacts_decl" => adapt_interacts(child),
        "uncertain_decl" => adapt_uncertain(child),
        "observe_decl" => adapt_observe(child),
        "relate_decl" => adapt_relate(child),
        "rule_decl" => adapt_rule(child),
        "functional_decl" => adapt_functional(child),
        "context_order_decl" => adapt_context_order(child),
        "query_decl" => adapt_query(child),
        "let_decl" => adapt_let(child),
        "symbol_decl" => adapt_symbol(child),
        "constrain_latex_decl" => adapt_constrain_latex(child),
        "constrain_decl" => adapt_constrain(child),
        "solve_decl" => adapt_solve(child),
        "check_decl" => Ok(Statement::Check),
        "optimize_decl" => adapt_optimize(child),
        "dictionary_decl" => adapt_dictionary(child),
        "define_decl" => adapt_define(child).map(Statement::Define),
        "rulebook_decl" => adapt_rulebook(child),
        "use_decl" => adapt_use(child),
        "import_decl" => adapt_import(child),
        other => Err(AdapterError::UnexpectedRule {
            expected: "one of prior_decl / contributes_decl / interacts_decl / uncertain_decl / observe_decl / query_decl / let_decl / symbol_decl / constrain_latex_decl / constrain_decl / solve_decl / check_decl / optimize_decl / dictionary_decl / define_decl / rulebook_decl / use_decl / import_decl",
            actual: other.to_string(),
        }),
    }
}

fn adapt_prior(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // prior_decl = "prior" NUMBER "for" term { annotation }
    let probability = expect_number_at(node, 1)?;
    let conclusion = expect_term_child(node, "prior_decl")?;
    let annotations = collect_annotations(node)?;
    Ok(Statement::Prior {
        probability,
        conclusion,
        annotations,
    })
}

fn adapt_contributes(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // contributes_decl = "contributes" NUMBER "from" evidence "to" term { annotation }
    //
    // The LR is the only NUMBER token that is a *direct* child of
    // contributes_decl: a predicate's threshold lives inside the nested
    // `evidence`/`predicate` node, so `expect_number_at(node, 1)` is
    // unambiguous. Likewise the conclusion is the only *direct* `term`
    // child — term-shaped evidence is nested inside the `evidence` node.
    let lr = expect_number_at(node, 1)?;
    let evidence_node = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "evidence" => Some(n),
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: "contributes_decl".into(),
            position: "evidence",
        })?;
    let evidence = adapt_evidence(evidence_node)?;
    let conclusion =
        collect_term_children(node)
            .into_iter()
            .next()
            .ok_or(AdapterError::MissingChild {
                rule: "contributes_decl".into(),
                position: "conclusion term",
            })?;
    let annotations = collect_annotations(node)?;
    Ok(Statement::Contributes {
        lr,
        evidence,
        conclusion,
        annotations,
    })
}

fn adapt_evidence(node: &GrammarASTNode) -> Result<Evidence, AdapterError> {
    // evidence = predicate | term
    //
    // The matched alternative is flattened into `evidence`'s children
    // (grammar groups/alternations don't introduce wrapper nodes), so
    // the first child node is either a `predicate` or a `term`.
    let child = first_child_node(node, "evidence", "predicate or term")?;
    match child.rule_name.as_str() {
        "predicate" => adapt_predicate(child),
        "term" => Ok(Evidence::Term(adapt_term(child)?)),
        other => Err(AdapterError::UnexpectedRule {
            expected: "predicate or term",
            actual: other.to_string(),
        }),
    }
}

fn adapt_predicate(node: &GrammarASTNode) -> Result<Evidence, AdapterError> {
    // predicate = IDENT ( GE | LE | GT | LT | EQEQ ) NUMBER
    //
    // The slot IDENT and the operator are both lexed as TokenType::Name
    // (the operator carries a custom type_name like "GE"), so we
    // distinguish by *value*: the operator's value is one of the five
    // comparison symbols; everything else Name-typed is the slot.
    let mut slot: Option<String> = None;
    let mut op: Option<CmpOp> = None;
    let mut value: Option<f64> = None;
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            match t.value.as_str() {
                ">=" => op = Some(CmpOp::Ge),
                "<=" => op = Some(CmpOp::Le),
                ">" => op = Some(CmpOp::Gt),
                "<" => op = Some(CmpOp::Lt),
                "==" => op = Some(CmpOp::Eq),
                _ if t.type_ == TokenType::Number => {
                    value = Some(parse_finite(&t.value, t.type_, "predicate")?);
                }
                _ if t.type_ == TokenType::Name && slot.is_none() => {
                    slot = Some(t.value.clone());
                }
                _ => {}
            }
        }
    }
    let slot = slot.ok_or(AdapterError::MissingChild {
        rule: "predicate".into(),
        position: "slot identifier",
    })?;
    let op = op.ok_or(AdapterError::MissingChild {
        rule: "predicate".into(),
        position: "comparison operator",
    })?;
    let value = value.ok_or(AdapterError::MissingChild {
        rule: "predicate".into(),
        position: "threshold NUMBER",
    })?;
    Ok(Evidence::Predicate { slot, op, value })
}

fn adapt_interacts(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // interacts_decl = "interacts" NUMBER "when" term "and" term { "and" term } "for" term { annotation }
    let lr = expect_number_at(node, 1)?;
    let terms = collect_term_children(node);
    if terms.len() < 3 {
        return Err(AdapterError::MissingChild {
            rule: "interacts_decl".into(),
            position: "at least two evidence terms + one conclusion",
        });
    }
    let conclusion = terms.last().cloned().unwrap();
    let evidence_set: Vec<Term> = terms[..terms.len() - 1].to_vec();
    let annotations = collect_annotations(node)?;
    Ok(Statement::Interacts {
        lr,
        evidence_set,
        conclusion,
        annotations,
    })
}

fn adapt_uncertain(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // uncertain_decl = "uncertain" LBRACE term { COMMA term } RBRACE "for" term { annotation }
    //
    // The grammar guarantees: 1+ domain terms inside the braces,
    // then 1 conclusion term after `for`. So `collect_term_children`
    // returns N+1 terms where the last one is the conclusion and
    // the first N are the domain.
    let terms = collect_term_children(node);
    if terms.len() < 2 {
        return Err(AdapterError::MissingChild {
            rule: "uncertain_decl".into(),
            position: "at least one domain term + one conclusion",
        });
    }
    let conclusion = terms.last().cloned().unwrap();
    let domain: Vec<Term> = terms[..terms.len() - 1].to_vec();
    let annotations = collect_annotations(node)?;
    Ok(Statement::Uncertain {
        domain,
        conclusion,
        annotations,
    })
}

fn adapt_observe(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // observe_decl = "observe" term
    let term = expect_term_child(node, "observe_decl")?;
    Ok(Statement::Observe { term })
}

fn adapt_relate(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // relate_decl = "relate" term { annotation }
    // The edge is the single direct `term` child (its functor is the relation).
    let edge = expect_term_child(node, "relate_decl")?;
    let annotations = collect_annotations(node)?;
    Ok(Statement::Relate { edge, annotations })
}

fn adapt_rule(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // rule_decl = "rule" "{" "head" ":" term "when" ":" body_literal {"," body_literal}
    //             { annotation } "}"
    // The HEAD is the single direct `term` child (body terms nest under body_literal).
    let head = expect_term_child(node, "rule_decl")?;
    let mut body = Vec::new();
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == "body_literal" {
                // body_literal = [ "not" ] term — negation-as-failure when `not` present.
                let negated = n.children.iter().any(|c| {
                    matches!(c, ASTNodeOrToken::Token(t)
                        if t.type_ == TokenType::Name && t.value == "not")
                });
                let term = expect_term_child(n, "body_literal")?;
                body.push(RuleLiteral { negated, term });
            }
        }
    }
    if body.is_empty() {
        return Err(AdapterError::MissingChild {
            rule: "rule_decl".into(),
            position: "when: body literal",
        });
    }
    let annotations = collect_annotations(node)?;
    // ADJ73: optional trailing `priority: <tier>` (PR-C) and `context: <name>` (PR-B). Each
    // value is the first Name token AFTER its keyword literal (the COLON is a separate token).
    // Only structural Name tokens appear at the rule_decl level (head/when/priority/context +
    // the two values); body/head TERMS are Nodes, so they cannot be mistaken for a value.
    let priority = ident_after_keyword(node, "priority");
    let context = ident_after_keyword(node, "context");
    Ok(Statement::Rule {
        head,
        body,
        annotations,
        priority,
        context,
    })
}

/// The IDENT token that immediately follows the keyword literal `keyword` among a node's direct
/// token children (e.g. the tier after `priority`, the context after `context`). `None` if the
/// keyword is absent.
fn ident_after_keyword(node: &GrammarASTNode, keyword: &str) -> Option<String> {
    let mut seen = false;
    for child in &node.children {
        if let ASTNodeOrToken::Token(t) = child {
            if t.type_ == TokenType::Name && t.value == keyword {
                seen = true;
            } else if seen && t.type_ == TokenType::Name {
                return Some(t.value.clone());
            }
        }
    }
    None
}

fn adapt_context_order(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // context_order_decl = "context_order" "{" IDENT ">" IDENT { "," IDENT ">" IDENT } "}"
    // The IDENT tokens come in (higher, lower) pairs. Collect the Name tokens in order and pair
    // them up, skipping the `context_order` keyword and the `>` operator (the lexer surfaces GT
    // as a Name token whose value is ">"). Commas/braces are non-Name tokens, already excluded.
    let idents: Vec<String> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t)
                if t.type_ == TokenType::Name && t.value != "context_order" && t.value != ">" =>
            {
                Some(t.value.clone())
            }
            _ => None,
        })
        .collect();
    if idents.is_empty() || idents.len() % 2 != 0 {
        return Err(AdapterError::MissingChild {
            rule: "context_order_decl".into(),
            position: "higher > lower context pairs",
        });
    }
    let edges = idents
        .chunks_exact(2)
        .map(|p| (p[0].clone(), p[1].clone()))
        .collect();
    Ok(Statement::ContextOrder { edges })
}

fn adapt_functional(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // functional_decl = "functional" term — declare a predicate functional on its last
    // argument. Only the functor + arity of the term matter (arg names are placeholders).
    let term = expect_term_child(node, "functional_decl")?;
    let (functor, arity) = match &term {
        Term::Compound { functor, args } => (functor.clone(), args.len()),
        Term::Atom(name) => (name.clone(), 0),
        _ => {
            return Err(AdapterError::MissingChild {
                rule: "functional_decl".into(),
                position: "predicate term (compound or atom)",
            })
        }
    };
    Ok(Statement::Functional { functor, arity })
}

fn adapt_query(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // query_decl = QUESTION term
    let conclusion = expect_term_child(node, "query_decl")?;
    Ok(Statement::Query { conclusion })
}

fn adapt_let(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // let_decl = "let" IDENT EQUALS expr
    //
    // Two Name tokens are direct children: the `let` keyword and the
    // binding name. `let` is not a reserved keyword (it lexes as a plain
    // Name), so we take the first Name token whose value isn't "let".
    let name = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name && t.value != "let" => {
                Some(t.value.clone())
            }
            _ => None,
        })
        .next()
        .ok_or(AdapterError::MissingChild {
            rule: "let_decl".into(),
            position: "binding name",
        })?;
    let expr_node = first_named_child(node, "expr").ok_or(AdapterError::MissingChild {
        rule: "let_decl".into(),
        position: "expr",
    })?;
    let expr = adapt_expr(expr_node)?;
    Ok(Statement::Let { name, expr })
}

fn adapt_symbol(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // symbol_decl = "symbol" IDENT COLON term
    // The name is the first Name token that isn't the `symbol` keyword; the
    // sort is the (only) direct `term` child.
    let name = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name && t.value != "symbol" => {
                Some(t.value.clone())
            }
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: "symbol_decl".into(),
            position: "symbol name",
        })?;
    let sort = expect_term_child(node, "symbol_decl")?;
    Ok(Statement::Symbol { name, sort })
}

fn adapt_constrain(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // constrain_decl = "constrain" expr relop expr
    let exprs: Vec<&GrammarASTNode> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "expr" => Some(n),
            _ => None,
        })
        .collect();
    let lhs = adapt_expr(exprs.first().ok_or(AdapterError::MissingChild {
        rule: "constrain_decl".into(),
        position: "left-hand expr",
    })?)?;
    let rhs = adapt_expr(exprs.get(1).ok_or(AdapterError::MissingChild {
        rule: "constrain_decl".into(),
        position: "right-hand expr",
    })?)?;
    let relop_node = first_named_child(node, "relop").ok_or(AdapterError::MissingChild {
        rule: "constrain_decl".into(),
        position: "relop",
    })?;
    let op = rel_op_from_node(relop_node)?;
    Ok(Statement::Constrain { lhs, op, rhs })
}

fn adapt_constrain_latex(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // constrain_latex_decl = "constrain" latex_relation
    // latex_relation = "latex" STRING
    let relation = first_named_child(node, "latex_relation").ok_or(AdapterError::MissingChild {
        rule: "constrain_latex_decl".into(),
        position: "latex relation",
    })?;
    let source = latex_string_from_node(relation, "latex_relation")?;
    let math = parse_latex_math(&source)?;
    let MathExpr::Rel(op, lhs, rhs) = math else {
        return Err(AdapterError::UnsupportedLatexMath {
            source,
            detail: "expected a LaTeX relation such as x^2 = 4".into(),
        });
    };
    let op = lower_latex_relop(op, &source)?;
    Ok(Statement::Constrain {
        lhs: latex_math_to_expr_ast(*lhs, &source)?,
        op,
        rhs: latex_math_to_expr_ast(*rhs, &source)?,
    })
}

fn adapt_solve(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // solve_decl = "solve" "for" LBRACE IDENT { COMMA IDENT } RBRACE
    // Every Name token except the `solve` / `for` keywords is a target.
    let names: Vec<String> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t)
                if t.type_ == TokenType::Name && t.value != "solve" && t.value != "for" =>
            {
                Some(t.value.clone())
            }
            _ => None,
        })
        .collect();
    if names.is_empty() {
        return Err(AdapterError::MissingChild {
            rule: "solve_decl".into(),
            position: "at least one target symbol",
        });
    }
    Ok(Statement::SolveFor { names })
}

fn adapt_optimize(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // optimize_decl = ( "minimize" | "maximize" ) expr
    // The direction is the leading Name token ("minimize"/"maximize", both
    // IDENT-matched literals); the objective is the `expr` child.
    let dir = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name && t.value == "minimize" => {
                Some(OptDir::Minimize)
            }
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name && t.value == "maximize" => {
                Some(OptDir::Maximize)
            }
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: "optimize_decl".into(),
            position: "minimize / maximize keyword",
        })?;
    let expr_node = first_named_child(node, "expr").ok_or(AdapterError::MissingChild {
        rule: "optimize_decl".into(),
        position: "objective expr",
    })?;
    let objective = adapt_expr(expr_node)?;
    Ok(Statement::Optimize { dir, objective })
}

/// The first `TokenType::Name` token whose value isn't `keyword`. Used to pull a
/// declaration's name out from beside its leading keyword.
fn first_name_not<'a>(node: &'a GrammarASTNode, keyword: &str) -> Option<&'a str> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name && t.value != keyword => {
            Some(t.value.as_str())
        }
        _ => None,
    })
}

fn adapt_dictionary(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // dictionary_decl = "dictionary" IDENT LBRACE { define_decl } RBRACE
    let name = first_name_not(node, "dictionary")
        .ok_or(AdapterError::MissingChild {
            rule: "dictionary_decl".into(),
            position: "dictionary name",
        })?
        .to_string();
    let defines = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "define_decl" => Some(adapt_define(n)),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Statement::Dictionary { name, defines })
}

fn adapt_define(node: &GrammarASTNode) -> Result<Define, AdapterError> {
    // define_decl = "define" IDENT COLON define_kind { surface_clause }
    let name = first_name_not(node, "define")
        .ok_or(AdapterError::MissingChild {
            rule: "define_decl".into(),
            position: "term name",
        })?
        .to_string();
    let kind_node = first_named_child(node, "define_kind").ok_or(AdapterError::MissingChild {
        rule: "define_decl".into(),
        position: "define_kind",
    })?;
    let kind = adapt_define_kind(kind_node)?;
    let mut surfaces = Vec::new();
    for c in &node.children {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == "surface_clause" {
                for sc in &n.children {
                    if let ASTNodeOrToken::Token(t) = sc {
                        if t.type_ == TokenType::String {
                            surfaces.push(unquote_string(&t.value));
                        }
                    }
                }
            }
        }
    }
    Ok(Define {
        name,
        kind,
        surfaces,
    })
}

fn adapt_define_kind(node: &GrammarASTNode) -> Result<DefineKind, AdapterError> {
    // define_kind = "hypothesis"
    //             | "finding" [ "values" LBRACK IDENT {COMMA IDENT} RBRACK ]
    //             | "entity"
    //             | "relation" "from" IDENT "to" IDENT
    // hypothesis/finding/values/entity/relation are IDENT-matched literals, so
    // they surface as Name tokens; `from`/`to` are lexer keywords (Keyword
    // tokens), so they do NOT appear in this Name list. The first Name selects
    // the kind; the rest are the value names (finding) or domain/range (relation).
    let names: Vec<&str> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t.value.as_str()),
            _ => None,
        })
        .collect();
    match names.first().copied() {
        Some("hypothesis") => Ok(DefineKind::Hypothesis),
        Some("entity") => Ok(DefineKind::Entity),
        Some("relation") => {
            // "relation" "from" IDENT "to" IDENT. The `from`/`to` structural words
            // surface as Name tokens too, so exclude them by value; the two
            // remaining names are the domain and range entity kinds.
            let entities: Vec<&str> = names
                .iter()
                .skip(1)
                .filter(|v| !matches!(**v, "from" | "to"))
                .copied()
                .collect();
            let from = entities
                .first()
                .ok_or(AdapterError::MissingChild {
                    rule: "define_kind".into(),
                    position: "relation domain (from)",
                })?
                .to_string();
            let to = entities
                .get(1)
                .ok_or(AdapterError::MissingChild {
                    rule: "define_kind".into(),
                    position: "relation range (to)",
                })?
                .to_string();
            Ok(DefineKind::Relation { from, to })
        }
        Some("finding") => {
            // The remaining Name tokens are the value names. `values` and the
            // brackets/comma are structural; exclude them by value.
            let values = names
                .iter()
                .skip(1)
                .filter(|v| !matches!(**v, "values" | "[" | "]" | ","))
                .map(|v| v.to_string())
                .collect();
            Ok(DefineKind::Finding { values })
        }
        _ => Err(AdapterError::MissingChild {
            rule: "define_kind".into(),
            position: "`hypothesis` / `finding` / `entity` / `relation`",
        }),
    }
}

fn adapt_rulebook(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // rulebook_decl = "rulebook" IDENT LBRACE { statement } RBRACE
    let name = first_name_not(node, "rulebook")
        .ok_or(AdapterError::MissingChild {
            rule: "rulebook_decl".into(),
            position: "rulebook name",
        })?
        .to_string();
    // The body is a sequence of `statement` nodes — adapt each through the
    // same dispatcher, so a rulebook may hold any clause (and its own `use`).
    let statements = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(adapt_statement(n)),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Statement::Rulebook { name, statements })
}

fn adapt_use(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // use_decl = "use" IDENT
    let name = first_name_not(node, "use")
        .ok_or(AdapterError::MissingChild {
            rule: "use_decl".into(),
            position: "dictionary name",
        })?
        .to_string();
    Ok(Statement::Use(name))
}

fn adapt_import(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // import_decl = "import" STRING
    let path = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::String => {
                Some(unquote_string(&t.value))
            }
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: "import_decl".into(),
            position: "import path string",
        })?;
    Ok(Statement::Import(path))
}

/// `relop = GE | LE | GT | LT | EQEQ | EQUALS | NE` — distinguish by the
/// operator's literal value.
fn rel_op_from_node(node: &GrammarASTNode) -> Result<RelOp, AdapterError> {
    node.children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) => match t.value.as_str() {
                ">=" => Some(RelOp::Ge),
                "<=" => Some(RelOp::Le),
                ">" => Some(RelOp::Gt),
                "<" => Some(RelOp::Lt),
                "==" | "=" => Some(RelOp::Eq),
                "!=" => Some(RelOp::Ne),
                _ => None,
            },
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: "relop".into(),
            position: "relational operator",
        })
}

/// `expr = term_expr { ( PLUS | MINUS ) term_expr }` — left-associative
/// fold over `+`/`-`, looser than `*`/`/`.
fn adapt_expr(node: &GrammarASTNode) -> Result<ExprAst, AdapterError> {
    fold_binary(node, "expr", "term_expr", adapt_term_expr)
}

/// `term_expr = factor { ( STAR | SLASH ) factor }` — left-associative
/// fold over `*`/`/`.
fn adapt_term_expr(node: &GrammarASTNode) -> Result<ExprAst, AdapterError> {
    fold_binary(node, "term_expr", "factor", adapt_factor)
}

/// Shared left-fold for the two binary-precedence levels: walk the
/// node's children in source order, building `Bin(op, acc, rhs)` as each
/// `(operator-token, operand)` pair appears. The operands are child
/// nodes named `operand_rule`, adapted by `adapt_operand`.
fn fold_binary(
    node: &GrammarASTNode,
    rule: &'static str,
    operand_rule: &'static str,
    adapt_operand: fn(&GrammarASTNode) -> Result<ExprAst, AdapterError>,
) -> Result<ExprAst, AdapterError> {
    let mut acc: Option<ExprAst> = None;
    let mut pending_op: Option<ArithOp> = None;
    for c in &node.children {
        match c {
            ASTNodeOrToken::Node(n) if n.rule_name == operand_rule => {
                let operand = adapt_operand(n)?;
                acc = Some(match (acc.take(), pending_op.take()) {
                    (None, _) => operand,
                    (Some(lhs), Some(op)) => ExprAst::Bin(op, Box::new(lhs), Box::new(operand)),
                    (Some(lhs), None) => lhs, // defensive: two operands, no op
                });
            }
            ASTNodeOrToken::Token(t) => {
                if let Some(op) = arith_op_from_value(&t.value) {
                    pending_op = Some(op);
                }
            }
            _ => {}
        }
    }
    acc.ok_or(AdapterError::MissingChild {
        rule: rule.into(),
        position: "at least one operand",
    })
}

/// `factor = agg | NUMBER | IDENT | LPAREN expr RPAREN`.
fn adapt_factor(node: &GrammarASTNode) -> Result<ExprAst, AdapterError> {
    // A parsed `agg` or parenthesised `expr` appears as a named child node;
    // a bare NUMBER or IDENT appears as a token. Check nodes first.
    if let Some(latex) = first_named_child(node, "latex_expr") {
        let source = latex_string_from_node(latex, "latex_expr")?;
        let math = parse_latex_math(&source)?;
        return latex_math_to_expr_ast(math, &source);
    }
    if let Some(agg) = first_named_child(node, "agg") {
        return adapt_agg(agg);
    }
    if let Some(inner) = first_named_child(node, "expr") {
        return adapt_expr(inner);
    }
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.type_ == TokenType::Number {
                return Ok(ExprAst::Lit(parse_finite(&t.value, t.type_, "factor")?));
            }
            if t.type_ == TokenType::Name {
                return Ok(ExprAst::Ref(t.value.clone()));
            }
        }
    }
    Err(AdapterError::MissingChild {
        rule: "factor".into(),
        position: "latex / agg / number / identifier / parenthesised expr",
    })
}

fn latex_string_from_node(node: &GrammarASTNode, rule: &str) -> Result<String, AdapterError> {
    node.children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::String => {
                Some(unquote_latex_string(&t.value))
            }
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: rule.into(),
            position: "STRING",
        })
}

fn parse_latex_math(source: &str) -> Result<MathExpr, AdapterError> {
    let math = strip_math_delimiters(source);
    let registry = latex::registry();
    registry
        .parse("latex", math)
        .map_err(|e| AdapterError::LatexParse {
            source: source.to_string(),
            detail: e.message,
        })
}

fn strip_math_delimiters(source: &str) -> &str {
    let mut s = source.trim();
    loop {
        let next = if s.starts_with("\\(") && s.ends_with("\\)") && s.len() >= 4 {
            Some(&s[2..s.len() - 2])
        } else if s.starts_with("\\[") && s.ends_with("\\]") && s.len() >= 4 {
            Some(&s[2..s.len() - 2])
        } else if s.starts_with("$$") && s.ends_with("$$") && s.len() >= 4 {
            Some(&s[2..s.len() - 2])
        } else if s.starts_with('$') && s.ends_with('$') && s.len() >= 2 {
            Some(&s[1..s.len() - 1])
        } else {
            None
        };
        match next {
            Some(inner) => s = inner.trim(),
            None => return s,
        }
    }
}

fn latex_math_to_expr_ast(expr: MathExpr, source: &str) -> Result<ExprAst, AdapterError> {
    match expr {
        MathExpr::Number(n) => number_to_lit(&n, source),
        MathExpr::Symbol(name) => Ok(ExprAst::Ref(name)),
        MathExpr::Group(inner) => latex_math_to_expr_ast(*inner, source),
        MathExpr::Unary(UnaryOp::Pos, inner) => latex_math_to_expr_ast(*inner, source),
        MathExpr::Unary(UnaryOp::Neg, inner) => Ok(ExprAst::Bin(
            ArithOp::Sub,
            Box::new(ExprAst::Lit(0.0)),
            Box::new(latex_math_to_expr_ast(*inner, source)?),
        )),
        MathExpr::Bin(BinOp::Add, lhs, rhs) => latex_bin(ArithOp::Add, *lhs, *rhs, source),
        MathExpr::Bin(BinOp::Sub, lhs, rhs) => latex_bin(ArithOp::Sub, *lhs, *rhs, source),
        MathExpr::Bin(BinOp::Mul, lhs, rhs) => latex_bin(ArithOp::Mul, *lhs, *rhs, source),
        MathExpr::Bin(BinOp::Div, lhs, rhs) | MathExpr::Frac(lhs, rhs) => {
            latex_bin(ArithOp::Div, *lhs, *rhs, source)
        }
        MathExpr::Bin(BinOp::Pow, base, exponent) => {
            let n = latex_power_exponent(&exponent, source)?;
            expand_power(*base, n, source)
        }
        MathExpr::Rel(_, _, _) => Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: "relation-valued LaTeX is only valid in `constrain latex`".into(),
        }),
        other => Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: format!("unsupported ADJ arithmetic subset: {other:?}"),
        }),
    }
}

fn latex_bin(
    op: ArithOp,
    lhs: MathExpr,
    rhs: MathExpr,
    source: &str,
) -> Result<ExprAst, AdapterError> {
    Ok(ExprAst::Bin(
        op,
        Box::new(latex_math_to_expr_ast(lhs, source)?),
        Box::new(latex_math_to_expr_ast(rhs, source)?),
    ))
}

fn number_to_lit(number: &Number, source: &str) -> Result<ExprAst, AdapterError> {
    let value = number
        .to_f64()
        .ok_or_else(|| AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: format!(
                "numeric literal is outside f64 range: {}",
                number.as_written()
            ),
        })?;
    if !value.is_finite() {
        return Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: format!("numeric literal is non-finite: {}", number.as_written()),
        });
    }
    Ok(ExprAst::Lit(value))
}

fn latex_power_exponent(expr: &MathExpr, source: &str) -> Result<usize, AdapterError> {
    let MathExpr::Number(n) = expr else {
        return Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: "only non-negative integer exponents are supported in ADJ arithmetic".into(),
        });
    };
    let Some(v) = n.to_f64() else {
        return Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: format!("exponent is outside f64 range: {}", n.as_written()),
        });
    };
    if !(v.is_finite() && v.fract() == 0.0 && v >= 0.0 && v <= 8.0) {
        return Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: "only integer exponents from 0 through 8 are supported".into(),
        });
    }
    Ok(v as usize)
}

fn expand_power(base: MathExpr, exponent: usize, source: &str) -> Result<ExprAst, AdapterError> {
    if exponent == 0 {
        return Ok(ExprAst::Lit(1.0));
    }
    let base = latex_math_to_expr_ast(base, source)?;
    let mut acc = base.clone();
    for _ in 1..exponent {
        acc = ExprAst::Bin(ArithOp::Mul, Box::new(acc), Box::new(base.clone()));
    }
    Ok(acc)
}

fn lower_latex_relop(op: MathRelOp, source: &str) -> Result<RelOp, AdapterError> {
    match op {
        MathRelOp::Eq => Ok(RelOp::Eq),
        MathRelOp::Ne => Ok(RelOp::Ne),
        MathRelOp::Lt => Ok(RelOp::Lt),
        MathRelOp::Le => Ok(RelOp::Le),
        MathRelOp::Gt => Ok(RelOp::Gt),
        MathRelOp::Ge => Ok(RelOp::Ge),
        MathRelOp::Approx | MathRelOp::Equiv => Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: "approx/equiv relations are not solver constraints".into(),
        }),
    }
}

/// `agg = ( "sum" | "count" | "min" | "max" | "avg" ) LPAREN IDENT RPAREN`.
/// Two Name tokens: the aggregation keyword and the slot it reduces.
fn adapt_agg(node: &GrammarASTNode) -> Result<ExprAst, AdapterError> {
    let names: Vec<&str> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t.value.as_str()),
            _ => None,
        })
        .collect();
    let op =
        names
            .first()
            .and_then(|kw| agg_op_from_keyword(kw))
            .ok_or(AdapterError::MissingChild {
                rule: "agg".into(),
                position: "aggregation keyword",
            })?;
    let slot = names
        .get(1)
        .map(|s| s.to_string())
        .ok_or(AdapterError::MissingChild {
            rule: "agg".into(),
            position: "slot identifier",
        })?;
    Ok(ExprAst::Agg(op, slot))
}

fn arith_op_from_value(v: &str) -> Option<ArithOp> {
    match v {
        "+" => Some(ArithOp::Add),
        "-" => Some(ArithOp::Sub),
        "*" => Some(ArithOp::Mul),
        "/" => Some(ArithOp::Div),
        _ => None,
    }
}

fn agg_op_from_keyword(kw: &str) -> Option<AggOp> {
    match kw {
        "sum" => Some(AggOp::Sum),
        "count" => Some(AggOp::Count),
        "min" => Some(AggOp::Min),
        "max" => Some(AggOp::Max),
        "avg" => Some(AggOp::Avg),
        _ => None,
    }
}

/// Find the first direct child node with the given rule name.
fn first_named_child<'a>(node: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == rule => Some(n),
        _ => None,
    })
}

fn adapt_term(node: &GrammarASTNode) -> Result<Term, AdapterError> {
    if node.rule_name != "term" {
        return Err(AdapterError::UnexpectedRule {
            expected: "term",
            actual: node.rule_name.clone(),
        });
    }
    // term = IDENT [ LPAREN ( term | NUMBER ) { COMMA ( term | NUMBER ) } RPAREN ]
    //
    // The functor is the first (and only) Name token that is a *direct*
    // child — each term argument's own IDENT is nested inside that
    // argument's `term` node. Arguments may be terms or numeric literals
    // (valued facts like `gross_income(18000)`); we walk the children in
    // source order so mixed argument lists keep their positions.
    let functor = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t.value.clone()),
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: "term".into(),
            position: "identifier",
        })?;
    let mut args = Vec::new();
    for c in &node.children {
        match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "term" => args.push(adapt_term(n)?),
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Number => {
                args.push(Term::Num(parse_finite(&t.value, t.type_, "term")?));
            }
            // A `$Enzyme` VAR surfaces as a Name token whose value begins with
            // `$` (unknown token names fall back to TokenType::Name). The functor
            // IDENT never starts with `$`, so it is skipped by this guard and
            // picked up by the functor `find_map` above. Strip the sigil — the
            // AST carries the bare variable name.
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name && t.value.starts_with('$') => {
                args.push(Term::Var(t.value[1..].to_string()));
            }
            _ => {}
        }
    }
    if args.is_empty() {
        Ok(Term::Atom(functor))
    } else {
        Ok(Term::Compound { functor, args })
    }
}

fn adapt_annotation(node: &GrammarASTNode) -> Result<Annotation, AdapterError> {
    if node.rule_name != "annotation" {
        return Err(AdapterError::UnexpectedRule {
            expected: "annotation",
            actual: node.rule_name.clone(),
        });
    }
    let child = first_child_node(node, "annotation", "kind")?;
    match child.rule_name.as_str() {
        "source_annotation" => {
            let value = expect_string_at(child, 1)?;
            Ok(Annotation::Source(value))
        }
        "locator_annotation" => {
            let value = expect_string_at(child, 1)?;
            Ok(Annotation::Locator(value))
        }
        "trust_annotation" => {
            // trust_annotation = "trust" trust_tier
            let tier_node = child
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Node(n) if n.rule_name == "trust_tier" => Some(n),
                    _ => None,
                })
                .ok_or(AdapterError::MissingChild {
                    rule: "trust_annotation".into(),
                    position: "trust_tier rule",
                })?;
            let tier = trust_tier_from_node(tier_node)?;
            Ok(Annotation::Trust(tier))
        }
        "cites_annotation" => {
            // cites_annotation = "cites" STRING "locator" STRING  (ADJ-A9)
            // Two STRING tokens in source order: [source, locator]. The
            // `locator` keyword between them is reused purely as a separator.
            let mut strings = child.children.iter().filter_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == TokenType::String => {
                    Some(unquote_string(&t.value))
                }
                _ => None,
            });
            let source = strings.next().ok_or(AdapterError::MissingChild {
                rule: "cites_annotation".into(),
                position: "source STRING",
            })?;
            let locator = strings.next().ok_or(AdapterError::MissingChild {
                rule: "cites_annotation".into(),
                position: "locator STRING",
            })?;
            Ok(Annotation::Cites { source, locator })
        }
        other => Err(AdapterError::UnexpectedRule {
            expected: "one of source_annotation / locator_annotation / trust_annotation / cites_annotation",
            actual: other.to_string(),
        }),
    }
}

fn trust_tier_from_node(node: &GrammarASTNode) -> Result<TrustTierName, AdapterError> {
    // trust_tier = "consensus" | "authoritative" | "empirical"
    //            | "inferred" | "unattributed"
    //
    // The grammar-driven lexer emits all identifier-shaped tokens
    // (including keywords) with TokenType::Name and the keyword name
    // in `value`. Distinguish by value, not by type. The grammar's
    // alternation has already constrained `value` to one of the five
    // trust-tier keywords; an unknown value here means the grammar
    // and the adapter drifted out of sync.
    let tier_kw = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t.value.clone()),
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: "trust_tier".into(),
            position: "keyword token",
        })?;
    match tier_kw.as_str() {
        "consensus" => Ok(TrustTierName::Consensus),
        "authoritative" => Ok(TrustTierName::Authoritative),
        "empirical" => Ok(TrustTierName::Empirical),
        "inferred" => Ok(TrustTierName::Inferred),
        "unattributed" => Ok(TrustTierName::Unattributed),
        _ => Err(AdapterError::UnknownTrustTier { actual: tier_kw }),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a NUMBER lexeme to `f64`, rejecting non-finite values. Rust's
/// `f64::from_str` accepts overflowing literals like `1e400` (→ `inf`);
/// an infinite or NaN threshold is meaningless in a rulebook, so we
/// reject it here with an accurate diagnostic rather than letting it
/// silently flow downstream.
fn parse_finite(raw: &str, kind: TokenType, rule: &str) -> Result<f64, AdapterError> {
    match raw.parse::<f64>() {
        Ok(x) if x.is_finite() => Ok(x),
        _ => Err(AdapterError::BadToken {
            rule: rule.to_string(),
            kind,
            value: raw.to_string(),
            reason: "not a finite f64",
        }),
    }
}

fn collect_term_children(node: &GrammarASTNode) -> Vec<Term> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "term" => adapt_term(n).ok(),
            _ => None,
        })
        .collect()
}

fn collect_annotations(node: &GrammarASTNode) -> Result<Vec<Annotation>, AdapterError> {
    let mut out = Vec::new();
    for c in &node.children {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == "annotation" {
                out.push(adapt_annotation(n)?);
            }
        }
    }
    Ok(out)
}

fn first_child_node<'a>(
    node: &'a GrammarASTNode,
    parent_rule: &'static str,
    position: &'static str,
) -> Result<&'a GrammarASTNode, AdapterError> {
    node.children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: parent_rule.into(),
            position,
        })
}

fn expect_number_at(node: &GrammarASTNode, skip: usize) -> Result<f64, AdapterError> {
    // The grammar guarantees the NUMBER token is the (skip)-th
    // child relative to the keyword. We linearly scan for the first
    // NUMBER token whose position respects `skip`.
    let mut seen = 0usize;
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.type_ == TokenType::Number {
                if seen == skip.saturating_sub(1) {
                    return t.value.parse::<f64>().map_err(|_| AdapterError::BadToken {
                        rule: node.rule_name.clone(),
                        kind: t.type_,
                        value: t.value.clone(),
                        reason: "not a finite f64",
                    });
                }
                seen += 1;
            }
        }
    }
    Err(AdapterError::MissingChild {
        rule: node.rule_name.clone(),
        position: "NUMBER token",
    })
}

fn expect_string_at(node: &GrammarASTNode, _skip: usize) -> Result<String, AdapterError> {
    // The grammar's source/locator annotations have at most one
    // STRING child. Return its unescaped value verbatim.
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.type_ == TokenType::String {
                return Ok(unquote_string(&t.value));
            }
        }
    }
    Err(AdapterError::MissingChild {
        rule: node.rule_name.clone(),
        position: "STRING token",
    })
}

fn expect_term_child(
    node: &GrammarASTNode,
    parent_rule: &'static str,
) -> Result<Term, AdapterError> {
    let term_node = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "term" => Some(n),
            _ => None,
        })
        .ok_or(AdapterError::MissingChild {
            rule: parent_rule.into(),
            position: "term",
        })?;
    adapt_term(term_node)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn parse_one(src: &str) -> Statement {
        let program = parse(src).expect("parse");
        program
            .statements
            .into_iter()
            .next()
            .expect("at least one statement")
    }

    #[test]
    fn round_trips_prior() {
        match parse_one("prior 0.10 for acs") {
            Statement::Prior {
                probability,
                conclusion,
                annotations,
            } => {
                assert_eq!(probability, 0.10);
                assert!(matches!(conclusion, Term::Atom(n) if n == "acs"));
                assert!(annotations.is_empty());
            }
            other => panic!("expected Prior, got {other:?}"),
        }
    }

    #[test]
    fn round_trips_contributes_with_source_and_trust() {
        let src = r#"contributes 1.5 from pmh(hypertension) to acs
            source "HEART Score"
            trust empirical"#;
        match parse_one(src) {
            Statement::Contributes {
                lr,
                evidence,
                conclusion,
                annotations,
            } => {
                assert_eq!(lr, 1.5);
                assert!(matches!(
                    evidence,
                    Evidence::Term(Term::Compound { ref functor, .. }) if functor == "pmh"
                ));
                assert!(matches!(conclusion, Term::Atom(n) if n == "acs"));
                assert_eq!(annotations.len(), 2);
                assert!(matches!(annotations[0], Annotation::Source(_)));
                assert!(matches!(
                    annotations[1],
                    Annotation::Trust(TrustTierName::Empirical)
                ));
            }
            other => panic!("expected Contributes, got {other:?}"),
        }
    }

    #[test]
    fn round_trips_predicate_evidence() {
        // contributes <lr> from <slot> >= <value> to <verdict>
        let src = "contributes 1000000 from gross_income >= 14600 to required_to_file";
        match parse_one(src) {
            Statement::Contributes {
                lr,
                evidence,
                conclusion,
                ..
            } => {
                assert_eq!(lr, 1_000_000.0);
                match evidence {
                    Evidence::Predicate { slot, op, value } => {
                        assert_eq!(slot, "gross_income");
                        assert_eq!(op, CmpOp::Ge);
                        assert_eq!(value, 14600.0);
                    }
                    other => panic!("expected predicate evidence, got {other:?}"),
                }
                assert!(matches!(conclusion, Term::Atom(n) if n == "required_to_file"));
            }
            other => panic!("expected Contributes, got {other:?}"),
        }
    }

    #[test]
    fn all_five_comparison_operators_round_trip() {
        for (sym, expected) in &[
            (">=", CmpOp::Ge),
            ("<=", CmpOp::Le),
            (">", CmpOp::Gt),
            ("<", CmpOp::Lt),
            ("==", CmpOp::Eq),
        ] {
            let src = format!("contributes 2.0 from age {sym} 18 to adult");
            match parse_one(&src) {
                Statement::Contributes {
                    evidence: Evidence::Predicate { op, value, slot },
                    ..
                } => {
                    assert_eq!(op, *expected, "operator {sym}");
                    assert_eq!(value, 18.0);
                    assert_eq!(slot, "age");
                }
                other => panic!("expected predicate Contributes for {sym}, got {other:?}"),
            }
        }
    }

    #[test]
    fn round_trips_valued_observation() {
        // observe <slot>(<number>) — a valued fact the predicate reads.
        match parse_one("observe gross_income(18000)") {
            Statement::Observe { term } => match term {
                Term::Compound { functor, args } => {
                    assert_eq!(functor, "gross_income");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(args[0], Term::Num(x) if x == 18000.0));
                }
                other => panic!("expected compound, got {other:?}"),
            },
            other => panic!("expected Observe, got {other:?}"),
        }
    }

    #[test]
    fn round_trips_interacts_with_two_evidence() {
        let src = "interacts 1.3 when symptom(a) and symptom(b) for acs";
        match parse_one(src) {
            Statement::Interacts {
                lr,
                evidence_set,
                conclusion,
                ..
            } => {
                assert_eq!(lr, 1.3);
                assert_eq!(evidence_set.len(), 2);
                assert!(matches!(conclusion, Term::Atom(n) if n == "acs"));
            }
            other => panic!("expected Interacts, got {other:?}"),
        }
    }

    #[test]
    fn line_comments_are_skipped() {
        let src = "% leading comment\n? acs";
        let stmt = parse_one(src);
        assert!(matches!(stmt, Statement::Query { .. }));
    }

    #[test]
    fn round_trips_uncertain_with_domain_and_annotations() {
        // Regression guard for the grammar-driven re-introduction of
        // `uncertain { e1, e2, ... } for <conclusion>`. The
        // hand-written 0.2.0 supported this; #4945's 0.3.0 had to
        // re-add it to the .grammar / .tokens / adapter triple.
        let src = r#"
            uncertain { precipitator(exertional),
                        precipitator(rest),
                        precipitator(positional) } for acs
              source "patient did not specify"
        "#;
        match parse_one(src) {
            Statement::Uncertain {
                domain,
                conclusion,
                annotations,
            } => {
                assert_eq!(domain.len(), 3);
                assert!(matches!(conclusion, Term::Atom(n) if n == "acs"));
                assert_eq!(annotations.len(), 1);
                assert!(matches!(annotations[0], Annotation::Source(_)));
            }
            other => panic!("expected Uncertain, got {other:?}"),
        }
    }

    #[test]
    fn all_five_trust_tier_keywords_round_trip() {
        for (kw, expected) in &[
            ("consensus", TrustTierName::Consensus),
            ("authoritative", TrustTierName::Authoritative),
            ("empirical", TrustTierName::Empirical),
            ("inferred", TrustTierName::Inferred),
            ("unattributed", TrustTierName::Unattributed),
        ] {
            let src = format!("contributes 1.0 from x to y trust {kw}");
            match parse_one(&src) {
                Statement::Contributes { annotations, .. } => match &annotations[0] {
                    Annotation::Trust(t) => assert_eq!(t, expected),
                    other => panic!("expected Trust, got {other:?}"),
                },
                other => panic!("expected Contributes, got {other:?}"),
            }
        }
    }

    // ---- string-literal escape handling (load-bearing for byte-provenance) ----
    //
    // `unquote_string` turns the raw lexeme (quotes included) into the semantic
    // value. These tests pin every row of the escape table so a `source "..."`
    // span that itself contains a quote, backslash, newline, or tab is restored
    // character-for-character — and so an escape we don't recognize is never
    // silently altered. The raw lexemes are written as Rust raw strings
    // (`r#"..."#`) so the backslashes below are literal, exactly as the ADJ
    // lexer would see them.

    #[test]
    fn unquote_string_strips_quotes_on_a_plain_literal() {
        assert_eq!(unquote_string(r#""plain text""#), "plain text");
    }

    #[test]
    fn unquote_string_restores_an_escaped_double_quote() {
        // ADJ source:  "shows \"Orphan Annie eye\" nuclei"
        // -> verbatim:  shows "Orphan Annie eye" nuclei
        assert_eq!(
            unquote_string(r#""shows \"Orphan Annie eye\" nuclei""#),
            "shows \"Orphan Annie eye\" nuclei"
        );
    }

    #[test]
    fn unquote_string_restores_backslash_newline_and_tab() {
        assert_eq!(unquote_string(r#""a\\b""#), "a\\b"); // \\  -> \
        assert_eq!(unquote_string(r#""l1\nl2""#), "l1\nl2"); // \n  -> newline
        assert_eq!(unquote_string(r#""c1\tc2""#), "c1\tc2"); // \t  -> tab
    }

    #[test]
    fn unquote_string_keeps_an_unrecognized_escape_verbatim() {
        // We never drop the backslash on an escape we don't understand —
        // mutating a citation we can't interpret would corrupt provenance.
        assert_eq!(unquote_string(r#""a\xb""#), r"a\xb");
    }

    #[test]
    fn unquote_string_keeps_a_dangling_backslash() {
        // Defensive: the real lexer can't emit this (its regex requires a char
        // after `\` plus a closing quote), but the unescaper must not panic or
        // eat a trailing backslash sitting just inside the closing quote.
        // Raw lexeme is the 5 chars  " a b \ "  -> inner "ab\" -> value "ab\".
        assert_eq!(unquote_string("\"ab\\\""), "ab\\");
    }

    #[test]
    fn unquote_latex_string_preserves_latex_commands() {
        assert_eq!(unquote_latex_string(r#""$5 \times 12$""#), r"$5 \times 12$");
        assert_eq!(unquote_latex_string(r#""\frac{12}{3}""#), r"\frac{12}{3}");
        assert_eq!(unquote_latex_string(r#""quote \" ok""#), "quote \" ok");
        assert_eq!(unquote_latex_string(r#""\\times""#), r"\times");
    }

    #[test]
    fn source_annotation_carries_an_escaped_quote_verbatim() {
        // End-to-end through the lexer + parser + adapter: a provenance span
        // containing a literal double quote survives as real bytes.
        let src = r#"contributes 1.0 from x to y
            source "shows \"Orphan Annie eye\" nuclei and psammoma bodies"
            trust authoritative"#;
        match parse_one(src) {
            Statement::Contributes { annotations, .. } => {
                let source = annotations
                    .iter()
                    .find_map(|a| match a {
                        Annotation::Source(s) => Some(s.clone()),
                        _ => None,
                    })
                    .expect("a Source annotation");
                assert_eq!(
                    source,
                    "shows \"Orphan Annie eye\" nuclei and psammoma bodies"
                );
                assert!(source.contains('"'), "the span must carry a real quote");
            }
            other => panic!("expected Contributes, got {other:?}"),
        }
    }
}

/// Strip surrounding double quotes and process backslash escapes.
///
/// The grammar-driven lexer matches a string with `/"([^"\\]|\\.)*"/` and
/// hands us the raw lexeme *including* the outer `"`. We strip the quotes and
/// translate the recognized escape sequences:
///
/// | in source | becomes  | why it matters                                   |
/// |-----------|----------|--------------------------------------------------|
/// | `\"`      | `"`      | a verbatim span may itself contain a quote (e.g. |
/// |           |          | a histology page's `"Orphan Annie eye"` nuclei)  |
/// | `\\`      | `\`      | a literal backslash                              |
/// | `\n`      | newline  | multi-line provenance text                       |
/// | `\t`      | tab      | tabular provenance text                          |
///
/// This is load-bearing for byte-provenance: a `source "..."` annotation must
/// reproduce the cited page's text *character-for-character* after unescaping,
/// so a span that contains a `"` is carried as `\"` and restored here. An
/// unrecognized escape (`\x`) is kept verbatim (`\x`) rather than silently
/// dropping the backslash — we never want to mutate a citation we don't
/// understand. See the `unquote_string_*` unit tests, which pin every row.
fn unquote_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(esc) = chars.next() {
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    other => {
                        // Unknown escape — keep verbatim.
                        out.push('\\');
                        out.push(other);
                    }
                }
            } else {
                // Dangling backslash at end of string — keep it verbatim.
                out.push('\\');
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Strip surrounding double quotes for `latex "..."` strings.
///
/// Unlike provenance strings, LaTeX strings must preserve command backslashes:
/// `\times` and `\frac` are math syntax, not `\t`/`\f` escapes. Only quote and
/// backslash escaping are interpreted here; every other backslash sequence is
/// passed through to the LaTeX parser verbatim.
fn unquote_latex_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}
