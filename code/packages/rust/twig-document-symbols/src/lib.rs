//! # `twig-document-symbols` — LSP outline view for Twig.
//!
//! Walks a parsed [`twig_parser::Program`] and returns a typed
//! `Vec<DocumentSymbol>` — the data feed for the editor's
//! **outline view** (VS Code's "Outline" pane, JetBrains'
//! "Structure" tool window, every LSP-aware editor's
//! `textDocument/documentSymbol` response).
//!
//! Together with [`twig-formatter`](../twig-formatter/) and
//! [`twig-semantic-tokens`](../twig-semantic-tokens/), this is the
//! third piece of the **Twig authoring-experience layer**.
//!
//! ## What's a document symbol
//!
//! Per the LSP spec, `DocumentSymbol` is a hierarchical structure
//! describing the named "things" in a file — functions, classes,
//! variables — for navigation features (outline, breadcrumbs,
//! workspace symbol search, "Go to Symbol in File").
//!
//! Twig's symbol vocabulary is small:
//!
//! - `(define name (lambda params body))` → `Function` symbol with
//!   `detail = "(params)"`.
//! - `(define name expr)` (any other expr) → `Variable` symbol.
//!
//! Bare top-level expressions are *not* symbols (they don't bind
//! a name).  Nested defines aren't a concern: the Twig grammar
//! only allows `(define …)` at the top level — the parser rejects
//! them in expression position before they reach this crate.
//!
//! ## Public API
//!
//! - [`document_symbols(source)`] — `&str` →
//!   `Result<Vec<DocumentSymbol>, TwigParseError>`.  The common
//!   case.
//! - [`symbols_for_program(program)`] — already-parsed `&Program`
//!   → `Vec<DocumentSymbol>`.  Skip the parse step when you have
//!   an AST.
//!
//! Symbols come back in **document order** (top to bottom) — what
//! LSP outline-view providers want.
//!
//! ## Position model
//!
//! All positions are **1-based** `(line, column)` matching
//! `twig-parser`.  V1 returns the start position only; the LSP
//! spec wants both `range` (full extent of the symbol declaration
//! including body) and `selectionRange` (just the name).  Adding
//! end positions requires threading them through `twig-parser`,
//! which is filed as a follow-up.
//!
//! ## What this crate does NOT do
//!
//! - **No `let`-binding symbols.**  Outline views typically don't
//!   list inner-scope bindings — those clutter the navigation
//!   pane.  Editors that want them can layer it on top.
//! - **No LSP wire encoding.**  Returns a typed Vec; the JSON
//!   `DocumentSymbol[]` shape is one level up so this crate stays
//!   usable from non-LSP consumers (e.g. CLI `twig outline foo.twig`).

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use twig_parser::{parse, Define, Expr, Form, Lambda, Program, TwigParseError};

// ---------------------------------------------------------------------------
// SymbolKind
// ---------------------------------------------------------------------------

/// Classification surfaced for each top-level `(define …)`.
///
/// Mirrors the LSP `SymbolKind` enum's `Function` / `Variable`
/// values; future extensions can add more variants
/// (`#[non_exhaustive]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SymbolKind {
    /// `(define name (lambda params body))` — a function binding.
    Function,
    /// `(define name expr)` where `expr` is anything other than a
    /// `lambda` — a value binding.
    Variable,
}

impl SymbolKind {
    /// Stable lowercase mnemonic, matching LSP's `SymbolKind` names
    /// where the meanings line up.
    pub fn mnemonic(self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Variable => "variable",
        }
    }
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mnemonic())
    }
}

// ---------------------------------------------------------------------------
// DocumentSymbol
// ---------------------------------------------------------------------------

/// One outline-view entry.
///
/// `detail` carries the lambda parameter signature (e.g.
/// `"(x y z)"`) for `Function` symbols, `None` for `Variable`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    /// Symbol name (the `define`'s left-hand side).
    pub name: String,
    /// Classification.
    pub kind: SymbolKind,
    /// Optional one-line summary surfaced next to the name in
    /// most outline UIs.
    pub detail: Option<String>,
    /// 1-based line of the `(define` form.
    pub line: u32,
    /// 1-based column of the `(define` form.
    pub column: u32,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse `source` and return its top-level document symbols in
/// document order.
pub fn document_symbols(source: &str) -> Result<Vec<DocumentSymbol>, TwigParseError> {
    let program = parse(source)?;
    Ok(symbols_for_program(&program))
}

/// Walk an already-parsed `Program` and return its top-level
/// document symbols in document order.
pub fn symbols_for_program(program: &Program) -> Vec<DocumentSymbol> {
    let mut out: Vec<DocumentSymbol> = Vec::new();
    for form in &program.forms {
        if let Form::Define(d) = form {
            out.push(symbol_for_define(d));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Define → DocumentSymbol
// ---------------------------------------------------------------------------

fn symbol_for_define(d: &Define) -> DocumentSymbol {
    let (kind, detail) = match &d.expr {
        Expr::Lambda(Lambda { params, .. }) => {
            (SymbolKind::Function, Some(format_param_list(params)))
        }
        _ => (SymbolKind::Variable, None),
    };
    DocumentSymbol {
        name: d.name.clone(),
        kind,
        detail,
        line: u32_of(d.line),
        column: u32_of(d.column),
    }
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn syms(src: &str) -> Vec<DocumentSymbol> {
        document_symbols(src).expect("parse")
    }

    // ---------- SymbolKind ----------

    #[test]
    fn mnemonic_function_and_variable() {
        assert_eq!(SymbolKind::Function.mnemonic(), "function");
        assert_eq!(SymbolKind::Variable.mnemonic(), "variable");
    }

    #[test]
    fn symbol_kind_displays_as_mnemonic() {
        assert_eq!(format!("{}", SymbolKind::Function), "function");
        assert_eq!(format!("{}", SymbolKind::Variable), "variable");
    }

    // ---------- Empty ----------

    #[test]
    fn empty_program_no_symbols() {
        assert!(syms("").is_empty());
    }

    #[test]
    fn bare_top_level_expressions_are_not_symbols() {
        // (+ 1 2) at top level binds no name.
        assert!(syms("(+ 1 2)").is_empty());
    }

    // ---------- Variable ----------

    #[test]
    fn define_value_binding_is_variable_symbol() {
        let s = syms("(define x 42)");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "x");
        assert_eq!(s[0].kind, SymbolKind::Variable);
        assert_eq!(s[0].detail, None);
    }

    #[test]
    fn define_with_complex_expr_is_still_variable() {
        let s = syms("(define greeting (cons 'hello 'world))");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].kind, SymbolKind::Variable);
    }

    // ---------- Function ----------

    #[test]
    fn define_lambda_is_function_symbol_with_param_signature() {
        let s = syms("(define f (lambda (x y) (+ x y)))");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "f");
        assert_eq!(s[0].kind, SymbolKind::Function);
        assert_eq!(s[0].detail.as_deref(), Some("(x y)"));
    }

    #[test]
    fn define_function_sugar_lowers_to_function_symbol() {
        // (define (square x) ...) is sugar for (define square (lambda (x) ...))
        let s = syms("(define (square x) (* x x))");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "square");
        assert_eq!(s[0].kind, SymbolKind::Function);
        assert_eq!(s[0].detail.as_deref(), Some("(x)"));
    }

    #[test]
    fn nullary_lambda_has_empty_param_signature() {
        let s = syms("(define const (lambda () 42))");
        assert_eq!(s[0].detail.as_deref(), Some("()"));
    }

    #[test]
    fn multi_param_signature_renders_with_spaces() {
        let s = syms("(define f (lambda (a b c d) a))");
        assert_eq!(s[0].detail.as_deref(), Some("(a b c d)"));
    }

    // ---------- Multi-form ----------

    #[test]
    fn multiple_top_level_defines_are_all_symbols() {
        let src = "(define x 1)\n(define y 2)\n(define f (lambda (z) z))";
        let s = syms(src);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].name, "x");
        assert_eq!(s[1].name, "y");
        assert_eq!(s[2].name, "f");
        assert_eq!(s[2].kind, SymbolKind::Function);
    }

    #[test]
    fn symbols_in_document_order() {
        let src = "(define a 1)\n(define b 2)\n(define c 3)";
        let s = syms(src);
        let names: Vec<&str> = s.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
        // Lines ascending.
        let lines: Vec<u32> = s.iter().map(|x| x.line).collect();
        let mut sorted = lines.clone();
        sorted.sort();
        assert_eq!(lines, sorted);
    }

    // ---------- Nested defines ----------

    #[test]
    fn nested_defines_rejected_at_parse_layer() {
        // The Twig parser does not allow `(define …)` inside an
        // expression position — it's a top-level-only form.  This
        // test documents the boundary: nested defines never reach
        // `symbols_for_program` because the parser rejects them
        // first.
        let src = "(define f (lambda (x) (define g 1) g))";
        assert!(document_symbols(src).is_err());
    }

    // ---------- Mixed ----------

    #[test]
    fn mixed_top_level_skips_bare_exprs_and_keeps_defines() {
        let src = "(+ 1 2)\n(define x 10)\n(* x x)\n(define y 20)";
        let s = syms(src);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].name, "x");
        assert_eq!(s[1].name, "y");
    }

    // ---------- Position ----------

    #[test]
    fn symbol_position_matches_define_form() {
        let src = "(define x 42)";
        let s = syms(src);
        assert_eq!(s[0].line, 1);
        assert_eq!(s[0].column, 1);
    }

    #[test]
    fn multi_line_positions_are_correct() {
        let src = "(define a 1)\n\n(define b 2)";
        let s = syms(src);
        assert_eq!(s[0].line, 1);
        assert_eq!(s[1].line, 3);
    }

    // ---------- Errors ----------

    #[test]
    fn unparseable_input_returns_parse_error() {
        let err = document_symbols("(unbalanced").unwrap_err();
        let _ = err;
    }

    // ---------- symbols_for_program direct path ----------

    #[test]
    fn symbols_for_program_skips_parse() {
        let p = twig_parser::parse("(define x 1)").expect("parse");
        let s = symbols_for_program(&p);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "x");
    }

    // ---------- Realistic example ----------

    #[test]
    fn realistic_module_outline() {
        let src = "(define greeting 'hello)\n\
                   (define (square x) (* x x))\n\
                   (define (factorial n)\n\
                     (if (= n 0) 1 (* n (factorial (- n 1)))))\n\
                   (define pi 3)";
        let s = syms(src);
        assert_eq!(s.len(), 4);
        // Types correct.
        assert_eq!(s[0].kind, SymbolKind::Variable);
        assert_eq!(s[1].kind, SymbolKind::Function);
        assert_eq!(s[2].kind, SymbolKind::Function);
        assert_eq!(s[3].kind, SymbolKind::Variable);
        // Function signatures populated.
        assert_eq!(s[1].detail.as_deref(), Some("(x)"));
        assert_eq!(s[2].detail.as_deref(), Some("(n)"));
    }

    // ---------- Long identifier handling ----------

    #[test]
    fn long_param_names_pass_through_in_signature() {
        let src = "(define (do-thing very-long-parameter-name another-one) very-long-parameter-name)";
        let s = syms(src);
        assert_eq!(
            s[0].detail.as_deref(),
            Some("(very-long-parameter-name another-one)")
        );
    }
}
