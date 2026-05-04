//! Parse a token stream using the grammar-tools GrammarParser.
//!
//! ## Implementation plan (LS02 PR A)
//!
//! 1. Construct `grammar_tools::GrammarParser::from_source(spec.grammar_source)`.
//!
//! 2. Run the parser over the token stream from tokenize::run() →
//!    Result<grammar_tools::GrammarASTNode, Vec<grammar_tools::ParseError>>.
//!
//! 3. Map parse errors to ls00::Diagnostic:
//!    - message: error description
//!    - range:   from error token's line/column
//!    - severity: DiagnosticSeverity::Error
//!
//! 4. Box the GrammarASTNode as Arc<GrammarASTNode> then Box<dyn Any + Send + Sync>.
//!    This allows ls00 to pass it back as &dyn Any and we downcast in each handler.
//!
//! 5. Return (Box<dyn Any + Send + Sync>, Vec<ls00::Diagnostic>).
//!
//! ## Key grammar-tools types to verify:
//!   GrammarASTNode { rule_name: String, children: Vec<GrammarASTChild> }
//!   GrammarASTChild = Token(LexToken) | Node(GrammarASTNode)
//!   Each node should carry start/end position — verify this; if missing,
//!   add line/col to GrammarASTNode in grammar-tools first (PREREQUISITE).

use crate::LanguageSpec;

/// Parse `source` using the syntactic grammar in `spec`.
///
/// Returns `(ast, diagnostics)`. Diagnostics include both lex and parse errors.
/// Even on parse error, a partial AST may be returned (error recovery).
///
/// ## TODO — implement (LS02 PR A)
pub fn run(
    spec: &'static LanguageSpec,
    source: &str,
) -> (Option<()>, Vec<()>) {
    // TODO: replace () with Box<dyn Any + Send + Sync> and ls00::Diagnostic.
    let _ = (spec, source);
    (None, vec![])
}
