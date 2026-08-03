// The large `Err` variant is the crate's public `CompileError` enum; boxing it
// would churn the public API and all call sites for no behavior change.
#![allow(clippy::result_large_err)]
//! # adj-lang — surface syntax for the adjudication framework
//!
//! A small probabilistic-logic DSL whose programs lower directly to
//! a `logic-engine` [`KnowledgeBase`]. Designed to be readable by a
//! domain expert (ED physician, M&A lawyer, security researcher,
//! investigative journalist) without a Rust compiler in scope.
//!
//! Dissolves ADJ46 awkwardness items
//! [A10](../../../specs/data/adj46/AWKWARDNESS.md) (rulebook surface
//! is hand-written Rust) and A4 (joint contributions look like
//! ordinary multi-body rules) by making each clause kind a distinct
//! keyword.
//!
//! ## Pipeline
//!
//! ```text
//!  .adj source
//!     │ [GrammarLexer with adj_lang.tokens]
//!     ▼
//!  Vec<Token>
//!     │ [GrammarParser with adj_lang.grammar]
//!     ▼
//!  GrammarASTNode (generic parse tree)
//!     │ [adapter::adapt_program]
//!     ▼
//!  ast::Program (typed)
//!     │ [lower::lower]
//!     ▼
//!  LoweredProgram { kb, queries }
//! ```
//!
//! The lexer and parser are not hand-written: they're driven by
//! `code/grammars/adj_lang.tokens` and `code/grammars/adj_lang.grammar`,
//! compiled into `_lexer_grammar.rs` / `_parser_grammar.rs` by the
//! `grammar-tools` CLI. This crate is therefore conformant with the
//! rest of the repo's grammar-driven language frontends — the same
//! grammars can be reused by any other language port of the
//! adj-lang frontend.
//!
//! See [`code/grammars/adj_lang.tokens`](../../../grammars/adj_lang.tokens)
//! and [`code/grammars/adj_lang.grammar`](../../../grammars/adj_lang.grammar)
//! for the canonical source of truth.

pub mod adapter;
pub mod ast;
pub mod lower;
pub mod resolve;
pub mod statemachine;

mod _lexer_grammar;
mod _parser_grammar;

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
use logic_engine::Differential;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParseError, GrammarParser};

pub use adapter::{adapt_program, AdapterError};
pub use ast::{Annotation, Define, DefineKind, OptDir, Program, RelOp, Statement, Term as AstTerm};
pub use lower::{
    lower, ConstraintSystem, LowerError, LoweredConstraint, LoweredExit, LoweredGuard,
    LoweredProgram, LoweredRangeLookup, LoweredState, LoweredStateMachine, LoweredTransition,
};
pub use resolve::{resolve_imports, ImportError, ImportLimits, ImportProvider};
pub use statemachine::{
    run_state_machine, RunStep, StateMachineOutcome, StateMachineRun, YieldValue,
};

/// Result of compilation. Either the typed program produced by the
/// adapter, or an error from the lexer, parser, adapter, or
/// lowering stage.
#[derive(Debug)]
pub enum CompileError {
    Lex(String),
    Parse(GrammarParseError),
    Adapt(AdapterError),
    Lower(LowerError),
}

/// A half-open UTF-8 byte range in the exact source passed to [`parse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

/// One formula as both the real typed AST and its exact source-byte envelope.
///
/// This is the parser-backed bridge used by provenance tooling. It avoids
/// rediscovering formulas with regular expressions and pairs each typed formula
/// with the parse-tree node that supplied its source envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaSource {
    pub formulabook: String,
    pub formula: ast::FormulaDef,
    pub declaration_span: SourceSpan,
    pub body_span: SourceSpan,
}

/// One ordinary `? term` query and the exact bytes that declared it.
#[derive(Debug, Clone, PartialEq)]
pub struct QuerySource {
    pub conclusion: ast::Term,
    pub declaration_span: SourceSpan,
}

/// One `import "..."` declaration and its exact source-byte envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportSource {
    pub literal: String,
    pub declaration_span: SourceSpan,
}

/// Parser-backed source locations for the program elements used by formula audit.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramSourceMap {
    pub formulas: Vec<FormulaSource>,
    pub imports: Vec<ImportSource>,
    pub queries: Vec<QuerySource>,
}

/// Failure to construct a trustworthy formula source map.
#[derive(Debug)]
pub enum FormulaSourceMapError {
    Compile(CompileError),
    Inconsistent(String),
}

impl From<CompileError> for FormulaSourceMapError {
    fn from(error: CompileError) -> Self {
        Self::Compile(error)
    }
}

/// Recursion-depth cap for the adj-lang [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything). `adj-lang` is reachable via `adj-lang-cli` on arbitrary
/// `.adj` files, a real attack surface.
///
/// # Three independent recursive shapes
///
/// Unlike most sibling grammars, adj-lang's grammar has three *independent*
/// recursion paths that must all be measured, since a single
/// `MAX_RULE_DEPTH` bounds the parser's internal rule-invocation counter for
/// any of them:
///
/// - **Paren nesting** — `factor = … | LPAREN expr RPAREN`, cascading
///   through `expr → term_expr → factor` (~3 rule-frames per real nesting
///   level).
/// - **Call nesting** — `term = IDENT [ LPAREN (term|NUMBER|VAR) {…} RPAREN
///   ]`, a direct self-recursive call (~1 rule-frame per real nesting
///   level, but each frame is heavier — more local state per call — so it
///   overflows the native stack at a *lower* rule-frame count than the
///   paren shape despite the lighter per-level rule-frame cost).
/// - **Rulebook nesting** — `rulebook_decl = "rulebook" IDENT LBRACE
///   {statement} RBRACE`, and `statement`'s own alternation includes
///   `rulebook_decl`, so `rulebook a { rulebook b { … } }` recurses
///   `statement → rulebook_decl → statement → …` once per nested block
///   (flagged by security review as a shape the first pass of this fix
///   missed).
///
/// Measured (binary search, uncapped parser, on the true default-stack
/// per-test worker thread — no `RUST_MIN_STACK` override and no explicit
/// `Builder::stack_size`, matching what `cargo test` and a production
/// caller both actually get — debug build, adversarial 5000-level input):
/// paren shape safe through 260 rule-frames, crashes at 262; rulebook shape
/// safe through 245, crashes at 250; call shape safe through 124, crashes
/// at 126 — call nesting is the *binding* (lower) floor of the three.
///
/// `MAX_RULE_DEPTH` is set to **90** — about 27% below the binding
/// 124-rule-frame floor (comparable margin to sibling crates' 25-45%
/// convention), independently confirmed not to crash a default-stack
/// thread even thousands of rule-frames past the cap for any of the three
/// shapes (see this crate's tests). Measured real-nesting headroom at 90
/// (capped parser, so no crash risk): paren nesting parses cleanly up to 28
/// levels (29 trips the cap), rulebook nesting up to 44 levels (45 trips
/// the cap), call nesting up to 86 levels (87 trips the cap) — comfortably
/// past any hand-written adj-lang program's real nesting.
const MAX_RULE_DEPTH: usize = 90;

fn parse_grammar_ast(src: &str) -> Result<GrammarASTNode, CompileError> {
    let token_grammar = _lexer_grammar::token_grammar();
    let mut grammar_lexer = GrammarLexer::new(src, &token_grammar);
    let tokens = grammar_lexer
        .tokenize()
        .map_err(|e| CompileError::Lex(format!("{e}")))?;
    let parser_grammar = _parser_grammar::parser_grammar();
    let mut grammar_parser =
        GrammarParser::new(tokens, parser_grammar).with_max_depth(MAX_RULE_DEPTH);
    grammar_parser.parse().map_err(CompileError::Parse)
}

/// Tokenize + parse + adapt: produce a typed [`Program`] from source text.
pub fn parse(src: &str) -> Result<Program, CompileError> {
    let tree = parse_grammar_ast(src)?;
    adapt_program(&tree).map_err(CompileError::Adapt)
}

/// Parse one source and inventory every formula with exact UTF-8 byte spans.
///
/// The returned body span names only the final executable expression. For a
/// multi-step formula it excludes the preceding `let` steps. Declaration spans
/// include the formula's provenance annotations because those bytes are part of
/// the authored claim. This inventory does not resolve imports, lower formulas,
/// or prove that an external source expression is equivalent to the body; those
/// remain separate validation stages.
pub fn formula_source_map(src: &str) -> Result<Vec<FormulaSource>, FormulaSourceMapError> {
    Ok(program_source_map(src)?.formulas)
}

/// Parse one source and locate formulas, ordinary queries, and imports in exact bytes.
pub fn program_source_map(src: &str) -> Result<ProgramSourceMap, FormulaSourceMapError> {
    let tree = parse_grammar_ast(src)?;
    let program = adapt_program(&tree).map_err(CompileError::Adapt)?;
    let locator = SourceLocator::new(src);
    let parsed_books = collect_nodes(&tree, "formulabook_decl");
    let mut typed_books = Vec::new();
    collect_typed_formulabooks(&program.statements, &mut typed_books);
    if parsed_books.len() != typed_books.len() {
        return Err(FormulaSourceMapError::Inconsistent(format!(
            "parser found {} formulabooks but adapter produced {}",
            parsed_books.len(),
            typed_books.len()
        )));
    }

    let mut inventory = Vec::new();
    for (book_node, (book_name, formulas)) in parsed_books.into_iter().zip(typed_books) {
        let parsed_name = direct_identifier_after(book_node, "formulabook").ok_or_else(|| {
            FormulaSourceMapError::Inconsistent("formulabook name is absent".into())
        })?;
        if parsed_name != book_name {
            return Err(FormulaSourceMapError::Inconsistent(format!(
                "formulabook parse tree names {parsed_name} but adapter produced {book_name}"
            )));
        }
        let formula_nodes = collect_nodes(book_node, "formula_decl");
        if formula_nodes.len() != formulas.len() {
            return Err(FormulaSourceMapError::Inconsistent(format!(
                "formulabook {book_name} has {} parsed formulas but {} adapted formulas",
                formula_nodes.len(),
                formulas.len()
            )));
        }
        for (formula_node, formula) in formula_nodes.into_iter().zip(formulas) {
            let parsed_name =
                direct_identifier_after(formula_node, "formula").ok_or_else(|| {
                    FormulaSourceMapError::Inconsistent("formula name is absent".into())
                })?;
            if parsed_name != formula.name {
                return Err(FormulaSourceMapError::Inconsistent(format!(
                    "formula parse tree names {parsed_name} but adapter produced {}",
                    formula.name
                )));
            }
            let body_node = direct_child(formula_node, "formula_body")
                .and_then(|body| direct_child(body, "expr"))
                .ok_or_else(|| {
                    FormulaSourceMapError::Inconsistent(format!(
                        "formula {} has no final body expression",
                        formula.name
                    ))
                })?;
            inventory.push(FormulaSource {
                formulabook: book_name.clone(),
                formula: formula.clone(),
                declaration_span: locator.node_span(formula_node)?,
                body_span: locator.node_span(body_node)?,
            });
        }
    }
    let parsed_imports = collect_nodes(&tree, "import_decl");
    let mut typed_imports = Vec::new();
    collect_typed_imports(&program.statements, &mut typed_imports);
    if parsed_imports.len() != typed_imports.len() {
        return Err(FormulaSourceMapError::Inconsistent(format!(
            "parser found {} imports but adapter produced {}",
            parsed_imports.len(),
            typed_imports.len()
        )));
    }
    let imports = parsed_imports
        .into_iter()
        .zip(typed_imports)
        .map(|(node, literal)| {
            Ok(ImportSource {
                literal: literal.clone(),
                declaration_span: locator.node_span(node)?,
            })
        })
        .collect::<Result<Vec<_>, FormulaSourceMapError>>()?;

    let parsed_queries: Vec<_> = collect_nodes(&tree, "query_decl")
        .into_iter()
        .filter(|node| direct_child(node, "term").is_some())
        .collect();
    let mut typed_queries = Vec::new();
    collect_typed_queries(&program.statements, &mut typed_queries);
    if parsed_queries.len() != typed_queries.len() {
        return Err(FormulaSourceMapError::Inconsistent(format!(
            "parser found {} ordinary queries but adapter produced {}",
            parsed_queries.len(),
            typed_queries.len()
        )));
    }
    let queries = parsed_queries
        .into_iter()
        .zip(typed_queries)
        .map(|(node, conclusion)| {
            Ok(QuerySource {
                conclusion: conclusion.clone(),
                declaration_span: locator.node_span(node)?,
            })
        })
        .collect::<Result<Vec<_>, FormulaSourceMapError>>()?;

    Ok(ProgramSourceMap {
        formulas: inventory,
        imports,
        queries,
    })
}

fn collect_nodes<'a>(node: &'a GrammarASTNode, rule_name: &str) -> Vec<&'a GrammarASTNode> {
    let mut found = Vec::new();
    for child in &node.children {
        if let ASTNodeOrToken::Node(child) = child {
            if child.rule_name == rule_name {
                found.push(child);
            } else {
                found.extend(collect_nodes(child, rule_name));
            }
        }
    }
    found
}

fn collect_typed_formulabooks<'a>(
    statements: &'a [ast::Statement],
    found: &mut Vec<(&'a String, &'a Vec<ast::FormulaDef>)>,
) {
    for statement in statements {
        match statement {
            ast::Statement::Formulabook { name, formulas, .. } => found.push((name, formulas)),
            ast::Statement::Rulebook { statements, .. } => {
                collect_typed_formulabooks(statements, found);
            }
            _ => {}
        }
    }
}

fn collect_typed_imports<'a>(statements: &'a [ast::Statement], found: &mut Vec<&'a String>) {
    for statement in statements {
        match statement {
            ast::Statement::Import(literal) => found.push(literal),
            ast::Statement::Rulebook { statements, .. } => {
                collect_typed_imports(statements, found);
            }
            _ => {}
        }
    }
}

fn collect_typed_queries<'a>(statements: &'a [ast::Statement], found: &mut Vec<&'a ast::Term>) {
    for statement in statements {
        match statement {
            ast::Statement::Query { conclusion } => found.push(conclusion),
            ast::Statement::Rulebook { statements, .. } => {
                collect_typed_queries(statements, found);
            }
            _ => {}
        }
    }
}

fn direct_child<'a>(node: &'a GrammarASTNode, rule_name: &str) -> Option<&'a GrammarASTNode> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Node(child) if child.rule_name == rule_name => Some(child),
        _ => None,
    })
}

fn direct_identifier_after<'a>(node: &'a GrammarASTNode, keyword: &str) -> Option<&'a str> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token)
            if token.type_ == TokenType::Name && token.value != keyword =>
        {
            Some(token.value.as_str())
        }
        _ => None,
    })
}

fn first_token(node: &GrammarASTNode) -> Option<&Token> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) => Some(token),
        ASTNodeOrToken::Node(child) => first_token(child),
    })
}

fn last_token(node: &GrammarASTNode) -> Option<&Token> {
    node.children.iter().rev().find_map(|child| match child {
        ASTNodeOrToken::Token(token) => Some(token),
        ASTNodeOrToken::Node(child) => last_token(child),
    })
}

struct SourceLocator<'a> {
    src: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> SourceLocator<'a> {
    fn new(src: &'a str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(src.match_indices('\n').map(|(offset, _)| offset + 1));
        Self { src, line_starts }
    }

    fn node_span(&self, node: &GrammarASTNode) -> Result<SourceSpan, FormulaSourceMapError> {
        let first = first_token(node).ok_or_else(|| {
            FormulaSourceMapError::Inconsistent(format!("{} has no first token", node.rule_name))
        })?;
        let last = last_token(node).ok_or_else(|| {
            FormulaSourceMapError::Inconsistent(format!("{} has no last token", node.rule_name))
        })?;
        let start = self.token_start(first)?;
        let end = self.token_end(last)?;
        if start >= end || end > self.src.len() {
            return Err(FormulaSourceMapError::Inconsistent(format!(
                "{} has invalid byte span {start}..{end}",
                node.rule_name
            )));
        }
        Ok(SourceSpan { start, end })
    }

    fn token_start(&self, token: &Token) -> Result<usize, FormulaSourceMapError> {
        let line_index = token.line.checked_sub(1).ok_or_else(|| {
            FormulaSourceMapError::Inconsistent("token line must be one-based".into())
        })?;
        let line_start = *self.line_starts.get(line_index).ok_or_else(|| {
            FormulaSourceMapError::Inconsistent(format!(
                "token line {} is outside the source",
                token.line
            ))
        })?;
        let line_end = self
            .line_starts
            .get(line_index + 1)
            .map_or(self.src.len(), |next| next - 1);
        let line = &self.src[line_start..line_end];
        let column_offset = if token.column == 1 {
            0
        } else {
            line.char_indices()
                .nth(token.column - 1)
                .map(|(offset, _)| offset)
                .ok_or_else(|| {
                    FormulaSourceMapError::Inconsistent(format!(
                        "token column {} is outside source line {}",
                        token.column, token.line
                    ))
                })?
        };
        Ok(line_start + column_offset)
    }

    fn token_end(&self, token: &Token) -> Result<usize, FormulaSourceMapError> {
        let start = self.token_start(token)?;
        let suffix = &self.src[start..];
        let is_string = token.type_ == TokenType::String
            || token
                .type_name
                .as_deref()
                .is_some_and(|name| name.ends_with("STRING"));
        if is_string {
            return self.quoted_token_end(start);
        }
        if !suffix.starts_with(&token.value) {
            return Err(FormulaSourceMapError::Inconsistent(format!(
                "token {:?} does not match source byte {}",
                token.value, start
            )));
        }
        Ok(start + token.value.len())
    }

    fn quoted_token_end(&self, start: usize) -> Result<usize, FormulaSourceMapError> {
        let suffix = &self.src[start..];
        let quote = suffix
            .chars()
            .next()
            .filter(|value| matches!(value, '\'' | '"'));
        let Some(quote) = quote else {
            return Err(FormulaSourceMapError::Inconsistent(format!(
                "string token at byte {start} lacks a quote"
            )));
        };
        let delimiter = if suffix.starts_with(&quote.to_string().repeat(3)) {
            quote.to_string().repeat(3)
        } else {
            quote.to_string()
        };
        let mut escaped = false;
        let mut cursor = delimiter.len();
        while cursor < suffix.len() {
            if !escaped && suffix[cursor..].starts_with(&delimiter) {
                return Ok(start + cursor + delimiter.len());
            }
            let character = suffix[cursor..].chars().next().ok_or_else(|| {
                FormulaSourceMapError::Inconsistent("string token ended unexpectedly".into())
            })?;
            cursor += character.len_utf8();
            escaped = delimiter.len() == 1 && character == '\\' && !escaped;
        }
        Err(FormulaSourceMapError::Inconsistent(format!(
            "string token at byte {start} is unterminated"
        )))
    }
}

/// Top-level convenience: source text → lowered program (KB +
/// queries).
pub fn compile(src: &str) -> Result<LoweredProgram, CompileError> {
    let program = parse(src)?;
    lower(&program).map_err(CompileError::Lower)
}

/// Return the provenance object the lowerer assigns to a formula definition.
pub fn formula_provenance(
    formula: &ast::FormulaDef,
) -> Result<logic_engine::Provenance, LowerError> {
    lower::formula_provenance(formula)
}

/// Result of compiling a program that may `import` other files: either an
/// import-graph failure (cycle, bound, missing/unparseable file) from the
/// [`resolve`] stage, or a lowering failure from the merged program.
#[derive(Debug)]
pub enum CompileWithImportsError {
    Import(ImportError),
    Lower(LowerError),
}

/// Import-aware compile: resolve the import graph rooted at `root_id` (driven by
/// the injected [`ImportProvider`]), then lower the merged program (MYCIN-2026
/// M3). The library performs **no** filesystem I/O — the provider is the only
/// thing that reads files, so the caller controls the sandbox. See
/// [`resolve::resolve_imports`] for the graph guarantees.
pub fn compile_with_imports(
    root_id: &str,
    provider: &dyn ImportProvider,
    limits: ImportLimits,
) -> Result<LoweredProgram, CompileWithImportsError> {
    let program =
        resolve_imports(root_id, provider, limits).map_err(CompileWithImportsError::Import)?;
    lower(&program).map_err(CompileWithImportsError::Lower)
}

/// Run a **differential** over a lowered program's `? h` query lines:
/// treat the program's queries as the competing hypotheses, rank them by
/// posterior, and return the comparative [`Differential`] decision (argmax +
/// between-hypothesis margin, with a kickback when an open uncertainty
/// could flip the ranking).
///
/// This is the natural reading of a multi-`?` adj-lang program: the queries
/// *are* the differential. A program with a single `?` yields a
/// determinate, single-hypothesis result.
pub fn decide(lowered: &LoweredProgram) -> Differential {
    logic_engine::differential(&lowered.queries, &lowered.kb)
}

/// Source text → differential decision in one step (`compile` then
/// [`decide`]).
pub fn compile_and_decide(src: &str) -> Result<Differential, CompileError> {
    let lowered = compile(src)?;
    Ok(decide(&lowered))
}

#[cfg(test)]
fn nested_paren_source(n: usize) -> String {
    format!("let x = {}1{}\n", "(".repeat(n), ")".repeat(n))
}

#[cfg(test)]
fn nested_term_source(n: usize) -> String {
    format!("observe {}x{}\n", "f(".repeat(n), ")".repeat(n))
}

#[cfg(test)]
fn nested_rulebook_source(n: usize) -> String {
    let mut src = String::new();
    for i in 0..n {
        src.push_str(&format!("rulebook r{i} {{ "));
    }
    src.push_str(&"}".repeat(n));
    src.push('\n');
    src
}

/// Deeply-nested paren input must produce a recoverable error, not overflow
/// the native stack. We parse 5000 levels — far past `MAX_RULE_DEPTH` — on a
/// worker thread with a generous 32 MiB stack, so the *guard* is what stops
/// the recursion, not the stack running out.
#[test]
fn test_deeply_nested_paren_input_returns_error_not_overflow() {
    let handle = std::thread::Builder::new()
        .name("adj-lang-depth-guard-paren-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = parse(&nested_paren_source(5000));
            assert!(
                result.is_err(),
                "deeply-nested paren input must fail with an error, not parse or crash"
            );
        })
        .expect("failed to spawn worker thread");
    handle
        .join()
        .expect("depth guard must keep the worker thread from crashing");
}

/// Deeply-nested call input (adj-lang's second, independent recursive
/// shape) must also produce a recoverable error, not overflow the native
/// stack.
#[test]
fn test_deeply_nested_term_input_returns_error_not_overflow() {
    let handle = std::thread::Builder::new()
        .name("adj-lang-depth-guard-term-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = parse(&nested_term_source(5000));
            assert!(
                result.is_err(),
                "deeply-nested call input must fail with an error, not parse or crash"
            );
        })
        .expect("failed to spawn worker thread");
    handle
        .join()
        .expect("depth guard must keep the worker thread from crashing");
}

/// Paren input that nests *exactly up to* `MAX_RULE_DEPTH` still parses
/// cleanly, and one layer deeper cleanly trips the guard. These exact
/// boundary counts (28 legitimate levels) were found empirically by
/// binary-searching against increasing nesting counts at the production
/// cap — see `MAX_RULE_DEPTH`'s doc comment.
#[test]
fn test_paren_nesting_up_to_cap_still_parses() {
    assert!(
        parse(&nested_paren_source(28)).is_ok(),
        "28 levels must stay under the cap"
    );
    assert!(
        parse(&nested_paren_source(29)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// Call input that nests *exactly up to* `MAX_RULE_DEPTH` still parses
/// cleanly, and one layer deeper cleanly trips the guard (86 legitimate
/// levels, empirically measured — see `MAX_RULE_DEPTH`'s doc comment).
#[test]
fn test_term_nesting_up_to_cap_still_parses() {
    assert!(
        parse(&nested_term_source(86)).is_ok(),
        "86 levels must stay under the cap"
    );
    assert!(
        parse(&nested_term_source(87)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
/// the native stack overflows on a default-stack thread — otherwise a
/// production caller (e.g. `adj-lang-cli`, or `cargo test`'s own per-test
/// thread) would still crash. We parse far-too-deep paren input on a
/// worker thread with **no** `stack_size` override (the same default a
/// thread gets in this environment, unmodified by any `RUST_MIN_STACK`
/// override). A clean `Err` (not a `join()`
/// failure from a crashed thread) proves `MAX_RULE_DEPTH` sits safely below
/// the native overflow point on the default stack.
#[test]
fn test_opt_in_cap_trips_before_paren_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = parse(&nested_paren_source(5000));
        assert!(
            result.is_err(),
            "deeply-nested paren input must error, not crash"
        );
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
}

/// Same as [`test_opt_in_cap_trips_before_paren_overflow_on_default_stack`]
/// but for the call-nesting shape — the *binding* (lower) native-stack
/// floor of the three, per `MAX_RULE_DEPTH`'s doc comment.
#[test]
fn test_opt_in_cap_trips_before_term_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = parse(&nested_term_source(5000));
        assert!(
            result.is_err(),
            "deeply-nested call input must error, not crash"
        );
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
}

/// Deeply-nested `rulebook { rulebook { … } }` input (adj-lang's third,
/// independent recursive shape — `statement → rulebook_decl → statement`,
/// flagged by security review) must also produce a recoverable error, not
/// overflow the native stack.
#[test]
fn test_deeply_nested_rulebook_input_returns_error_not_overflow() {
    let handle = std::thread::Builder::new()
        .name("adj-lang-depth-guard-rulebook-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = parse(&nested_rulebook_source(5000));
            assert!(
                result.is_err(),
                "deeply-nested rulebook input must fail with an error, not parse or crash"
            );
        })
        .expect("failed to spawn worker thread");
    handle
        .join()
        .expect("depth guard must keep the worker thread from crashing");
}

/// Rulebook-nesting input that nests *exactly up to* `MAX_RULE_DEPTH` still
/// parses cleanly, and one layer deeper cleanly trips the guard (44
/// legitimate levels, empirically measured — see `MAX_RULE_DEPTH`'s doc
/// comment).
#[test]
fn test_rulebook_nesting_up_to_cap_still_parses() {
    assert!(
        parse(&nested_rulebook_source(44)).is_ok(),
        "44 levels must stay under the cap"
    );
    assert!(
        parse(&nested_rulebook_source(45)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// Same as [`test_opt_in_cap_trips_before_paren_overflow_on_default_stack`]
/// but for the rulebook-nesting shape.
#[test]
fn test_opt_in_cap_trips_before_rulebook_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = parse(&nested_rulebook_source(5000));
        assert!(
            result.is_err(),
            "deeply-nested rulebook input must error, not crash"
        );
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
}
