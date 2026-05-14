//! Walk a `GrammarASTNode` tree → typed Twig AST.
//!
//! The grammar-driven [`parser::grammar_parser::GrammarParser`] emits one
//! node per non-terminal — wrappers like `expr` / `compound` /
//! `name_or_signature` exist to keep `code/grammars/twig.grammar`
//! readable but carry no semantic content of their own.  This module
//! lifts each meaningful subtree into one of the typed nodes from
//! [`crate::ast_nodes`].
//!
//! ## How a few key shapes lower
//!
//! | Grammar production                                           | Typed AST              |
//! |--------------------------------------------------------------|------------------------|
//! | `(define name expr)`                                         | `Define { Expr }`      |
//! | `(define (name args) body+)`                                 | `Define { Lambda }`    |
//! | `'foo`                                                       | `SymLit`               |
//! | `(quote foo)`                                                | `SymLit`               |
//! | `(if c t e)`                                                 | `If`                   |
//! | `(let ((x 1) (y 2)) body+)`                                  | `Let`                  |
//! | `(lambda (p) body+)`                                         | `Lambda`               |
//! | `(fn arg0 arg1 ...)`                                         | `Apply`                |
//! | `(match e arm+)` (LANG48)                                    | `Match`                |
//! | `(type Name expr)` (LANG48)                                  | `TypeAlias`            |
//! | `(record Name fields)` (LANG48)                              | `RecordDef`            |
//! | `(union Name variants)` (LANG48)                             | `UnionDef`             |
//!
//! Both quote forms (`'foo` and `(quote foo)`) collapse to a single
//! `SymLit { name: "foo" }`; downstream code therefore handles symbol
//! literals in one place.  Same for `define`-sugar — the function form
//! lowers to `Define { Lambda }` so the IR compiler only ever sees the
//! lambda shape.
//!
//! ## Stack-overflow guard
//!
//! The walker is recursive (`extract_expr` recurses into compound
//! children).  To defend against pathological input that produces an
//! arbitrarily deep AST (e.g. `((((...))))` with tens of thousands of
//! opens), every recursive descent passes a `depth: usize` and bails
//! out with a `TwigParseError` once it exceeds [`MAX_AST_DEPTH`].
//! Without this guard a deeply-nested input could exhaust the OS thread
//! stack and abort the process (Rust does not catch stack overflow).

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::ast_nodes::{
    Apply, Begin, BoolLit, Define, Expr, Form, If, IntLit, Lambda, Let, Match, MatchArm, MatchPat,
    ModuleInfo, NilLit, Program, RecordDef, RecordField, SymLit, TypeAlias, TypeAnnotation,
    TypeExpr, TypedMode, UnionDef, UnionVariant, VarRef,
};
use crate::TwigParseError;

/// Maximum AST-nesting depth the extractor will descend.
///
/// 256 is far above any realistic Twig program (hand-written code is
/// single-digit-deep) while keeping a comfortable margin under the
/// typical 2 MiB OS thread stack on macOS.
pub const MAX_AST_DEPTH: usize = 256;

/// Maximum number of integer values allowed in a `(Member int (...))`
/// refinement-type annotation.
///
/// Each value lowers to one equality predicate in `constraint-core`,
/// and all equalities are wrapped in an `Or`.  Naive CNF distribution
/// of an `Or` of N `And`-wrapped equalities is O(2^N); even with the
/// `MAX_CNF_CLAUSES` ceiling in `constraint-core`, a large membership
/// set wastes significant work before the budget fires.  Capping at
/// parse time is the cheapest defence.
///
/// 256 is generous for any realistic enum-style refinement (HTTP status
/// codes, MIDI velocities, small finite domains) while preventing
/// adversarial blowup.
pub const MAX_MEMBERSHIP_INT_VALUES: usize = 256;

/// Validate that a `(Member int (...))` annotation contains at most
/// [`MAX_MEMBERSHIP_INT_VALUES`] values.
///
/// Called by the LANG23 PR 23-E annotation extractor before constructing
/// `TypeAnnotation::MembershipInt { values }`.  Returns a parse error at
/// `(line, column)` if `count` exceeds the limit.
pub fn check_membership_int_count(
    count: usize,
    line: usize,
    column: usize,
) -> Result<(), crate::TwigParseError> {
    if count > MAX_MEMBERSHIP_INT_VALUES {
        Err(crate::TwigParseError {
            message: format!(
                "(Member ...) annotation has {count} values but the maximum is \
                 {MAX_MEMBERSHIP_INT_VALUES}; split into smaller membership sets or \
                 use a Range annotation instead"
            ),
            line,
            column,
        })
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Token helpers
// ---------------------------------------------------------------------------

/// True iff `child` is a raw `Token` whose grammar name (or enum
/// fallback) equals `name`.
fn is_token_named(child: &ASTNodeOrToken, name: &str) -> bool {
    match child {
        ASTNodeOrToken::Token(t) => t.effective_type_name() == name,
        ASTNodeOrToken::Node(_) => false,
    }
}

/// Filter to only the nested ASTNode children, dropping bare punctuation
/// tokens (LPAREN / RPAREN / KEYWORDs / NAMEs introduced as literals).
/// Convenient when a grammar rule's children list mixes structural
/// sub-rules with literal punctuation we can ignore.
fn ast_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// Best-effort source-position extraction.  Falls back to (1, 1) if the
/// node has no recorded position (rare — the GrammarParser always
/// records positions for non-empty matches).
fn pos(node: &GrammarASTNode) -> (usize, usize) {
    (node.start_line.unwrap_or(1), node.start_column.unwrap_or(1))
}

fn tok_pos(t: &Token) -> (usize, usize) {
    (t.line, t.column)
}

/// Find the first child token that matches `name` and return it.  Used
/// when the grammar rule has exactly one such token (e.g. the NAME in
/// `(quote NAME)`).
fn first_token_named<'a>(node: &'a GrammarASTNode, name: &str) -> Option<&'a Token> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.effective_type_name() == name => Some(t),
        _ => None,
    })
}

/// Collect every child token of the given grammar name, in order.
fn tokens_named<'a>(node: &'a GrammarASTNode, name: &str) -> Vec<&'a Token> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.effective_type_name() == name => Some(t),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Depth-bounded recursion guard
// ---------------------------------------------------------------------------

fn check_depth(depth: usize, line: usize, column: usize) -> Result<(), TwigParseError> {
    if depth > MAX_AST_DEPTH {
        Err(TwigParseError {
            message: format!(
                "AST nesting exceeds MAX_AST_DEPTH ({MAX_AST_DEPTH}) — \
                 refusing to recurse further to avoid stack overflow"
            ),
            line,
            column,
        })
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Public entry: program
// ---------------------------------------------------------------------------

/// Convert a parsed `program` [`GrammarASTNode`] into a typed [`Program`].
///
/// Expects `root.rule_name == "program"` (the grammar's start symbol).
/// The first child may be a `module_form` (extracted into
/// [`Program::module_info`]); subsequent children are `form` non-terminals.
pub fn extract_program(root: &GrammarASTNode) -> Result<Program, TwigParseError> {
    if root.rule_name != "program" {
        let (line, column) = pos(root);
        return Err(TwigParseError {
            message: format!("expected 'program' root, got {:?}", root.rule_name),
            line,
            column,
        });
    }
    let mut forms = Vec::new();
    let mut module_info: Option<ModuleInfo> = None;

    for child in ast_children(root) {
        match child.rule_name.as_str() {
            "module_form" => {
                // TW04 / LANG48: extract the (module …) declaration.
                // If multiple module forms appear, the last one wins
                // (semantic duplicate-check is left to a later pass).
                module_info = Some(extract_module_form(child)?);
            }
            "form" => {
                forms.push(extract_form(child, 0)?);
            }
            other => {
                let (line, column) = pos(child);
                return Err(TwigParseError {
                    message: format!(
                        "unexpected program child: {other:?}; expected 'module_form' or 'form'"
                    ),
                    line,
                    column,
                });
            }
        }
    }
    Ok(Program { forms, module_info })
}

// ---------------------------------------------------------------------------
// module_form — (module name { module_clause })
// ---------------------------------------------------------------------------

/// Extract a `module_form` node into [`ModuleInfo`].
///
/// Grammar: `LPAREN "module" NAME { module_clause } RPAREN`
///
/// Module clauses:
/// - `export_clause = LPAREN "export" { NAME } RPAREN`
/// - `import_clause = LPAREN "import" { NAME } RPAREN`
/// - `typed_clause  = LPAREN "typed" NAME RPAREN`    (LANG48)
fn extract_module_form(node: &GrammarASTNode) -> Result<ModuleInfo, TwigParseError> {
    let (line, column) = pos(node);
    // The module name is the first NAME token.
    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "(module ...) missing module name".into(),
        line,
        column,
    })?;
    let mut exports: Vec<String> = Vec::new();
    let mut imports: Vec<String> = Vec::new();
    let mut typed_mode: Option<TypedMode> = None;

    for clause_node in ast_children(node) {
        match clause_node.rule_name.as_str() {
            "module_clause" => {
                // Unwrap the single child of module_clause.
                for inner in ast_children(clause_node) {
                    match inner.rule_name.as_str() {
                        "export_clause" => {
                            for t in tokens_named(inner, "NAME") {
                                exports.push(t.value.clone());
                            }
                        }
                        "import_clause" => {
                            for t in tokens_named(inner, "NAME") {
                                imports.push(t.value.clone());
                            }
                        }
                        "typed_clause" => {
                            // (typed strict|lenient|off)
                            if let Some(mode_tok) = first_token_named(inner, "NAME") {
                                typed_mode = Some(match mode_tok.value.as_str() {
                                    "strict"  => TypedMode::Strict,
                                    "lenient" => TypedMode::Lenient,
                                    "off"     => TypedMode::Off,
                                    other => return Err(TwigParseError {
                                        message: format!(
                                            "(typed ...) expects strict, lenient, or off; got {:?}",
                                            other
                                        ),
                                        line: mode_tok.line,
                                        column: mode_tok.column,
                                    }),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    Ok(ModuleInfo {
        name: name_tok.value.clone(),
        typed_mode,
        exports,
        imports,
    })
}

// ---------------------------------------------------------------------------
// form = define | type_alias | record_def | union_def | expr
// ---------------------------------------------------------------------------

fn extract_form(node: &GrammarASTNode, depth: usize) -> Result<Form, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    let inner = ast_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| TwigParseError {
            message: "empty form node".into(),
            line,
            column,
        })?;
    match inner.rule_name.as_str() {
        "define"     => Ok(Form::Define(extract_define(inner, depth + 1)?)),
        "type_alias" => Ok(Form::TypeAlias(extract_type_alias(inner, depth + 1)?)),
        "record_def" => Ok(Form::RecordDef(extract_record_def(inner, depth + 1)?)),
        "union_def"  => Ok(Form::UnionDef(extract_union_def(inner, depth + 1)?)),
        "expr"       => Ok(Form::Expr(extract_expr(inner, depth + 1)?)),
        other => Err(TwigParseError {
            message: format!("unexpected form child: {other:?}"),
            line,
            column,
        }),
    }
}

// ---------------------------------------------------------------------------
// LANG48 / TW05-A — type expression extractor
// ---------------------------------------------------------------------------

/// Convert a `type_annotation` grammar node into a [`TypeExpr`].
///
/// The LANG48 grammar for `type_annotation` is fully recursive:
/// ```text
/// type_annotation = LPAREN { type_annotation } RPAREN | NAME | INTEGER ;
/// ```
///
/// This function handles all three alternatives and recurses for the
/// parenthesised form.
pub fn extract_type_expr(node: &GrammarASTNode, depth: usize) -> Result<TypeExpr, TwigParseError> {
    // Depth guard — belt-and-braces against adversarial input that bypasses the
    // parser's MAX_PAREN_DEPTH pre-scan (e.g. hand-built ASTs or future parser
    // changes).  The check mirrors the pattern used everywhere else in this file.
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;

    // The grammar produces a type_annotation node.  Its children are either:
    //   Case A: a single NAME token
    //   Case B: a single INTEGER token
    //   Case C: LPAREN { type_annotation sub-nodes } RPAREN

    // Check for a leading LPAREN.
    let has_outer_paren = node.children.iter().any(|c| is_token_named(c, "LPAREN"));

    if !has_outer_paren {
        // Case A or B — single token.
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                    return Ok(TypeExpr::Name(t.value.clone()));
                }
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "INTEGER" => {
                    let v: i64 = t.value.parse().map_err(|_| TwigParseError {
                        message: format!("integer {:?} in type expression does not fit in i64", t.value),
                        line: t.line,
                        column: t.column,
                    })?;
                    return Ok(TypeExpr::Int(v));
                }
                _ => {}
            }
        }
        let (line, column) = pos(node);
        return Err(TwigParseError {
            message: "type_annotation: expected NAME or INTEGER".into(),
            line,
            column,
        });
    }

    // Case C — parenthesised: collect sub-nodes as TypeExpr list.
    let mut items: Vec<TypeExpr> = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) if t.effective_type_name() == "LPAREN"
                || t.effective_type_name() == "RPAREN" => {
                // Skip brackets.
            }
            ASTNodeOrToken::Token(t) => {
                // A bare token inside a parenthesised type_annotation.
                // This can happen if the grammar inlines a terminal.
                let (l, c) = tok_pos(t);
                match t.effective_type_name() {
                    "NAME" => items.push(TypeExpr::Name(t.value.clone())),
                    "INTEGER" => {
                        let v: i64 = t.value.parse().map_err(|_| TwigParseError {
                            message: format!("integer {:?} in type expression does not fit in i64", t.value),
                            line: l,
                            column: c,
                        })?;
                        items.push(TypeExpr::Int(v));
                    }
                    _ => {} // skip punctuation
                }
            }
            ASTNodeOrToken::Node(sub) if sub.rule_name == "type_annotation" => {
                items.push(extract_type_expr(sub, depth + 1)?);
            }
            ASTNodeOrToken::Node(_) => {} // skip unrecognised sub-nodes
        }
    }
    Ok(TypeExpr::List(items))
}

// ---------------------------------------------------------------------------
// LANG23 PR 23-E — type annotation extraction (updated for LANG48)
// ---------------------------------------------------------------------------

/// Extract a LANG23 v1 [`TypeAnnotation`] from a `type_annotation` grammar node.
///
/// Strategy: first converts the grammar node to a [`TypeExpr`] using
/// [`extract_type_expr`], then attempts to recognise the three LANG23 forms
/// by structural pattern-matching on the resulting tree:
///
/// 1. Bare name `int`, `any`, `bool` → the three well-known unrefined kinds.
/// 2. `List([Name("Int"), Int(lo), Int(hi)])` → `RangeInt { lo, hi }`.
/// 3. `List([Name("Member"), Name("int"), List([Int(v0), …])])` → `MembershipInt`.
///
/// Anything else (unknown bare names, non-LANG23 compound forms like
/// `(Index source-len)`, `(fn (n) …)`, …) becomes [`TypeAnnotation::Opaque`]
/// for TW05-B to interpret.
///
/// ## Why convert to TypeExpr first?
///
/// The LANG48 grammar replaces the old LANG23 grammar with a fully recursive
/// s-expression rule:
/// ```text
/// type_annotation = LPAREN { type_annotation } RPAREN | NAME | INTEGER ;
/// ```
/// Under this grammar, `(Int 0 128)` produces a tree where `Int`, `0`, and
/// `128` are each wrapped in a `type_annotation` sub-node — there are **no**
/// direct `NAME` or `INTEGER` tokens at the outer level.  Matching directly
/// on tokens would miss them.  Converting to `TypeExpr` normalises both the
/// old and new grammar shapes into one uniform representation before matching.
fn extract_type_annotation(node: &GrammarASTNode) -> Result<TypeAnnotation, TwigParseError> {
    let (line, column) = pos(node);
    // Build the TypeExpr tree first; then pattern-match LANG23 shapes on it.
    // Depth starts at 0 — the parser's MAX_PAREN_DEPTH pre-scan already caps
    // nesting, so fresh-context annotations are safe.  extract_type_expr's own
    // check_depth call provides a belt-and-braces guard for hand-built trees.
    let type_expr = extract_type_expr(node, 0).map_err(|e| TwigParseError {
        message: format!("type annotation error: {}", e.message),
        ..e
    })?;

    match &type_expr {
        // ── Bare names ────────────────────────────────────────────────────
        TypeExpr::Name(n) => match n.as_str() {
            "int"  => Ok(TypeAnnotation::UnrefinedInt),
            "any"  => Ok(TypeAnnotation::Any),
            "bool" => Ok(TypeAnnotation::UnrefinedBool),
            _      => Ok(TypeAnnotation::Opaque(type_expr)),
        },

        // ── Compound forms ────────────────────────────────────────────────
        TypeExpr::List(parts) => {
            // (Int lo hi) — range annotation.
            //
            // Both `lo` and `hi` must be plain integers in the TypeExpr.  If
            // either is a Name (e.g. `(Int 0 _)` where `_` is a wildcard) the
            // match falls through to `Opaque`, which is correct: `(Int 0 _)`
            // is a TW05-A form that the TW05-B checker handles.
            if let [TypeExpr::Name(n), TypeExpr::Int(lo), TypeExpr::Int(hi)] = parts.as_slice() {
                if n == "Int" {
                    return Ok(TypeAnnotation::RangeInt {
                        lo: *lo as i128,
                        hi: *hi as i128,
                    });
                }
            }

            // (Member int (v0 v1 …)) — membership annotation.
            //
            // The inner list must contain only integers (TypeExpr::Int).  If
            // any element is a non-integer the match falls through to `Opaque`.
            if let [TypeExpr::Name(n), TypeExpr::Name(kind), TypeExpr::List(values)] =
                parts.as_slice()
            {
                if n == "Member" && kind == "int" {
                    let int_values: Option<Vec<i128>> = values
                        .iter()
                        .map(|v| match v {
                            TypeExpr::Int(i) => Some(*i as i128),
                            _ => None,
                        })
                        .collect();
                    if let Some(int_values) = int_values {
                        check_membership_int_count(int_values.len(), line, column)?;
                        return Ok(TypeAnnotation::MembershipInt { values: int_values });
                    }
                }
            }

            // Any other parenthesised form → opaque.
            Ok(TypeAnnotation::Opaque(type_expr))
        }

        // ── Bare integer in annotation position → opaque. ─────────────────
        TypeExpr::Int(_) => Ok(TypeAnnotation::Opaque(type_expr)),
    }
}

// ---------------------------------------------------------------------------
// LANG48 — type alias extractor
// ---------------------------------------------------------------------------

/// Extract a `type_alias` node: `(type Name type_annotation)`.
fn extract_type_alias(node: &GrammarASTNode, depth: usize) -> Result<TypeAlias, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;

    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "(type ...) missing alias name".into(),
        line,
        column,
    })?;

    let ann_node = ast_children(node)
        .into_iter()
        .find(|c| c.rule_name == "type_annotation")
        .ok_or_else(|| TwigParseError {
            message: "(type ...) missing type expression".into(),
            line,
            column,
        })?;

    let expr = extract_type_expr(ann_node, depth + 1)?;

    Ok(TypeAlias {
        name: name_tok.value.clone(),
        expr,
        line,
        column,
    })
}

// ---------------------------------------------------------------------------
// LANG48 — record definition extractor
// ---------------------------------------------------------------------------

/// Extract a `record_def` node: `(record Name { record_field })`.
fn extract_record_def(node: &GrammarASTNode, depth: usize) -> Result<RecordDef, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;

    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "(record ...) missing type name".into(),
        line,
        column,
    })?;

    let mut fields: Vec<RecordField> = Vec::new();
    for child in ast_children(node) {
        if child.rule_name == "record_field" {
            fields.push(extract_record_field(child)?);
        }
    }

    Ok(RecordDef {
        name: name_tok.value.clone(),
        fields,
        line,
        column,
    })
}

/// Extract a `record_field` node: `(NAME COLON type_annotation)`.
fn extract_record_field(node: &GrammarASTNode) -> Result<RecordField, TwigParseError> {
    let (line, column) = pos(node);

    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "record field missing field name".into(),
        line,
        column,
    })?;

    let ann_node = ast_children(node)
        .into_iter()
        .find(|c| c.rule_name == "type_annotation")
        .ok_or_else(|| TwigParseError {
            message: format!(
                "record field {:?} missing type annotation",
                name_tok.value
            ),
            line,
            column,
        })?;

    Ok(RecordField {
        name: name_tok.value.clone(),
        type_annotation: extract_type_annotation(ann_node)?,
    })
}

// ---------------------------------------------------------------------------
// LANG48 — union definition extractor
// ---------------------------------------------------------------------------

/// Extract a `union_def` node: `(union Name { union_variant })`.
fn extract_union_def(node: &GrammarASTNode, depth: usize) -> Result<UnionDef, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;

    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "(union ...) missing type name".into(),
        line,
        column,
    })?;

    let mut variants: Vec<UnionVariant> = Vec::new();
    for child in ast_children(node) {
        if child.rule_name == "union_variant" {
            variants.push(extract_union_variant(child)?);
        }
    }

    Ok(UnionDef {
        name: name_tok.value.clone(),
        variants,
        line,
        column,
    })
}

/// Extract a `union_variant` node: `(NAME { record_field })`.
fn extract_union_variant(node: &GrammarASTNode) -> Result<UnionVariant, TwigParseError> {
    let (line, column) = pos(node);

    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "union variant missing variant name".into(),
        line,
        column,
    })?;

    let mut fields: Vec<RecordField> = Vec::new();
    for child in ast_children(node) {
        if child.rule_name == "record_field" {
            fields.push(extract_record_field(child)?);
        }
    }

    Ok(UnionVariant {
        name: name_tok.value.clone(),
        fields,
    })
}

// ---------------------------------------------------------------------------
// define = LPAREN "define" name_or_signature expr { expr } RPAREN
//
// name_or_signature = NAME [ COLON type_annotation ]
//                   | LPAREN NAME { typed_param } [ NAME type_annotation ] RPAREN
//
// LANG23 PR 23-E: the signature may carry per-parameter type annotations
// and/or a return-type annotation.
// ---------------------------------------------------------------------------

fn extract_define(node: &GrammarASTNode, depth: usize) -> Result<Define, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    let children = ast_children(node);
    let sig_node = children.first().ok_or_else(|| TwigParseError {
        message: "(define ...) missing signature".into(),
        line,
        column,
    })?;
    if sig_node.rule_name != "name_or_signature" {
        return Err(TwigParseError {
            message: format!(
                "expected 'name_or_signature', got {:?}",
                sig_node.rule_name
            ),
            line,
            column,
        });
    }

    // The body expressions are the expr children of the define node
    // (i.e. all ast_children except the sig_node at index 0).
    let body_exprs: Vec<Expr> = children
        .iter()
        .skip(1)
        .map(|c| extract_expr(c, depth + 1))
        .collect::<Result<_, _>>()?;
    if body_exprs.is_empty() {
        return Err(TwigParseError {
            message: "(define ...) must have a body expression".into(),
            line,
            column,
        });
    }

    // ── Distinguish value-define vs function-sugar ───────────────────────
    //
    // The signature node wraps one of:
    //   NAME [ COLON type_annotation ]                          — value-define
    //   LPAREN NAME { typed_param } [ ARROW type_annotation ] RPAREN — fn-sugar
    //
    // We detect the function form by looking for an LPAREN token among the
    // sig_node's *direct* children (i.e. the LPAREN that opens the param
    // list, not any nested parens inside `typed_param` sub-nodes).

    let has_paren = sig_node
        .children
        .iter()
        .any(|c| is_token_named(c, "LPAREN"));

    if !has_paren {
        // ── Value-define: (define name expr) or (define name : Type expr) ──

        if body_exprs.len() != 1 {
            return Err(TwigParseError {
                message: "(define name expr) takes exactly one body expression — \
                          use (define (name args...) body+) for multi-expression bodies"
                    .into(),
                line,
                column,
            });
        }
        let name_tok = first_token_named(sig_node, "NAME").ok_or_else(|| TwigParseError {
            message: "(define ...) missing a name".into(),
            line,
            column,
        })?;

        // Check for an optional `: type_annotation` sub-node.
        let type_annotation = sig_node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "type_annotation" => Some(n),
                _ => None,
            })
            .map(|ann_node| extract_type_annotation(ann_node))
            .transpose()?;

        let expr = body_exprs.into_iter().next().unwrap();
        return Ok(Define {
            name: name_tok.value.clone(),
            type_annotation,
            expr,
            line,
            column,
        });
    }

    // ── Function-sugar: (define (name typed_param...) body+) ────────────
    //
    // The sig_node's children (after stripping LPARENs/RPARENs) are:
    //   NAME           — the function name (first NAME token in sig_node)
    //   typed_param*   — zero or more sub-nodes (each is a typed_param)
    //   ARROW? type_annotation?  — optional `->` arrow + return annotation
    //                              (`->` is now a dedicated ARROW token, not a NAME)
    //
    // We walk sig_node's children in order:
    //   1. Skip the outer LPAREN token.
    //   2. Take the first NAME token as the function name.
    //   3. For each following child:
    //      - If it's a `typed_param` node → extract param name + optional annotation.
    //      - If it's an ARROW token and the next sibling is a `type_annotation`
    //        node → extract the return annotation.
    //      - If it's RPAREN → stop.

    let (fn_name, params, param_annotations, return_annotation) =
        extract_fn_signature(sig_node)?;

    let (sig_line, sig_col) = {
        // Source position from the first NAME token (the fn name).
        let name_tok = first_token_named(sig_node, "NAME").unwrap();
        tok_pos(name_tok)
    };

    let lam = Lambda {
        params,
        param_annotations,
        return_annotation,
        body: body_exprs,
        line: sig_line,
        column: sig_col,
    };
    Ok(Define {
        name: fn_name,
        type_annotation: None, // function defines carry annotation inside Lambda
        expr: Expr::Lambda(lam),
        line,
        column,
    })
}

/// Walk the children of a `name_or_signature` node that has a leading LPAREN
/// (i.e. the function-definition sugar form).  Returns
/// `(fn_name, params, param_annotations, return_annotation)`.
///
/// Child structure (after the outer LPAREN has been matched by the grammar):
/// ```text
/// LPAREN  NAME  { typed_param }  [ ARROW type_annotation ]  RPAREN
/// ```
/// - `LPAREN` and `RPAREN` are punctuation tokens — we skip them.
/// - The first `NAME` token is the function name.
/// - `typed_param` sub-nodes carry the parameters (each may or may not have
///   a type annotation).
/// - An optional trailing `ARROW type_annotation` pair encodes `-> RetType`.
///
/// `->` is now a dedicated ARROW token (not a NAME whose value happens to be
/// `"->"`).  Defining ARROW as an exact literal before the NAME pattern in
/// `twig.tokens` prevents the `{ typed_param }` repetition from consuming
/// the arrow as a bare-NAME parameter, which previously caused a
/// "Expected COLON, got '0'" parse error when the type annotation followed.
fn extract_fn_signature(
    sig_node: &GrammarASTNode,
) -> Result<(String, Vec<String>, Vec<Option<TypeAnnotation>>, Option<TypeAnnotation>), TwigParseError> {
    let (line, column) = pos(sig_node);

    let mut fn_name: Option<String> = None;
    let mut params: Vec<String> = Vec::new();
    let mut param_annotations: Vec<Option<TypeAnnotation>> = Vec::new();
    let mut return_annotation: Option<TypeAnnotation> = None;

    // We need to peek ahead to detect the `-> type_annotation` trailer.
    // Build a flat list of children so we can index into them.
    let children: Vec<&ASTNodeOrToken> = sig_node.children.iter().collect();
    let n = children.len();
    let mut i = 0;

    while i < n {
        let child = children[i];
        match child {
            ASTNodeOrToken::Token(t) => {
                let kind = t.effective_type_name();
                match kind {
                    "LPAREN" | "RPAREN" => { i += 1; } // punctuation — skip
                    "KEYWORD" if t.value == "define" => { i += 1; } // in case define keyword leaks through
                    "ARROW" => {
                        // LANG23 PR 23-E: `->` is now a dedicated ARROW token
                        // (was previously a NAME whose value == "->").
                        // The next sibling must be a `type_annotation` sub-node.
                        i += 1;
                        if i < n {
                            if let ASTNodeOrToken::Node(ann_node) = children[i] {
                                if ann_node.rule_name == "type_annotation" {
                                    return_annotation =
                                        Some(extract_type_annotation(ann_node)?);
                                    i += 1;
                                    continue;
                                }
                            }
                        }
                        return Err(TwigParseError {
                            message: "'->' in function signature must be followed by a type annotation".into(),
                            line: t.line,
                            column: t.column,
                        });
                    }
                    "NAME" => {
                        if fn_name.is_none() {
                            // First NAME after the opening LPAREN → function name.
                            fn_name = Some(t.value.clone());
                            i += 1;
                        } else {
                            // A bare NAME parameter (unannotated).
                            params.push(t.value.clone());
                            param_annotations.push(None);
                            i += 1;
                        }
                    }
                    _ => { i += 1; }
                }
            }
            ASTNodeOrToken::Node(n_node) => {
                match n_node.rule_name.as_str() {
                    "typed_param" => {
                        let (param_name, ann) = extract_typed_param(n_node)?;
                        params.push(param_name);
                        param_annotations.push(ann);
                        i += 1;
                    }
                    "type_annotation" => {
                        // A standalone type_annotation without a preceding ARROW.
                        // This should not normally appear at this level — skip it
                        // to stay robust against future grammar extensions.
                        i += 1;
                    }
                    _ => { i += 1; }
                }
            }
        }
    }

    let fn_name = fn_name.ok_or_else(|| TwigParseError {
        message: "(define ...) missing function name in signature".into(),
        line,
        column,
    })?;
    Ok((fn_name, params, param_annotations, return_annotation))
}

/// Extract one `typed_param` grammar node.
///
/// A `typed_param` is either:
/// - `NAME` (bare, unannotated): returns `(name, None)`
/// - `LPAREN NAME COLON type_annotation RPAREN` (annotated): returns `(name, Some(ann))`
fn extract_typed_param(
    node: &GrammarASTNode,
) -> Result<(String, Option<TypeAnnotation>), TwigParseError> {
    let (line, column) = pos(node);
    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "typed_param: missing parameter name".into(),
        line,
        column,
    })?;
    let ann = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "type_annotation" => Some(n),
            _ => None,
        })
        .map(|ann_node| extract_type_annotation(ann_node))
        .transpose()?;
    Ok((name_tok.value.clone(), ann))
}

// ---------------------------------------------------------------------------
// LANG48 — match form extractor
// ---------------------------------------------------------------------------

/// Extract a `match_form` node: `(match expr { match_arm })`.
fn extract_match(node: &GrammarASTNode, depth: usize) -> Result<Match, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;

    // The first `expr` child is the scrutinee; subsequent `match_arm` children
    // are the arms.
    let children = ast_children(node);
    let scrutinee_node = children.iter()
        .find(|c| c.rule_name == "expr")
        .ok_or_else(|| TwigParseError {
            message: "(match ...) missing scrutinee expression".into(),
            line,
            column,
        })?;
    let scrutinee = Box::new(extract_expr(scrutinee_node, depth + 1)?);

    let mut arms: Vec<MatchArm> = Vec::new();
    for child in &children {
        if child.rule_name == "match_arm" {
            arms.push(extract_match_arm(child, depth + 1)?);
        }
    }

    if arms.is_empty() {
        return Err(TwigParseError {
            message: "(match ...) needs at least one arm".into(),
            line,
            column,
        });
    }

    Ok(Match { scrutinee, arms, line, column })
}

/// Extract a `match_arm` node: `(match_pat expr { expr })`.
fn extract_match_arm(node: &GrammarASTNode, depth: usize) -> Result<MatchArm, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;

    let children = ast_children(node);

    let pat_node = children.iter()
        .find(|c| c.rule_name == "match_pat")
        .ok_or_else(|| TwigParseError {
            message: "match arm missing pattern".into(),
            line,
            column,
        })?;
    let pat = extract_match_pat(pat_node)?;

    let body: Vec<Expr> = children.iter()
        .filter(|c| c.rule_name == "expr")
        .map(|c| extract_expr(c, depth + 1))
        .collect::<Result<_, _>>()?;

    if body.is_empty() {
        return Err(TwigParseError {
            message: "match arm missing body expression".into(),
            line,
            column,
        });
    }

    Ok(MatchArm { pat, body })
}

/// Extract a `match_pat` node.
///
/// Grammar:
/// ```text
/// match_pat = LPAREN NAME { NAME } RPAREN | NAME ;
/// ```
fn extract_match_pat(node: &GrammarASTNode) -> Result<MatchPat, TwigParseError> {
    let (line, column) = pos(node);
    let has_paren = node.children.iter().any(|c| is_token_named(c, "LPAREN"));

    if !has_paren {
        // Bare NAME — wildcard or binding.
        let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
            message: "match_pat: expected NAME".into(),
            line,
            column,
        })?;
        return Ok(if name_tok.value == "_" {
            MatchPat::Wildcard
        } else {
            MatchPat::Binding(name_tok.value.clone())
        });
    }

    // Parenthesised — variant pattern: (VariantName b1 b2 ...).
    let name_toks: Vec<&Token> = tokens_named(node, "NAME");
    if name_toks.is_empty() {
        return Err(TwigParseError {
            message: "match_pat: variant pattern missing constructor name".into(),
            line,
            column,
        });
    }
    let variant_name = name_toks[0].value.clone();
    let bindings: Vec<String> = name_toks[1..].iter().map(|t| t.value.clone()).collect();
    Ok(MatchPat::Variant { name: variant_name, bindings })
}

// ---------------------------------------------------------------------------
// expr = atom | quoted | compound
// ---------------------------------------------------------------------------

fn extract_expr(node: &GrammarASTNode, depth: usize) -> Result<Expr, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    if node.rule_name != "expr" {
        return Err(TwigParseError {
            message: format!("expected 'expr', got {:?}", node.rule_name),
            line,
            column,
        });
    }
    let inner = ast_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| TwigParseError {
            message: "empty expr node".into(),
            line,
            column,
        })?;
    match inner.rule_name.as_str() {
        "atom" => extract_atom(inner),
        "quoted" => Ok(Expr::SymLit(extract_quoted(inner)?)),
        "compound" => extract_compound(inner, depth + 1),
        other => Err(TwigParseError {
            message: format!("unexpected expr child: {other:?}"),
            line,
            column,
        }),
    }
}

// ---------------------------------------------------------------------------
// atom = INTEGER | BOOL_TRUE | BOOL_FALSE | "nil" | NAME
// ---------------------------------------------------------------------------

fn extract_atom(node: &GrammarASTNode) -> Result<Expr, TwigParseError> {
    let (line, column) = pos(node);
    // Atom children are bare tokens, not nested nodes.
    let tok = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t),
            ASTNodeOrToken::Node(_) => None,
        })
        .ok_or_else(|| TwigParseError {
            message: "empty atom".into(),
            line,
            column,
        })?;
    let (l, c) = tok_pos(tok);
    let name = tok.effective_type_name();
    match name {
        "INTEGER" => {
            let value: i64 = tok.value.parse().map_err(|_| TwigParseError {
                message: format!("integer literal {:?} does not fit in i64", tok.value),
                line: l,
                column: c,
            })?;
            Ok(Expr::IntLit(IntLit { value, line: l, column: c }))
        }
        "BOOL_TRUE" => Ok(Expr::BoolLit(BoolLit { value: true, line: l, column: c })),
        "BOOL_FALSE" => Ok(Expr::BoolLit(BoolLit { value: false, line: l, column: c })),
        "KEYWORD" if tok.value == "nil" => Ok(Expr::NilLit(NilLit { line: l, column: c })),
        "NAME" => Ok(Expr::VarRef(VarRef { name: tok.value.clone(), line: l, column: c })),
        // LANG51: double-quoted string literals.
        //
        // The lexer preserves the surrounding quotes in `tok.value`; the
        // extractor strips them and decodes the LANG51 escape sequences:
        //   \\  →  \
        //   \"  →  "
        //   \n  →  newline (U+000A)
        //   \t  →  tab (U+0009)
        //   \r  →  carriage return (U+000D)
        //
        // Any other `\x` sequence is a parse error — better to fail loudly
        // now than to silently misinterpret the program.
        "STRING" => {
            // The GrammarLexer automatically handles both quote-stripping AND
            // escape-sequence decoding before the token reaches the parser:
            //
            //   1. It strips the surrounding `"` characters (quote_len = 1).
            //   2. It runs `process_escapes` on the inner content, which converts:
            //         \\  →  \       \n  →  newline     \t  →  tab
            //         \r  →  CR      \"  →  "           \'  →  '
            //      Unknown sequences `\x` are passed through as just `x`.
            //
            // So `tok.value` is ALREADY the fully-decoded string content.
            // Example: Twig source `"say \"hi\"\n"` arrives here as `say "hi"\n`
            // (with an actual newline byte, not the two-char sequence `\n`).
            //
            // We therefore use `tok.value` directly — no second decoding pass.
            Ok(Expr::StrLit(crate::ast_nodes::StrLit {
                value: tok.value.clone(),
                line: l,
                column: c,
            }))
        }
        other => Err(TwigParseError {
            message: format!(
                "unexpected atom token: type={other:?} value={:?}",
                tok.value
            ),
            line: l,
            column: c,
        }),
    }
}

// ---------------------------------------------------------------------------
// quoted = QUOTE NAME
// ---------------------------------------------------------------------------

fn extract_quoted(node: &GrammarASTNode) -> Result<SymLit, TwigParseError> {
    let (line, column) = pos(node);
    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "expected NAME after '".into(),
        line,
        column,
    })?;
    Ok(SymLit { name: name_tok.value.clone(), line, column })
}

// ---------------------------------------------------------------------------
// compound = if_form | let_form | begin_form | lambda_form | quote_form
//          | match_form | apply
// ---------------------------------------------------------------------------

fn extract_compound(node: &GrammarASTNode, depth: usize) -> Result<Expr, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    let inner = ast_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| TwigParseError {
            message: "empty compound node".into(),
            line,
            column,
        })?;
    match inner.rule_name.as_str() {
        "if_form"     => Ok(Expr::If(extract_if(inner, depth + 1)?)),
        "let_form"    => Ok(Expr::Let(extract_let(inner, depth + 1)?)),
        "begin_form"  => Ok(Expr::Begin(extract_begin(inner, depth + 1)?)),
        "lambda_form" => Ok(Expr::Lambda(extract_lambda(inner, depth + 1)?)),
        "quote_form"  => Ok(Expr::SymLit(extract_quote_form(inner)?)),
        "match_form"  => Ok(Expr::Match(extract_match(inner, depth + 1)?)),
        "apply"       => Ok(Expr::Apply(extract_apply(inner, depth + 1)?)),
        other => Err(TwigParseError {
            message: format!("unexpected compound child: {other:?}"),
            line,
            column,
        }),
    }
}

fn extract_if(node: &GrammarASTNode, depth: usize) -> Result<If, TwigParseError> {
    let (line, column) = pos(node);
    let exprs: Vec<Expr> = ast_children(node)
        .into_iter()
        .filter(|c| c.rule_name == "expr")
        .map(|c| extract_expr(c, depth + 1))
        .collect::<Result<_, _>>()?;
    if exprs.len() != 3 {
        return Err(TwigParseError {
            message: "(if ...) takes exactly 3 expressions".into(),
            line,
            column,
        });
    }
    let mut it = exprs.into_iter();
    let cond = Box::new(it.next().unwrap());
    let then_branch = Box::new(it.next().unwrap());
    let else_branch = Box::new(it.next().unwrap());
    Ok(If { cond, then_branch, else_branch, line, column })
}

fn extract_let(node: &GrammarASTNode, depth: usize) -> Result<Let, TwigParseError> {
    let (line, column) = pos(node);
    let mut bindings: Vec<(String, Expr)> = Vec::new();
    let mut body: Vec<Expr> = Vec::new();
    for child in ast_children(node) {
        match child.rule_name.as_str() {
            "binding" => bindings.push(extract_binding(child, depth + 1)?),
            "expr" => body.push(extract_expr(child, depth + 1)?),
            _ => {}
        }
    }
    if body.is_empty() {
        return Err(TwigParseError {
            message: "(let (...) ...) needs at least one body expression".into(),
            line,
            column,
        });
    }
    Ok(Let { bindings, body, line, column })
}

fn extract_binding(
    node: &GrammarASTNode,
    depth: usize,
) -> Result<(String, Expr), TwigParseError> {
    let (line, column) = pos(node);
    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "malformed binding — expected (name expr)".into(),
        line,
        column,
    })?;
    let expr_node = ast_children(node)
        .into_iter()
        .find(|c| c.rule_name == "expr")
        .ok_or_else(|| TwigParseError {
            message: "malformed binding — missing expression".into(),
            line,
            column,
        })?;
    let expr = extract_expr(expr_node, depth + 1)?;
    Ok((name_tok.value.clone(), expr))
}

fn extract_begin(node: &GrammarASTNode, depth: usize) -> Result<Begin, TwigParseError> {
    let (line, column) = pos(node);
    let exprs: Vec<Expr> = ast_children(node)
        .into_iter()
        .filter(|c| c.rule_name == "expr")
        .map(|c| extract_expr(c, depth + 1))
        .collect::<Result<_, _>>()?;
    if exprs.is_empty() {
        return Err(TwigParseError {
            message: "(begin ...) needs at least one expression".into(),
            line,
            column,
        });
    }
    Ok(Begin { exprs, line, column })
}

fn extract_lambda(node: &GrammarASTNode, depth: usize) -> Result<Lambda, TwigParseError> {
    let (line, column) = pos(node);
    // Walk children left-to-right: collect NAMEs (params) before the
    // first nested `expr` ASTNode, then collect `expr` ASTNodes as body.
    //
    // Anonymous lambdas (`(lambda ...)`) do not support type annotations in
    // the v1 grammar — only `define` function sugar does.  So `typed_param`
    // sub-nodes will never appear here, and `param_annotations` is all None.
    let mut params: Vec<String> = Vec::new();
    let mut body: Vec<Expr> = Vec::new();
    let mut seen_first_expr = false;
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(n) if n.rule_name == "expr" => {
                body.push(extract_expr(n, depth + 1)?);
                seen_first_expr = true;
            }
            ASTNodeOrToken::Token(t)
                if !seen_first_expr && t.effective_type_name() == "NAME" =>
            {
                params.push(t.value.clone());
            }
            _ => {}
        }
    }
    if body.is_empty() {
        return Err(TwigParseError {
            message: "(lambda (...) ...) needs at least one body expression".into(),
            line,
            column,
        });
    }
    let param_count = params.len();
    Ok(Lambda {
        params,
        param_annotations: vec![None; param_count], // anonymous lambdas are unannotated
        return_annotation: None,
        body,
        line,
        column,
    })
}

fn extract_quote_form(node: &GrammarASTNode) -> Result<SymLit, TwigParseError> {
    let (line, column) = pos(node);
    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "(quote ...) needs a name".into(),
        line,
        column,
    })?;
    Ok(SymLit { name: name_tok.value.clone(), line, column })
}

fn extract_apply(node: &GrammarASTNode, depth: usize) -> Result<Apply, TwigParseError> {
    let (line, column) = pos(node);
    let exprs: Vec<Expr> = ast_children(node)
        .into_iter()
        .filter(|c| c.rule_name == "expr")
        .map(|c| extract_expr(c, depth + 1))
        .collect::<Result<_, _>>()?;
    if exprs.is_empty() {
        return Err(TwigParseError {
            message: "empty application '()' — use 'nil' for the empty list".into(),
            line,
            column,
        });
    }
    let mut it = exprs.into_iter();
    let fn_expr = Box::new(it.next().unwrap());
    let args: Vec<Expr> = it.collect();
    Ok(Apply { fn_expr, args, line, column })
}

// ---------------------------------------------------------------------------
// Design note — LANG51 string escape handling
// ---------------------------------------------------------------------------
//
// String escape sequences (`\\`, `\"`, `\n`, `\t`, `\r`) are decoded by the
// GrammarLexer's `process_escapes` helper BEFORE the token is handed to the
// parser.  The GrammarLexer also strips the surrounding double-quote delimiters.
// Therefore `tok.value` for a STRING token already contains the fully-decoded
// content, and `extract_atom` uses it directly without a second decode pass.
//
// The set of supported sequences is defined in `lexer/src/grammar_lexer.rs`
// (`process_escapes`).  As of LANG51: `\n`, `\t`, `\r`, `\\`, `\"`, `\'`.
// Unknown sequences `\x` are silently passed through as just `x` — future
// LANGs can add stricter validation here if needed.
