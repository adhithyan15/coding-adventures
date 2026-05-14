//! # twig-parser — thin wrapper over the generic `GrammarParser`.
//!
//! Twig's grammar lives in `code/grammars/twig.grammar` and is shared
//! across every Twig implementation in the repo (Python, Rust, any
//! future port).  This crate is the Rust binding to that file:
//!
//! 1. Tokenises the source via [`twig_lexer::tokenize_twig`].
//! 2. Loads `twig.grammar` via
//!    [`grammar_tools::parser_grammar::parse_parser_grammar`].
//! 3. Runs the generic [`parser::grammar_parser::GrammarParser`] over
//!    the tokens to get a [`parser::grammar_parser::GrammarASTNode`].
//! 4. Walks that generic tree in [`ast_extract::extract_program`] to
//!    produce a typed [`Program`] (with `IntLit`, `BoolLit`, `Lambda`,
//!    `If`, `Apply`, etc.) — analogous to the Python package's
//!    `twig.ast_extract` module.
//!
//! The same pattern is used by every other Rust language frontend in
//! this repo (brainfuck, dartmouth-basic, …); see
//! [`code/packages/rust/brainfuck/src/parser.rs`](../brainfuck/src/parser.rs)
//! for the canonical reference.
//!
//! ## Why a typed AST on top?
//!
//! `GrammarASTNode` is generic — every node carries a `rule_name`
//! string and a heterogeneous `children` list.  Walking that tree
//! directly in the IR compiler means a sea of string-comparison
//! dispatches with no static guarantees.  Twig's eight semantic forms
//! lift cleanly into a small, exhaustive enum ([`Expr`]) that the IR
//! compiler can `match` over with the type system enforcing
//! exhaustiveness.
//!
//! ## Pipeline
//!
//! ```text
//! Twig source
//!     │
//!     ▼  twig_lexer::tokenize_twig
//! Vec<Token>
//!     │
//!     ▼  parser::grammar_parser::GrammarParser
//! GrammarASTNode (generic tree)
//!     │
//!     ▼  ast_extract::extract_program          ← THIS CRATE
//! Program (typed AST)
//!     │
//!     ▼  twig-ir-compiler
//! IIRModule
//! ```
//!
//! ## Example
//!
//! ```no_run
//! use twig_parser::{parse, Form, Expr};
//!
//! let program = parse("(define x 42)").unwrap();
//! match &program.forms[0] {
//!     Form::Define(d) => {
//!         assert_eq!(d.name, "x");
//!         assert!(matches!(d.expr, Expr::IntLit(_)));
//!     }
//!     _ => unreachable!(),
//! }
//! ```

pub mod ast_extract;
pub mod ast_nodes;
pub mod type_decls;

use std::fmt;
use std::sync::OnceLock;

use grammar_tools::parser_grammar::ParserGrammar;
use lexer::token::Token;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
use twig_lexer::{tokenize_twig, LexerError};

// ---------------------------------------------------------------------------
// Generated grammar (build.rs → grammar-tools::compiler → Rust)
// ---------------------------------------------------------------------------
//
// The build script (`build.rs`) compiles `code/grammars/twig.grammar`
// at cargo-build time via
// `grammar_tools::compiler::compile_parser_grammar`.  The output is
// Rust source code that defines `pub fn parser_grammar() ->
// ParserGrammar` returning a fresh struct.  We `include!` it in a
// private module and wrap the constructor in `OnceLock<ParserGrammar>`
// so the struct is materialised exactly once per process.
//
// See twig-lexer/src/lib.rs for the matching pattern on the token
// grammar side.

mod generated_grammar {
    include!(concat!(env!("OUT_DIR"), "/twig_parser_grammar.rs"));
}

static TWIG_PARSER_GRAMMAR: OnceLock<ParserGrammar> = OnceLock::new();

fn twig_parser_grammar() -> &'static ParserGrammar {
    TWIG_PARSER_GRAMMAR.get_or_init(generated_grammar::parser_grammar)
}

pub use ast_extract::{
    check_membership_int_count, extract_program, MAX_AST_DEPTH, MAX_MEMBERSHIP_INT_VALUES,
};
pub use type_decls::emit_type_declarations;
pub use ast_nodes::{
    // ── Atoms ─────────────────────────────────────────────────────────────
    Apply, Begin, BoolLit, Define, Expr, Form, If, IntLit, Lambda, Let, NilLit, Program,
    // LANG51: string literal AST node
    StrLit, SymLit,
    TypeAnnotation, VarRef,
    // ── LANG48 / TW05-A — type expressions ────────────────────────────────
    TypeExpr,
    // ── LANG48 / TW05-A — module metadata ────────────────────────────────
    ModuleInfo, TypedMode,
    // ── LANG48 / TW05-A — new top-level declarations ─────────────────────
    RecordDef, RecordField, TypeAlias, UnionDef, UnionVariant,
    // ── LANG48 / TW05-A — match expression ───────────────────────────────
    Match, MatchArm, MatchPat,
};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Parse-time error.
///
/// Wraps either:
/// - a malformed shape detected by the AST extractor,
/// - a lexer failure (invalid character),
/// - a `GrammarParser` failure (unmatched paren, missing token, …).
///
/// Source positions are 1-indexed and match the position of the
/// offending token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwigParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for TwigParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TwigParseError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for TwigParseError {}

impl From<parser::grammar_parser::GrammarParseError> for TwigParseError {
    fn from(e: parser::grammar_parser::GrammarParseError) -> Self {
        TwigParseError {
            message: e.message,
            line: e.token.line,
            column: e.token.column,
        }
    }
}

impl From<LexerError> for TwigParseError {
    fn from(e: LexerError) -> Self {
        TwigParseError {
            message: format!("lexer error: {}", e.message),
            line: e.line,
            column: e.column,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a [`GrammarParser`] configured for Twig source.
///
/// Tokenises the source and constructs a `GrammarParser` over the
/// build-time-compiled Twig parser grammar (no runtime file I/O).
/// Use this when you want access to the parser object itself (e.g.
/// for tracing or alternative entry rules); otherwise reach for
/// [`parse`].
///
/// # Errors
///
/// Returns a `TwigParseError` if tokenisation fails (e.g. an unknown
/// character in the source).
pub fn create_twig_parser(source: &str) -> Result<GrammarParser, TwigParseError> {
    let tokens = tokenize_twig(source)?;
    Ok(create_twig_parser_from_tokens(tokens))
}

/// Build a [`GrammarParser`] from a pre-tokenised stream.
///
/// Useful for incremental editors / LSP-style integrations that already
/// have a `Vec<Token>` (e.g. from `twig_lexer::create_twig_lexer`).
///
/// Uses the build-time-compiled grammar — no runtime file I/O, no
/// re-parsing per parser construction.  `GrammarParser::new` takes
/// ownership of the grammar, so we clone the static reference; the
/// shared underlying parsed grammar is also reused across calls.
pub fn create_twig_parser_from_tokens(tokens: Vec<Token>) -> GrammarParser {
    GrammarParser::new(tokens, twig_parser_grammar().clone())
}

/// Borrow the build-time-compiled Twig parser grammar.
///
/// Re-exported here for callers that need to inspect the rules (e.g.
/// LSP-style introspection, debugger).
pub fn twig_grammar() -> &'static ParserGrammar {
    twig_parser_grammar()
}

/// Maximum LPAREN nesting depth permitted before parsing.
///
/// The downstream [`GrammarParser`] is recursive — each LPAREN-led
/// compound form produces several recursion frames (program → form →
/// expr → compound → apply → expr → …).  Empirically ~10 frames per
/// LPAREN, and Rust test threads run with a 2 MiB stack by default —
/// so we cap at 64 levels, which leaves a comfortable safety margin
/// while still admitting any realistic Twig program (hand-written
/// code is single-digit-deep in this repo).  Sources past this limit
/// are rejected before invoking the GrammarParser to avoid the OS
/// thread stack-overflow abort that Rust cannot catch.
pub const MAX_PAREN_DEPTH: usize = 64;

/// Reject a source whose maximum LPAREN nesting depth exceeds
/// [`MAX_PAREN_DEPTH`].  Returns the offending token's position when it
/// fires.
#[allow(clippy::ptr_arg)]
fn check_paren_depth(tokens: &[Token]) -> Result<(), TwigParseError> {
    use lexer::token::TokenType;
    let mut depth: usize = 0;
    for t in tokens {
        match t.type_ {
            TokenType::LParen => {
                depth += 1;
                if depth > MAX_PAREN_DEPTH {
                    return Err(TwigParseError {
                        message: format!(
                            "source LPAREN nesting exceeds MAX_PAREN_DEPTH \
                             ({MAX_PAREN_DEPTH}) — refusing to invoke parser \
                             to avoid stack overflow"
                        ),
                        line: t.line,
                        column: t.column,
                    });
                }
            }
            TokenType::RParen => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Parse Twig source into a generic [`GrammarASTNode`].
///
/// This is the lower-level entry point — most callers want [`parse`]
/// (which goes one step further and returns a typed [`Program`]).
///
/// # Errors
///
/// Returns a `TwigParseError` for any grammar mismatch (unmatched paren,
/// unexpected token, …) or for paren nesting deeper than
/// [`MAX_PAREN_DEPTH`].  Source positions point at the offending token.
pub fn parse_to_ast(source: &str) -> Result<GrammarASTNode, TwigParseError> {
    let tokens = tokenize_twig(source)?;
    check_paren_depth(&tokens)?;
    let mut p = create_twig_parser_from_tokens(tokens);
    p.parse().map_err(Into::into)
}

/// Parse Twig source into a typed [`Program`].
///
/// One-call entry: tokenise → grammar-parse → extract typed AST.
///
/// # Errors
///
/// Returns a `TwigParseError` for grammar mismatches *or* extractor-
/// detected shape problems (integer overflow, AST nesting deeper than
/// [`MAX_AST_DEPTH`], etc.).
///
/// # Example
///
/// ```no_run
/// use twig_parser::{parse, Form};
///
/// let p = parse("(+ 1 2)").unwrap();
/// assert_eq!(p.forms.len(), 1);
/// assert!(matches!(&p.forms[0], Form::Expr(_)));
/// ```
pub fn parse(source: &str) -> Result<Program, TwigParseError> {
    let ast = parse_to_ast(source)?;
    extract_program(&ast)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn first_expr(src: &str) -> Expr {
        let p = parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
        match p.forms.into_iter().next().expect("at least one form") {
            Form::Expr(e) => e,
            Form::Define(_) => panic!("expected an expression form, got define"),
            Form::TypeAlias(_) => panic!("expected an expression form, got type alias"),
            Form::RecordDef(_) => panic!("expected an expression form, got record def"),
            Form::UnionDef(_) => panic!("expected an expression form, got union def"),
        }
    }

    // ---- Empty / whitespace ----

    #[test]
    fn empty_program() {
        let p = parse("").unwrap();
        assert!(p.forms.is_empty());
    }

    #[test]
    fn whitespace_and_comments_only() {
        let p = parse("\n\n  ; comment\n").unwrap();
        assert!(p.forms.is_empty());
    }

    // ---- Atoms ----

    #[test]
    fn integer_literal() {
        match first_expr("42") {
            Expr::IntLit(n) => assert_eq!(n.value, 42),
            other => panic!("expected IntLit, got {other:?}"),
        }
    }

    #[test]
    fn negative_integer_literal() {
        match first_expr("-7") {
            Expr::IntLit(n) => assert_eq!(n.value, -7),
            other => panic!("expected IntLit, got {other:?}"),
        }
    }

    #[test]
    fn integer_overflow_errors() {
        let err = parse("99999999999999999999").unwrap_err();
        assert!(err.message.contains("does not fit in i64"));
    }

    #[test]
    fn bool_true_and_false() {
        assert!(matches!(first_expr("#t"), Expr::BoolLit(b) if b.value));
        assert!(matches!(first_expr("#f"), Expr::BoolLit(b) if !b.value));
    }

    #[test]
    fn nil_literal() {
        assert!(matches!(first_expr("nil"), Expr::NilLit(_)));
    }

    #[test]
    fn name_reference() {
        match first_expr("foo") {
            Expr::VarRef(v) => assert_eq!(v.name, "foo"),
            other => panic!("expected VarRef, got {other:?}"),
        }
    }

    #[test]
    fn operator_name_lexes_as_var_ref() {
        match first_expr("+") {
            Expr::VarRef(v) => assert_eq!(v.name, "+"),
            other => panic!("expected VarRef, got {other:?}"),
        }
    }

    // ---- Quoted symbols ----

    #[test]
    fn quoted_short_form() {
        match first_expr("'foo") {
            Expr::SymLit(s) => assert_eq!(s.name, "foo"),
            other => panic!("expected SymLit, got {other:?}"),
        }
    }

    #[test]
    fn quoted_long_form() {
        match first_expr("(quote bar)") {
            Expr::SymLit(s) => assert_eq!(s.name, "bar"),
            other => panic!("expected SymLit, got {other:?}"),
        }
    }

    // ---- Apply ----

    #[test]
    fn simple_application() {
        match first_expr("(+ 1 2)") {
            Expr::Apply(a) => {
                assert_eq!(a.args.len(), 2);
                assert!(matches!(*a.fn_expr, Expr::VarRef(_)));
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    #[test]
    fn nested_application() {
        match first_expr("(+ (* 2 3) 4)") {
            Expr::Apply(a) => {
                assert_eq!(a.args.len(), 2);
                assert!(matches!(a.args[0], Expr::Apply(_)));
                assert!(matches!(a.args[1], Expr::IntLit(_)));
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    #[test]
    fn higher_order_call() {
        match first_expr("((make-adder 5) 3)") {
            Expr::Apply(a) => {
                assert!(matches!(*a.fn_expr, Expr::Apply(_)));
                assert_eq!(a.args.len(), 1);
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    // ---- if / let / begin / lambda ----

    #[test]
    fn if_form() {
        match first_expr("(if #t 1 2)") {
            Expr::If(i) => {
                assert!(matches!(*i.cond, Expr::BoolLit(_)));
                assert!(matches!(*i.then_branch, Expr::IntLit(_)));
                assert!(matches!(*i.else_branch, Expr::IntLit(_)));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn let_with_two_bindings_and_multi_body() {
        match first_expr("(let ((x 1) (y 2)) (+ x y) (* x y))") {
            Expr::Let(l) => {
                assert_eq!(l.bindings.len(), 2);
                assert_eq!(l.body.len(), 2);
            }
            other => panic!("expected Let, got {other:?}"),
        }
    }

    #[test]
    fn let_with_zero_bindings() {
        match first_expr("(let () 1)") {
            Expr::Let(l) => {
                assert!(l.bindings.is_empty());
                assert_eq!(l.body.len(), 1);
            }
            other => panic!("expected Let, got {other:?}"),
        }
    }

    #[test]
    fn begin_form() {
        match first_expr("(begin 1 2 3)") {
            Expr::Begin(b) => assert_eq!(b.exprs.len(), 3),
            other => panic!("expected Begin, got {other:?}"),
        }
    }

    #[test]
    fn lambda_with_no_params() {
        match first_expr("(lambda () 42)") {
            Expr::Lambda(l) => {
                assert!(l.params.is_empty());
                assert_eq!(l.body.len(), 1);
            }
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    #[test]
    fn lambda_with_params() {
        match first_expr("(lambda (x y) (+ x y))") {
            Expr::Lambda(l) => {
                assert_eq!(l.params, vec!["x".to_string(), "y".to_string()]);
                assert_eq!(l.body.len(), 1);
            }
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    // ---- define ----

    #[test]
    fn define_value() {
        let p = parse("(define x 42)").unwrap();
        match &p.forms[0] {
            Form::Define(d) => {
                assert_eq!(d.name, "x");
                assert!(matches!(d.expr, Expr::IntLit(_)));
            }
            _ => panic!("expected Define"),
        }
    }

    #[test]
    fn define_function_sugar_lowers_to_lambda() {
        let p = parse("(define (square x) (* x x))").unwrap();
        match &p.forms[0] {
            Form::Define(d) => {
                assert_eq!(d.name, "square");
                match &d.expr {
                    Expr::Lambda(l) => {
                        assert_eq!(l.params, vec!["x".to_string()]);
                        assert_eq!(l.body.len(), 1);
                    }
                    _ => panic!("expected Lambda"),
                }
            }
            _ => panic!("expected Define"),
        }
    }

    #[test]
    fn define_function_with_multi_expr_body() {
        let p = parse("(define (f x) (+ x 1) (* x 2))").unwrap();
        match &p.forms[0] {
            Form::Define(d) => match &d.expr {
                Expr::Lambda(l) => assert_eq!(l.body.len(), 2),
                _ => panic!("expected Lambda"),
            },
            _ => panic!("expected Define"),
        }
    }

    #[test]
    fn define_value_rejects_extra_body() {
        let err = parse("(define x 1 2)").unwrap_err();
        assert!(err.message.contains("(define name expr)"));
    }

    // ---- Multiple top-level forms ----

    #[test]
    fn multiple_top_level_forms() {
        let p = parse("(define x 1) (define y 2) (+ x y)").unwrap();
        assert_eq!(p.forms.len(), 3);
        assert!(matches!(&p.forms[0], Form::Define(_)));
        assert!(matches!(&p.forms[1], Form::Define(_)));
        assert!(matches!(&p.forms[2], Form::Expr(_)));
    }

    // ---- LANG48 / TW05-A ----

    // -- (module …) parsing --

    #[test]
    fn module_form_no_clauses() {
        let p = parse("(module mylib)").unwrap();
        assert!(p.forms.is_empty(), "module form should not appear as a Form");
        let mi = p.module_info.expect("module_info should be Some");
        assert_eq!(mi.name, "mylib");
        assert_eq!(mi.typed_mode, None);
        assert!(mi.exports.is_empty());
        assert!(mi.imports.is_empty());
    }

    #[test]
    fn module_form_with_typed_strict() {
        let p = parse("(module compiler/lexer (typed strict))").unwrap();
        let mi = p.module_info.unwrap();
        assert_eq!(mi.name, "compiler/lexer");
        assert_eq!(mi.typed_mode, Some(TypedMode::Strict));
    }

    #[test]
    fn module_form_with_typed_lenient() {
        let p = parse("(module foo (typed lenient))").unwrap();
        let mi = p.module_info.unwrap();
        assert_eq!(mi.typed_mode, Some(TypedMode::Lenient));
    }

    #[test]
    fn module_form_with_typed_off() {
        let p = parse("(module foo (typed off))").unwrap();
        let mi = p.module_info.unwrap();
        assert_eq!(mi.typed_mode, Some(TypedMode::Off));
    }

    #[test]
    fn module_form_with_export_import() {
        let p = parse("(module stdlib/io (export read write) (import stdlib/base))").unwrap();
        let mi = p.module_info.unwrap();
        assert_eq!(mi.name, "stdlib/io");
        assert_eq!(mi.exports, vec!["read".to_string(), "write".to_string()]);
        assert_eq!(mi.imports, vec!["stdlib/base".to_string()]);
    }

    #[test]
    fn module_form_full_clauses() {
        let src = r#"
          (module compiler/lexer
            (typed strict)
            (export lex)
            (import compiler/token compiler/span))
          (define x 1)
        "#;
        let p = parse(src).unwrap();
        assert_eq!(p.forms.len(), 1);
        let mi = p.module_info.unwrap();
        assert_eq!(mi.name, "compiler/lexer");
        assert_eq!(mi.typed_mode, Some(TypedMode::Strict));
        assert_eq!(mi.exports, vec!["lex".to_string()]);
        assert_eq!(mi.imports, vec!["compiler/token".to_string(), "compiler/span".to_string()]);
    }

    #[test]
    fn program_without_module_has_no_module_info() {
        let p = parse("(define x 42)").unwrap();
        assert!(p.module_info.is_none());
    }

    // -- (type …) alias --

    #[test]
    fn type_alias_bare_name() {
        let p = parse("(type Nat int)").unwrap();
        assert_eq!(p.forms.len(), 1);
        match &p.forms[0] {
            Form::TypeAlias(ta) => {
                assert_eq!(ta.name, "Nat");
                assert_eq!(ta.expr, TypeExpr::Name("int".to_string()));
            }
            other => panic!("expected TypeAlias, got {other:?}"),
        }
    }

    #[test]
    fn type_alias_range() {
        let p = parse("(type Byte (Int 0 256))").unwrap();
        match &p.forms[0] {
            Form::TypeAlias(ta) => {
                assert_eq!(ta.name, "Byte");
                // (Int 0 256) → List([Name("Int"), Int(0), Int(256)])
                assert_eq!(
                    ta.expr,
                    TypeExpr::List(vec![
                        TypeExpr::Name("Int".to_string()),
                        TypeExpr::Int(0),
                        TypeExpr::Int(256),
                    ])
                );
            }
            other => panic!("expected TypeAlias, got {other:?}"),
        }
    }

    #[test]
    fn type_alias_dependent_fn_type() {
        // (type Index (fn (len) (Int 0 len)))
        let p = parse("(type Index (fn (len) (Int 0 len)))").unwrap();
        match &p.forms[0] {
            Form::TypeAlias(ta) => {
                assert_eq!(ta.name, "Index");
                // Verify the outer List has 3 children: fn, (len), (Int 0 len)
                match &ta.expr {
                    TypeExpr::List(parts) => {
                        assert_eq!(parts.len(), 3);
                        assert_eq!(parts[0], TypeExpr::Name("fn".to_string()));
                    }
                    other => panic!("expected List, got {other:?}"),
                }
            }
            other => panic!("expected TypeAlias, got {other:?}"),
        }
    }

    // -- (record …) --

    #[test]
    fn record_def_simple() {
        let p = parse("(record Span (start : Nat) (end : Nat))").unwrap();
        assert_eq!(p.forms.len(), 1);
        match &p.forms[0] {
            Form::RecordDef(r) => {
                assert_eq!(r.name, "Span");
                assert_eq!(r.fields.len(), 2);
                assert_eq!(r.fields[0].name, "start");
                assert_eq!(r.fields[1].name, "end");
            }
            other => panic!("expected RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn record_def_with_opaque_type() {
        let p = parse("(record Token (kind : TokenKind) (span : Span))").unwrap();
        match &p.forms[0] {
            Form::RecordDef(r) => {
                assert_eq!(r.fields[0].name, "kind");
                // TokenKind is not a LANG23 type → Opaque(Name("TokenKind"))
                assert!(
                    matches!(
                        &r.fields[0].type_annotation,
                        TypeAnnotation::Opaque(TypeExpr::Name(n)) if n == "TokenKind"
                    ),
                    "expected Opaque(Name(\"TokenKind\")), got {:?}",
                    r.fields[0].type_annotation
                );
            }
            other => panic!("expected RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn record_def_zero_fields() {
        // A record with no fields is grammatically valid.
        let p = parse("(record Unit)").unwrap();
        match &p.forms[0] {
            Form::RecordDef(r) => {
                assert_eq!(r.name, "Unit");
                assert!(r.fields.is_empty());
            }
            other => panic!("expected RecordDef, got {other:?}"),
        }
    }

    // -- (union …) --

    #[test]
    fn union_def_two_variants() {
        let p = parse("(union Expr (IntLit (value : int)) (NameRef (name : any)))").unwrap();
        assert_eq!(p.forms.len(), 1);
        match &p.forms[0] {
            Form::UnionDef(u) => {
                assert_eq!(u.name, "Expr");
                assert_eq!(u.variants.len(), 2);
                assert_eq!(u.variants[0].name, "IntLit");
                assert_eq!(u.variants[0].fields.len(), 1);
                assert_eq!(u.variants[0].fields[0].name, "value");
                assert_eq!(u.variants[1].name, "NameRef");
            }
            other => panic!("expected UnionDef, got {other:?}"),
        }
    }

    #[test]
    fn union_def_unit_variant() {
        // Variants without fields are valid.
        let p = parse("(union Bool (True) (False))").unwrap();
        match &p.forms[0] {
            Form::UnionDef(u) => {
                assert_eq!(u.variants.len(), 2);
                assert!(u.variants[0].fields.is_empty());
                assert!(u.variants[1].fields.is_empty());
            }
            other => panic!("expected UnionDef, got {other:?}"),
        }
    }

    // -- (match …) --

    #[test]
    fn match_variant_arm() {
        let e = first_expr("(match x ((IntLit v s) v))");
        match e {
            Expr::Match(m) => {
                assert!(matches!(*m.scrutinee, Expr::VarRef(_)));
                assert_eq!(m.arms.len(), 1);
                match &m.arms[0].pat {
                    MatchPat::Variant { name, bindings } => {
                        assert_eq!(name, "IntLit");
                        assert_eq!(bindings, &["v".to_string(), "s".to_string()]);
                    }
                    other => panic!("expected Variant, got {other:?}"),
                }
                assert_eq!(m.arms[0].body.len(), 1);
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn match_wildcard_arm() {
        let e = first_expr("(match x (_ 42))");
        match e {
            Expr::Match(m) => {
                assert_eq!(m.arms.len(), 1);
                assert!(matches!(m.arms[0].pat, MatchPat::Wildcard));
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn match_binding_arm() {
        let e = first_expr("(match x (y y))");
        match e {
            Expr::Match(m) => {
                assert_eq!(m.arms.len(), 1);
                assert!(
                    matches!(&m.arms[0].pat, MatchPat::Binding(n) if n == "y"),
                    "expected Binding(\"y\"), got {:?}",
                    m.arms[0].pat
                );
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn match_multi_arm() {
        let e = first_expr("(match expr ((IntLit v s) v) ((NameRef n s) n) (_ nil))");
        match e {
            Expr::Match(m) => {
                assert_eq!(m.arms.len(), 3);
                assert!(matches!(m.arms[0].pat, MatchPat::Variant { .. }));
                assert!(matches!(m.arms[1].pat, MatchPat::Variant { .. }));
                assert!(matches!(m.arms[2].pat, MatchPat::Wildcard));
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn match_multi_body_arm() {
        // An arm body can contain multiple expressions; only the last is the value.
        let e = first_expr("(match x ((A v) (begin 1 2) v))");
        match e {
            Expr::Match(m) => {
                assert_eq!(m.arms[0].body.len(), 2);
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    // -- type_annotation round-trips --

    #[test]
    fn type_annotation_bare_int_kind() {
        // `int` as a LANG23 bare kind → UnrefinedInt
        let p = parse("(define x : int 42)").unwrap();
        match &p.forms[0] {
            Form::Define(d) => {
                assert_eq!(d.type_annotation, Some(TypeAnnotation::UnrefinedInt));
            }
            other => panic!("expected Define, got {other:?}"),
        }
    }

    #[test]
    fn type_annotation_range_int() {
        let p = parse("(define x : (Int 0 128) 42)").unwrap();
        match &p.forms[0] {
            Form::Define(d) => {
                assert_eq!(
                    d.type_annotation,
                    Some(TypeAnnotation::RangeInt { lo: 0, hi: 128 })
                );
            }
            other => panic!("expected Define, got {other:?}"),
        }
    }

    #[test]
    fn type_annotation_opaque_parametric() {
        // (Index source-len) is not LANG23 → Opaque
        let p = parse("(define x : (Index source-len) 0)").unwrap();
        match &p.forms[0] {
            Form::Define(d) => {
                assert!(
                    matches!(&d.type_annotation, Some(TypeAnnotation::Opaque(_))),
                    "expected Opaque, got {:?}",
                    d.type_annotation
                );
            }
            other => panic!("expected Define, got {other:?}"),
        }
    }

    // ---- Position tracking ----

    #[test]
    fn integer_position_recorded() {
        match first_expr("\n\n  42") {
            Expr::IntLit(i) => {
                assert_eq!(i.line, 3);
                assert_eq!(i.column, 3);
            }
            other => panic!("expected IntLit, got {other:?}"),
        }
    }

    // ---- Errors ----

    #[test]
    fn unmatched_lparen_errors() {
        assert!(parse("(+ 1 2").is_err());
    }

    #[test]
    fn lone_rparen_errors() {
        assert!(parse(")").is_err());
    }

    #[test]
    fn lexer_error_propagates_as_parse_error() {
        // A stray `@` would previously panic in the wrapper; it must
        // now surface as a structured TwigParseError so untrusted
        // input cannot abort the process.
        let err = parse("@").unwrap_err();
        assert!(err.message.contains("lexer error"));
    }

    #[test]
    fn non_ascii_unicode_errors_not_panics() {
        let err = parse("(+ 1 €)").unwrap_err();
        assert!(err.message.contains("lexer error"));
    }

    // ---- Realistic full program ----

    #[test]
    fn factorial_program_parses() {
        let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))\n(fact 5)";
        let p = parse(src).unwrap();
        assert_eq!(p.forms.len(), 2);
        match &p.forms[0] {
            Form::Define(d) => {
                assert_eq!(d.name, "fact");
                assert!(matches!(d.expr, Expr::Lambda(_)));
            }
            _ => panic!("expected Define"),
        }
    }

    // ---- Stack-overflow guard ----

    #[test]
    fn extreme_nesting_returns_error_not_panic() {
        // Build deep ((((...)))) nesting — would otherwise overflow the
        // GrammarParser's internal recursion stack.
        let src = format!(
            "{open}+ 1{close}",
            open = "(".repeat(MAX_PAREN_DEPTH + 50),
            close = ")".repeat(MAX_PAREN_DEPTH + 50),
        );
        let err = parse(&src).unwrap_err();
        assert!(
            err.message.contains("MAX_PAREN_DEPTH"),
            "expected paren-depth error, got: {err}"
        );
    }

    #[test]
    fn nesting_below_cap_works() {
        // Stay comfortably under the cap to verify it isn't over-aggressive.
        let src = format!(
            "{open}+ 1{close}",
            open = "(".repeat(20),
            close = ")".repeat(20),
        );
        let p = parse(&src).expect("20-deep nest should parse");
        assert_eq!(p.forms.len(), 1);
    }
}
