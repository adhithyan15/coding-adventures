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
use math_frontend::{BigOp, BinOp, Func, MathExpr, Number, RelOp as MathRelOp, UnaryOp};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::ast::{
    AggOp, Annotation, ArithOp, BinFn, CmpOp, Define, DefineKind, Evidence, ExprAst, FormulaDef,
    NamedFn, OptDir, Program, RelOp, RuleLiteral, Statement, Term, TrustTierName,
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
    /// An `asciimath "<math>"` expression could not be parsed by the repo's
    /// AsciiMath MathFrontend. AsciiMath is a SECOND math frontend that lowers
    /// through the same neutral `MathExpr` pipeline as `latex`; only the parse
    /// step is frontend-specific, so this is its counterpart to
    /// [`AdapterError::LatexParse`]. Once parsed, an unsupported node still
    /// surfaces as [`AdapterError::UnsupportedLatexMath`] — the shared
    /// neutral-tree lowering names its errors after that first frontend.
    AsciiMathParse { source: String, detail: String },
    /// A `mathml "<math>"` expression could not be parsed by the repo's MathML
    /// MathFrontend. MathML is a THIRD math frontend that lowers through the same
    /// neutral `MathExpr` pipeline as `latex`/`asciimath`; only the parse step is
    /// frontend-specific, so this is its counterpart to
    /// [`AdapterError::LatexParse`]. Once parsed, an unsupported node still
    /// surfaces as [`AdapterError::UnsupportedLatexMath`] — the shared
    /// neutral-tree lowering names its errors after that first frontend.
    MathMlParse { source: String, detail: String },
    /// A `unicodemath "<math>"` expression could not be parsed by the repo's
    /// Unicode-math MathFrontend. Unicode-math is a FOURTH math frontend that
    /// lowers through the same neutral `MathExpr` pipeline as
    /// `latex`/`asciimath`/`mathml`; only the parse step is frontend-specific, so
    /// this is its counterpart to [`AdapterError::LatexParse`]. Once parsed, an
    /// unsupported node still surfaces as [`AdapterError::UnsupportedLatexMath`] —
    /// the shared neutral-tree lowering names its errors after that first frontend.
    UnicodeMathParse { source: String, detail: String },
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
        "constrain_asciimath_decl" => adapt_constrain_asciimath(child),
        "constrain_decl" => adapt_constrain(child),
        "solve_decl" => adapt_solve(child),
        "check_decl" => Ok(Statement::Check),
        "optimize_decl" => adapt_optimize(child),
        "dictionary_decl" => adapt_dictionary(child),
        "define_decl" => adapt_define(child).map(Statement::Define),
        "rulebook_decl" => adapt_rulebook(child),
        "formulabook_decl" => adapt_formulabook(child),
        "use_decl" => adapt_use(child),
        "import_decl" => adapt_import(child),
        other => Err(AdapterError::UnexpectedRule {
            expected: "one of prior_decl / contributes_decl / interacts_decl / uncertain_decl / observe_decl / query_decl / let_decl / symbol_decl / constrain_latex_decl / constrain_asciimath_decl / constrain_decl / solve_decl / check_decl / optimize_decl / dictionary_decl / define_decl / rulebook_decl / use_decl / import_decl",
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
    // predicate = IDENT ( GE | LE | GT | LT | EQEQ ) expr
    //
    // The slot IDENT and the operator are both lexed as TokenType::Name
    // (the operator carries a custom type_name like "GE"), so we
    // distinguish by *value*: the operator's value is one of the five
    // comparison symbols; everything else Name-typed is the slot.
    let mut slot: Option<String> = None;
    let mut op: Option<CmpOp> = None;
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            match t.value.as_str() {
                ">=" => op = Some(CmpOp::Ge),
                "<=" => op = Some(CmpOp::Le),
                ">" => op = Some(CmpOp::Gt),
                "<" => op = Some(CmpOp::Lt),
                "==" => op = Some(CmpOp::Eq),
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
    let rhs = if let Some(expr) = first_named_child(node, "expr") {
        adapt_expr(expr)?
    } else {
        // Bootstrap compatibility while regenerating from the previous grammar,
        // whose predicate RHS was a direct NUMBER token.
        node.children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == TokenType::Number => {
                    Some(parse_finite(&t.value, t.type_, "predicate").map(ExprAst::Lit))
                }
                _ => None,
            })
            .transpose()?
            .ok_or(AdapterError::MissingChild {
                rule: "predicate".into(),
                position: "right-hand expression",
            })?
    };
    Ok(Evidence::Predicate { slot, op, rhs })
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
    if idents.is_empty() || !idents.len().is_multiple_of(2) {
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
    let MathExpr::Rel(op, lhs, rhs) = &math else {
        return Err(AdapterError::UnsupportedLatexMath {
            source,
            detail: "expected a LaTeX relation such as x^2 = 4".into(),
        });
    };
    let op = lower_latex_relop(*op, &source)?;
    Ok(Statement::Constrain {
        lhs: latex_math_to_expr_ast(lhs, &source)?,
        op,
        rhs: latex_math_to_expr_ast(rhs, &source)?,
    })
}

/// Lower `constrain asciimath "x^2 = 4"` to a `Statement::Constrain`.
///
/// This is the exact mirror of [`adapt_constrain_latex`]: the ONLY difference is
/// which frontend parses the string. The AsciiMath frontend yields the SAME
/// neutral `MathExpr::Rel(op, lhs, rhs)`, so the relation operator lowers through
/// the same `lower_latex_relop` and both sides through the same
/// `latex_math_to_expr_ast` used by every other math surface. No new relation
/// semantics, no new tree-walker — one constraint code path, one frontend swap.
fn adapt_constrain_asciimath(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // constrain_asciimath_decl = "constrain" asciimath_relation
    // asciimath_relation = "asciimath" STRING
    let relation =
        first_named_child(node, "asciimath_relation").ok_or(AdapterError::MissingChild {
            rule: "constrain_asciimath_decl".into(),
            position: "asciimath relation",
        })?;
    let source = latex_string_from_node(relation, "asciimath_relation")?;
    let math = parse_asciimath_math(&source)?;
    let MathExpr::Rel(op, lhs, rhs) = &math else {
        return Err(AdapterError::UnsupportedLatexMath {
            source,
            detail: "expected an AsciiMath relation such as x^2 = 4".into(),
        });
    };
    let op = lower_latex_relop(*op, &source)?;
    Ok(Statement::Constrain {
        lhs: latex_math_to_expr_ast(lhs, &source)?,
        op,
        rhs: latex_math_to_expr_ast(rhs, &source)?,
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

fn adapt_formulabook(node: &GrammarASTNode) -> Result<Statement, AdapterError> {
    // formulabook_decl = "formulabook" IDENT LBRACE { formulabook_item } RBRACE
    // The name is the first Name token that isn't the `formulabook` keyword; each
    // `formulabook_item` wraps either a `use_decl` (a vocabulary binding) or a
    // `formula_decl` (a definition).
    let name = first_name_not(node, "formulabook")
        .ok_or(AdapterError::MissingChild {
            rule: "formulabook_decl".into(),
            position: "formulabook name",
        })?
        .to_string();
    let mut uses = Vec::new();
    let mut formulas = Vec::new();
    for c in &node.children {
        if let ASTNodeOrToken::Node(item) = c {
            if item.rule_name == "formulabook_item" {
                let inner = first_child_node(item, "formulabook_item", "use_decl or formula_decl")?;
                match inner.rule_name.as_str() {
                    "use_decl" => {
                        if let Statement::Use(u) = adapt_use(inner)? {
                            uses.push(u);
                        }
                    }
                    "formula_decl" => formulas.push(adapt_formula(inner)?),
                    other => {
                        return Err(AdapterError::UnexpectedRule {
                            expected: "use_decl or formula_decl",
                            actual: other.to_string(),
                        })
                    }
                }
            }
        }
    }
    Ok(Statement::Formulabook {
        name,
        uses,
        formulas,
    })
}

fn adapt_formula(node: &GrammarASTNode) -> Result<FormulaDef, AdapterError> {
    // formula_decl = "formula" IDENT LPAREN [ formula_params ] RPAREN EQUALS expr { annotation }
    //
    // The name is the first Name token that isn't the `formula` keyword (the
    // parameter Name tokens live INSIDE the nested `formula_params` node, so they
    // are not direct children and cannot be mistaken for the name).
    let name = first_name_not(node, "formula")
        .ok_or(AdapterError::MissingChild {
            rule: "formula_decl".into(),
            position: "formula name",
        })?
        .to_string();
    // formula_params = IDENT { COMMA IDENT } — the parameter names are the Name
    // tokens of the `formula_params` child (COMMA is a punctuation token). Absent
    // (a zero-parameter formula) yields an empty vector.
    let params = first_named_child(node, "formula_params")
        .map(|p| {
            p.children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t.value.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    // The body reuses the EXISTING `let` expression grammar/adapter verbatim.
    let expr_node = first_named_child(node, "expr").ok_or(AdapterError::MissingChild {
        rule: "formula_decl".into(),
        position: "body expr",
    })?;
    let body = adapt_expr(expr_node)?;
    // Same provenance envelope as every grounded clause (`{ annotation }`).
    let annotations = collect_annotations(node)?;
    Ok(FormulaDef {
        name,
        params,
        body,
        annotations,
    })
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
        return latex_math_to_expr_ast(&math, &source);
    }
    // A SECOND math frontend on the same factor position: `asciimath "..."`.
    // Only the parse step differs — the resulting neutral `MathExpr` flows
    // through the identical `latex_math_to_expr_ast` lowering, so the whole
    // arithmetic + named-function surface is available for free (PFE01).
    if let Some(am) = first_named_child(node, "asciimath_expr") {
        let source = latex_string_from_node(am, "asciimath_expr")?;
        let math = parse_asciimath_math(&source)?;
        return latex_math_to_expr_ast(&math, &source);
    }
    // A THIRD math frontend on the same factor position: `mathml "<...>"`.
    // Same story — only the parse step differs; the neutral `MathExpr` flows
    // through the identical `latex_math_to_expr_ast` lowering (PFE01).
    if let Some(mm) = first_named_child(node, "mathml_expr") {
        let source = latex_string_from_node(mm, "mathml_expr")?;
        let math = parse_mathml_math(&source)?;
        return latex_math_to_expr_ast(&math, &source);
    }
    // A FOURTH math frontend on the same factor position: `unicodemath "<...>"`.
    // Same story — only the parse step differs; the neutral `MathExpr` flows
    // through the identical `latex_math_to_expr_ast` lowering (PFE01).
    if let Some(um) = first_named_child(node, "unicodemath_expr") {
        let source = latex_string_from_node(um, "unicodemath_expr")?;
        let math = parse_unicodemath_math(&source)?;
        return latex_math_to_expr_ast(&math, &source);
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

/// Parse an `asciimath "..."` string with the repo's AsciiMath `MathFrontend`.
///
/// This is the AsciiMath counterpart to [`parse_latex_math`] — the ONLY
/// frontend-specific step in the pipeline. Its output is the same neutral
/// [`MathExpr`] the LaTeX frontend produces, which the caller lowers with the
/// shared [`latex_math_to_expr_ast`]. (We strip `$…$`-style math delimiters
/// first, harmlessly: AsciiMath does not use them, but a model may wrap either
/// dialect in them, and stripping keeps the two surfaces symmetric.)
///
/// The AsciiMath parser lives in its own crate and owns its own recursion/DoS
/// discipline; this adapter adds no new tree-walker, so it introduces no new
/// stack-overflow surface here.
fn parse_asciimath_math(source: &str) -> Result<MathExpr, AdapterError> {
    use math_frontend::MathFrontend;
    let math = strip_math_delimiters(source);
    asciimath::AsciiMath
        .parse(math)
        .map_err(|e| AdapterError::AsciiMathParse {
            source: source.to_string(),
            detail: e.message,
        })
}

/// Parse a `mathml "..."` string with the repo's MathML `MathFrontend`.
///
/// The MathML counterpart to [`parse_latex_math`]/[`parse_asciimath_math`] — the
/// only frontend-specific step. Its output is the same neutral [`MathExpr`] the
/// LaTeX frontend produces, lowered by the shared [`latex_math_to_expr_ast`].
/// (We reuse `strip_math_delimiters` for symmetry with the other two surfaces;
/// presentation MathML never carries `$…$`-style delimiters, so it is a no-op.)
///
/// The MathML parser lives in its own crate with its own recursion guard
/// (`MAX_DEPTH`) and `#![forbid(unsafe_code)]`; this adapter adds no new
/// tree-walker, so it introduces no new stack-overflow surface here.
fn parse_mathml_math(source: &str) -> Result<MathExpr, AdapterError> {
    use math_frontend::MathFrontend;
    let math = strip_math_delimiters(source);
    mathml::MathMl
        .parse(math)
        .map_err(|e| AdapterError::MathMlParse {
            source: source.to_string(),
            detail: e.message,
        })
}

/// Parse a `unicodemath "..."` string with the repo's Unicode-math `MathFrontend`.
///
/// The Unicode-math counterpart to the other `parse_*_math` helpers — the only
/// frontend-specific step. Its output is the same neutral [`MathExpr`] the LaTeX
/// frontend produces, lowered by the shared [`latex_math_to_expr_ast`]. (We reuse
/// `strip_math_delimiters` for symmetry with the sibling surfaces; raw Unicode
/// math never carries `$…$`-style delimiters, so it is a no-op.)
///
/// The Unicode-math parser lives in its own crate with its own recursion guard
/// (`MAX_DEPTH`) and `#![forbid(unsafe_code)]`; this adapter adds no new
/// tree-walker, so it introduces no new stack-overflow surface here.
fn parse_unicodemath_math(source: &str) -> Result<MathExpr, AdapterError> {
    use math_frontend::MathFrontend;
    let math = strip_math_delimiters(source);
    unicode_math::UnicodeMath
        .parse(math)
        .map_err(|e| AdapterError::UnicodeMathParse {
            source: source.to_string(),
            detail: e.message,
        })
}

// The `\(…\)`, `\[…\]` and `$$…$$` branches intentionally share the same body
// (`&s[2..len-2]`): they are distinct delimiter pairs of equal width, kept
// separate for readability rather than merged into one `||` condition.
#[allow(clippy::if_same_then_else)]
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

// Takes `&MathExpr` (not by value): the neutral `MathExpr` now implements `Drop` (so it is
// safe to free at any depth), and a type with an explicit `Drop` cannot have its fields moved
// out of a `match`. Borrowing and recursing on the references — cloning only the small leaves
// we keep — is the idiomatic way to consume it, and avoids E0509.
fn latex_math_to_expr_ast(expr: &MathExpr, source: &str) -> Result<ExprAst, AdapterError> {
    match expr {
        MathExpr::Number(n) => number_to_lit(n, source),
        MathExpr::Symbol(name) => Ok(ExprAst::Ref(name.clone())),
        // `x_i` / `x_1` / `V_{max}` — a SUBSCRIPTED variable. A subscript does not compute:
        // `x_1` and `x_2` are two DISTINCT observed quantities, and `V_{max}` / `C_{peak}` are
        // named readings, not `V` times something. So the adapter mangles the subscript into a
        // single flat identifier `base_sub` (`x_i` -> `Ref("x_i")`, `V_{max}` -> `Ref("V_max")`),
        // which binds to a matching `observe x_i(...)` / `observe V_max(...)` exactly like any
        // underscored name (`tidal_volume`). The frontend parses the subscript body per its own
        // rules — a single letter/number is a `Symbol`/`Number`, but a BRACED multi-letter
        // subscript `{max}` becomes a juxtaposition chain of single-letter `Symbol`s
        // (`Bin(Mul, Bin(Mul, m, a), x)`) — so `subscript_ident_part` flattens either shape back
        // into the word it spells. Anything the helper cannot name (e.g. an arithmetic subscript
        // `x_{i+1}`) returns an explicit `UnsupportedLatexMath` — never a silent mis-binding.
        MathExpr::Subscript(base, sub) => {
            let base_name = subscript_ident_part(base).ok_or_else(|| {
                AdapterError::UnsupportedLatexMath {
                    source: source.to_string(),
                    detail: "subscript base is not a plain identifier".to_string(),
                }
            })?;
            let sub_name = subscript_ident_part(sub).ok_or_else(|| {
                AdapterError::UnsupportedLatexMath {
                    source: source.to_string(),
                    detail: "subscript index is not a plain identifier or number".to_string(),
                }
            })?;
            Ok(ExprAst::Ref(format!("{base_name}_{sub_name}")))
        }
        MathExpr::Group(inner) => latex_math_to_expr_ast(inner, source),
        // A delimited group (`(x)`, `[x]`, `|x|`) unwraps the same as a plain group for arithmetic
        // lowering — the fence delimiters are presentation, not an operation. (latex now lowers a
        // single-body fence to `Fenced` carrying its delimiters instead of the delimiter-less
        // `Group`; this arm keeps the arithmetic meaning identical.)
        // A `|x|` / `\left|x\right|` fence is an ABSOLUTE VALUE — it lowers to the
        // native `ComputeOp::Abs` (dimension-preserving), NOT to a bare `x`. Any
        // OTHER delimiter pair (`(x)`, `[x]`, `\langle x\rangle`) is presentation
        // grouping and unwraps to the body's arithmetic, exactly as before. (Before
        // this arm, `|x|` silently dropped its bars and computed `x` — a quiet
        // wrong answer; now the absolute value is honoured.)
        MathExpr::Fenced { open, body, close } if open == "|" && close == "|" => Ok(ExprAst::Abs(
            Box::new(latex_math_to_expr_ast(body, source)?),
        )),
        // A `\left\lfloor x\right\rfloor` fence is a FLOOR, `\left\lceil x\right\rceil`
        // a CEILING — each lowers to its native dimension-preserving unary op, NOT to
        // a bare `x`. The latex frontend carries the control-word delimiters verbatim
        // (`\lfloor`/`\rfloor`, `\lceil`/`\rceil`), exactly as it carries `|` for abs,
        // so we match on those delimiter strings. Any OTHER pair falls through to the
        // presentation-grouping arm below.
        MathExpr::Fenced { open, body, close } if open == "\\lfloor" && close == "\\rfloor" => {
            Ok(ExprAst::Floor(Box::new(latex_math_to_expr_ast(body, source)?)))
        }
        MathExpr::Fenced { open, body, close } if open == "\\lceil" && close == "\\rceil" => {
            Ok(ExprAst::Ceil(Box::new(latex_math_to_expr_ast(body, source)?)))
        }
        // The standard NEAREST-INTEGER fence is asymmetric — floor on the left,
        // ceiling on the right — `\left\lfloor x\right\rceil`. It rounds to the
        // nearest integer (ties away from zero) and lowers to `ComputeOp::Round`.
        // Its distinct closing delimiter (`\rceil`, not `\rfloor`) keeps it separate
        // from the plain floor arm above.
        MathExpr::Fenced { open, body, close } if open == "\\lfloor" && close == "\\rceil" => {
            Ok(ExprAst::Round(Box::new(latex_math_to_expr_ast(body, source)?)))
        }
        MathExpr::Fenced { body, .. } => latex_math_to_expr_ast(body, source),
        MathExpr::Unary(UnaryOp::Pos, inner) => latex_math_to_expr_ast(inner, source),
        MathExpr::Unary(UnaryOp::Neg, inner) => Ok(ExprAst::Bin(
            ArithOp::Sub,
            Box::new(ExprAst::Lit(0.0)),
            Box::new(latex_math_to_expr_ast(inner, source)?),
        )),
        MathExpr::Bin(BinOp::Add, lhs, rhs) => latex_bin(ArithOp::Add, lhs, rhs, source),
        MathExpr::Bin(BinOp::Sub, lhs, rhs) => latex_bin(ArithOp::Sub, lhs, rhs, source),
        // `\operatorname{trunc}(x)` — truncation toward zero. `\operatorname{…}` is a
        // TEXT command (like `\text`/`\mathrm`), so the frontend parses
        // `\operatorname{trunc}(x)` NOT as a function call but as an *implicit
        // multiplication* (juxtaposition) of the operator name `Text("trunc")` and its
        // parenthesised argument: `Bin(Mul, Text("trunc"), (x))`. We recognise that
        // exact shape here — the operator-name juxtaposition path — and lower it to the
        // dimension-preserving `ComputeOp::Trunc` (via `ExprAst::Trunc`). This arm sits
        // ABOVE the general `Bin(Mul, …)` so a genuine product (`2x`) still multiplies;
        // only a `trunc`-named text factor is intercepted, and only when it is the LEFT
        // operand of the juxtaposition (`\operatorname{trunc}(x)` itself). Anything else
        // named (`\operatorname{sgn}(x)`) falls through to the general Mul arm, where the
        // bare `Text` factor is an explicit `UnsupportedLatexMath` — never a mis-lowering.
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "trunc") => {
            Ok(ExprAst::Trunc(Box::new(latex_math_to_expr_ast(rhs, source)?)))
        }
        // `\operatorname{sgn}(x)` — the sign function. Same operator-name-juxtaposition
        // shape as `\operatorname{trunc}` above (`\operatorname{…}` is a TEXT command,
        // so `\operatorname{sgn}(x)` parses as `Bin(Mul, Text("sgn"), (x))`). Lowers to
        // the native `ComputeOp::Sign` (via `ExprAst::Sign`), which collapses the result
        // to a dimensionless `Scalar` (a sign is ±1/0) while accepting a dimensioned
        // operand — so `\operatorname{sgn}(a - b)` (the sign of a net quantity) computes.
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "sgn") => {
            Ok(ExprAst::Sign(Box::new(latex_math_to_expr_ast(rhs, source)?)))
        }
        // `\operatorname{floor}(x)` / `\operatorname{ceil}(x)` / `\operatorname{round}(x)` —
        // the word-spelled roundings. These are the operator-name twins of the Unicode-
        // bracket forms already lowered elsewhere (`⌊x⌋` → `ExprAst::Floor`, `⌈x⌉` →
        // `ExprAst::Ceil`, `⌊x⌉` → `ExprAst::Round`): a model that writes the *name* instead
        // of the bracket should reach the SAME `ComputeOp`. `\operatorname{…}` is a TEXT
        // command, so — exactly like `\operatorname{trunc}`/`\operatorname{sgn}` above —
        // `\operatorname{floor}(x)` parses as the juxtaposition `Bin(Mul, Text("floor"), (x))`.
        // We intercept that shape here, ABOVE the general `Bin(Mul, …)` product arm, so a
        // genuine product (`2x`) still multiplies; only a `floor`/`ceil`/`round`-named text
        // LEFT factor is captured, and each maps to its existing dimension-preserving
        // `ExprAst` node (no engine/AST/lowering change — pure adapter recognition).
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "floor") => {
            Ok(ExprAst::Floor(Box::new(latex_math_to_expr_ast(rhs, source)?)))
        }
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "ceil") => {
            Ok(ExprAst::Ceil(Box::new(latex_math_to_expr_ast(rhs, source)?)))
        }
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "round") => {
            Ok(ExprAst::Round(Box::new(latex_math_to_expr_ast(rhs, source)?)))
        }
        // `\operatorname{abs}(x)` / `\operatorname{exp}(x)` / `\operatorname{log}(x)` /
        // `\operatorname{ln}(x)` — the operator-name spellings of single-argument unary
        // functions that ALREADY have a native op. `\exp`/`\ln`/`\log` lower in the `Call`
        // arm below (`Func::Exp`/`Ln`/`Log` → the matching `NamedFn`), and `|x|` lowers to
        // `ExprAst::Abs` via the absolute-value `Fenced` arm; a model that writes the
        // operator *name* — `\operatorname{abs}(x)`, `\operatorname{exp}(x)` — should reach
        // the SAME node. But `\operatorname{…}` is a TEXT command, so — exactly like the
        // `\operatorname{floor}`/`sgn`/`trunc` roundings above — these parse NOT as a `Call`
        // but as the operator-name juxtaposition `Bin(Mul, Text("exp"), (x))`. We recognise
        // that shape here, ABOVE the general product arm, and lower to the existing node:
        // `abs` → `ExprAst::Abs` (dimension-preserving), `exp`/`log`/`ln` →
        // `ExprAst::Call(NamedFn::…)` (transcendental, Scalar→Scalar). Pure adapter
        // recognition: no engine, AST, or lowering change.
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "abs") => {
            Ok(ExprAst::Abs(Box::new(latex_math_to_expr_ast(rhs, source)?)))
        }
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "exp") => {
            Ok(ExprAst::Call(
                NamedFn::Exp,
                Box::new(latex_math_to_expr_ast(rhs, source)?),
            ))
        }
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "log") => {
            Ok(ExprAst::Call(
                NamedFn::Log,
                Box::new(latex_math_to_expr_ast(rhs, source)?),
            ))
        }
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "ln") => {
            Ok(ExprAst::Call(
                NamedFn::Ln,
                Box::new(latex_math_to_expr_ast(rhs, source)?),
            ))
        }
        // `\operatorname{sin}(x)` / `\operatorname{cos}(x)` / … — the operator-name spellings
        // of the whole trigonometric family (direct sin/cos/tan/cot/sec/csc, inverse
        // asin/acos/atan and their `arc…` aliases, hyperbolic sinh/cosh/tanh). The
        // backslash-macro spellings (`\sin(x)`, `\arctan(x)`) already lower in the `Call`
        // arm below via `Func::Sin`/`Atan`/…; a model that writes the operator *name* should
        // reach the SAME native `NamedFn`. But `\operatorname{…}` is a TEXT command, so —
        // exactly like `\operatorname{exp}`/`floor`/`sgn` above — `\operatorname{sin}(x)`
        // parses NOT as a `Call` but as the operator-name juxtaposition
        // `Bin(Mul, Text("sin"), (x))`. One consolidated arm handles the family: the
        // `operator_name_trig_fn` helper maps the text name (trimmed) to its `NamedFn`, and
        // we lower to `ExprAst::Call` (transcendental, Scalar→Scalar) — the same node the
        // macro path produces. Pure adapter recognition: no engine, AST, or lowering change.
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_trig_fn(lhs).is_some() => {
            let named = operator_name_trig_fn(lhs).expect("guard guarantees Some");
            Ok(ExprAst::Call(
                named,
                Box::new(latex_math_to_expr_ast(rhs, source)?),
            ))
        }
        // `\operatorname{min}(a, b)` / `\operatorname{max}(…)` / `\operatorname{gcd}(…)` /
        // `\operatorname{lcm}(…)` — the operator-name spellings of the variadic binary
        // functions. The function-call spellings (`\min(a, b)`, `\gcd(a, b, c)`) already
        // lower in the `Call` arm below via `Func::Min`/`Max`/`Gcd`/`Lcm`; a model that
        // writes `\operatorname{gcd}` instead of `\gcd` should reach the SAME native op.
        // But `\operatorname{…}` is a TEXT command, so — exactly like the single-argument
        // `\operatorname{floor}`/`\operatorname{sgn}` above — `\operatorname{gcd}(a, b)`
        // does NOT parse as a `Call`; it parses as the juxtaposition
        // `Bin(Mul, Text("gcd"), (a, b))`, whose right factor is the parenthesised
        // comma-list. We recognise that exact shape here, ABOVE the general product arm, and
        // reuse the SAME `latex_nary_fold` that the `Call` arm uses: the argument sequence
        // left-folds into a chain of the binary `Call2` op (`gcd(a, b, c)` →
        // `gcd(gcd(a, b), c)`), so no engine/AST/lowering change — pure adapter recognition.
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "min") => {
            latex_nary_fold(rhs, source, BinFn::Min, "min")
        }
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "max") => {
            latex_nary_fold(rhs, source, BinFn::Max, "max")
        }
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "gcd") => {
            latex_nary_fold(rhs, source, BinFn::Gcd, "gcd")
        }
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if operator_name_is(lhs, "lcm") => {
            latex_nary_fold(rhs, source, BinFn::Lcm, "lcm")
        }
        // `a \bmod b` / `a \pmod{b}` — the modulo operator. `\bmod`/`\pmod` are not in
        // the frontend's operator tables, so they lower to a bare `Symbol("bmod")` /
        // `Symbol("pmod")` and the whole expression parses as a LEFT-associated implicit
        // multiplication (juxtaposition): `a \bmod b` → `Bin(Mul, Bin(Mul, a, bmod), b)`.
        // We recognise that exact shape — the operator-name-juxtaposition path, just like
        // `\operatorname{trunc}(x)` above — and lower it to `ArithOp::Mod` (→
        // `ComputeOp::Mod`): `real_lhs mod rhs`. This arm sits ABOVE the general
        // `Bin(Mul, …)` so a genuine product (`2x`) still multiplies; only the
        // `bmod`/`pmod` marker as the RIGHT factor of the LEFT operand is intercepted.
        // (The congruence form `x \equiv y \pmod{n}` parses as a `Rel(Equiv, …)`, which
        // the ADJ arithmetic subset rejects — so only the direct `a \bmod b` /
        // `a \pmod{b}` remainder computes, never a mis-lowered congruence.)
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if mod_juxtaposition_lhs(lhs).is_some() => {
            let real_lhs = mod_juxtaposition_lhs(lhs).expect("guard checked Some");
            Ok(ExprAst::Bin(
                ArithOp::Mod,
                Box::new(latex_math_to_expr_ast(real_lhs, source)?),
                Box::new(latex_math_to_expr_ast(rhs, source)?),
            ))
        }
        // `\coth(x)` / `\sech(x)` / `\csch(x)` — the reciprocal hyperbolic functions. The frontend
        // has `Func` variants for `\sinh`/`\cosh`/`\tanh` (they lower in the `Call` arm) but NOT for
        // their reciprocals, so `\coth` is an UNKNOWN control sequence: it lowers to a bare
        // `Symbol("coth")` and the whole `\coth(x)` parses as the operator-name juxtaposition
        // `Bin(Mul, Symbol("coth"), (x))` — exactly the shape `\operatorname{trunc}(x)` /
        // `\operatorname{sinh}(x)` take (those arrive as `Bin(Mul, Text("…"), (x))`). A bare
        // `Symbol("coth")` can ONLY come from the `\coth` macro (plain `coth` in math mode is the
        // product `c·o·t·h`), so matching it is unambiguous. There is no dedicated engine op, but
        // each reciprocal hyperbolic is the EXACT reciprocal of a hyperbolic `NamedFn` that IS
        // wired — coth = 1/tanh, sech = 1/cosh, csch = 1/sinh — so we compose `1 / f(x)` from the
        // existing `Tanh`/`Cosh`/`Sinh` calls. Pure adapter recognition: no engine, AST, or lowering
        // change (the argument recurses through the SAME `latex_math_to_expr_ast` the `\sin`/`\exp`
        // arms use — no new tree-walk). Handles both the bare-macro (`\coth(x)`) and operator-name
        // (`\operatorname{coth}(x)`) spellings. This closes the trig/hyperbolic symmetry: the
        // circular reciprocals `cot`/`sec`/`csc` already lower (in the `Call` arm); their hyperbolic
        // twins now do too.
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if reciprocal_hyperbolic_den(lhs).is_some() => {
            let den = reciprocal_hyperbolic_den(lhs).expect("guard guarantees Some");
            Ok(ExprAst::Bin(
                ArithOp::Div,
                Box::new(ExprAst::Lit(1.0)),
                Box::new(ExprAst::Call(
                    den,
                    Box::new(latex_math_to_expr_ast(rhs, source)?),
                )),
            ))
        }
        // `\operatorname{arsinh}(x)` / `\operatorname{arcosh}(x)` / `\operatorname{artanh}(x)` — the
        // INVERSE hyperbolic (area-hyperbolic) functions, the mirror of the reciprocal-hyperbolic arm
        // just above. Like `\coth` these have no dedicated engine op, and like `\coth` they arrive as
        // an operator-name juxtaposition — `Bin(Mul, Text("arsinh"), (x))` for the `\operatorname{…}`
        // spelling, or `Bin(Mul, Symbol("arsinh"), (x))` for the bare `\arsinh` macro (an unknown
        // control sequence). But each inverse hyperbolic has a closed-form LOGARITHM identity built
        // from primitives the engine ALREADY has — the natural log (`NamedFn::Ln`), the power op
        // (`ArithOp::Pow`, used here for both squaring `^2` and the square root `^0.5`), and plain
        // arithmetic — so we compose the identity directly, no engine/AST/lowering change (exactly the
        // `\coth = 1/tanh` trick, one level richer). The identities:
        //   arsinh(x) = ln( x + (x^2 + 1)^0.5 )   [ all real x ]
        //   arcosh(x) = ln( x + (x^2 - 1)^0.5 )   [ x >= 1; the engine yields NaN below, matching the
        //                                           real-valued function's domain ]
        //   artanh(x) = 0.5 * ln( (1 + x) / (1 - x) )   [ |x| < 1 ]
        // The argument recurses through the SAME `latex_math_to_expr_ast` the `\sin`/`\coth` arms use
        // (no new tree-walk, so no new stack-overflow surface), then is CLONED where the identity names
        // `x` more than once — cloning an already-lowered, already-bounded `ExprAst` adds no recursion.
        // Common spellings are all accepted (`arsinh`/`arcsinh`/`asinh`, etc.); see
        // `inverse_hyperbolic_kind`. This finishes the hyperbolic family: direct (`sinh`/`cosh`/`tanh`),
        // reciprocal (`coth`/`sech`/`csch`), and now inverse (`arsinh`/`arcosh`/`artanh`) all lower.
        MathExpr::Bin(BinOp::Mul, lhs, rhs) if inverse_hyperbolic_kind(lhs).is_some() => {
            let kind = inverse_hyperbolic_kind(lhs).expect("guard guarantees Some");
            let arg = latex_math_to_expr_ast(rhs, source)?;
            Ok(lower_inverse_hyperbolic(kind, arg))
        }
        MathExpr::Bin(BinOp::Mul, lhs, rhs) => latex_bin(ArithOp::Mul, lhs, rhs, source),
        MathExpr::Bin(BinOp::Div, lhs, rhs) | MathExpr::Frac(lhs, rhs) => {
            latex_bin(ArithOp::Div, lhs, rhs, source)
        }
        MathExpr::Bin(BinOp::Pow, base, exponent) => {
            // `x^n` lowers to a single native power node (`ArithOp::Pow` →
            // `ComputeOp::Pow`), not the old parse-time `x*x*…` expansion — so
            // there is no integer-exponent cap and the derivation tree shows one
            // `^` step. BOTH the base AND the exponent lower as general expressions,
            // so the exponent may be a literal (`x^{2}`), a SYMBOLIC/observed value
            // (`x^y` with `y` observed → `x` raised to `y`), or itself computed
            // (`x^{a+b}`). The engine's `ComputeOp::Pow` evaluates the exponent at
            // run time and enforces its own rules — the exponent must be
            // dimensionless (you cannot raise to a `3 dollars` power) and finite,
            // and a non-integer exponent on a dimensioned base is rejected (no
            // fractional dimension) — so a symbolic exponent computes for the
            // dimensionless (Scalar) case and is cleanly rejected otherwise, with no
            // adapter-side literal restriction.
            Ok(ExprAst::Bin(
                ArithOp::Pow,
                Box::new(latex_math_to_expr_ast(base, source)?),
                Box::new(latex_math_to_expr_ast(exponent, source)?),
            ))
        }
        // A square root `\sqrt{x}` (a `Root` with no explicit degree) lowers to
        // `x ^ 0.5`, reusing the native `ComputeOp::Pow`. The engine computes it
        // for a dimensionless (Scalar) base — `√9 = 3` — and cleanly rejects a
        // dimensioned base (a `√dollars` has no representable half-dimension), so
        // no new engine op is needed.
        MathExpr::Root {
            degree: None,
            radicand,
        } => Ok(ExprAst::Bin(
            ArithOp::Pow,
            Box::new(latex_math_to_expr_ast(radicand, source)?),
            Box::new(ExprAst::Lit(0.5)),
        )),
        // An nth root `\sqrt[n]{x}` (degree present) lowers to `x ^ (1/n)`, again
        // reusing the native `ComputeOp::Pow` — the cube root `\sqrt[3]{27}`
        // computes `27 ^ (1/3) = 3`. The degree `n` must be a *positive integer*
        // literal (`\sqrt[3]{…}`, `\sqrt[4]{…}`); a symbolic degree (`\sqrt[k]{…}`)
        // and a non-positive degree are rejected. The fractional exponent `1/n` is
        // computed once at adapt time and emitted as a single `Lit`, so — exactly
        // like the square root — the engine sees one `Pow` node and applies its own
        // dimensional rule (a fractional power of a dimensioned base has no
        // representable dimension and is rejected; a Scalar base computes cleanly).
        MathExpr::Root {
            degree: Some(degree),
            radicand,
        } => {
            let n = latex_root_degree(degree, source)?;
            Ok(ExprAst::Bin(
                ArithOp::Pow,
                Box::new(latex_math_to_expr_ast(radicand, source)?),
                Box::new(ExprAst::Lit(1.0 / n)),
            ))
        }
        // A named-function call. The single-argument transcendental set
        // (`\sin(x)`, `\ln(x)`, `\exp(x)`, …) lowers to the matching native
        // transcendental op via `ExprAst::Call`; the variadic `\min` / `\max` /
        // `\gcd` / `\lcm` (`\min(a, b)`, `\max(a, b, c)`, …) left-fold their
        // two-or-more comma-separated operands into a chain of the native binary op
        // via `ExprAst::Call2` (the argument is a `Sequence`, usually inside the
        // call's parentheses). (`\operatorname{trunc}(x)` is NOT a `Call` — an
        // operator name is text, so it arrives as a `Bin(Mul, Text("trunc"), (x))`
        // juxtaposition handled in the `Bin` arm above.) The remaining `Func` variants
        // (`det` and an unknown `Other`) have no lowering yet and are a clean, explicit
        // error rather than a silent mis-lowering.
        MathExpr::Call { func, arg } => {
            // Binary functions first — their argument is a comma-list, not a scalar.
            let binfn = match func {
                Func::Min => Some(BinFn::Min),
                Func::Max => Some(BinFn::Max),
                Func::Gcd => Some(BinFn::Gcd),
                Func::Lcm => Some(BinFn::Lcm),
                _ => None,
            };
            if let Some(bin) = binfn {
                // Two OR MORE comma-separated arguments. `min`/`max`/`gcd`/`lcm` are
                // associative, so an n-ary call left-folds into a chain of the binary
                // `Call2` node — `min(a, b, c)` becomes `min(min(a, b), c)` — which is
                // exact and needs no n-ary engine op (the fold reuses `ComputeOp::Min2`
                // /`Max2`/`Gcd`/`Lcm`). A two-arg call folds to a single `Call2`,
                // identical to before. `latex_nary_fold` does the same work for the
                // `\operatorname{min}(…)` operator-name spelling in the `Bin` arm above.
                return latex_nary_fold(arg, source, bin, &format!("{func:?}"));
            }
            let named = match func {
                Func::Sin => NamedFn::Sin,
                Func::Cos => NamedFn::Cos,
                Func::Tan => NamedFn::Tan,
                Func::Ln => NamedFn::Ln,
                Func::Log => NamedFn::Log,
                Func::Exp => NamedFn::Exp,
                Func::Asin => NamedFn::Asin,
                Func::Acos => NamedFn::Acos,
                Func::Atan => NamedFn::Atan,
                Func::Sinh => NamedFn::Sinh,
                Func::Cosh => NamedFn::Cosh,
                Func::Tanh => NamedFn::Tanh,
                Func::Cot => NamedFn::Cot,
                Func::Sec => NamedFn::Sec,
                Func::Csc => NamedFn::Csc,
                other => {
                    return Err(AdapterError::UnsupportedLatexMath {
                        source: source.to_string(),
                        detail: format!(
                            "named function not yet supported in ADJ arithmetic: {other:?}"
                        ),
                    })
                }
            };
            Ok(ExprAst::Call(
                named,
                Box::new(latex_math_to_expr_ast(arg, source)?),
            ))
        }
        // `\hat{x}` / `\bar{x}` / `\vec{x}` / `\tilde{x}` / … — an accent over an operand. In
        // arithmetic an accent is a NOTATIONAL decoration, not an operation: a model that writes a
        // statistics formula like `\hat{p}(1 - \hat{p})` (estimated-variance numerator) or
        // `\bar{x} - \bar{y}` (difference of means) means the accented symbol to carry the value of
        // its operand — the hat/bar just marks it as an estimate/mean in prose. So we lower an
        // `Accent` TRANSPARENTLY to its inner body: `\hat{a}(b - \hat{a})` computes as `a·(b − a)`,
        // dimension and value flowing through the decoration unchanged. Pure adapter recognition:
        // no engine, AST, or lowering change — the accent simply disappears at adapt time.
        MathExpr::Accent { body, .. } => latex_math_to_expr_ast(body, source),
        // `\overset{note}{base}` / `\underset{note}{base}` (and `\overbrace{base}` /
        // `\underbrace{base}`, which parse as an `Overset`/`Underset` whose mark is a brace symbol) —
        // an annotation placed OVER or UNDER a base. Like an accent, an over/under annotation is a
        // NOTATIONAL decoration, not an operation: `\overbrace{a + b}^{\text{sum}}` labels the sum
        // in prose but computes `a + b`, and `\underset{x \to 0}{\lim}`-style marks annotate without
        // changing the base's value. So we lower `Overset`/`Underset` TRANSPARENTLY to the `base`,
        // discarding the `over`/`under` mark — exactly as the `Accent` arm above drops its diacritic.
        // The value and dimension flow through the annotation unchanged. Pure adapter recognition: no
        // engine/AST/lowering change, and (like the accent arm) it recurses into a single strict
        // sub-node, so no new deep-walk vector.
        MathExpr::Overset { base, .. } | MathExpr::Underset { base, .. } => {
            latex_math_to_expr_ast(base, source)
        }
        // `\sum_{i=1}^{3} body` / `\prod_{k=1}^{4} body` — a big operator with CONCRETE finite
        // integer bounds. A summation/product isn't a single arithmetic op; it iterates the body
        // over the index. We handle the decidable case — both bounds concrete integers — by
        // UNROLLING: substitute `i := lo, lo+1, …, hi` into the body and fold the resulting terms
        // with `+` (sum) or `·` (product). Composes with subscripts: `\sum_{i=1}^{3} x_i` expands to
        // `x_1 + x_2 + x_3` (each `x_k` then binds to its own `observe`). A symbolic bound
        // (`\sum_{i=1}^{n}`), an integral (`\int`), or an over-large range is an explicit
        // `UnsupportedLatexMath` — never a guess. See `lower_bigop`.
        MathExpr::BigOp {
            op,
            lower,
            upper,
            body,
        } => lower_bigop(op, lower.as_deref(), upper.as_deref(), body, source),
        // `\binom{n}{k}` / `\dbinom{n}{k}` / `\tbinom{n}{k}` — a binomial coefficient "n choose k"
        // (the frontend lowers all three spellings to `MathExpr::Binom`). This is DISTINCT from
        // `\frac{n}{k}`: it denotes the COUNT C(n, k) = n! / (k!·(n−k)!), not the ratio n/k. A
        // binomial is not a single arithmetic op, so — mirroring the finite-`\sum`/`\prod` arm above
        // — we evaluate the decidable case (both arguments CONCRETE non-negative integers with
        // k ≤ n ≤ cap) to its exact integer value via a bounded product loop, and reject a symbolic,
        // negative, out-of-order, oversized, or too-large-to-represent binomial as an explicit
        // `UnsupportedLatexMath`. See `lower_binom`.
        MathExpr::Binom(n, k) => lower_binom(n, k, source),
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

/// If `expr` names a reciprocal hyperbolic function (`\coth`/`\sech`/`\csch`), return the
/// hyperbolic [`NamedFn`] it is the reciprocal OF, so the caller can compose `1 / f(x)`:
/// coth = 1/tanh → [`NamedFn::Tanh`], sech = 1/cosh → [`NamedFn::Cosh`], csch = 1/sinh →
/// [`NamedFn::Sinh`]. Matches the bare-macro spelling (`\coth` → `Symbol("coth")`, an unknown
/// control sequence — and a `Symbol` named exactly `coth` can ONLY arise from that macro) and the
/// operator-name spelling (`\operatorname{coth}` / `\mathrm{coth}` → `Text("coth")`). Returns
/// `None` for anything else, so a genuine product (`2x`) or an unrelated symbol falls through to
/// the general multiplication arm.
fn reciprocal_hyperbolic_den(expr: &MathExpr) -> Option<NamedFn> {
    let name = match expr {
        MathExpr::Symbol(s) => s.as_str(),
        MathExpr::Text(s) => s.trim(),
        _ => return None,
    };
    match name {
        "coth" => Some(NamedFn::Tanh),
        "sech" => Some(NamedFn::Cosh),
        "csch" => Some(NamedFn::Sinh),
        _ => None,
    }
}

/// Which inverse hyperbolic (area-hyperbolic) function `expr` names, if any. Recognised so the
/// caller can compose the closed-form logarithm identity (see `lower_inverse_hyperbolic`). Matches
/// the same two spellings the reciprocal-hyperbolic arm handles — the bare macro (`\arsinh` →
/// `Symbol("arsinh")`, an unknown control sequence) and the operator name (`\operatorname{arsinh}`
/// / `\mathrm{arsinh}` → `Text("arsinh")`) — across the three common surface spellings of each: the
/// ISO/area form (`arsinh`), the inverse-notation form (`arcsinh`), and the terse form (`asinh`).
/// Returns `None` for anything else, so a genuine product falls through to the general
/// multiplication arm.
fn inverse_hyperbolic_kind(expr: &MathExpr) -> Option<InverseHyperbolic> {
    let name = match expr {
        MathExpr::Symbol(s) => s.as_str(),
        MathExpr::Text(s) => s.trim(),
        _ => return None,
    };
    match name {
        "arsinh" | "arcsinh" | "asinh" => Some(InverseHyperbolic::ArSinh),
        "arcosh" | "arccosh" | "acosh" => Some(InverseHyperbolic::ArCosh),
        "artanh" | "arctanh" | "atanh" => Some(InverseHyperbolic::ArTanh),
        _ => None,
    }
}

/// The three inverse hyperbolic functions the adapter lowers via logarithm identities.
// The shared `Ar` prefix is the standard mathematical spelling
// (arsinh/arcosh/artanh), not an accidental naming collision.
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy)]
enum InverseHyperbolic {
    /// `arsinh(x) = ln(x + (x^2 + 1)^0.5)`.
    ArSinh,
    /// `arcosh(x) = ln(x + (x^2 - 1)^0.5)`.
    ArCosh,
    /// `artanh(x) = 0.5 * ln((1 + x) / (1 - x))`.
    ArTanh,
}

/// Compose an inverse hyperbolic function from its closed-form logarithm identity, using only
/// primitives the engine already evaluates — `NamedFn::Ln`, `ArithOp::Pow` (squaring and square
/// root), and plain arithmetic. `arg` is the ALREADY-lowered argument expression; it is cloned
/// where the identity names `x` more than once (arsinh/arcosh use it twice, artanh twice). Cloning
/// a bounded `ExprAst` introduces no recursion, so this adds no stack-overflow surface. The results
/// evaluate to the standard real-valued branch, and to NaN outside each function's real domain
/// (arcosh below 1, artanh at/outside ±1) — the same as the underlying `ln`/root would give.
fn lower_inverse_hyperbolic(kind: InverseHyperbolic, arg: ExprAst) -> ExprAst {
    // Small constructors keep the identity trees readable.
    fn lit(v: f64) -> Box<ExprAst> {
        Box::new(ExprAst::Lit(v))
    }
    fn bin(op: ArithOp, a: Box<ExprAst>, b: Box<ExprAst>) -> Box<ExprAst> {
        Box::new(ExprAst::Bin(op, a, b))
    }
    match kind {
        // ln( x + (x^2 + 1)^0.5 )
        InverseHyperbolic::ArSinh => {
            let x_squared = bin(ArithOp::Pow, Box::new(arg.clone()), lit(2.0));
            let radicand = bin(ArithOp::Add, x_squared, lit(1.0));
            let root = bin(ArithOp::Pow, radicand, lit(0.5));
            let sum = bin(ArithOp::Add, Box::new(arg), root);
            ExprAst::Call(NamedFn::Ln, sum)
        }
        // ln( x + (x^2 - 1)^0.5 )
        InverseHyperbolic::ArCosh => {
            let x_squared = bin(ArithOp::Pow, Box::new(arg.clone()), lit(2.0));
            let radicand = bin(ArithOp::Sub, x_squared, lit(1.0));
            let root = bin(ArithOp::Pow, radicand, lit(0.5));
            let sum = bin(ArithOp::Add, Box::new(arg), root);
            ExprAst::Call(NamedFn::Ln, sum)
        }
        // 0.5 * ln( (1 + x) / (1 - x) )
        InverseHyperbolic::ArTanh => {
            let numerator = bin(ArithOp::Add, lit(1.0), Box::new(arg.clone()));
            let denominator = bin(ArithOp::Sub, lit(1.0), Box::new(arg));
            let ratio = bin(ArithOp::Div, numerator, denominator);
            let log = Box::new(ExprAst::Call(NamedFn::Ln, ratio));
            ExprAst::Bin(ArithOp::Mul, lit(0.5), log)
        }
    }
}

/// Is `expr` an operator-name text (`\operatorname{name}`, `\mathrm{name}`, `\text{name}`)
/// whose content equals `name`? The frontend lowers all of these to `MathExpr::Text`, so a
/// juxtaposed `\operatorname{trunc}(x)` arrives as `Bin(Mul, Text("trunc"), (x))`. We match
/// on the trimmed content (the raw text group can carry surrounding spaces) so
/// `\operatorname{trunc}` and `\operatorname{ trunc }` both count.
fn operator_name_is(expr: &MathExpr, name: &str) -> bool {
    matches!(expr, MathExpr::Text(s) if s.trim() == name)
}

/// Flatten a subscript base or index `MathExpr` into the identifier fragment it names, or `None`
/// if it is not a plain name/number. Used by the `MathExpr::Subscript` arm to mangle `x_i` /
/// `V_{max}` into a single `base_sub` identifier. A single-letter subscript is a `Symbol` and a
/// digit subscript a `Number` (`x_1` -> `"1"` via `as_written`), while a BRACED multi-letter
/// subscript `{max}` is parsed by the frontend as a juxtaposition chain of single-letter `Symbol`s
/// (`Bin(Mul, Bin(Mul, m, a), x)`), so the `Bin(Mul, ..)` arm concatenates the fragments back into
/// the word ("max") — no separator, since juxtaposed letters spell ONE name. A nested subscript
/// (`x_{i_j}`) joins with `_`. Anything else (an arithmetic subscript like `x_{i+1}`, a fraction, …)
/// returns `None`, so the caller reports `UnsupportedLatexMath` rather than inventing a binding.
///
/// **Iterative, not recursive, on purpose.** A subscript body can be an ARBITRARILY deep
/// left-associative `Bin(Mul)` spine: the frontend builds juxtaposition chains (`x_{aaaa…a}`) in a
/// loop that does NOT charge the latex parser's `MAX_DEPTH`, so the tree depth is unbounded by input
/// length (the `latex` crate parses 50 000-term chains in its own tests). A naive recursion here
/// would overflow the thread stack on adversarial input — an *uncatchable* abort (a DoS). So we
/// walk the tree with an explicit heap work-stack (mirroring the `latex` crate's own iterative
/// `lower`): stack depth is heap-bounded by input length, call depth is O(1).
fn subscript_ident_part(expr: &MathExpr) -> Option<String> {
    // A work item is either a subtree still to flatten, or a literal separator to emit. We push
    // children in REVERSE so the LIFO stack pops them left-to-right.
    enum Work<'a> {
        Node(&'a MathExpr),
        Sep,
    }
    let mut out = String::new();
    let mut stack: Vec<Work> = vec![Work::Node(expr)];
    while let Some(item) = stack.pop() {
        match item {
            Work::Sep => out.push('_'),
            Work::Node(node) => match node {
                MathExpr::Symbol(s) => out.push_str(s),
                MathExpr::Text(s) => out.push_str(s.trim()),
                MathExpr::Number(n) => out.push_str(n.as_written()),
                MathExpr::Group(inner) => stack.push(Work::Node(inner)),
                // `{max}` -> juxtaposed letters spell ONE word: emit left then right, no separator.
                MathExpr::Bin(BinOp::Mul, l, r) => {
                    stack.push(Work::Node(r));
                    stack.push(Work::Node(l));
                }
                // Nested subscript `x_{i_j}` -> base `_` index.
                MathExpr::Subscript(b, s) => {
                    stack.push(Work::Node(s));
                    stack.push(Work::Sep);
                    stack.push(Work::Node(b));
                }
                // Anything else is not a plain name — abort the whole flatten (the caller then
                // reports `UnsupportedLatexMath`, never inventing a partial binding).
                _ => return None,
            },
        }
    }
    Some(out)
}

/// The largest number of terms a `\sum`/`\prod` may unroll to. A finite summation is expanded into
/// that many `+`/`·` operands; the cap bounds the work (and the emitted AST size) so an adversarial
/// `\sum_{i=1}^{100000000}` is rejected rather than exploded.
const BIGOP_UNROLL_CAP: i64 = 256;

/// The recursion budget for `substitute_index`. A subscript/juxtaposition body can be an arbitrarily
/// deep `Bin` spine that the latex parser's `MAX_DEPTH` does NOT bound, so the substitution walker is
/// depth-budgeted: it returns `None` (→ `UnsupportedLatexMath`) rather than recursing without limit
/// and overflowing the stack. 96 comfortably covers any realistic summation body.
const SUBST_DEPTH_BUDGET: u32 = 96;

/// Cap on a binomial's upper argument `n` in `\binom{n}{k}`. The evaluation loop runs at most
/// `min(k, n-k) <= n/2 <= BINOM_N_CAP/2` steps, so this bounds the work; a binomial with `n` beyond
/// the cap is rejected as `UnsupportedLatexMath` rather than looped over. 1000 comfortably covers any
/// realistic combinatorial expression while keeping the exact-value guard (below) the effective
/// limit — most binomials overflow the f64 exact-integer range long before `n` reaches 1000.
const BINOM_N_CAP: i64 = 1000;

/// A short constructor for the adapter's catch-all "not supported" error.
fn unsupported_latex(source: &str, detail: &str) -> AdapterError {
    AdapterError::UnsupportedLatexMath {
        source: source.to_string(),
        detail: detail.to_string(),
    }
}

/// Extract a concrete non-negative-magnitude integer from a numeric `MathExpr` (a `Number`, or a
/// `Group`/`Fenced` wrapping one). Returns `None` for a symbol, a fraction, or a non-integral /
/// out-of-range value — the caller then rejects the summation rather than guessing a bound.
fn number_as_i64(expr: &MathExpr) -> Option<i64> {
    match expr {
        MathExpr::Number(n) => {
            let v = n.to_f64()?;
            // Reject non-finite, non-integral, or magnitudes beyond exact f64 integer range.
            if v.is_finite() && v.fract() == 0.0 && v.abs() < 9.0e15 {
                Some(v as i64)
            } else {
                None
            }
        }
        MathExpr::Group(inner) | MathExpr::Fenced { body: inner, .. } => number_as_i64(inner),
        _ => None,
    }
}

/// Evaluate a binomial coefficient `\binom{n}{k}` = "n choose k" to its concrete integer value.
///
/// A binomial is not a single arithmetic op — it is the COUNT
///
/// ```text
/// C(n, k) = n! / (k! * (n - k)!)      ("n choose k")
/// ```
///
/// We handle the decidable case — both arguments CONCRETE non-negative integers with `k <= n <= cap`
/// — by evaluating the **multiplicative product formula**
///
/// ```text
/// C(n, k) = product over i in 1..=k of (n - k + i) / i
/// ```
///
/// which needs only `k` multiply/divide steps and is exact: after step `i` the running value equals
/// `C(n−k+i, i)`, an integer (the product of any `i` consecutive integers is divisible by `i!`), so
/// no rounding accumulates while the result stays within the f64 exact-integer range. We iterate over
/// `min(k, n−k)` — using the symmetry `C(n, k) = C(n, n−k)` — so the loop is at most `n/2` steps.
///
/// A symbolic argument (`\binom{n}{k}` with variables), a negative argument, `k > n`, an `n` beyond
/// `BINOM_N_CAP`, or a result too large to represent exactly as an f64 integer is an explicit
/// `UnsupportedLatexMath` — never a guess, an approximation, or a silently-rounded literal.
///
/// SAFETY / no new deep walk: both arguments are read with the NON-recursive [`number_as_i64`], which
/// unwraps only `Group`/`Fenced` (nesting the parser bounds by `MAX_DEPTH`) and returns `None` on any
/// other shape WITHOUT descending into it. A pathological argument like `\binom{aaaa…}{2}` — a long
/// juxtaposition that parses as a deep left-associative `Bin(Mul)` spine in the `n` slot — makes
/// `number_as_i64` return `None` on the outermost `Bin`, so we reject immediately and NEVER recurse
/// into the spine (and never hand it to `latex_math_to_expr_ast`). The bounded loop is the only
/// iteration, so this arm adds no new unbounded tree-walk.
fn lower_binom(n_expr: &MathExpr, k_expr: &MathExpr, source: &str) -> Result<ExprAst, AdapterError> {
    let n = number_as_i64(n_expr).ok_or_else(|| {
        unsupported_latex(
            source,
            "binomial upper argument must be a concrete non-negative integer",
        )
    })?;
    let k = number_as_i64(k_expr).ok_or_else(|| {
        unsupported_latex(
            source,
            "binomial lower argument must be a concrete non-negative integer",
        )
    })?;
    if n < 0 || k < 0 {
        return Err(unsupported_latex(
            source,
            "binomial arguments must be non-negative",
        ));
    }
    if k > n {
        return Err(unsupported_latex(
            source,
            "binomial lower argument exceeds the upper argument",
        ));
    }
    if n > BINOM_N_CAP {
        return Err(unsupported_latex(
            source,
            "binomial upper argument is too large to expand",
        ));
    }
    // Symmetry C(n, k) = C(n, n−k): iterate over the smaller of `k` and `n−k` so the loop is ≤ n/2.
    let kk = k.min(n - k);
    let mut result: f64 = 1.0;
    for i in 1..=kk {
        result = result * (n - kk + i) as f64 / i as f64;
    }
    // The exact integer C(n, k) can exceed the f64 exact-integer range even for modest `n` (e.g.
    // C(60, 30) ≈ 1.18e17). Beyond that range a literal would silently lose precision, so we reject
    // rather than emit an inexact value — mirroring `number_as_i64`'s own 9.0e15 exact-integer bound.
    if !result.is_finite() || result.abs() >= 9.0e15 {
        return Err(unsupported_latex(
            source,
            "binomial coefficient is too large to represent exactly",
        ));
    }
    Ok(ExprAst::Lit(result))
}

/// Lower a big operator (`\sum`/`\prod`) with CONCRETE finite integer bounds by unrolling. The lower
/// bound must be `index = <integer>` and the upper bound a concrete integer; for each `k` in
/// `lo..=hi` the loop variable is substituted into the body and the terms are folded with `+`
/// (`Sum`) or `·` (`Prod`). Symbolic bounds (`\sum_{i=1}^{n}`), integrals, or ranges beyond
/// `BIGOP_UNROLL_CAP` are rejected as `UnsupportedLatexMath` — never approximated.
fn lower_bigop(
    op: &BigOp,
    lower: Option<&MathExpr>,
    upper: Option<&MathExpr>,
    body: &MathExpr,
    source: &str,
) -> Result<ExprAst, AdapterError> {
    let fold_op = match op {
        BigOp::Sum => ArithOp::Add,
        BigOp::Prod => ArithOp::Mul,
        _ => {
            return Err(unsupported_latex(
                source,
                "only finite \\sum and \\prod with concrete integer bounds are supported",
            ))
        }
    };
    // The lower bound carries the index variable: `\sum_{i=1}^{…}` parses its subscript as
    // `Rel(Eq, Symbol("i"), Number(1))`.
    let (index, lo) = match lower {
        Some(MathExpr::Rel(MathRelOp::Eq, lhs, rhs)) => {
            let index = match lhs.as_ref() {
                MathExpr::Symbol(s) => s.clone(),
                _ => {
                    return Err(unsupported_latex(
                        source,
                        "summation index must be a plain variable",
                    ))
                }
            };
            let lo = number_as_i64(rhs).ok_or_else(|| {
                unsupported_latex(source, "summation lower bound must be a concrete integer")
            })?;
            (index, lo)
        }
        _ => {
            return Err(unsupported_latex(
                source,
                "summation needs a lower bound of the form `index = <integer>`",
            ))
        }
    };
    let hi = match upper {
        Some(u) => number_as_i64(u).ok_or_else(|| {
            unsupported_latex(source, "summation upper bound must be a concrete integer")
        })?,
        None => {
            return Err(unsupported_latex(
                source,
                "summation needs an explicit integer upper bound",
            ))
        }
    };
    if hi < lo {
        return Err(unsupported_latex(
            source,
            "summation upper bound is below the lower bound",
        ));
    }
    if hi - lo + 1 > BIGOP_UNROLL_CAP {
        return Err(unsupported_latex(
            source,
            "summation range is too large to expand",
        ));
    }
    // Unroll: substitute `index := k` into the body for each k and fold the terms. `hi >= lo` so at
    // least one term is produced and `acc` ends up `Some`.
    let mut acc: Option<ExprAst> = None;
    let mut k = lo;
    while k <= hi {
        let substituted = substitute_index(body, &index, k, SUBST_DEPTH_BUDGET).ok_or_else(|| {
            unsupported_latex(
                source,
                "summation body is too deeply nested or not a plain arithmetic term",
            )
        })?;
        let term = latex_math_to_expr_ast(&substituted, source)?;
        acc = Some(match acc {
            None => term,
            Some(prev) => ExprAst::Bin(fold_op, Box::new(prev), Box::new(term)),
        });
        k += 1;
    }
    acc.ok_or_else(|| unsupported_latex(source, "summation produced no terms"))
}

/// Return a copy of `expr` with every free occurrence of the loop variable `idx` replaced by the
/// integer `k`, or `None` if the body contains a construct that cannot be a plain arithmetic term
/// (so the whole summation is rejected rather than mis-expanded). Substituting `i := 2` into
/// `Subscript(Symbol("x"), Symbol("i"))` yields `Subscript(Symbol("x"), Number(2))`, which the
/// subscript arm then mangles to `x_2` — that is how `\sum_{i=1}^{3} x_i` becomes `x_1 + x_2 + x_3`.
///
/// **Depth-budgeted, not unbounded.** `budget` is decremented per level and exhaustion returns
/// `None`: a body whose `Bin` spine is deeper than the budget is rejected, so an adversarial deep
/// juxtaposition body cannot overflow the stack (the same DoS class the latex crate's `MAX_DEPTH`
/// does not cover for left-associative spines).
fn substitute_index(expr: &MathExpr, idx: &str, k: i64, budget: u32) -> Option<MathExpr> {
    let budget = budget.checked_sub(1)?;
    let sub = |e: &MathExpr| substitute_index(e, idx, k, budget);
    Some(match expr {
        MathExpr::Symbol(s) if s == idx => MathExpr::Number(Number::from_i64(k)),
        MathExpr::Symbol(s) => MathExpr::Symbol(s.clone()),
        MathExpr::Number(n) => MathExpr::Number(n.clone()),
        MathExpr::Bin(op, l, r) => MathExpr::Bin(*op, Box::new(sub(l)?), Box::new(sub(r)?)),
        MathExpr::Unary(op, inner) => MathExpr::Unary(*op, Box::new(sub(inner)?)),
        MathExpr::Frac(a, b) => MathExpr::Frac(Box::new(sub(a)?), Box::new(sub(b)?)),
        MathExpr::Binom(a, b) => MathExpr::Binom(Box::new(sub(a)?), Box::new(sub(b)?)),
        MathExpr::Group(inner) => MathExpr::Group(Box::new(sub(inner)?)),
        MathExpr::Fenced { open, body, close } => MathExpr::Fenced {
            open: open.clone(),
            body: Box::new(sub(body)?),
            close: close.clone(),
        },
        MathExpr::Subscript(b, s) => MathExpr::Subscript(Box::new(sub(b)?), Box::new(sub(s)?)),
        MathExpr::Call { func, arg } => MathExpr::Call {
            func: func.clone(),
            arg: Box::new(sub(arg)?),
        },
        MathExpr::Root { degree, radicand } => MathExpr::Root {
            degree: match degree {
                Some(d) => Some(Box::new(sub(d)?)),
                None => None,
            },
            radicand: Box::new(sub(radicand)?),
        },
        MathExpr::Accent { accent, body } => MathExpr::Accent {
            accent: accent.clone(),
            body: Box::new(sub(body)?),
        },
        MathExpr::Overset { over, base } => MathExpr::Overset {
            over: Box::new(sub(over)?),
            base: Box::new(sub(base)?),
        },
        MathExpr::Underset { under, base } => MathExpr::Underset {
            under: Box::new(sub(under)?),
            base: Box::new(sub(base)?),
        },
        // Text, Rel, Matrix, Sequence, a nested BigOp — not a plain arithmetic term. Reject the
        // whole unroll rather than substitute into something we cannot faithfully expand.
        _ => return None,
    })
}

/// If `expr` is a trigonometric operator-name text (`\operatorname{sin}`, `\operatorname{arctan}`,
/// `\operatorname{sinh}`, …), return the matching `NamedFn`; otherwise `None`. Same recognition as
/// `operator_name_is` (a `MathExpr::Text` whose trimmed content is the name), but consolidated for
/// the whole trig family so one adapter arm can lower `\operatorname{sin}(x)` to the SAME
/// `ExprAst::Call(NamedFn::Sin)` the `\sin(x)` macro produces. The `arc…` spellings are accepted as
/// aliases for the inverse functions (a model may write either `\operatorname{asin}` or
/// `\operatorname{arcsin}`). `exp`/`log`/`ln` are intentionally NOT here — they have their own
/// dedicated arms above (and `abs` lowers to `ExprAst::Abs`, not a `Call`).
fn operator_name_trig_fn(expr: &MathExpr) -> Option<NamedFn> {
    let MathExpr::Text(s) = expr else { return None };
    match s.trim() {
        "sin" => Some(NamedFn::Sin),
        "cos" => Some(NamedFn::Cos),
        "tan" => Some(NamedFn::Tan),
        "cot" => Some(NamedFn::Cot),
        "sec" => Some(NamedFn::Sec),
        "csc" => Some(NamedFn::Csc),
        "asin" | "arcsin" => Some(NamedFn::Asin),
        "acos" | "arccos" => Some(NamedFn::Acos),
        "atan" | "arctan" => Some(NamedFn::Atan),
        "sinh" => Some(NamedFn::Sinh),
        "cosh" => Some(NamedFn::Cosh),
        "tanh" => Some(NamedFn::Tanh),
        _ => None,
    }
}

/// Is `expr` the LEFT operand of a `\bmod`/`\pmod` juxtaposition — i.e.
/// `Bin(Mul, real_lhs, Symbol("bmod"|"pmod"))`? The frontend has no operator table
/// entry for `\bmod`/`\pmod`, so it lowers them to a bare `Symbol` that ends up as the
/// right factor of the left operand of the surrounding implicit multiplication. If the
/// shape matches, return `real_lhs` (the dividend expression); otherwise `None`.
fn mod_juxtaposition_lhs(expr: &MathExpr) -> Option<&MathExpr> {
    if let MathExpr::Bin(BinOp::Mul, real_lhs, marker) = expr {
        if matches!(marker.as_ref(), MathExpr::Symbol(s) if s == "bmod" || s == "pmod") {
            return Some(real_lhs);
        }
    }
    None
}

fn latex_bin(
    op: ArithOp,
    lhs: &MathExpr,
    rhs: &MathExpr,
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

/// Peel a variadic named-function argument (`\min(a, b)`, `\max(a, b, c)`, …) into
/// its operand list. The latex frontend parses the parenthesised comma-list as a
/// `Sequence([a, b, …])`, usually wrapped in a `Group`/`Fenced` (the `(…)`), so we
/// strip those transparent wrappers and require **two or more** items — the caller
/// left-folds them into a chain of the associative binary op. A one-arg (`\min(a)`)
/// call, or a non-comma argument, has no such lowering and is a clean, explicit
/// error rather than a silent mis-lowering.
fn latex_nary_fold(
    arg: &MathExpr,
    source: &str,
    bin: BinFn,
    label: &str,
) -> Result<ExprAst, AdapterError> {
    // Strip transparent parenthesisation to reach the underlying sequence. The
    // function-call spelling wraps its args in a `Fenced` (the call's parentheses); the
    // operator-name spelling arrives here already unwrapped to the `Fenced` right factor —
    // both reduce to the inner `Sequence`.
    let mut inner = arg;
    loop {
        match inner {
            MathExpr::Group(b) => inner = b,
            MathExpr::Fenced { body, .. } => inner = body,
            _ => break,
        }
    }
    let items = match inner {
        MathExpr::Sequence(items) if items.len() >= 2 => items,
        _ => {
            return Err(AdapterError::UnsupportedLatexMath {
                source: source.to_string(),
                detail: format!(
                    "{label} takes two or more comma-separated arguments in ADJ arithmetic \
                     (e.g. \\min(a, b) or \\min(a, b, c)); got {inner:?}"
                ),
            })
        }
    };
    // `min`/`max`/`gcd`/`lcm` are associative, so the ≥2 operands left-fold into a chain of
    // the binary `Call2` op — `gcd(a, b, c)` → `gcd(gcd(a, b), c)` — which is exact and
    // needs no n-ary engine op.
    let mut operands = items.iter();
    // The `len() >= 2` guard above guarantees the first `next()` is `Some`.
    let first = operands.next().expect("checked items.len() >= 2");
    let mut acc = latex_math_to_expr_ast(first, source)?;
    for operand in operands {
        acc = ExprAst::Call2(
            bin,
            Box::new(acc),
            Box::new(latex_math_to_expr_ast(operand, source)?),
        );
    }
    Ok(acc)
}

/// Validate an nth-root degree (`n` in `\sqrt[n]{x}`) and return it as a
/// whole-number `f64`, so the caller can build the reciprocal exponent `1/n`. The
/// degree must be a **positive integer literal** (`\sqrt[3]{…}`, `\sqrt[4]{…}`): a
/// symbolic degree (`\sqrt[k]{…}`) has no numeric value, and a zero or negative
/// degree has no root meaning (a `1/0` exponent is undefined). The degree must be
/// a finite positive integer literal (unlike the general `x^y` power exponent,
/// which may be symbolic/computed, a root DEGREE must be a concrete integer to form
/// the reciprocal `1/n`), excluding `0` (the exponent's denominator) — `n ≥ 1`, so
/// `\sqrt[1]{x}` degenerates cleanly to
/// `x^1 = x`, and `\sqrt[3]{27}` becomes `27^(1/3) = 3`.
fn latex_root_degree(expr: &MathExpr, source: &str) -> Result<f64, AdapterError> {
    let MathExpr::Number(n) = expr else {
        return Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: "only a positive integer root degree is supported in ADJ arithmetic".into(),
        });
    };
    let Some(v) = n.to_f64() else {
        return Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: format!("root degree is outside f64 range: {}", n.as_written()),
        });
    };
    if !(v.is_finite() && v.fract() == 0.0 && v >= 1.0) {
        return Err(AdapterError::UnsupportedLatexMath {
            source: source.to_string(),
            detail: "only a positive integer root degree is supported".into(),
        });
    }
    Ok(v)
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
                    Evidence::Predicate { slot, op, rhs } => {
                        assert_eq!(slot, "gross_income");
                        assert_eq!(op, CmpOp::Ge);
                        assert!(matches!(rhs, ExprAst::Lit(v) if v == 14600.0));
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
                    evidence: Evidence::Predicate { op, rhs, slot },
                    ..
                } => {
                    assert_eq!(op, *expected, "operator {sym}");
                    assert!(matches!(rhs, ExprAst::Lit(v) if v == 18.0));
                    assert_eq!(slot, "age");
                }
                other => panic!("expected predicate Contributes for {sym}, got {other:?}"),
            }
        }
    }

    #[test]
    fn predicate_rhs_can_be_an_arithmetic_expression() {
        let src = "contributes 1000000 from answer == 3 / 10 to opt_a";
        match parse_one(src) {
            Statement::Contributes {
                evidence: Evidence::Predicate { slot, op, rhs },
                ..
            } => {
                assert_eq!(slot, "answer");
                assert_eq!(op, CmpOp::Eq);
                assert!(matches!(rhs, ExprAst::Bin(ArithOp::Div, _, _)));
            }
            other => panic!("expected predicate Contributes, got {other:?}"),
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
