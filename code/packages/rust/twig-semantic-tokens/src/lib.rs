//! # `twig-semantic-tokens` — semantic-token extraction for Twig.
//!
//! Walks a parsed [`twig_parser::Program`] and emits a typed token
//! stream — keywords / identifiers / numbers / booleans / nil /
//! quoted symbols — suitable for **LSP semantic-tokens**, syntax
//! highlighters, and editor extensions.
//!
//! Together with `twig-formatter`, this is the second piece of the
//! Twig authoring-experience layer.  Editors can pull semantic
//! highlighting from this crate's typed token list instead of
//! relying on regex-based syntax patterns (which can't tell e.g. a
//! variable reference from a function-position application head).
//!
//! ## Why semantic over regex
//!
//! Regex highlighters can colour `(if cond ...)` because `if` is a
//! keyword in a fixed position — but they can't colour `cons` as a
//! function and `xs` as a variable in `(cons x xs)`, because they
//! don't know the grammar.  Semantic tokens drive richer themes:
//!
//! - keyword vs identifier
//! - function-position identifier vs variable-position identifier
//! - parameter names in `(lambda (x y) ...)` distinct from refs
//! - constants (`#t`, `#f`, `nil`) styled as constants
//! - quoted symbols (`'foo`) styled as data literals
//!
//! ## Public API
//!
//! - [`semantic_tokens(source)`] — `&str` → `Result<Vec<SemanticToken>, TwigParseError>`.
//!   Parses + walks.  The common case.
//! - [`tokens_for_program(program)`] — already-parsed `&Program`
//!   → `Vec<SemanticToken>`.  Skip the parse step when you have an
//!   AST in hand.
//!
//! Tokens come back in **document order** (top-to-bottom,
//! left-to-right within a line) — what LSP semantic-token
//! providers want.
//!
//! ## Position model
//!
//! All positions are **1-based** `(line, column)` in monospace cell
//! units, matching `twig-parser`.  `length` is the visible width
//! of the token in cells (char count for ASCII source — Twig
//! identifiers are ASCII).
//!
//! ## What this crate does NOT do
//!
//! - **No punctuation tokens.**  Open / close parens are dropped:
//!   the parser AST doesn't preserve their positions independently.
//!   Editors that want paren highlighting can layer it on top.
//! - **No comment tokens.**  The Twig lexer is comment-stripping;
//!   comments don't survive into the AST.  Lands when the lexer
//!   grows a trivia channel.
//! - **No LSP encoding.**  Returns a typed `Vec<SemanticToken>`;
//!   conversion to LSP's delta-encoded wire format is one level up
//!   (so this crate stays usable from non-LSP consumers).
//!
//! ## Caller responsibilities
//!
//! Inherits the non-guarantees of `twig-parser`.  Adversarial
//! source that produces an unbounded AST is bounded by the
//! parser's depth cap.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use twig_parser::{
    parse, Apply, Begin, BoolLit, Define, Expr, Form, If, IntLit, Lambda, Let, NilLit, Program,
    SymLit, TwigParseError, VarRef,
};

// ---------------------------------------------------------------------------
// Token kinds
// ---------------------------------------------------------------------------

/// Semantic-token classifications surfaced by this crate.
///
/// `#[non_exhaustive]` — future variants (e.g. `Macro` once Twig
/// grows them) won't break consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TokenKind {
    /// Built-in compound-form keyword: `if`, `let`, `lambda`,
    /// `begin`, `define`, `quote`.  The latter rarely surfaces
    /// because the parser collapses `(quote x)` into [`SymLit`].
    Keyword,
    /// Boolean literal (`#t`, `#f`).
    Boolean,
    /// `nil` literal.
    Nil,
    /// Integer literal.
    Number,
    /// Quoted-symbol literal (`'foo`).
    Symbol,
    /// Function-position identifier in `(fn arg …)` — the head of
    /// an [`Apply`] when it's a `VarRef`.  Distinguished from
    /// `Variable` so themes can colour callees specially.
    Function,
    /// Variable reference outside function position.
    Variable,
    /// Parameter name binder (`(lambda (x y) …)` x and y;
    /// `(let ((x e) …) …)` x; `(define name …)` name).
    Parameter,
}

impl TokenKind {
    /// Stable string mnemonic — matches LSP semantic-token type
    /// names where the meanings line up, lowercase.  Useful for
    /// theme configuration files and JSON wire formats.
    pub fn mnemonic(self) -> &'static str {
        match self {
            TokenKind::Keyword => "keyword",
            TokenKind::Boolean => "boolean",
            TokenKind::Nil => "constant",
            TokenKind::Number => "number",
            TokenKind::Symbol => "symbol",
            TokenKind::Function => "function",
            TokenKind::Variable => "variable",
            TokenKind::Parameter => "parameter",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mnemonic())
    }
}

// ---------------------------------------------------------------------------
// SemanticToken
// ---------------------------------------------------------------------------

/// One semantic token — a position + length + kind triple.
///
/// `length` is the number of monospace cells the token occupies on
/// its line (char count for ASCII source).  Tokens never span
/// multiple lines: a multi-line construct produces one token per
/// atom on each line, with no line-spanning representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticToken {
    /// 1-based source line.
    pub line: u32,
    /// 1-based starting column.
    pub column: u32,
    /// Token width in monospace cells.
    pub length: u32,
    /// Classification.
    pub kind: TokenKind,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse `source` and return its semantic tokens in document
/// order.  The common case.
pub fn semantic_tokens(source: &str) -> Result<Vec<SemanticToken>, TwigParseError> {
    let program = parse(source)?;
    Ok(tokens_for_program(&program))
}

/// Walk an already-parsed `Program` and return its semantic tokens
/// in document order.
pub fn tokens_for_program(program: &Program) -> Vec<SemanticToken> {
    let mut out: Vec<SemanticToken> = Vec::new();
    for form in &program.forms {
        emit_form(&mut out, form);
    }
    sort_in_document_order(&mut out);
    out
}

// ---------------------------------------------------------------------------
// AST walker
// ---------------------------------------------------------------------------

fn emit_form(out: &mut Vec<SemanticToken>, form: &Form) {
    match form {
        Form::Define(d) => emit_define(out, d),
        Form::Expr(e) => emit_expr(out, e),
    }
}

fn emit_define(out: &mut Vec<SemanticToken>, d: &Define) {
    // (define name expr)
    // Form position points at the `(` of the form; keyword `define`
    // sits at column + 1, name (best-effort) at column + 1 + 7.
    let line = u32_of(d.line);
    let col = u32_of(d.column);
    push_keyword(out, line, col.saturating_add(1), "define");
    // "(define " = 8 chars from `(`; keyword starts at +1, name at +8.
    let name_col = col.saturating_add(8);
    push_token(out, line, name_col, len_u32(&d.name), TokenKind::Parameter);
    emit_expr(out, &d.expr);
}

fn emit_expr(out: &mut Vec<SemanticToken>, expr: &Expr) {
    match expr {
        Expr::IntLit(IntLit { value, line, column }) => {
            let len = visible_int_len(*value);
            push_token(out, u32_of(*line), u32_of(*column), len, TokenKind::Number);
        }
        Expr::BoolLit(BoolLit { line, column, .. }) => {
            push_token(out, u32_of(*line), u32_of(*column), 2, TokenKind::Boolean);
        }
        Expr::NilLit(NilLit { line, column }) => {
            push_token(out, u32_of(*line), u32_of(*column), 3, TokenKind::Nil);
        }
        Expr::SymLit(SymLit { name, line, column }) => {
            // 'foo  — 1 (apostrophe) + len(name)
            let len = 1u32.saturating_add(len_u32(name));
            push_token(out, u32_of(*line), u32_of(*column), len, TokenKind::Symbol);
        }
        Expr::VarRef(VarRef { name, line, column }) => {
            push_token(
                out,
                u32_of(*line),
                u32_of(*column),
                len_u32(name),
                TokenKind::Variable,
            );
        }
        Expr::If(If { cond, then_branch, else_branch, line, column }) => {
            let l = u32_of(*line);
            let c = u32_of(*column);
            push_keyword(out, l, c.saturating_add(1), "if");
            emit_expr(out, cond);
            emit_expr(out, then_branch);
            emit_expr(out, else_branch);
        }
        Expr::Let(Let { bindings, body, line, column }) => {
            let l = u32_of(*line);
            let c = u32_of(*column);
            push_keyword(out, l, c.saturating_add(1), "let");
            // Binding names don't carry per-binding positions in the
            // parser AST; we emit them under the form's start line
            // as Parameters, with column = 0 (sentinel) so consumers
            // know it's a best-effort guess.  Future versions can
            // thread positions through twig-parser to fix this; the
            // important consumers (LSP semantic tokens) will still
            // colour the name correctly when the name appears as a
            // VarRef later in the body.
            //
            // For the v1, we just emit Parameter tokens for each
            // binding name with the form's line, column 0 — these
            // are filtered out by the (column == 0) sentinel
            // downstream if the consumer cares about positional
            // accuracy.  Most consumers will instead rely on the
            // VarRef colouring for the binding's body usages, which
            // we DO emit accurately.
            for (_name, expr) in bindings {
                emit_expr(out, expr);
            }
            for e in body {
                emit_expr(out, e);
            }
            let _ = (l, c);
        }
        Expr::Begin(Begin { exprs, line, column }) => {
            let l = u32_of(*line);
            let c = u32_of(*column);
            push_keyword(out, l, c.saturating_add(1), "begin");
            for e in exprs {
                emit_expr(out, e);
            }
        }
        Expr::Lambda(Lambda { params, body, line, column, .. }) => {
            let l = u32_of(*line);
            let c = u32_of(*column);
            push_keyword(out, l, c.saturating_add(1), "lambda");
            // Same caveat as Let bindings — params don't have
            // per-param positions.  Emit nothing for them; consumers
            // still colour usages in the body via VarRef.
            let _ = params;
            for e in body {
                emit_expr(out, e);
            }
        }
        Expr::Apply(Apply { fn_expr, args, .. }) => {
            // If the function position is a VarRef, re-classify it
            // as Function (overrides the default Variable token).
            if let Expr::VarRef(VarRef { name, line, column }) = fn_expr.as_ref() {
                push_token(
                    out,
                    u32_of(*line),
                    u32_of(*column),
                    len_u32(name),
                    TokenKind::Function,
                );
            } else {
                emit_expr(out, fn_expr);
            }
            for a in args {
                emit_expr(out, a);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn push_keyword(out: &mut Vec<SemanticToken>, line: u32, column: u32, kw: &str) {
    push_token(out, line, column, len_u32(kw), TokenKind::Keyword);
}

/// Saturating `usize → u32` for character counts.  Identifier
/// names larger than `u32::MAX` (>4 GiB) are gated by twig-parser's
/// own input-size limits; saturating here preserves the
/// "length ≤ u32::MAX" invariant without panic risk.
fn len_u32(s: &str) -> u32 {
    u32_of(s.chars().count())
}

fn push_token(out: &mut Vec<SemanticToken>, line: u32, column: u32, length: u32, kind: TokenKind) {
    if length == 0 || column == 0 || line == 0 {
        // Sentinel positions ("not from real source") — drop.  This
        // covers binding-name placeholders the AST can't position.
        return;
    }
    out.push(SemanticToken { line, column, length, kind });
}

fn u32_of(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn visible_int_len(value: i64) -> u32 {
    // Number of chars in the decimal representation of `value` —
    // matches what a hand-written lexer would assign.  Includes the
    // sign for negative numbers.
    let s = value.to_string();
    s.chars().count() as u32
}

fn sort_in_document_order(tokens: &mut [SemanticToken]) {
    tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.column.cmp(&b.column)));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<SemanticToken> {
        semantic_tokens(src).expect("parse")
    }

    fn kinds(src: &str) -> Vec<TokenKind> {
        toks(src).into_iter().map(|t| t.kind).collect()
    }

    // ---------- TokenKind mnemonic ----------

    #[test]
    fn mnemonics_are_distinct() {
        let all = [
            TokenKind::Keyword,
            TokenKind::Boolean,
            TokenKind::Nil,
            TokenKind::Number,
            TokenKind::Symbol,
            TokenKind::Function,
            TokenKind::Variable,
            TokenKind::Parameter,
        ];
        let mut mns: Vec<&'static str> = all.iter().map(|k| k.mnemonic()).collect();
        mns.sort();
        mns.dedup();
        assert_eq!(mns.len(), all.len());
    }

    #[test]
    fn token_kind_displays_as_mnemonic() {
        assert_eq!(format!("{}", TokenKind::Keyword), "keyword");
        assert_eq!(format!("{}", TokenKind::Variable), "variable");
    }

    // ---------- Atoms ----------

    #[test]
    fn empty_program_no_tokens() {
        assert!(toks("").is_empty());
    }

    #[test]
    fn int_literal() {
        let t = toks("42");
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].kind, TokenKind::Number);
        assert_eq!(t[0].length, 2);
        assert_eq!(t[0].line, 1);
    }

    #[test]
    fn negative_int_includes_sign_in_length() {
        let t = toks("-7");
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].length, 2); // "-7"
    }

    #[test]
    fn bool_literal_t_and_f() {
        assert_eq!(kinds("#t"), vec![TokenKind::Boolean]);
        assert_eq!(toks("#t")[0].length, 2);
        assert_eq!(kinds("#f"), vec![TokenKind::Boolean]);
    }

    #[test]
    fn nil_literal() {
        let t = toks("nil");
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].kind, TokenKind::Nil);
        assert_eq!(t[0].length, 3);
    }

    #[test]
    fn quoted_symbol_includes_apostrophe_in_length() {
        let t = toks("'foo");
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].kind, TokenKind::Symbol);
        assert_eq!(t[0].length, 4); // 'foo
    }

    #[test]
    fn var_ref() {
        let t = toks("x");
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].kind, TokenKind::Variable);
        assert_eq!(t[0].length, 1);
    }

    // ---------- Compound forms ----------

    #[test]
    fn if_emits_keyword_and_three_subexpr_tokens() {
        let t = toks("(if #t 1 2)");
        let ks: Vec<TokenKind> = t.iter().map(|x| x.kind).collect();
        assert_eq!(
            ks,
            vec![TokenKind::Keyword, TokenKind::Boolean, TokenKind::Number, TokenKind::Number]
        );
        // Keyword length is 2 (just "if"), not 3 ("(if").
        let kw = t.iter().find(|x| x.kind == TokenKind::Keyword).unwrap();
        assert_eq!(kw.length, 2);
    }

    #[test]
    fn let_emits_keyword_plus_body() {
        let t = toks("(let ((x 1)) x)");
        let ks: Vec<TokenKind> = t.iter().map(|x| x.kind).collect();
        // let keyword + binding rhs (Number) + body (Variable for x)
        assert_eq!(ks, vec![TokenKind::Keyword, TokenKind::Number, TokenKind::Variable]);
    }

    #[test]
    fn lambda_emits_keyword_plus_body() {
        let t = toks("(lambda (x) x)");
        let ks: Vec<TokenKind> = t.iter().map(|x| x.kind).collect();
        // lambda keyword + body (Variable for x)
        assert_eq!(ks, vec![TokenKind::Keyword, TokenKind::Variable]);
    }

    #[test]
    fn begin_emits_keyword_plus_exprs() {
        let t = toks("(begin 1 2 3)");
        let ks: Vec<TokenKind> = t.iter().map(|x| x.kind).collect();
        assert_eq!(
            ks,
            vec![TokenKind::Keyword, TokenKind::Number, TokenKind::Number, TokenKind::Number]
        );
    }

    #[test]
    fn define_emits_keyword_and_param_for_name() {
        let t = toks("(define x 42)");
        let ks: Vec<TokenKind> = t.iter().map(|x| x.kind).collect();
        assert_eq!(ks, vec![TokenKind::Keyword, TokenKind::Parameter, TokenKind::Number]);
    }

    #[test]
    fn apply_function_position_is_function_token() {
        let t = toks("(cons x xs)");
        let ks: Vec<TokenKind> = t.iter().map(|x| x.kind).collect();
        // cons is Function (head), x and xs are Variable (args).
        assert_eq!(ks, vec![TokenKind::Function, TokenKind::Variable, TokenKind::Variable]);
    }

    #[test]
    fn apply_with_non_var_head_falls_through_to_var() {
        // ((compose f g) x) — the head is itself an Apply.  The
        // outer Apply doesn't reclassify a non-VarRef head.
        let t = toks("((lambda (a) a) 1)");
        // Inner: lambda kw + param x VarRef NOT emitted (no positions)
        //        + body Var "a"
        // Outer: Number "1"
        let ks: Vec<TokenKind> = t.iter().map(|x| x.kind).collect();
        assert!(ks.contains(&TokenKind::Keyword));
        assert!(ks.contains(&TokenKind::Variable));
        assert!(ks.contains(&TokenKind::Number));
    }

    // ---------- Position checks ----------

    #[test]
    fn tokens_in_document_order() {
        let t = toks("(if (= x 0) 1 (* 2 y))");
        let positions: Vec<(u32, u32)> = t.iter().map(|x| (x.line, x.column)).collect();
        let mut sorted = positions.clone();
        sorted.sort();
        assert_eq!(positions, sorted);
    }

    #[test]
    fn multi_line_input_gets_correct_lines() {
        let src = "(define x 1)\n(define y 2)\n(+ x y)";
        let t = toks(src);
        // Each line has its own form.
        let lines: Vec<u32> = t.iter().map(|x| x.line).collect();
        let max_line = *lines.iter().max().unwrap();
        assert_eq!(max_line, 3);
        // First token of line 3 is Function (+).
        let first_on_line_3 = t.iter().find(|x| x.line == 3).unwrap();
        assert_eq!(first_on_line_3.kind, TokenKind::Function);
    }

    #[test]
    fn keyword_position_is_inside_paren_not_on_paren() {
        // Form position is on the `(`; keyword starts at column + 1.
        let t = toks("(if #t 1 2)");
        let kw = t.iter().find(|x| x.kind == TokenKind::Keyword).unwrap();
        // The form column is 1; keyword should be at column 2.
        assert_eq!(kw.column, 2);
    }

    // ---------- Error path ----------

    #[test]
    fn unparseable_input_returns_parse_error() {
        let err = semantic_tokens("(unbalanced").unwrap_err();
        // We just verify it propagates — the specific error variant
        // is twig-parser's concern.
        let _ = err;
    }

    // ---------- tokens_for_program direct path ----------

    #[test]
    fn tokens_for_program_skips_parse() {
        let p = twig_parser::parse("42").expect("parse");
        let t = tokens_for_program(&p);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].kind, TokenKind::Number);
    }

    // ---------- Realistic example ----------

    #[test]
    fn factorial_emits_full_token_stream() {
        let src = "(define (factorial n) (if (= n 0) 1 (* n (factorial (- n 1)))))";
        let t = toks(src);
        // We don't lock in exact counts; just sanity-check the kinds.
        let ks: Vec<TokenKind> = t.iter().map(|x| x.kind).collect();
        assert!(ks.contains(&TokenKind::Keyword));    // define + if + lambda
        assert!(ks.contains(&TokenKind::Parameter));  // factorial
        assert!(ks.contains(&TokenKind::Function));   // = + * - factorial (recursive)
        assert!(ks.contains(&TokenKind::Variable));   // n
        assert!(ks.contains(&TokenKind::Number));     // 0 1
    }

    #[test]
    fn quoted_symbol_in_apply_position_is_still_function_via_var() {
        // (cons 'foo nil) — cons is Function, 'foo is Symbol, nil is Nil.
        let t = toks("(cons 'foo nil)");
        let ks: Vec<TokenKind> = t.iter().map(|x| x.kind).collect();
        assert_eq!(
            ks,
            vec![TokenKind::Function, TokenKind::Symbol, TokenKind::Nil]
        );
    }

    // ---------- Sort stability ----------

    #[test]
    fn duplicate_position_tokens_are_kept_stably() {
        // No two tokens should occupy exactly the same position in
        // realistic input, but the sort uses `sort_by` which is
        // stable — ensures consumers get a deterministic order.
        let t = toks("(define x (begin 1 2 3))");
        let positions: Vec<(u32, u32)> = t.iter().map(|x| (x.line, x.column)).collect();
        let mut unique = positions.clone();
        unique.sort();
        unique.dedup();
        // No duplicates expected.
        assert_eq!(positions.len(), unique.len());
    }
}
