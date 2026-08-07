//! # `mccarthy-lisp-parser` — S-expression parser for McCarthy Lisp 1.0.
//!
//! Turns McCarthy 1960 Lisp source into a typed [`LispExpr`] AST that
//! matches McCarthy's 1960 paper definition exactly:
//!
//! ```text
//! e ::= NIL                            ; the empty list  ()
//!     | Symbol                         ; FOO, CAR, X
//!     | Integer                        ; 42, -1, 0
//!     | (e . e)                        ; dotted pair (cons cell)
//! ```
//!
//! A "list" in McCarthy's paper is a *nested cons cell terminated by
//! NIL*: `(A B C)` is sugar for `(A . (B . (C . NIL)))`.  The
//! extractor performs that desugaring — every list literal materialises
//! as [`LispExpr::Cons`] cells with a [`LispExpr::Nil`] terminator.
//!
//! ## This crate is a *thin wrapper*, not a hand-written parser
//!
//! The S-expression grammar lives in
//! [`code/grammars/mccarthy_lisp.grammar`](../../../grammars/mccarthy_lisp.grammar),
//! which `build.rs` compiles to Rust at build time.  Parsing happens in
//! two stages — exactly the `twig-parser` pattern (see
//! [`feedback_no_handwritten_lexers_parsers`]):
//!
//! 1. **Grammar parse** — the shared [`parser::grammar_parser::GrammarParser`]
//!    turns the token stream into a generic concrete syntax tree
//!    ([`GrammarASTNode`]).  All the structural rules — balanced parens,
//!    at-most-one dotted tail, "dot must follow an element" — are
//!    enforced *here, by the grammar*, so there is no hand-written
//!    validation to drift out of sync.
//! 2. **AST extraction** — [`extract_program`] lowers that CST to the
//!    typed [`LispExpr`] tree, applying the two sugar expansions:
//!      * `'X` → `(QUOTE X)`
//!      * `(A B C)` → `(A . (B . (C . NIL)))`
//!
//! ## NIL vs ()
//!
//! Like the v0.1 hand-written parser, the *symbol* `NIL` and the empty
//! list `()` are kept distinct at this layer: `()` extracts to
//! [`LispExpr::Nil`], while a literal `NIL` token extracts to
//! `Symbol("NIL")`.  Unifying them (as real Lisp does) is a semantic
//! decision left to the L2 `mccarthy-lisp-iir-compiler`.
//!
//! ## Quick start
//!
//! ```
//! use mccarthy_lisp_parser::{parse, LispExpr};
//!
//! // (CAR '(A B C)) → take CAR of a literal list
//! let prog = parse("(CAR '(A B C))").expect("parse");
//! assert_eq!(prog.len(), 1);
//!
//! let expected = LispExpr::list([
//!     LispExpr::sym("CAR"),
//!     LispExpr::quote(LispExpr::list([
//!         LispExpr::sym("A"),
//!         LispExpr::sym("B"),
//!         LispExpr::sym("C"),
//!     ])),
//! ]);
//! assert_eq!(prog[0], expected);
//! ```

#![warn(missing_docs)]

use std::fmt;
use std::sync::OnceLock;

use grammar_tools::parser_grammar::ParserGrammar;
use lexer::token::{Token, TokenType};
use mccarthy_lisp_lexer::{tokenize_mccarthy, LexerError};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParser};

// ===========================================================================
// AST
// ===========================================================================

/// McCarthy 1960 Lisp S-expression.
///
/// Four cases — matches the abstract grammar of the 1960 paper exactly.
/// Modern Lisps add strings, vectors, hash tables, etc.; McCarthy 1.0
/// has only these four.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LispExpr {
    /// `()` — the empty list, also the false value (any other value is
    /// true under McCarthy's `COND`).
    Nil,
    /// `FOO` — an all-uppercase identifier (interned later as a symbol
    /// pointer at runtime).
    Symbol(String),
    /// `42` — a signed 64-bit integer.
    Int(i64),
    /// `(car . cdr)` — a cons cell, the only compound shape.
    Cons(Box<LispExpr>, Box<LispExpr>),
}

impl LispExpr {
    /// Convenience: build a `Symbol`.
    pub fn sym(name: impl Into<String>) -> Self {
        LispExpr::Symbol(name.into())
    }

    /// Build a McCarthy-style list (`a b c`) as nested `Cons` cells
    /// terminated by `Nil`: `(a . (b . (c . NIL)))`.
    pub fn list<I: IntoIterator<Item = LispExpr>>(items: I) -> Self {
        let v: Vec<LispExpr> = items.into_iter().collect();
        let mut acc = LispExpr::Nil;
        for x in v.into_iter().rev() {
            acc = LispExpr::Cons(Box::new(x), Box::new(acc));
        }
        acc
    }

    /// Wrap an expression in `(QUOTE …)` — the standard expansion of the
    /// `'` reader macro.
    pub fn quote(inner: LispExpr) -> Self {
        LispExpr::list([LispExpr::sym("QUOTE"), inner])
    }
}

impl fmt::Display for LispExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LispExpr::Nil => write!(f, "NIL"),
            LispExpr::Symbol(s) => write!(f, "{s}"),
            LispExpr::Int(n) => write!(f, "{n}"),
            LispExpr::Cons(car, cdr) => write!(f, "({car} . {cdr})"),
        }
    }
}

/// Iterative destructor — drops the tree without recursing.
///
/// The default (compiler-generated) `Drop` for a recursive boxed enum
/// unwinds one stack frame per `Cons` cell.  A *flat* list `(A A … A)`
/// of N elements is only paren-depth 1 (so it slips past the parser's
/// `MAX_PAREN_DEPTH` guard) yet is N cons cells long, so default-dropping
/// it would recurse N native frames deep and overflow the stack — a
/// cheap single-line DoS on any consumer that builds and drops such an
/// AST (including [`parse`] itself).
///
/// We instead dismantle the tree iteratively: replace each `Cons`'s
/// children with `Nil`, pushing the real children onto a heap-allocated
/// work stack, and let each detached node drop trivially (its remaining
/// children are `Nil`).  Stack usage is O(1); the work list lives on the
/// heap.
impl Drop for LispExpr {
    fn drop(&mut self) {
        // Only `Cons` owns heap children worth dismantling.
        let mut stack: Vec<LispExpr> = Vec::new();
        if let LispExpr::Cons(car, cdr) = self {
            stack.push(std::mem::replace(car.as_mut(), LispExpr::Nil));
            stack.push(std::mem::replace(cdr.as_mut(), LispExpr::Nil));
        }
        while let Some(mut node) = stack.pop() {
            if let LispExpr::Cons(car, cdr) = &mut node {
                stack.push(std::mem::replace(car.as_mut(), LispExpr::Nil));
                stack.push(std::mem::replace(cdr.as_mut(), LispExpr::Nil));
            }
            // `node` is now a leaf or a `Cons(Nil, Nil)`; dropping it here
            // recurses at most one level (into two `Nil`s) — never deep.
        }
    }
}

// ===========================================================================
// Errors
// ===========================================================================

/// A parse-time error.
///
/// Wraps any of:
/// - a lexer failure (invalid character — e.g. lowercase, a bare `-`),
/// - a [`GrammarParser`] failure (unbalanced paren, stray dot, …),
/// - an extractor-detected shape problem (integer overflow, depth
///   limit).
///
/// Source positions are 1-indexed and point at the offending token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable explanation.
    pub message: String,
    /// 1-based line number of the offending token.
    pub line: usize,
    /// 1-based column number of the offending token.
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<LexerError> for ParseError {
    fn from(e: LexerError) -> Self {
        ParseError { message: format!("lexer error: {}", e.message), line: e.line, column: e.column }
    }
}

impl From<parser::grammar_parser::GrammarParseError> for ParseError {
    fn from(e: parser::grammar_parser::GrammarParseError) -> Self {
        ParseError { message: e.message, line: e.token.line, column: e.token.column }
    }
}

// ===========================================================================
// DoS-hardening depth guards
// ===========================================================================

/// Maximum *structural nesting* depth permitted before the grammar
/// parser runs.
///
/// The downstream [`GrammarParser`] is recursive and has **no** internal
/// recursion-depth limit, so a deeply-nested input drives it into
/// native call-stack recursion — one frame per nesting level — until
/// the OS thread stack overflows.  That overflow is a `SIGABRT` Rust
/// cannot catch: it crashes the whole process.  We therefore reject
/// over-deep sources up front by scanning the token stream.
///
/// **Two token shapes add nesting depth, not just parens:**
///
/// - `LPAREN` — `((((…))))`, via the `list` rule.
/// - `QUOTE`  — `''''…X`, via the `quoted = QUOTE sexpr` rule, which
///   recurses *without consuming a paren*.  A long run of `'` has
///   paren-depth 0 yet unbounded parser-recursion depth — a ~5 KB
///   all-`'` file is enough to abort the process if only parens are
///   counted.  [`check_nesting_depth`] therefore bounds the *combined*
///   paren + pending-quote depth.
///
/// The cap is **64**, matching `twig-parser`'s `MAX_PAREN_DEPTH`.  The
/// generic `GrammarParser` spends several recursion frames per nesting
/// level (`program → sexpr → list → list_body → sexpr → …`), so a
/// default 2 MiB Rust test-thread stack overflows somewhere around ~80
/// levels — 64 leaves a comfortable margin while still admitting any
/// realistic McCarthy program (the deepest example in the 1960 paper is
/// ~10 levels deep).  Note this is necessarily lower than the v0.1
/// hand-written parser's 256: that recursive-descent parser used far
/// less stack per level than the shared grammar engine does.
pub const MAX_PAREN_DEPTH: usize = 64;

/// Maximum AST-extraction recursion depth.
///
/// A second guard, applied while lowering the CST to [`LispExpr`], in
/// case a future grammar change lets a deeply-nested tree through the
/// paren check (e.g. long `'''''…X` quote chains, which add depth
/// without parens).
pub const MAX_AST_DEPTH: usize = 256;

/// Reject a source whose maximum *structural nesting* (parens **and**
/// pending quotes) exceeds [`MAX_PAREN_DEPTH`], pointing at the token
/// that tipped it over.
///
/// This runs **before** [`GrammarParser::parse`], which has no internal
/// recursion guard, so it is the only thing standing between untrusted
/// input and a stack-overflow `SIGABRT`.  It must therefore account for
/// *every* token shape that adds parser-recursion depth — both `LPAREN`
/// (the `list` rule) and `QUOTE` (the `quoted` rule, which recurses
/// without a paren).
///
/// We track an explicit stack of open contexts.  A `QUOTE` opens a
/// pending-quote context that is *discharged* as soon as its single
/// operand sexpr completes (an atom, or a list closed by `RPAREN`); an
/// `LPAREN` opens a list context closed by its `RPAREN`.  The high-water
/// mark of the stack is the maximum live recursion depth the grammar
/// parser will reach.  (Slight miscounting on *malformed* input is
/// harmless — such input is rejected by the grammar anyway; what matters
/// is that we never *under*-count a well-formed deep nest.)
fn check_nesting_depth(tokens: &[Token]) -> Result<(), ParseError> {
    // 'p' = open paren (list), 'q' = pending quote.
    let mut stack: Vec<u8> = Vec::new();

    // Discharge every pending quote sitting on top of the stack: a
    // completed sexpr satisfies all quotes directly wrapping it.
    fn discharge_quotes(stack: &mut Vec<u8>) {
        while matches!(stack.last(), Some(b'q')) {
            stack.pop();
        }
    }

    for t in tokens {
        match t.type_ {
            TokenType::LParen => stack.push(b'p'),
            TokenType::RParen => {
                // The list just closed is itself a completed sexpr: pop
                // its paren, then discharge any quotes wrapping the list.
                // Pop down to and including the nearest paren (tolerating
                // stray pending quotes on malformed input).
                while matches!(stack.last(), Some(b'q')) {
                    stack.pop();
                }
                if matches!(stack.last(), Some(b'p')) {
                    stack.pop();
                }
                discharge_quotes(&mut stack);
            }
            // A bare quote token: only QUOTE maps to type_name "QUOTE".
            _ if t.effective_type_name() == "QUOTE" => stack.push(b'q'),
            // An atom (SYMBOL / INTEGER) completes a sexpr.
            _ if matches!(t.effective_type_name(), "SYMBOL" | "INTEGER") => {
                discharge_quotes(&mut stack);
            }
            _ => {}
        }

        if stack.len() > MAX_PAREN_DEPTH {
            return Err(ParseError {
                message: format!(
                    "structural nesting (parens + quotes) exceeds MAX_PAREN_DEPTH \
                     ({MAX_PAREN_DEPTH}) — refusing to invoke the parser to avoid \
                     stack overflow"
                ),
                line: t.line,
                column: t.column,
            });
        }
    }
    Ok(())
}

fn check_depth(depth: usize, line: usize, column: usize) -> Result<(), ParseError> {
    if depth > MAX_AST_DEPTH {
        Err(ParseError {
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

// ===========================================================================
// Build-time-compiled parser grammar
// ===========================================================================

mod generated_grammar {
    include!(concat!(env!("OUT_DIR"), "/mccarthy_lisp_parser_grammar.rs"));
}

static MCCARTHY_PARSER_GRAMMAR: OnceLock<ParserGrammar> = OnceLock::new();

fn mccarthy_parser_grammar() -> &'static ParserGrammar {
    MCCARTHY_PARSER_GRAMMAR.get_or_init(generated_grammar::parser_grammar)
}

/// Borrow the build-time-compiled McCarthy Lisp [`ParserGrammar`].
///
/// Re-exported for tooling that needs to inspect the rules without
/// re-parsing the canonical `.grammar` file.
pub fn mccarthy_grammar() -> &'static ParserGrammar {
    mccarthy_parser_grammar()
}

// ===========================================================================
// Public API
// ===========================================================================

/// Recursion-depth cap for the [`GrammarParser`] itself — a *second*,
/// independent layer of defense alongside this crate's existing pre-scan
/// (`check_nesting_depth`/[`MAX_PAREN_DEPTH`]). That pre-scan already
/// rejects excessive combined paren+quote nesting before the parser ever
/// runs, but `create_mccarthy_parser_from_tokens` is a public entry point
/// (documented for editor/LSP integrations that already hold a token
/// stream) that bypasses it entirely, so a caller invoking it directly
/// with adversarial tokens could still hit the same
/// native-stack-overflow DoS the pre-scan was meant to close.
///
/// # Two independent recursive shapes
///
/// - **List nesting** — `sexpr -> list -> list_body -> sexpr` (3
///   rule-frames per real nesting level).
/// - **Quote-chain** — `sexpr -> quoted -> sexpr` (2 rule-frames per real
///   nesting level).
///
/// Measured (binary search, uncapped parser, on the true default-stack
/// per-test worker thread — no `RUST_MIN_STACK` override and no explicit
/// `Builder::stack_size`, matching what `cargo test` and a production
/// caller both actually get — debug build, adversarial 5000-level input,
/// bypassing `check_nesting_depth` entirely to measure the parser's own
/// floor): list nesting (the *binding*, lower floor) safe through 260
/// rule-frames, crashes at 262; quote-chain safe through 280, crashes at
/// 290.
///
/// `MAX_RULE_DEPTH` is set to **180** — about 31% below the binding
/// 260-rule-frame floor (comparable margin to sibling crates' 25-45%
/// convention), independently confirmed not to crash a default-stack
/// thread even thousands of rule-frames past the cap for either shape
/// (see this crate's tests). Measured real-nesting headroom at 180
/// (capped parser, so no crash risk): list nesting parses cleanly up to
/// 59 levels (60 trips the cap), quote-chain up to 88 levels (89 trips
/// the cap) — comfortably past any hand-written Lisp program's real
/// nesting.
///
/// Note this cap is *more* restrictive than `MAX_PAREN_DEPTH` (64) for the
/// list-nesting shape specifically: `MAX_PAREN_DEPTH`'s own pre-scan still
/// admits up to 64 levels of list nesting through `parse`/`parse_to_cst`,
/// but at 3 rule-frames/level that would need `MAX_RULE_DEPTH` ≥ 192 to
/// avoid re-rejecting anything `MAX_PAREN_DEPTH` already allows. Unlike
/// twig-parser's analogous note, 192 is *not* blocked by the other shape's
/// safety floor here — quote-chain's 280-rule-frame floor has headroom to
/// spare either way, and list-nesting is itself the binding (lower) floor
/// at 260, so 192 would still sit safely below it. The reason `180` was
/// chosen over a value nearer `192` is simply the same 25-45% margin
/// convention used across sibling crates, not a hard ceiling imposed by a
/// second shape. Real Lisp programs are single-digit-deep (per
/// `MAX_PAREN_DEPTH`'s own doc comment), so the narrower envelope (59 vs.
/// 64 levels) has no practical effect; it's flagged here so a future
/// re-tuning of either constant doesn't assume the two guards agree.
const MAX_RULE_DEPTH: usize = 180;

/// Build a [`GrammarParser`] from a pre-tokenized stream.
///
/// Useful for editor / LSP integrations that already hold a
/// `Vec<Token>` (e.g. from `mccarthy_lisp_lexer::create_mccarthy_lexer`).
/// The recursion-depth guard ([`MAX_RULE_DEPTH`]) is enabled as a second,
/// independent layer of defense alongside [`check_nesting_depth`].
pub fn create_mccarthy_parser_from_tokens(tokens: Vec<Token>) -> GrammarParser {
    GrammarParser::new(tokens, mccarthy_parser_grammar().clone()).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse McCarthy Lisp source into the generic [`GrammarASTNode`] CST.
///
/// The lower-level entry point — most callers want [`parse`], which
/// goes one step further and returns a typed `Vec<LispExpr>`.
///
/// # Errors
///
/// Returns a [`ParseError`] for any lex failure, paren-depth-limit
/// breach, or grammar mismatch (unbalanced paren, stray dot, …).
pub fn parse_to_cst(source: &str) -> Result<GrammarASTNode, ParseError> {
    let tokens = tokenize_mccarthy(source)?;
    check_nesting_depth(&tokens)?;
    let mut p = create_mccarthy_parser_from_tokens(tokens);
    p.parse().map_err(Into::into)
}

/// Parse McCarthy 1960 Lisp source into the top-level form sequence.
///
/// A program is zero-or-more S-expressions (McCarthy's "program =
/// sequence of forms" reading).
///
/// # Errors
///
/// Returns a [`ParseError`] for lex/grammar mismatches *or*
/// extractor-detected shape problems (integer overflow, depth limit).
pub fn parse(source: &str) -> Result<Vec<LispExpr>, ParseError> {
    let cst = parse_to_cst(source)?;
    extract_program(&cst)
}

// ===========================================================================
// CST → typed AST extraction
// ===========================================================================

/// Best-effort source position of a CST node (falls back to 1:1).
fn pos(node: &GrammarASTNode) -> (usize, usize) {
    (node.start_line.unwrap_or(1), node.start_column.unwrap_or(1))
}

/// The nested-node children of `node`, dropping bare punctuation tokens.
fn ast_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// Lower a parsed `program` CST node into the typed form sequence.
///
/// Expects `root.rule_name == "program"` (the grammar's start symbol).
///
/// # Errors
///
/// Returns a [`ParseError`] if the tree shape is unexpected (should not
/// happen for output of [`parse_to_cst`]) or an integer literal
/// overflows `i64`.
pub fn extract_program(root: &GrammarASTNode) -> Result<Vec<LispExpr>, ParseError> {
    if root.rule_name != "program" {
        let (line, column) = pos(root);
        return Err(ParseError {
            message: format!("expected 'program' root, got {:?}", root.rule_name),
            line,
            column,
        });
    }
    ast_children(root).into_iter().map(|c| extract_sexpr(c, 0)).collect()
}

/// `sexpr = atom | list | quoted` — dispatch on the single child rule.
fn extract_sexpr(node: &GrammarASTNode, depth: usize) -> Result<LispExpr, ParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    if node.rule_name != "sexpr" {
        return Err(ParseError {
            message: format!("expected 'sexpr', got {:?}", node.rule_name),
            line,
            column,
        });
    }
    let inner = ast_children(node).into_iter().next().ok_or_else(|| ParseError {
        message: "empty sexpr node".into(),
        line,
        column,
    })?;
    match inner.rule_name.as_str() {
        "atom" => extract_atom(inner),
        "list" => extract_list(inner, depth + 1),
        "quoted" => extract_quoted(inner, depth + 1),
        other => Err(ParseError {
            message: format!("unexpected sexpr child: {other:?}"),
            line,
            column,
        }),
    }
}

/// `atom = SYMBOL | INTEGER` — a single bare token.
fn extract_atom(node: &GrammarASTNode) -> Result<LispExpr, ParseError> {
    let (line, column) = pos(node);
    let tok = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t),
            ASTNodeOrToken::Node(_) => None,
        })
        .ok_or_else(|| ParseError { message: "empty atom node".into(), line, column })?;
    let (l, c) = (tok.line, tok.column);
    match tok.effective_type_name() {
        "SYMBOL" => Ok(LispExpr::Symbol(tok.value.clone())),
        "INTEGER" => {
            let value: i64 = tok.value.parse().map_err(|_| ParseError {
                message: format!("integer literal {:?} does not fit in i64", tok.value),
                line: l,
                column: c,
            })?;
            Ok(LispExpr::Int(value))
        }
        other => Err(ParseError {
            message: format!("unexpected atom token: type={other:?} value={:?}", tok.value),
            line: l,
            column: c,
        }),
    }
}

/// `list = LPAREN list_body RPAREN`, where
/// `list_body = [ sexpr { sexpr } [ DOT sexpr ] ]`.
///
/// Builds the nested-`Cons` encoding.  A list with no dotted tail is
/// `NIL`-terminated; an explicit `DOT` tail becomes the final `cdr`.
fn extract_list(node: &GrammarASTNode, depth: usize) -> Result<LispExpr, ParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;

    // The single nested child is `list_body` (the LPAREN/RPAREN are bare
    // tokens, dropped by `ast_children`).  An empty list `()` may yield a
    // list_body node with no children — treat a missing/empty body as NIL.
    let body = match ast_children(node).into_iter().next() {
        Some(b) => b,
        None => return Ok(LispExpr::Nil),
    };

    let mut elems: Vec<LispExpr> = Vec::new();
    let mut tail: Option<LispExpr> = None;
    let mut after_dot = false;

    for child in &body.children {
        match child {
            ASTNodeOrToken::Token(t) if t.effective_type_name() == "DOT" => {
                after_dot = true;
            }
            ASTNodeOrToken::Node(n) if n.rule_name == "sexpr" => {
                let expr = extract_sexpr(n, depth + 1)?;
                if after_dot {
                    tail = Some(expr);
                } else {
                    elems.push(expr);
                }
            }
            // Any other token (none expected from this grammar) is ignored.
            _ => {}
        }
    }

    let mut acc = tail.unwrap_or(LispExpr::Nil);
    for x in elems.into_iter().rev() {
        acc = LispExpr::Cons(Box::new(x), Box::new(acc));
    }
    Ok(acc)
}

/// `quoted = QUOTE sexpr` — rewrite `'X` to `(QUOTE X)`.
fn extract_quoted(node: &GrammarASTNode, depth: usize) -> Result<LispExpr, ParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    let inner = ast_children(node).into_iter().next().ok_or_else(|| ParseError {
        message: "expected an expression after '".into(),
        line,
        column,
    })?;
    Ok(LispExpr::quote(extract_sexpr(inner, depth + 1)?))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn one(src: &str) -> LispExpr {
        let mut p = parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
        assert_eq!(p.len(), 1, "expected exactly one top-level form");
        p.pop().unwrap()
    }

    #[test]
    fn empty_program() {
        assert!(parse("").unwrap().is_empty());
        assert!(parse("  ; just a comment\n").unwrap().is_empty());
    }

    #[test]
    fn atoms() {
        assert_eq!(one("CAR"), LispExpr::Symbol("CAR".into()));
        assert_eq!(one("42"), LispExpr::Int(42));
        assert_eq!(one("-1"), LispExpr::Int(-1));
    }

    #[test]
    fn empty_list_is_nil() {
        assert_eq!(one("()"), LispExpr::Nil);
    }

    #[test]
    fn nil_symbol_is_distinct_from_empty_list() {
        assert_eq!(one("NIL"), LispExpr::Symbol("NIL".into()));
    }

    #[test]
    fn proper_list_desugars_to_nested_cons() {
        assert_eq!(
            one("(A B C)"),
            LispExpr::list([LispExpr::sym("A"), LispExpr::sym("B"), LispExpr::sym("C")])
        );
    }

    #[test]
    fn dotted_pair() {
        assert_eq!(
            one("(A . B)"),
            LispExpr::Cons(Box::new(LispExpr::sym("A")), Box::new(LispExpr::sym("B")))
        );
    }

    #[test]
    fn dotted_tail_after_elements() {
        // (A B . C) → (A . (B . C))
        assert_eq!(
            one("(A B . C)"),
            LispExpr::Cons(
                Box::new(LispExpr::sym("A")),
                Box::new(LispExpr::Cons(
                    Box::new(LispExpr::sym("B")),
                    Box::new(LispExpr::sym("C"))
                ))
            )
        );
    }

    #[test]
    fn quote_sugar_expands() {
        assert_eq!(one("'X"), LispExpr::quote(LispExpr::sym("X")));
        assert_eq!(
            one("'(A B)"),
            LispExpr::quote(LispExpr::list([LispExpr::sym("A"), LispExpr::sym("B")]))
        );
    }

    #[test]
    fn the_canonical_car_example() {
        assert_eq!(
            one("(CAR '(A B C))"),
            LispExpr::list([
                LispExpr::sym("CAR"),
                LispExpr::quote(LispExpr::list([
                    LispExpr::sym("A"),
                    LispExpr::sym("B"),
                    LispExpr::sym("C"),
                ])),
            ])
        );
    }

    #[test]
    fn the_identity_lambda() {
        assert_eq!(
            one("(LAMBDA (X) X)"),
            LispExpr::list([
                LispExpr::sym("LAMBDA"),
                LispExpr::list([LispExpr::sym("X")]),
                LispExpr::sym("X"),
            ])
        );
    }

    #[test]
    fn multiple_top_level_forms() {
        let forms = parse("(CAR X) (CDR X)").unwrap();
        assert_eq!(forms.len(), 2);
    }

    #[test]
    fn display_round_trips_dotted_form() {
        // Display always prints the fully-dotted cons form.
        assert_eq!(one("(A B)").to_string(), "(A . (B . NIL))");
        assert_eq!(one("(A . B)").to_string(), "(A . B)");
    }

    // ---- error paths ----

    #[test]
    fn unbalanced_paren_is_an_error() {
        assert!(parse("(CAR").is_err());
        assert!(parse("CAR)").is_err());
    }

    #[test]
    fn stray_dot_forms_are_grammar_errors() {
        assert!(parse("(. X)").is_err()); // dot with no head
        assert!(parse("(A . B C)").is_err()); // extra after dotted tail
        assert!(parse("(A . . B)").is_err()); // two dots
    }

    #[test]
    fn lexer_errors_propagate() {
        assert!(parse("car").is_err()); // lowercase
        assert!(parse("(- A B)").is_err()); // operator symbol
    }

    #[test]
    fn integer_overflow_is_an_error() {
        let err = parse("99999999999999999999").unwrap_err();
        assert!(err.message.contains("does not fit in i64"));
    }

    #[test]
    fn deeply_nested_parens_are_rejected() {
        let deep = format!("{}{}", "(".repeat(MAX_PAREN_DEPTH + 5), ")".repeat(MAX_PAREN_DEPTH + 5));
        let err = parse(&deep).unwrap_err();
        assert!(err.message.contains("MAX_PAREN_DEPTH"));
    }

    #[test]
    fn deeply_nested_quotes_are_rejected() {
        // Regression for the quote-chain stack-overflow DoS: a long run
        // of `'` has paren-depth 0 but unbounded parser recursion.  It
        // must be rejected *before* the GrammarParser runs, not abort
        // the process.  MAX+5_000 is far past the cap and well into
        // crash territory if the guard regresses.
        let deep = format!("{}X", "'".repeat(MAX_PAREN_DEPTH + 5_000));
        let err = parse(&deep).unwrap_err();
        assert!(err.message.contains("MAX_PAREN_DEPTH"));
    }

    #[test]
    fn quote_then_deep_parens_rejected() {
        // A quote wrapping a deep list — the quote adds one level on top
        // of the parens, so the combined depth must be what trips.
        let deep = format!("'{}A{}", "(".repeat(MAX_PAREN_DEPTH), ")".repeat(MAX_PAREN_DEPTH));
        assert!(parse(&deep).is_err());
    }

    #[test]
    fn legal_moderate_nesting_is_accepted() {
        // Comfortably under MAX_PAREN_DEPTH (64) — must parse cleanly.
        let n = 40;
        let src = format!("{}A{}", "(".repeat(n), ")".repeat(n));
        assert!(parse(&src).is_ok());
    }

    #[test]
    fn legal_moderate_quote_chain_is_accepted() {
        // A quote chain just under the cap parses fine (and does not
        // overflow): '''...'X with 40 quotes → 40 nested QUOTE forms.
        let n = 40;
        let src = format!("{}X", "'".repeat(n));
        let forms = parse(&src).expect("should parse");
        assert_eq!(forms.len(), 1);
    }
}

/// Regression tests for [`MAX_RULE_DEPTH`], one triple per independent
/// recursive shape (see that constant's doc comment). These call
/// [`create_mccarthy_parser_from_tokens`] directly (bypassing
/// `check_nesting_depth`) so they exercise the new guard specifically,
/// not the crate's existing pre-scan.
#[cfg(test)]
mod depth_guard_tests {
    fn nested_list_source(n: usize) -> String {
        format!("{}A{}", "(".repeat(n), ")".repeat(n))
    }

    fn nested_quote_source(n: usize) -> String {
        format!("{}A", "'".repeat(n))
    }

    macro_rules! depth_guard_triple {
        ($mod_name:ident, $source_fn:ident, $up_to_cap:expr, $one_past_cap:expr) => {
            mod $mod_name {
                use super::$source_fn as nested_source;

                fn parse_bypassing_prescan(src: &str) -> Result<(), String> {
                    let tokens = super::super::tokenize_mccarthy(src).map_err(|e| format!("{e}"))?;
                    super::super::create_mccarthy_parser_from_tokens(tokens)
                        .parse()
                        .map(|_| ())
                        .map_err(|e| format!("{e}"))
                }

                /// Deeply-nested input must produce a recoverable error, not
                /// overflow the native stack. Parses 5000 levels — far past
                /// `MAX_RULE_DEPTH` — on a worker thread with a generous
                /// 32 MiB stack, so the *guard* is what stops the
                /// recursion, not the stack running out.
                #[test]
                fn test_deeply_nested_input_returns_error_not_overflow() {
                    let handle = std::thread::Builder::new()
                        .name(
                            concat!(
                                "mccarthy-lisp-depth-guard-",
                                stringify!($mod_name),
                                "-regression"
                            )
                            .to_string(),
                        )
                        .stack_size(32 * 1024 * 1024)
                        .spawn(|| {
                            let result = parse_bypassing_prescan(&nested_source(5000));
                            assert!(
                                result.is_err(),
                                "deeply-nested input must fail with an error, not parse or crash"
                            );
                        })
                        .expect("failed to spawn worker thread");
                    handle
                        .join()
                        .expect("depth guard must keep the worker thread from crashing");
                }

                /// Input that nests *exactly up to* `MAX_RULE_DEPTH` still
                /// parses cleanly, and one layer deeper cleanly trips the
                /// guard. These exact boundary counts were found
                /// empirically by binary-searching against increasing
                /// nesting counts at the production cap — see
                /// `MAX_RULE_DEPTH`'s doc comment.
                #[test]
                fn test_nesting_up_to_cap_still_parses() {
                    assert!(
                        parse_bypassing_prescan(&nested_source($up_to_cap)).is_ok(),
                        "{} levels must stay under the cap",
                        $up_to_cap
                    );
                    assert!(
                        parse_bypassing_prescan(&nested_source($one_past_cap)).is_err(),
                        "one nesting level past the cap's measured limit must fail"
                    );
                }

                /// A caller relying on `MAX_RULE_DEPTH` must have the guard
                /// trip *before* the native stack overflows on a
                /// default-stack thread — otherwise a production caller
                /// (e.g. an LSP integration calling
                /// `create_mccarthy_parser_from_tokens` directly, or
                /// `cargo test`'s own per-test thread) would still crash.
                /// Parses far-too-deep input on a worker thread with
                /// **no** `stack_size` override (the same default a
                /// thread gets in this environment, unmodified by any
                /// `RUST_MIN_STACK` override). A clean `Err` (not a
                /// `join()` failure from a crashed thread) proves
                /// `MAX_RULE_DEPTH` sits safely below the native overflow
                /// point on the default stack.
                #[test]
                fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
                    let handle = std::thread::spawn(|| {
                        let result = parse_bypassing_prescan(&nested_source(5000));
                        assert!(result.is_err(), "deeply-nested input must error, not crash");
                    });
                    handle.join().expect(
                        "MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack",
                    );
                }
            }
        };
    }

    depth_guard_triple!(list_shape, nested_list_source, 59, 60);
    depth_guard_triple!(quote_shape, nested_quote_source, 88, 89);
}
