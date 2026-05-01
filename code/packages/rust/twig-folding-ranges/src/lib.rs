//! # `twig-folding-ranges` — LSP folding-range extraction for Twig.
//!
//! Walks a parsed [`twig_parser::Program`] and returns the typed
//! list of multi-line foldable regions for the editor's
//! `textDocument/foldingRange` response.  Drives "fold all
//! defines" / "collapse this `let` body" / "show outline only"
//! commands across every LSP-aware editor.
//!
//! Together with [`twig-formatter`](../twig-formatter/),
//! [`twig-semantic-tokens`](../twig-semantic-tokens/), and
//! [`twig-document-symbols`](../twig-document-symbols/), this is
//! the **fourth piece of the Twig authoring-experience layer**.
//!
//! ## What folds
//!
//! Any compound form that **spans more than one source line**:
//!
//! - `(define name expr)` where `expr` extends past the `(define`
//!   line.
//! - `(let ((bindings)) body)` where the bindings or body extend
//!   past line 1.
//! - `(begin e1 e2 …)` similar.
//! - `(lambda (params) body)` similar.
//! - `(if cond then else)` similar.
//! - `(fn arg1 arg2 …)` (function application) similar.
//!
//! Single-line forms (`(define x 42)` on one line) are **not**
//! folded — there's nothing to collapse.
//!
//! ## Public API
//!
//! - [`folding_ranges(source)`] — `&str` →
//!   `Result<Vec<FoldingRange>, TwigParseError>`.
//! - [`ranges_for_program(program)`] — already-parsed `&Program`
//!   → `Vec<FoldingRange>`.
//!
//! Ranges come back in **document order** (start line ascending,
//! then end line ascending for ties).
//!
//! ## Position model
//!
//! All positions are **1-based** lines matching `twig-parser`.
//! V1 is line-based (no columns) — sufficient for every LSP
//! folding-range consumer; columns can be added later if a
//! consumer ever needs them.
//!
//! End lines are **derived** from the maximum line of any
//! position in the form's subtree.  This is approximate (it
//! doesn't see the closing paren if it's on a line past every
//! atom), but tracks the visible region the user wants to
//! collapse, which is what folding-range consumers care about.
//!
//! ## What this crate does NOT do
//!
//! - **No comment regions.**  The Twig lexer is comment-stripping;
//!   comments don't survive into the AST.
//! - **No `#region` / `#endregion` markers.**  Twig has no such
//!   convention.
//! - **No LSP wire encoding.**  Returns a typed Vec; the
//!   `FoldingRange[]` JSON shape is one level up so this crate
//!   stays usable from non-LSP consumers.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use twig_parser::{
    parse, Apply, Begin, Expr, Form, If, Lambda, Let, Program, TwigParseError,
};

// ---------------------------------------------------------------------------
// FoldingRangeKind
// ---------------------------------------------------------------------------

/// Folding-range classification.  Mirrors LSP's `FoldingRangeKind`
/// (`Comment`, `Imports`, `Region`).  Twig only needs `Region` in
/// v1; future extensions add variants (`#[non_exhaustive]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum FoldingRangeKind {
    /// Generic foldable region (any compound form).
    #[default]
    Region,
}

impl FoldingRangeKind {
    /// Stable lowercase mnemonic matching LSP's
    /// `FoldingRangeKind` values where the meanings line up.
    pub fn mnemonic(self) -> &'static str {
        match self {
            FoldingRangeKind::Region => "region",
        }
    }
}

impl fmt::Display for FoldingRangeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mnemonic())
    }
}

// ---------------------------------------------------------------------------
// FoldingRange
// ---------------------------------------------------------------------------

/// One foldable line range.
///
/// `start_line < end_line` always — single-line forms aren't
/// emitted (nothing to fold).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FoldingRange {
    /// 1-based starting line (line of the `(` that begins the form).
    pub start_line: u32,
    /// 1-based ending line (max line touched by any subtree position).
    pub end_line: u32,
    /// Classification.
    pub kind: FoldingRangeKind,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse `source` and return its folding ranges in document order.
pub fn folding_ranges(source: &str) -> Result<Vec<FoldingRange>, TwigParseError> {
    let program = parse(source)?;
    Ok(ranges_for_program(&program))
}

/// Walk an already-parsed `Program` and return its folding ranges
/// in document order.
pub fn ranges_for_program(program: &Program) -> Vec<FoldingRange> {
    let mut out: Vec<FoldingRange> = Vec::new();
    for form in &program.forms {
        emit_form(&mut out, form);
    }
    sort_in_document_order(&mut out);
    out
}

// ---------------------------------------------------------------------------
// Walker
// ---------------------------------------------------------------------------

fn emit_form(out: &mut Vec<FoldingRange>, form: &Form) {
    match form {
        Form::Define(d) => {
            let start = u32_of(d.line);
            let end = max_line_in_expr(&d.expr);
            push_if_multiline(out, start, end);
            descend_expr(out, &d.expr);
        }
        Form::Expr(e) => {
            push_if_multiline(out, u32_of(e.pos().0), max_line_in_expr(e));
            descend_expr(out, e);
        }
    }
}

fn descend_expr(out: &mut Vec<FoldingRange>, expr: &Expr) {
    match expr {
        Expr::IntLit(_) | Expr::BoolLit(_) | Expr::NilLit(_) | Expr::SymLit(_) | Expr::VarRef(_) => {}
        Expr::If(If { cond, then_branch, else_branch, line, .. }) => {
            let start = u32_of(*line);
            let end = max_line_in_expr(cond)
                .max(max_line_in_expr(then_branch))
                .max(max_line_in_expr(else_branch));
            push_if_multiline(out, start, end);
            descend_expr(out, cond);
            descend_expr(out, then_branch);
            descend_expr(out, else_branch);
        }
        Expr::Let(Let { bindings, body, line, .. }) => {
            let start = u32_of(*line);
            let end = max_line_in_bindings(bindings).max(max_line_in_exprs(body));
            push_if_multiline(out, start, end);
            for (_n, e) in bindings {
                descend_expr(out, e);
            }
            for e in body {
                descend_expr(out, e);
            }
        }
        Expr::Begin(Begin { exprs, line, .. }) => {
            let start = u32_of(*line);
            let end = max_line_in_exprs(exprs);
            push_if_multiline(out, start, end);
            for e in exprs {
                descend_expr(out, e);
            }
        }
        Expr::Lambda(Lambda { body, line, .. }) => {
            let start = u32_of(*line);
            let end = max_line_in_exprs(body);
            push_if_multiline(out, start, end);
            for e in body {
                descend_expr(out, e);
            }
        }
        Expr::Apply(Apply { fn_expr, args, line, .. }) => {
            let start = u32_of(*line);
            let end = max_line_in_expr(fn_expr).max(max_line_in_exprs(args));
            push_if_multiline(out, start, end);
            descend_expr(out, fn_expr);
            for a in args {
                descend_expr(out, a);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Max-line derivation
// ---------------------------------------------------------------------------

fn max_line_in_expr(expr: &Expr) -> u32 {
    match expr {
        Expr::IntLit(n) => u32_of(n.line),
        Expr::BoolLit(b) => u32_of(b.line),
        Expr::NilLit(n) => u32_of(n.line),
        Expr::SymLit(s) => u32_of(s.line),
        Expr::VarRef(v) => u32_of(v.line),
        Expr::If(If { cond, then_branch, else_branch, line, .. }) => u32_of(*line)
            .max(max_line_in_expr(cond))
            .max(max_line_in_expr(then_branch))
            .max(max_line_in_expr(else_branch)),
        Expr::Let(Let { bindings, body, line, .. }) => u32_of(*line)
            .max(max_line_in_bindings(bindings))
            .max(max_line_in_exprs(body)),
        Expr::Begin(Begin { exprs, line, .. }) => u32_of(*line).max(max_line_in_exprs(exprs)),
        Expr::Lambda(Lambda { body, line, .. }) => u32_of(*line).max(max_line_in_exprs(body)),
        Expr::Apply(Apply { fn_expr, args, line, .. }) => u32_of(*line)
            .max(max_line_in_expr(fn_expr))
            .max(max_line_in_exprs(args)),
    }
}

fn max_line_in_exprs(exprs: &[Expr]) -> u32 {
    exprs.iter().map(max_line_in_expr).max().unwrap_or(0)
}

fn max_line_in_bindings(bindings: &[(String, Expr)]) -> u32 {
    bindings.iter().map(|(_, e)| max_line_in_expr(e)).max().unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn push_if_multiline(out: &mut Vec<FoldingRange>, start_line: u32, end_line: u32) {
    if start_line > 0 && end_line > start_line {
        out.push(FoldingRange { start_line, end_line, kind: FoldingRangeKind::Region });
    }
}

fn u32_of(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn sort_in_document_order(ranges: &mut [FoldingRange]) {
    ranges.sort_by(|a, b| {
        a.start_line.cmp(&b.start_line).then(a.end_line.cmp(&b.end_line))
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fr(src: &str) -> Vec<FoldingRange> {
        folding_ranges(src).expect("parse")
    }

    // ---------- FoldingRangeKind ----------

    #[test]
    fn region_mnemonic() {
        assert_eq!(FoldingRangeKind::Region.mnemonic(), "region");
        assert_eq!(format!("{}", FoldingRangeKind::Region), "region");
    }

    #[test]
    fn default_kind_is_region() {
        assert_eq!(FoldingRangeKind::default(), FoldingRangeKind::Region);
    }

    // ---------- Empty / single-line ----------

    #[test]
    fn empty_program_no_ranges() {
        assert!(fr("").is_empty());
    }

    #[test]
    fn single_line_define_does_not_fold() {
        assert!(fr("(define x 42)").is_empty());
    }

    #[test]
    fn single_line_if_does_not_fold() {
        assert!(fr("(if #t 1 2)").is_empty());
    }

    #[test]
    fn single_line_apply_does_not_fold() {
        assert!(fr("(+ 1 2 3)").is_empty());
    }

    // ---------- Multi-line forms ----------

    #[test]
    fn multi_line_define_folds() {
        let src = "(define x\n  42)";
        let r = fr(src);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].start_line, 1);
        assert_eq!(r[0].end_line, 2);
    }

    #[test]
    fn multi_line_if_folds() {
        let src = "(if cond\n  then\n  else)";
        let r = fr(src);
        // The (if …) form folds.  Inner atoms don't (single-line each).
        assert!(r.iter().any(|x| x.start_line == 1 && x.end_line == 3));
    }

    #[test]
    fn multi_line_lambda_folds() {
        let src = "(define f\n  (lambda (x)\n    (* x x)))";
        let r = fr(src);
        // Two ranges: outer define + inner lambda + inner apply.
        // Just ensure top-level fold is present.
        assert!(r.iter().any(|x| x.start_line == 1));
    }

    #[test]
    fn multi_line_let_folds() {
        let src = "(let ((a 1)\n      (b 2))\n  (+ a b))";
        let r = fr(src);
        assert!(r.iter().any(|x| x.start_line == 1 && x.end_line == 3));
    }

    #[test]
    fn multi_line_begin_folds() {
        let src = "(begin\n  (do-thing)\n  (do-another))";
        let r = fr(src);
        assert!(r.iter().any(|x| x.start_line == 1 && x.end_line == 3));
    }

    // ---------- Nested ----------

    #[test]
    fn nested_multi_line_forms_each_emit_a_range() {
        let src = "(define f\n  (lambda (x)\n    (if (> x 0)\n        x\n        (- x))))";
        let r = fr(src);
        // We expect ranges for: define (1..5), lambda (2..5),
        // if (3..5), maybe apply (5..5 single-line).  At least
        // three multi-line ranges.
        assert!(r.len() >= 3, "expected multiple nested ranges, got: {r:?}");
        // Outermost starts at line 1 and ends at line 5.
        assert!(r.iter().any(|x| x.start_line == 1 && x.end_line == 5));
    }

    #[test]
    fn ranges_in_document_order() {
        let src = "(define a\n  1)\n\n(define b\n  2)";
        let r = fr(src);
        let starts: Vec<u32> = r.iter().map(|x| x.start_line).collect();
        let mut sorted = starts.clone();
        sorted.sort();
        assert_eq!(starts, sorted);
    }

    // ---------- Mixed top-level ----------

    #[test]
    fn mixed_top_level_skips_single_line_keeps_multi_line() {
        let src = "(define x 1)\n(define y\n  2)\n(define z 3)";
        let r = fr(src);
        // Only the multi-line define on lines 2-3 folds.
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].start_line, 2);
        assert_eq!(r[0].end_line, 3);
    }

    // ---------- Apply nested deeply ----------

    #[test]
    fn deeply_nested_apply_folds_at_each_multi_line_level() {
        let src = "(outer\n  (inner\n    arg1\n    arg2))";
        let r = fr(src);
        // Outer (1..4) and inner (2..4) both fold.
        assert!(r.iter().any(|x| x.start_line == 1 && x.end_line == 4));
        assert!(r.iter().any(|x| x.start_line == 2 && x.end_line == 4));
    }

    // ---------- Edge: end == start (single line) is filtered ----------

    #[test]
    fn equal_start_and_end_filtered() {
        // A let with single-line content.
        let r = fr("(let ((x 1)) x)");
        assert!(r.is_empty());
    }

    // ---------- Errors ----------

    #[test]
    fn unparseable_input_returns_parse_error() {
        let err = folding_ranges("(unbalanced").unwrap_err();
        let _ = err;
    }

    // ---------- ranges_for_program direct path ----------

    #[test]
    fn ranges_for_program_skips_parse() {
        let p = twig_parser::parse("(define x\n  42)").expect("parse");
        let r = ranges_for_program(&p);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].start_line, 1);
        assert_eq!(r[0].end_line, 2);
    }

    // ---------- Realistic ----------

    #[test]
    fn realistic_module_emits_ranges_for_multi_line_defines() {
        let src = "(define x 42)\n\
                   \n\
                   (define (factorial n)\n\
                     (if (= n 0)\n\
                         1\n\
                         (* n (factorial (- n 1)))))\n\
                   \n\
                   (define result\n\
                     (factorial 10))";
        let r = fr(src);
        // factorial define (3..6), if inside it (4..6), result define (8..9).
        assert!(r.iter().any(|x| x.start_line == 3 && x.end_line == 6));
        assert!(r.iter().any(|x| x.start_line == 8 && x.end_line == 9));
    }

    // ---------- All-atom expression doesn't fold ----------

    #[test]
    fn atoms_alone_dont_fold() {
        for src in ["42", "#t", "#f", "nil", "'foo", "x"] {
            assert!(fr(src).is_empty(), "atom {src} should not fold");
        }
    }
}
