//! # `twig-formatter` — canonical Twig pretty-printer.
//!
//! The prettier / rustfmt equivalent for Twig.  The first
//! authoring-experience deliverable: drop noisy whitespace
//! arguments at PR time, give every Twig file one canonical shape.
//!
//! ## Architecture
//!
//! ```text
//! Twig source string
//!         │
//!         ▼ twig_parser::parse
//!   twig_parser::Program (AST)
//!         │
//!         ▼ this crate's emit_*  (AST → Doc)
//!   format_doc::Doc
//!         │
//!         ▼ format_doc::layout_doc  (width-aware realisation)
//!   format_doc::DocLayoutTree
//!         │
//!         ▼ format_doc::render_text  (or, in future, paint pipeline)
//! Canonical Twig source string
//! ```
//!
//! Layout decisions (compact vs block) are made by `format-doc`'s
//! Wadler-style realiser — this crate just describes the shape of
//! every Twig form using `group` + `softline`/`line`/`indent`
//! combinators.  Same `Doc` builds drive ASCII output today and
//! the paint-vm pipeline tomorrow.
//!
//! ## Public API
//!
//! - [`format`] — `&str` → `Result<String, FormatError>`.  Parses
//!   the input, builds a [`format_doc::Doc`], realises at the
//!   default 80-column width, and returns canonical text with one
//!   trailing newline.  The common case.
//! - [`format_program`] — `&Program` + `&Config` → `String`.  Skip
//!   the parse step when you already have an AST (LSP code action,
//!   incremental editor integration).
//! - [`program_to_doc`] — `&Program` → `format_doc::Doc`.  When the
//!   caller wants to plug the doc into its own paint pipeline
//!   instead of going through `render_text`.
//!
//! ## Guarantees
//!
//! - **Idempotency.**  `format(&format(s)?)? == format(s)?`.  Once
//!   formatted, re-formatting is a no-op.
//! - **Semantic preservation.**  `parse(&format(s)?)? == parse(s)?`
//!   modulo source positions.
//! - **Determinism.**  Same input + same config → same output,
//!   byte-for-byte.
//! - **Width-aware layout.**  Width budget defaults to 80 columns;
//!   the realiser breaks long forms across multiple lines and
//!   keeps short ones compact.
//!
//! ## Style rules
//!
//! Twig is a Lisp; rules mirror canonical scheme/clojure layout:
//!
//! - Atoms render as their source form (`42`, `#t`, `#f`, `nil`,
//!   `'foo`, `x`).
//! - Compound forms use `group` so the realiser picks compact when
//!   it fits, indented multi-line otherwise:
//!   - `(if cond then else)` — children indented 4 cols (under `(if `).
//!   - `(let ((x e1) (y e2)) body)` — bindings indented 6 cols
//!     (under `((`); body indented 2.
//!   - `(begin e1 e2 …)` — exprs indented 2 cols.
//!   - `(lambda (x y) body)` — params on the same line as `lambda`;
//!     body indented 2.
//!   - `(define name expr)` — name same line; expr indented 2 if
//!     it doesn't fit.
//!   - `(f a1 a2 …)` (apply) — first arg same line as fn;
//!     subsequent args indented under the first.
//! - Top-level forms separated by a single blank line.
//! - Output ends with a single trailing newline (POSIX text-file
//!   convention).
//!
//! ## What this formatter does NOT preserve
//!
//! - **Comments.**  The Twig grammar is comment-stripping at the
//!   lexer layer, so comments don't survive into the AST.  Don't
//!   run on commented files; rustfmt-style trivia preservation is
//!   the appropriate follow-up.
//! - **Whitespace and column layout.**  The whole point.
//! - **Surface form of `'foo` vs `(quote foo)`.**  The parser
//!   collapses both to `SymLit`; the formatter emits `'foo`.
//!
//! ## Caller responsibilities
//!
//! Inherits the non-guarantees of `twig-parser` and `format-doc`.
//! Adversarial input that produces an unbounded AST will be
//! bounded by the parser's depth cap; the doc-building pass here
//! is iterative over the AST and doesn't add a separate stack.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use format_doc::{
    concat, group, hardline, indent, layout_doc, line, nil, render_text, text, Doc,
    LayoutOptions,
};
use twig_parser::{
    parse, Apply, Begin, BoolLit, Define, Expr, Form, If, IntLit, Lambda, Let, NilLit, Program,
    SymLit, TwigParseError, VarRef,
};

// ---------------------------------------------------------------------------
// Public configuration
// ---------------------------------------------------------------------------

/// Default line-width target (columns).
pub const DEFAULT_PRINT_WIDTH: usize = 80;

/// Default indent step (columns per indent level).
pub const DEFAULT_INDENT_WIDTH: usize = 2;

/// Formatter configuration.  `Default` returns industry-standard
/// settings (80 columns, 2-space indent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Wrap any expression that wouldn't fit in this many columns.
    pub print_width: usize,
    /// Indentation in columns added at each nesting level.
    pub indent_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            print_width: DEFAULT_PRINT_WIDTH,
            indent_width: DEFAULT_INDENT_WIDTH,
        }
    }
}

impl Config {
    fn to_layout(self) -> LayoutOptions {
        LayoutOptions {
            print_width: self.print_width,
            indent_width: self.indent_width,
            line_height: 1,
        }
    }
}

/// Errors raised by [`format`].  Currently a thin wrapper over
/// [`TwigParseError`]; future reformat-only errors land here
/// without breaking the public API.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatError {
    /// The input couldn't be parsed.
    Parse(TwigParseError),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Parse(e) => write!(f, "parse error: {e:?}"),
        }
    }
}

impl std::error::Error for FormatError {}

impl From<TwigParseError> for FormatError {
    fn from(e: TwigParseError) -> Self {
        FormatError::Parse(e)
    }
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse `source` and return canonically-formatted Twig text.
///
/// Equivalent to `format_program(&twig_parser::parse(source)?,
/// &Config::default())`.
pub fn format(source: &str) -> Result<String, FormatError> {
    let program = parse(source)?;
    Ok(format_program(&program, &Config::default()))
}

/// Format an already-parsed [`Program`] under `config`.  Output ends
/// with a single trailing newline (POSIX text-file convention) for
/// non-empty programs.
///
/// Top-level forms are realised **independently** — each gets its
/// own [`layout_doc`] call.  This is intentional: format-doc's
/// `fits()` look-ahead would otherwise see hardline separators in
/// the pending stack and force every form into broken mode, even
/// short ones that should stay compact.  The one-blank-line
/// separator between forms is then a plain string join, not a
/// `hardline()` in the doc tree.
pub fn format_program(program: &Program, config: &Config) -> String {
    if program.forms.is_empty() {
        return String::new();
    }
    let layout_opts = config.to_layout();
    let mut out = String::new();
    for (i, form) in program.forms.iter().enumerate() {
        if i > 0 {
            // One blank line between top-level forms.
            out.push_str("\n\n");
        }
        let layout = layout_doc(form_to_doc(form), &layout_opts);
        out.push_str(&render_text(&layout));
    }
    out.push('\n');
    out
}

/// Build a [`format_doc::Doc`] for the entire program in a single
/// tree, with `hardline()` separators between forms.  Useful for
/// callers that want one doc to plug into their own paint pipeline.
///
/// **Caveat**: realising this combined doc forces every form into
/// broken mode (the trailing hardlines fail `fits()`).  For pretty
/// per-form layout, use [`form_to_doc`] on each top-level form
/// independently and assemble layouts at the caller level — that's
/// what [`format_program`] does.
pub fn program_to_doc(program: &Program) -> Doc {
    if program.forms.is_empty() {
        return nil();
    }
    let mut parts: Vec<Doc> = Vec::with_capacity(program.forms.len() * 3);
    for (i, form) in program.forms.iter().enumerate() {
        if i > 0 {
            parts.push(hardline());
            parts.push(hardline());
        }
        parts.push(form_to_doc(form));
    }
    concat(parts)
}

/// Build a [`format_doc::Doc`] for one top-level form.  Public so
/// callers integrating into their own paint pipeline can realise
/// each form independently — see the [`format_program`] caveat
/// about hardline-separator leakage.
pub fn form_to_doc_pub(form: &Form) -> Doc {
    form_to_doc(form)
}

// ---------------------------------------------------------------------------
// Form / Expr → Doc compilers
// ---------------------------------------------------------------------------

fn form_to_doc(form: &Form) -> Doc {
    match form {
        Form::Define(d) => define_to_doc(d),
        Form::Expr(e) => expr_to_doc(e),
    }
}

fn define_to_doc(d: &Define) -> Doc {
    // (define name expr)
    // Compact when it fits; otherwise:
    //   (define name
    //     expr)
    let name = text(d.name.clone());
    let expr = expr_to_doc(&d.expr);
    group(concat([
        text("(define "),
        name,
        // Either " expr" (flat) or "\n  expr" (broken).
        indent(concat([line(), expr]), 1),
        text(")"),
    ]))
}

fn expr_to_doc(expr: &Expr) -> Doc {
    match expr {
        Expr::IntLit(IntLit { value, .. }) => text(value.to_string()),
        Expr::BoolLit(BoolLit { value, .. }) => text(if *value { "#t" } else { "#f" }),
        Expr::NilLit(NilLit { .. }) => text("nil"),
        Expr::SymLit(SymLit { name, .. }) => text(format!("'{name}")),
        Expr::VarRef(VarRef { name, .. }) => text(name.clone()),
        Expr::If(i) => if_to_doc(i),
        Expr::Let(l) => let_to_doc(l),
        Expr::Begin(b) => begin_to_doc(b),
        Expr::Lambda(l) => lambda_to_doc(l),
        Expr::Apply(a) => apply_to_doc(a),
    }
}

fn if_to_doc(i: &If) -> Doc {
    // (if cond then else)
    // Block form aligns children under `cond` (4 columns past `(if`):
    //
    //   (if cond
    //       then
    //       else)
    //
    // The 4-col indent only kicks in when broken; flat keeps
    // everything on one line.
    let head = text("(if ");
    let children = group(concat([
        expr_to_doc(&i.cond),
        line(),
        expr_to_doc(&i.then_branch),
        line(),
        expr_to_doc(&i.else_branch),
    ]));
    // Children share an indent block of 4 cols, but `(if ` occupies
    // the first 4 cols on the head line.  Use indent(2) so each
    // wrapped line lands under the `c` of `cond`.  (4 cols past `(`
    // = 2 indent levels of 2 = "    ").
    concat([head, indent(children, 2), text(")")])
}

fn let_to_doc(l: &Let) -> Doc {
    // (let ((x e1)
    //       (y e2))
    //   body1
    //   body2)
    //
    // Bindings group: align under `((` (6 cols past `(`).  Body
    // indented 2.
    let bindings_doc = if l.bindings.is_empty() {
        text("()")
    } else {
        let mut binding_parts: Vec<Doc> = Vec::with_capacity(l.bindings.len() * 2 - 1);
        for (i, (name, expr)) in l.bindings.iter().enumerate() {
            if i > 0 {
                binding_parts.push(line());
            }
            binding_parts.push(group(concat([
                text("("),
                text(name.clone()),
                indent(concat([line(), expr_to_doc(expr)]), 1),
                text(")"),
            ])));
        }
        concat([text("("), group(concat(binding_parts)), text(")")])
    };
    let mut body_parts: Vec<Doc> = Vec::with_capacity(l.body.len() * 2);
    for e in &l.body {
        body_parts.push(line());
        body_parts.push(expr_to_doc(e));
    }
    group(concat([
        text("(let "),
        bindings_doc,
        indent(concat(body_parts), 1),
        text(")"),
    ]))
}

fn begin_to_doc(b: &Begin) -> Doc {
    // (begin e1 e2 ...)
    // Compact when it fits; otherwise:
    //   (begin
    //     e1
    //     e2)
    if b.exprs.is_empty() {
        return text("(begin)");
    }
    let mut parts: Vec<Doc> = Vec::with_capacity(b.exprs.len() * 2);
    for e in &b.exprs {
        parts.push(line());
        parts.push(expr_to_doc(e));
    }
    group(concat([text("(begin"), indent(concat(parts), 1), text(")")]))
}

fn lambda_to_doc(l: &Lambda) -> Doc {
    // (lambda (x y) body+)
    // Params on the same line as `lambda`; body indented 2 when
    // broken.
    let params_doc = if l.params.is_empty() {
        text("()")
    } else {
        let parts: Vec<Doc> = l
            .params
            .iter()
            .enumerate()
            .flat_map(|(i, p)| {
                let mut v: Vec<Doc> = Vec::new();
                if i > 0 {
                    v.push(text(" "));
                }
                v.push(text(p.clone()));
                v
            })
            .collect();
        concat([text("("), concat(parts), text(")")])
    };
    let mut body_parts: Vec<Doc> = Vec::with_capacity(l.body.len() * 2);
    for e in &l.body {
        body_parts.push(line());
        body_parts.push(expr_to_doc(e));
    }
    group(concat([
        text("(lambda "),
        params_doc,
        indent(concat(body_parts), 1),
        text(")"),
    ]))
}

fn apply_to_doc(a: &Apply) -> Doc {
    // (fn arg1 arg2 ...)
    // Compact when it fits; broken aligns subsequent args under arg1.
    //
    // Implementation: emit `(fn arg1<line>arg2<line>...)`, wrap the
    // arg block in indent(1).  When realised flat the `line`s
    // become spaces; broken, they become newlines indented 2 cols
    // past the surrounding `(fn` (which is approximately under arg1
    // for typical short fn names — close enough to the "align under
    // arg1" idiom for the common case, and any deeper alignment
    // would require width-aware tracking that complicates the
    // combinator API).
    let head = expr_to_doc(&a.fn_expr);
    if a.args.is_empty() {
        return concat([text("("), head, text(")")]);
    }
    let mut arg_parts: Vec<Doc> = Vec::with_capacity(a.args.len() * 2);
    for (i, arg) in a.args.iter().enumerate() {
        if i > 0 {
            arg_parts.push(line());
        }
        arg_parts.push(expr_to_doc(arg));
    }
    group(concat([
        text("("),
        head,
        text(" "),
        indent(concat(arg_parts), 1),
        text(")"),
    ]))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use twig_parser::parse;

    fn fmt(s: &str) -> String {
        format(s).expect("format")
    }

    fn fmt_with(s: &str, config: Config) -> String {
        let p = parse(s).expect("parse");
        format_program(&p, &config)
    }

    fn assert_idempotent(s: &str) {
        let once = fmt(s);
        let twice = fmt(&once);
        assert_eq!(
            once, twice,
            "formatter not idempotent.\nfirst:\n{once}\nsecond:\n{twice}"
        );
    }

    fn assert_semantic_preserving(s: &str) {
        let formatted = fmt(s);
        let original_ast = parse(s).expect("original parses");
        let formatted_ast = parse(&formatted).expect("formatted parses");
        assert_eq!(
            strip_positions(&original_ast),
            strip_positions(&formatted_ast),
            "formatting changed AST shape\nfrom:\n{s}\nto:\n{formatted}"
        );
    }

    fn strip_positions(p: &Program) -> Program {
        Program { forms: p.forms.iter().map(strip_form).collect() }
    }
    fn strip_form(f: &Form) -> Form {
        match f {
            Form::Define(d) => Form::Define(Define {
                name: d.name.clone(),
                type_annotation: d.type_annotation.clone(),
                expr: strip_expr(&d.expr),
                line: 0,
                column: 0,
            }),
            Form::Expr(e) => Form::Expr(strip_expr(e)),
        }
    }
    fn strip_expr(e: &Expr) -> Expr {
        match e {
            Expr::IntLit(n) => Expr::IntLit(IntLit { value: n.value, line: 0, column: 0 }),
            Expr::BoolLit(b) => Expr::BoolLit(BoolLit { value: b.value, line: 0, column: 0 }),
            Expr::NilLit(_) => Expr::NilLit(NilLit { line: 0, column: 0 }),
            Expr::SymLit(s) => Expr::SymLit(SymLit { name: s.name.clone(), line: 0, column: 0 }),
            Expr::VarRef(v) => Expr::VarRef(VarRef { name: v.name.clone(), line: 0, column: 0 }),
            Expr::If(i) => Expr::If(If {
                cond: Box::new(strip_expr(&i.cond)),
                then_branch: Box::new(strip_expr(&i.then_branch)),
                else_branch: Box::new(strip_expr(&i.else_branch)),
                line: 0,
                column: 0,
            }),
            Expr::Let(l) => Expr::Let(Let {
                bindings: l.bindings.iter().map(|(n, e)| (n.clone(), strip_expr(e))).collect(),
                body: l.body.iter().map(strip_expr).collect(),
                line: 0,
                column: 0,
            }),
            Expr::Begin(b) => Expr::Begin(Begin {
                exprs: b.exprs.iter().map(strip_expr).collect(),
                line: 0,
                column: 0,
            }),
            Expr::Lambda(l) => Expr::Lambda(Lambda {
                params: l.params.clone(),
                param_annotations: l.param_annotations.clone(),
                return_annotation: l.return_annotation.clone(),
                body: l.body.iter().map(strip_expr).collect(),
                line: 0,
                column: 0,
            }),
            Expr::Apply(a) => Expr::Apply(Apply {
                fn_expr: Box::new(strip_expr(&a.fn_expr)),
                args: a.args.iter().map(strip_expr).collect(),
                line: 0,
                column: 0,
            }),
        }
    }

    // ---------- Atoms ----------

    #[test]
    fn empty_program() {
        assert_eq!(fmt(""), "");
    }

    #[test]
    fn int_literal() {
        assert_eq!(fmt("42"), "42\n");
        assert_eq!(fmt("-7"), "-7\n");
    }

    #[test]
    fn bool_literal() {
        assert_eq!(fmt("#t"), "#t\n");
        assert_eq!(fmt("#f"), "#f\n");
    }

    #[test]
    fn nil_literal() {
        assert_eq!(fmt("nil"), "nil\n");
    }

    #[test]
    fn quoted_symbol_collapses_to_short_form() {
        assert_eq!(fmt("'foo"), "'foo\n");
        assert_eq!(fmt("(quote foo)"), "'foo\n");
    }

    #[test]
    fn var_ref() {
        assert_eq!(fmt("x"), "x\n");
    }

    // ---------- Compact compound forms ----------

    #[test]
    fn compact_if() {
        assert_eq!(fmt("(if  #t   1   2)"), "(if #t 1 2)\n");
    }

    #[test]
    fn compact_apply() {
        assert_eq!(fmt("(+   1   2 3)"), "(+ 1 2 3)\n");
    }

    #[test]
    fn compact_lambda() {
        assert_eq!(fmt("(lambda (x) x)"), "(lambda (x) x)\n");
    }

    #[test]
    fn compact_let() {
        assert_eq!(fmt("(let ((x 1)) x)"), "(let ((x 1)) x)\n");
    }

    #[test]
    fn compact_begin() {
        assert_eq!(fmt("(begin 1 2 3)"), "(begin 1 2 3)\n");
    }

    #[test]
    fn compact_define() {
        assert_eq!(fmt("(define x 42)"), "(define x 42)\n");
    }

    #[test]
    fn nullary_apply() {
        assert_eq!(fmt("(f)"), "(f)\n");
    }

    // ---------- Block forms ----------

    #[test]
    fn long_apply_breaks_args() {
        let src = "(my-very-long-function-name argument-one argument-two argument-three argument-four)";
        let out = fmt(src);
        let lines: Vec<&str> = out.trim_end().split('\n').collect();
        assert!(lines.len() >= 2, "expected multi-line, got:\n{out}");
        // First line starts with "(my-very-long-function-name"
        assert!(lines[0].starts_with("(my-very-long-function-name"));
    }

    #[test]
    fn long_if_breaks() {
        let src = "(if (some-predicate that-is-long) (then-do-this thing) (else-do-that other-thing))";
        let out = fmt(src);
        assert!(out.contains('\n'));
        assert!(out.starts_with("(if "));
    }

    #[test]
    fn long_let_breaks() {
        let src = "(let ((aaa 111) (bbb 222) (ccc 333) (ddd 444) (eee 555) (fff 666)) (+ aaa bbb))";
        let out = fmt(src);
        assert!(out.contains('\n'));
        assert!(out.starts_with("(let ("));
    }

    #[test]
    fn long_begin_breaks() {
        let src = "(begin (do-thing-one with arg) (do-thing-two with arg) (do-thing-three with arg))";
        let out = fmt(src);
        assert!(out.contains('\n'));
        assert!(out.starts_with("(begin"));
    }

    #[test]
    fn long_lambda_breaks_body() {
        let src = "(lambda (x y) (some-long-thing x) (another-long-thing y) (final-result))";
        let out = fmt(src);
        assert!(out.contains('\n'));
        assert!(out.starts_with("(lambda "));
    }

    // ---------- Define ----------

    #[test]
    fn define_function_round_trips_through_lambda_lowering() {
        // (define (f x) body) is sugar for (define f (lambda (x) body)).
        // The parser lowers; the formatter emits the lowered form.
        let formatted = fmt("(define (square x) (* x x))");
        assert!(formatted.contains("(lambda"), "got:\n{formatted}");
        assert_semantic_preserving("(define (square x) (* x x))");
    }

    // ---------- Multi-form programs ----------

    #[test]
    fn multiple_top_level_forms_separated_by_blank_line() {
        let src = "(define x 1) (define y 2) (+ x y)";
        let out = fmt(src);
        assert_eq!(out, "(define x 1)\n\n(define y 2)\n\n(+ x y)\n", "got:\n{out}");
    }

    #[test]
    fn collapses_arbitrary_whitespace() {
        let src = "
            (define   x
                42)


            (+    x    1)
        ";
        let out = fmt(src);
        assert_eq!(out, "(define x 42)\n\n(+ x 1)\n");
    }

    // ---------- Idempotency ----------

    #[test]
    fn idempotent_on_atoms() {
        for src in ["42", "-7", "#t", "#f", "nil", "'foo", "x", "very-long-identifier-name"] {
            assert_idempotent(src);
        }
    }

    #[test]
    fn idempotent_on_compact_forms() {
        for src in [
            "(if #t 1 2)",
            "(let ((x 1) (y 2)) (+ x y))",
            "(begin 1 2 3)",
            "(lambda (x y) (+ x y))",
            "(+ 1 2 3 4 5)",
            "(define x 42)",
        ] {
            assert_idempotent(src);
        }
    }

    #[test]
    fn idempotent_on_block_forms() {
        for src in [
            "(my-long-function arg1 arg2 arg3 arg4 arg5 arg6 arg7 arg8 arg9)",
            "(if (some-predicate is-it-true here) (then-do action-one) (else-do action-two))",
            "(let ((aaa 1) (bbb 2) (ccc 3) (ddd 4) (eee 5)) (+ aaa bbb ccc ddd eee))",
            "(define square (lambda (x) (* x x)))",
        ] {
            assert_idempotent(src);
        }
    }

    // ---------- Semantic preservation ----------

    #[test]
    fn semantic_preserving_atoms() {
        for src in ["42", "#t", "nil", "'foo", "x"] {
            assert_semantic_preserving(src);
        }
    }

    #[test]
    fn semantic_preserving_compact_forms() {
        for src in [
            "(if #t 1 2)",
            "(let ((x 1) (y 2)) (+ x y))",
            "(begin 1 2 3)",
            "(lambda (x y) (+ x y))",
            "(+ 1 2 3 4 5)",
            "(define x 42)",
        ] {
            assert_semantic_preserving(src);
        }
    }

    #[test]
    fn semantic_preserving_block_forms() {
        for src in [
            "(my-long-function arg1 arg2 arg3 arg4 arg5 arg6 arg7 arg8 arg9)",
            "(if (some-predicate is-it-true here) (then-do action-one) (else-do action-two))",
            "(let ((aaa 1) (bbb 2) (ccc 3) (ddd 4) (eee 5)) (+ aaa bbb ccc ddd eee))",
            "(define square (lambda (x) (* x x)))",
            "(define (factorial n) (if (= n 0) 1 (* n (factorial (- n 1)))))",
        ] {
            assert_semantic_preserving(src);
        }
    }

    // ---------- Trailing newline ----------

    #[test]
    fn output_ends_with_single_newline_for_non_empty_input() {
        assert!(fmt("42").ends_with('\n'));
        assert!(fmt("(define x 42)").ends_with('\n'));
        assert!(!fmt("42").ends_with("\n\n"));
    }

    #[test]
    fn empty_program_has_no_trailing_newline() {
        assert_eq!(fmt(""), "");
    }

    // ---------- Config ----------

    #[test]
    fn narrow_config_breaks_aggressively() {
        let cfg = Config { print_width: 20, indent_width: 2 };
        let out = fmt_with("(if #t (do-something) (do-another-thing))", cfg);
        assert!(out.contains('\n'), "narrow cap should break, got:\n{out}");
    }

    #[test]
    fn wide_config_keeps_compact() {
        let cfg = Config { print_width: 1000, indent_width: 2 };
        let out = fmt_with(
            "(if (some-predicate is-it-true here) (then-do action-one) (else-do action-two))",
            cfg,
        );
        // Should fit on one line (single line + trailing newline).
        assert_eq!(out.matches('\n').count(), 1, "got:\n{out}");
    }

    #[test]
    fn custom_indent_width() {
        let cfg = Config { print_width: 10, indent_width: 4 };
        let src = "(begin (long-thing-one) (long-thing-two))";
        let out = fmt_with(src, cfg);
        assert!(out.contains('\n'));
        // Nested content should be indented 4 cols.
        assert!(out.contains("    "));
    }

    // ---------- Errors ----------

    #[test]
    fn unparseable_input_returns_format_error() {
        let err = format("(unbalanced").unwrap_err();
        assert!(matches!(err, FormatError::Parse(_)));
    }

    #[test]
    fn format_error_displays() {
        let err = format("(unbalanced").unwrap_err();
        let s = err.to_string();
        assert!(s.starts_with("parse error:"), "got: {s}");
    }

    // ---------- program_to_doc surface ----------

    #[test]
    fn program_to_doc_returns_a_doc_for_external_paint_pipelines() {
        let p = parse("(define x 42)").expect("parse");
        let doc = program_to_doc(&p);
        // Smoke check: the layout via format-doc renders the expected text.
        let layout = layout_doc(doc, &LayoutOptions::default());
        let s = render_text(&layout);
        assert_eq!(s, "(define x 42)");
    }

    // ---------- Real-world snippets ----------

    #[test]
    fn factorial_canonical_layout() {
        let src = "(define (factorial n) (if (= n 0) 1 (* n (factorial (- n 1)))))";
        let formatted = fmt(src);
        // Whatever the layout, it must round-trip through parse.
        assert_semantic_preserving(src);
        // And formatter must be idempotent.
        assert_idempotent(src);
        // Trailing newline.
        assert!(formatted.ends_with('\n'));
    }

    #[test]
    fn nested_let_canonical_layout() {
        let src = "(let ((x 1)) (let ((y 2)) (+ x y)))";
        assert_semantic_preserving(src);
        assert_idempotent(src);
    }
}
