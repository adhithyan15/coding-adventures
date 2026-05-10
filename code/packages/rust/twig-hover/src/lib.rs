//! # `twig-hover` — LSP hover-info extraction for Twig.
//!
//! Given a parsed [`twig_parser::Program`] and a `(line, column)`
//! cursor position, returns the symbol under the cursor — name,
//! kind, and signature (for functions).  Drives the editor's
//! "what's this?" tooltip (LSP `textDocument/hover`).
//!
//! The fifth piece of the **Twig authoring-experience layer**
//! (alongside [`twig-formatter`](../twig-formatter/),
//! [`twig-semantic-tokens`](../twig-semantic-tokens/),
//! [`twig-document-symbols`](../twig-document-symbols/),
//! [`twig-folding-ranges`](../twig-folding-ranges/)).
//!
//! ## What surfaces in hover
//!
//! | Cursor on…                | Hover shows                                     |
//! |---------------------------|-------------------------------------------------|
//! | A `VarRef` whose name binds a top-level `(define …)` | The define's signature (function or value) |
//! | A `VarRef` with no matching define | "Variable" + name (parameter / unknown) |
//! | A `BoolLit` / `NilLit` / `IntLit` / `SymLit` | The literal kind + value |
//! | A keyword (`if`/`let`/…)  | "Keyword" + the form name (best-effort)         |
//! | Anywhere else             | `None`                                          |
//!
//! ## Public API
//!
//! - [`hover_at(source, line, column)`] — `&str` + position →
//!   `Result<Option<Hover>, TwigParseError>`.  The common case.
//! - [`hover_for_program(program, line, column)`] — already-parsed
//!   `&Program` + position → `Option<Hover>`.
//!
//! ## Position model
//!
//! All positions are **1-based** `(line, column)` matching
//! `twig-parser`.  A token at `(line, col)` with length `len`
//! "contains" cursor `(L, C)` iff `L == line` and
//! `col <= C <= col + len`.  (The trailing `=` is intentional — a
//! cursor sitting just past the last character of an identifier
//! still counts as "on" it, the prettier convention.)
//!
//! ## What this crate does NOT do
//!
//! - **No type information.**  Twig has no type-checker yet.
//!   When one ships, hover gains an inferred-type field.
//! - **No documentation comments.**  The Twig lexer is comment-
//!   stripping; doc comments don't survive into the AST.  Lands
//!   when the lexer grows a trivia channel.
//! - **No LSP wire encoding.**  Returns a typed `Option<Hover>`;
//!   the JSON `Hover` shape is one level up.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use twig_document_symbols::{symbols_for_program, DocumentSymbol, SymbolKind};
use twig_parser::{
    parse, Apply, Begin, BoolLit, Expr, Form, If, IntLit, Lambda, Let, NilLit, Program, SymLit,
    TwigParseError, VarRef,
};

// ---------------------------------------------------------------------------
// HoverKind
// ---------------------------------------------------------------------------

/// Classification of what's under the cursor.
///
/// `#[non_exhaustive]` — future variants (e.g. `Type` once a
/// type-checker exists, `Constant` for theme distinctions) won't
/// break consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HoverKind {
    /// Defined function — usually has a `signature` filled in.
    Function,
    /// Defined value — the cursor sits on a `VarRef` whose name
    /// binds a top-level `(define name expr)` with a non-lambda
    /// `expr`.
    Variable,
    /// Variable reference with no matching top-level `define` —
    /// could be a parameter, a `let` binding, or a typo.  Hover
    /// shows the name without further info.
    UnresolvedVariable,
    /// Boolean literal (`#t` / `#f`).
    Boolean,
    /// `nil` literal.
    Nil,
    /// Integer literal.
    Number,
    /// Quoted-symbol literal (`'foo`).
    Symbol,
    /// Built-in keyword (`if` / `let` / `lambda` / `begin` /
    /// `define`).
    Keyword,
}

impl HoverKind {
    /// Stable lowercase mnemonic.
    pub fn mnemonic(self) -> &'static str {
        match self {
            HoverKind::Function => "function",
            HoverKind::Variable => "variable",
            HoverKind::UnresolvedVariable => "unresolved-variable",
            HoverKind::Boolean => "boolean",
            HoverKind::Nil => "nil",
            HoverKind::Number => "number",
            HoverKind::Symbol => "symbol",
            HoverKind::Keyword => "keyword",
        }
    }
}

impl fmt::Display for HoverKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mnemonic())
    }
}

// ---------------------------------------------------------------------------
// Hover
// ---------------------------------------------------------------------------

/// Hover info returned for a cursor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    /// Classification.
    pub kind: HoverKind,
    /// Display name (identifier, literal text, keyword spelling).
    pub name: String,
    /// Optional one-line summary (e.g. lambda parameter signature).
    pub signature: Option<String>,
    /// 1-based line where the hovered token starts.
    pub line: u32,
    /// 1-based column where the hovered token starts.
    pub column: u32,
    /// Token length in monospace cells.
    pub length: u32,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse `source` and return hover info at `(line, column)` if any
/// token sits there, otherwise `Ok(None)`.
pub fn hover_at(
    source: &str,
    line: u32,
    column: u32,
) -> Result<Option<Hover>, TwigParseError> {
    let program = parse(source)?;
    Ok(hover_for_program(&program, line, column))
}

/// Walk an already-parsed `Program` and return hover info at
/// `(line, column)` if any token sits there.
pub fn hover_for_program(program: &Program, line: u32, column: u32) -> Option<Hover> {
    // Build a symbol table from top-level defines so VarRefs can
    // be resolved to function/variable signatures.
    let symbols = symbols_for_program(program);

    // Walk every token and find the one that contains the cursor.
    let mut found: Option<Hover> = None;
    for form in &program.forms {
        visit_form(form, line, column, &symbols, &mut found);
    }
    found
}

// ---------------------------------------------------------------------------
// Walker — emits a Hover when it finds a token containing (line, column)
// ---------------------------------------------------------------------------

fn visit_form(
    form: &Form,
    line: u32,
    column: u32,
    symbols: &[DocumentSymbol],
    found: &mut Option<Hover>,
) {
    match form {
        Form::Define(d) => {
            // The keyword `define` sits at (line, column + 1) past
            // the open paren.  We surface it as Keyword.
            try_keyword(found, line, column, u32_of(d.line), u32_of(d.column).saturating_add(1), "define");
            // The bound name itself sits at (line, column + 8) — best-
            // effort guess (after `(define `).  Surface as Function or
            // Variable depending on what `expr` is.
            let name_kind = match &d.expr {
                Expr::Lambda(_) => HoverKind::Function,
                _ => HoverKind::Variable,
            };
            let signature = match &d.expr {
                Expr::Lambda(Lambda { params, .. }) => Some(format_param_list(params)),
                _ => None,
            };
            try_token(
                found,
                line,
                column,
                Hover {
                    kind: name_kind,
                    name: d.name.clone(),
                    signature,
                    line: u32_of(d.line),
                    column: u32_of(d.column).saturating_add(8),
                    length: len_u32(&d.name),
                },
            );
            visit_expr(&d.expr, line, column, symbols, found);
        }
        Form::Expr(e) => visit_expr(e, line, column, symbols, found),
    }
}

fn visit_expr(
    expr: &Expr,
    line: u32,
    column: u32,
    symbols: &[DocumentSymbol],
    found: &mut Option<Hover>,
) {
    match expr {
        Expr::IntLit(IntLit { value, line: l, column: c }) => {
            try_token(
                found,
                line,
                column,
                Hover {
                    kind: HoverKind::Number,
                    name: value.to_string(),
                    signature: None,
                    line: u32_of(*l),
                    column: u32_of(*c),
                    length: len_u32(&value.to_string()),
                },
            );
        }
        Expr::BoolLit(BoolLit { value, line: l, column: c }) => {
            let name = if *value { "#t" } else { "#f" };
            try_token(
                found,
                line,
                column,
                Hover {
                    kind: HoverKind::Boolean,
                    name: name.to_string(),
                    signature: None,
                    line: u32_of(*l),
                    column: u32_of(*c),
                    length: 2,
                },
            );
        }
        Expr::NilLit(NilLit { line: l, column: c }) => {
            try_token(
                found,
                line,
                column,
                Hover {
                    kind: HoverKind::Nil,
                    name: "nil".into(),
                    signature: None,
                    line: u32_of(*l),
                    column: u32_of(*c),
                    length: 3,
                },
            );
        }
        Expr::SymLit(SymLit { name, line: l, column: c }) => {
            let display = format!("'{name}");
            let len = len_u32(&display);
            try_token(
                found,
                line,
                column,
                Hover {
                    kind: HoverKind::Symbol,
                    name: display,
                    signature: None,
                    line: u32_of(*l),
                    column: u32_of(*c),
                    length: len,
                },
            );
        }
        Expr::VarRef(VarRef { name, line: l, column: c }) => {
            // Resolve via symbol table: if the name binds a top-level
            // define, surface its kind + signature.  Otherwise it's
            // unresolved (parameter / let-binding / typo).
            let (kind, signature) = match resolve_symbol(name, symbols) {
                Some(sym) => (
                    match sym.kind {
                        SymbolKind::Function => HoverKind::Function,
                        SymbolKind::Variable => HoverKind::Variable,
                        // SymbolKind is `#[non_exhaustive]`; future
                        // variants surface as Variable (the safest
                        // default) until this crate is updated.
                        _ => HoverKind::Variable,
                    },
                    sym.detail.clone(),
                ),
                None => (HoverKind::UnresolvedVariable, None),
            };
            try_token(
                found,
                line,
                column,
                Hover {
                    kind,
                    name: name.clone(),
                    signature,
                    line: u32_of(*l),
                    column: u32_of(*c),
                    length: len_u32(name),
                },
            );
        }
        Expr::If(If { cond, then_branch, else_branch, line: l, column: c }) => {
            try_keyword(
                found,
                line,
                column,
                u32_of(*l),
                u32_of(*c).saturating_add(1),
                "if",
            );
            visit_expr(cond, line, column, symbols, found);
            visit_expr(then_branch, line, column, symbols, found);
            visit_expr(else_branch, line, column, symbols, found);
        }
        Expr::Let(Let { bindings, body, line: l, column: c }) => {
            try_keyword(found, line, column, u32_of(*l), u32_of(*c).saturating_add(1), "let");
            for (_n, e) in bindings {
                visit_expr(e, line, column, symbols, found);
            }
            for e in body {
                visit_expr(e, line, column, symbols, found);
            }
        }
        Expr::Begin(Begin { exprs, line: l, column: c }) => {
            try_keyword(found, line, column, u32_of(*l), u32_of(*c).saturating_add(1), "begin");
            for e in exprs {
                visit_expr(e, line, column, symbols, found);
            }
        }
        Expr::Lambda(Lambda { body, line: l, column: c, .. }) => {
            try_keyword(
                found,
                line,
                column,
                u32_of(*l),
                u32_of(*c).saturating_add(1),
                "lambda",
            );
            for e in body {
                visit_expr(e, line, column, symbols, found);
            }
        }
        Expr::Apply(Apply { fn_expr, args, .. }) => {
            visit_expr(fn_expr, line, column, symbols, found);
            for a in args {
                visit_expr(a, line, column, symbols, found);
            }
        }
    }
}

fn try_keyword(
    found: &mut Option<Hover>,
    cursor_line: u32,
    cursor_col: u32,
    line: u32,
    column: u32,
    keyword: &'static str,
) {
    let length = len_u32(keyword);
    if contains_cursor(cursor_line, cursor_col, line, column, length) {
        *found = Some(Hover {
            kind: HoverKind::Keyword,
            name: keyword.to_string(),
            signature: None,
            line,
            column,
            length,
        });
    }
}

fn try_token(found: &mut Option<Hover>, cursor_line: u32, cursor_col: u32, candidate: Hover) {
    if contains_cursor(cursor_line, cursor_col, candidate.line, candidate.column, candidate.length) {
        *found = Some(candidate);
    }
}

/// `(token_line, token_col, token_len)` "contains" `(cursor_line, cursor_col)`
/// iff the cursor sits on the token's line, at or past the token's
/// start column, and at or before one past the token's last column.
fn contains_cursor(
    cursor_line: u32,
    cursor_col: u32,
    token_line: u32,
    token_col: u32,
    token_len: u32,
) -> bool {
    if cursor_line != token_line || cursor_line == 0 || token_line == 0 {
        return false;
    }
    if token_col == 0 || cursor_col == 0 {
        return false;
    }
    let end_col = token_col.saturating_add(token_len);
    cursor_col >= token_col && cursor_col <= end_col
}

fn resolve_symbol<'a>(name: &str, symbols: &'a [DocumentSymbol]) -> Option<&'a DocumentSymbol> {
    symbols.iter().find(|s| s.name == name)
}

fn format_param_list(params: &[String]) -> String {
    let mut out = String::with_capacity(params.iter().map(|p| p.len() + 1).sum::<usize>() + 2);
    out.push('(');
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(p);
    }
    out.push(')');
    out
}

fn u32_of(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn len_u32(s: &str) -> u32 {
    u32_of(s.chars().count())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn h(src: &str, line: u32, column: u32) -> Option<Hover> {
        hover_at(src, line, column).expect("parse")
    }

    // ---------- HoverKind ----------

    #[test]
    fn hover_kind_mnemonics_distinct() {
        let all = [
            HoverKind::Function,
            HoverKind::Variable,
            HoverKind::UnresolvedVariable,
            HoverKind::Boolean,
            HoverKind::Nil,
            HoverKind::Number,
            HoverKind::Symbol,
            HoverKind::Keyword,
        ];
        let mut mns: Vec<&'static str> = all.iter().map(|k| k.mnemonic()).collect();
        mns.sort();
        mns.dedup();
        assert_eq!(mns.len(), all.len());
    }

    #[test]
    fn hover_kind_displays_as_mnemonic() {
        assert_eq!(format!("{}", HoverKind::Function), "function");
    }

    // ---------- Empty / out of range ----------

    #[test]
    fn no_hover_in_empty_program() {
        assert_eq!(h("", 1, 1), None);
    }

    #[test]
    fn no_hover_at_out_of_range_position() {
        // Source has one form on line 1, but cursor is on line 99.
        assert_eq!(h("(define x 42)", 99, 1), None);
    }

    #[test]
    fn no_hover_for_zero_position() {
        // (line, column) = (0, 0) is the parser's SYNTHETIC sentinel —
        // never matches a real token.
        assert_eq!(h("(define x 42)", 0, 0), None);
    }

    // ---------- Atoms ----------

    #[test]
    fn hover_on_int_literal() {
        let r = h("42", 1, 1).unwrap();
        assert_eq!(r.kind, HoverKind::Number);
        assert_eq!(r.name, "42");
        assert_eq!(r.length, 2);
    }

    #[test]
    fn hover_on_negative_int_literal() {
        let r = h("-7", 1, 1).unwrap();
        assert_eq!(r.kind, HoverKind::Number);
        assert_eq!(r.name, "-7");
    }

    #[test]
    fn hover_on_bool_literal() {
        let r = h("#t", 1, 1).unwrap();
        assert_eq!(r.kind, HoverKind::Boolean);
        assert_eq!(r.name, "#t");
        assert_eq!(r.length, 2);
    }

    #[test]
    fn hover_on_nil_literal() {
        let r = h("nil", 1, 1).unwrap();
        assert_eq!(r.kind, HoverKind::Nil);
        assert_eq!(r.name, "nil");
        assert_eq!(r.length, 3);
    }

    #[test]
    fn hover_on_quoted_symbol() {
        let r = h("'foo", 1, 1).unwrap();
        assert_eq!(r.kind, HoverKind::Symbol);
        assert_eq!(r.name, "'foo");
        assert_eq!(r.length, 4);
    }

    // ---------- VarRef resolution ----------

    #[test]
    fn hover_on_unresolved_var_ref() {
        // Cursor on `x` at column 1 — no top-level define.
        let r = h("x", 1, 1).unwrap();
        assert_eq!(r.kind, HoverKind::UnresolvedVariable);
        assert_eq!(r.name, "x");
        assert!(r.signature.is_none());
    }

    #[test]
    fn hover_on_var_ref_resolved_to_function() {
        // (define square (lambda (x) (* x x)))
        // (square 5)
        let src = "(define (square x) (* x x))\n(square 5)";
        // Cursor on `square` in the apply on line 2 (column 2..7 covers `square`).
        let r = h(src, 2, 2).unwrap();
        assert_eq!(r.kind, HoverKind::Function);
        assert_eq!(r.name, "square");
        assert_eq!(r.signature.as_deref(), Some("(x)"));
    }

    #[test]
    fn hover_on_var_ref_resolved_to_variable() {
        let src = "(define greeting 'hello)\n(echo greeting)";
        // `greeting` in the apply is at line 2, column 7..14.
        let r = h(src, 2, 7).unwrap();
        assert_eq!(r.kind, HoverKind::Variable);
        assert_eq!(r.name, "greeting");
        assert!(r.signature.is_none());
    }

    // ---------- Keywords ----------

    #[test]
    fn hover_on_if_keyword() {
        // `if` is at column 2 (just past the open paren on line 1).
        let r = h("(if #t 1 2)", 1, 2).unwrap();
        assert_eq!(r.kind, HoverKind::Keyword);
        assert_eq!(r.name, "if");
    }

    #[test]
    fn hover_on_let_keyword() {
        let r = h("(let ((x 1)) x)", 1, 2).unwrap();
        assert_eq!(r.kind, HoverKind::Keyword);
        assert_eq!(r.name, "let");
    }

    #[test]
    fn hover_on_lambda_keyword() {
        let r = h("(lambda (x) x)", 1, 2).unwrap();
        assert_eq!(r.kind, HoverKind::Keyword);
        assert_eq!(r.name, "lambda");
    }

    #[test]
    fn hover_on_begin_keyword() {
        let r = h("(begin 1 2)", 1, 2).unwrap();
        assert_eq!(r.kind, HoverKind::Keyword);
        assert_eq!(r.name, "begin");
    }

    #[test]
    fn hover_on_define_keyword() {
        let r = h("(define x 1)", 1, 2).unwrap();
        assert_eq!(r.kind, HoverKind::Keyword);
        assert_eq!(r.name, "define");
    }

    // ---------- Cursor at boundary ----------

    #[test]
    fn cursor_at_token_end_still_hovers() {
        // `42` at column 1..3.  Cursor at column 3 (just past) should
        // still hover on it (prettier convention).
        let r = h("42", 1, 3).unwrap();
        assert_eq!(r.kind, HoverKind::Number);
    }

    #[test]
    fn cursor_just_past_token_end_does_not_hover() {
        // Cursor at column 4 — past the `42` end-boundary at col 3.
        assert_eq!(h("42", 1, 4), None);
    }

    // ---------- Define-name surfaces with signature ----------

    #[test]
    fn hover_on_define_name_function_shows_signature() {
        // `(define (square x) (* x x))` — cursor on `square` (column 9..15
        // because `(define ` is 8 chars).
        let src = "(define (square x) (* x x))";
        let r = h(src, 1, 9).unwrap();
        assert_eq!(r.kind, HoverKind::Function);
        assert_eq!(r.name, "square");
        assert_eq!(r.signature.as_deref(), Some("(x)"));
    }

    #[test]
    fn hover_on_define_name_variable_no_signature() {
        let src = "(define greeting 'hello)";
        // `greeting` starts at column 9.
        let r = h(src, 1, 9).unwrap();
        assert_eq!(r.kind, HoverKind::Variable);
        assert_eq!(r.name, "greeting");
        assert!(r.signature.is_none());
    }

    // ---------- Innermost-token wins ----------

    #[test]
    fn cursor_inside_nested_form_picks_innermost_atom() {
        // (if (= n 0) 1 2) — cursor on `n` (inside the `=` apply).
        let src = "(if (= n 0) 1 2)";
        // Position of `n`: column 8.
        let r = h(src, 1, 8).unwrap();
        assert_eq!(r.kind, HoverKind::UnresolvedVariable);
        assert_eq!(r.name, "n");
    }

    // ---------- Multi-line ----------

    #[test]
    fn hover_works_across_multiple_lines() {
        let src = "(define x 1)\n\n(define y 2)";
        // Cursor on `y` in second define (line 3, column 9).
        let r = h(src, 3, 9).unwrap();
        assert_eq!(r.kind, HoverKind::Variable);
        assert_eq!(r.name, "y");
    }

    // ---------- Errors ----------

    #[test]
    fn unparseable_input_returns_parse_error() {
        let err = hover_at("(unbalanced", 1, 1).unwrap_err();
        let _ = err;
    }

    // ---------- hover_for_program direct path ----------

    #[test]
    fn hover_for_program_skips_parse() {
        let p = twig_parser::parse("(define x 1)").expect("parse");
        let r = hover_for_program(&p, 1, 9).unwrap();
        assert_eq!(r.name, "x");
    }
}
