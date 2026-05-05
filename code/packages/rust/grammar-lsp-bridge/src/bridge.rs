//! [`GrammarLanguageBridge`] — the generic `LanguageBridge` implementation.
//!
//! ## Architecture
//!
//! At construction time we parse the two grammar files embedded in the
//! `LanguageSpec` into in-memory data structures:
//!
//! ```text
//! spec.tokens_source  ──► parse_token_grammar()  ──► TokenGrammar (stored)
//! spec.grammar_source ──► parse_parser_grammar() ──► ParserGrammar (stored)
//! ```
//!
//! At runtime, each `LanguageBridge` method call uses those structures:
//!
//! | Method             | Runtime work                                            |
//! |--------------------|---------------------------------------------------------|
//! | `tokenize`         | `GrammarLexer` → convert to `ls00::Token`               |
//! | `parse`            | `GrammarLexer` + `GrammarParser` → `GrammarASTNode`     |
//! | `semantic_tokens`  | Map `ls00::Token::token_type` via `token_kind_map`      |
//! | `document_symbols` | `find_nodes` for each `declaration_rules` entry         |
//! | `folding_ranges`   | Walk AST for multi-line nodes                           |
//! | `hover`            | Find innermost AST node at cursor position              |
//! | `completion`       | Keywords + declaration names from AST                   |
//! | `format`           | Delegate to `spec.format_fn`                            |

use std::any::Any;

use coding_adventures_ls00::{
    language_bridge::LanguageBridge,
    types::{
        CompletionItem, CompletionItemKind, Diagnostic, DiagnosticSeverity, DocumentSymbol,
        FoldingRange, HoverResult, Position, Range, SemanticToken, SymbolKind, TextEdit, Token,
    },
};
use grammar_tools::{
    parser_grammar::{parse_parser_grammar, ParserGrammar},
    token_grammar::{parse_token_grammar, TokenGrammar},
};
use lexer::grammar_lexer::GrammarLexer;
use parser::grammar_parser::{
    collect_tokens, find_nodes, ASTNodeOrToken, GrammarASTNode, GrammarParser,
};

use crate::spec::LanguageSpec;

// ===========================================================================
// GrammarLanguageBridge
// ===========================================================================

/// Generic LSP bridge parameterised by a [`LanguageSpec`].
///
/// Construct with [`GrammarLanguageBridge::new`] and pass to
/// `coding_adventures_ls00::LspServer::new`.
///
/// ## Example
///
/// ```rust,ignore
/// let bridge = GrammarLanguageBridge::new(&MY_SPEC);
/// coding_adventures_ls00::serve_stdio(bridge).expect("LSP error");
/// ```
pub struct GrammarLanguageBridge {
    spec: &'static LanguageSpec,
    /// Pre-parsed token grammar (used by GrammarLexer at tokenize/parse time).
    token_grammar: TokenGrammar,
    /// Pre-parsed parser grammar (cloned into GrammarParser on each parse call).
    parser_grammar: ParserGrammar,
}

impl GrammarLanguageBridge {
    /// Build a bridge from a static `LanguageSpec`.
    ///
    /// Parses both grammar files at construction time. **Panics** if either
    /// grammar file is malformed — grammar files are embedded compile-time
    /// constants and any parse failure is a programming error, not a runtime
    /// error.
    pub fn new(spec: &'static LanguageSpec) -> Self {
        let token_grammar = parse_token_grammar(spec.tokens_source).unwrap_or_else(|e| {
            panic!(
                "grammar-lsp-bridge: invalid tokens_source for '{}': {}",
                spec.name, e
            )
        });
        let parser_grammar = parse_parser_grammar(spec.grammar_source).unwrap_or_else(|e| {
            panic!(
                "grammar-lsp-bridge: invalid grammar_source for '{}': {}",
                spec.name, e
            )
        });
        GrammarLanguageBridge { spec, token_grammar, parser_grammar }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Lex `source` with `GrammarLexer` and return the raw lexer tokens.
    ///
    /// The GrammarLexer already skips WHITESPACE/COMMENT (from the `skip:`
    /// section of the `.tokens` file), so the returned list contains only
    /// meaningful tokens plus a final EOF sentinel.
    fn lex_raw(&self, source: &str) -> Result<Vec<lexer::token::Token>, String> {
        let mut lex = GrammarLexer::new(source, &self.token_grammar);
        lex.tokenize().map_err(|e| e.to_string())
    }

    /// Build an error `GrammarASTNode` (returned when parsing fails).
    fn error_ast() -> GrammarASTNode {
        GrammarASTNode {
            rule_name: "<error>".to_string(),
            children: vec![],
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        }
    }

    /// Find the innermost `GrammarASTNode` that contains `pos` (0-based LSP).
    fn node_at_pos<'a>(node: &'a GrammarASTNode, pos: &Position) -> Option<&'a GrammarASTNode> {
        // pos.line is 0-based; AST lines are 1-based.
        let target_line = (pos.line + 1) as usize;
        let start = node.start_line.unwrap_or(0);
        let end   = node.end_line.unwrap_or(0);

        if start > target_line || end < target_line {
            return None;
        }

        // Recurse into children to find a tighter-fitting node.
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                if let Some(found) = Self::node_at_pos(child_node, pos) {
                    return Some(found);
                }
            }
        }

        Some(node)
    }

    /// Extract the first NAME identifier token from `node` (recursive).
    /// Returns the token value, or `None` if no NAME token is found.
    fn first_name_token(node: &GrammarASTNode) -> Option<String> {
        collect_tokens(node, Some("NAME"))
            .into_iter()
            .next()
            .map(|t| t.value)
    }

    /// Convert 1-based `(line, col)` to a 0-based LSP `Position`.
    fn to_lsp_pos(line1: usize, col1: usize) -> Position {
        Position {
            line:      (line1 as i32).saturating_sub(1),
            character: (col1  as i32).saturating_sub(1),
        }
    }

    /// Build a `Range` from two 1-based (line, col) pairs.
    fn to_lsp_range(
        start_line: usize, start_col: usize,
        end_line:   usize, end_col:   usize,
    ) -> Range {
        Range {
            start: Self::to_lsp_pos(start_line, start_col),
            end:   Self::to_lsp_pos(end_line,   end_col),
        }
    }
}

// ===========================================================================
// LanguageBridge implementation
// ===========================================================================

impl LanguageBridge for GrammarLanguageBridge {
    // -----------------------------------------------------------------------
    // Required: tokenize
    // -----------------------------------------------------------------------

    /// Lex `source` and convert the token stream to `ls00::types::Token`.
    ///
    /// The conversion:
    /// - `token_type` ← `lexer_token.effective_type_name()` (the name from the
    ///   `.tokens` grammar, e.g. `"KEYWORD"`, `"NAME"`, `"INTEGER"`).
    /// - `line` / `column` ← 1-based (matching the lexer output).
    /// - EOF tokens are excluded from the result.
    fn tokenize(&self, source: &str) -> Result<Vec<Token>, String> {
        let raw = self.lex_raw(source)?;
        let tokens = raw
            .into_iter()
            .filter(|t| t.effective_type_name() != "EOF")
            .map(|t| Token {
                token_type: t.effective_type_name().to_string(),
                value:      t.value.clone(),
                line:       t.line   as i32,
                column:     t.column as i32,
            })
            .collect();
        Ok(tokens)
    }

    // -----------------------------------------------------------------------
    // Required: parse
    // -----------------------------------------------------------------------

    /// Parse `source` using the grammar-driven parser.
    ///
    /// Returns a boxed `GrammarASTNode` as the AST.  All subsequent optional
    /// bridge methods downcast it back to `&GrammarASTNode`.
    ///
    /// On tokenisation error or parse error a `Diagnostic` is produced and an
    /// empty (error) AST node is returned so the server can continue showing
    /// other diagnostics.
    fn parse(
        &self,
        source: &str,
    ) -> Result<(Box<dyn Any + Send + Sync>, Vec<Diagnostic>), String> {
        // Step 1: tokenise with the grammar-driven lexer.
        let raw_tokens = match self.lex_raw(source) {
            Ok(t) => t,
            Err(e) => {
                let diag = Diagnostic {
                    range:    Range {
                        start: Position { line: 0, character: 0 },
                        end:   Position { line: 0, character: 0 },
                    },
                    severity: DiagnosticSeverity::Error,
                    message:  format!("Lexer error: {e}"),
                    code:     None,
                };
                return Ok((Box::new(Self::error_ast()), vec![diag]));
            }
        };

        // Step 2: parse with the grammar-driven parser.
        let mut gp = GrammarParser::new(raw_tokens, self.parser_grammar.clone());
        match gp.parse() {
            Ok(ast) => Ok((Box::new(ast) as Box<dyn Any + Send + Sync>, vec![])),
            Err(e) => {
                let pos = Position {
                    line:      (e.token.line   as i32).saturating_sub(1),
                    character: (e.token.column as i32).saturating_sub(1),
                };
                let diag = Diagnostic {
                    range:    Range { start: pos.clone(), end: pos },
                    severity: DiagnosticSeverity::Error,
                    message:  e.message.clone(),
                    code:     None,
                };
                Ok((Box::new(Self::error_ast()), vec![diag]))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Optional: semantic_tokens
    // -----------------------------------------------------------------------

    /// Map each token's type through `spec.token_kind_map` and emit
    /// `SemanticToken` entries.
    ///
    /// Tokens whose type is not in `token_kind_map` (e.g. punctuation) are
    /// silently skipped — they get no semantic token.
    ///
    /// Coordinate conversion: `ls00::Token` is 1-based; `SemanticToken` is
    /// 0-based, measured in UTF-16 code units.  For ASCII-only source,
    /// UTF-16 code units equal byte length.
    fn semantic_tokens(
        &self,
        _source: &str,
        tokens: &[Token],
    ) -> Option<Result<Vec<SemanticToken>, String>> {
        let sem: Vec<SemanticToken> = tokens
            .iter()
            .filter_map(|t| {
                // Look up the grammar token name in the spec's kind map.
                let lsp_type = self.spec.token_kind_map
                    .iter()
                    .find(|(name, _)| *name == t.token_type.as_str())
                    .map(|(_, ty)| ty)?;

                Some(SemanticToken {
                    line:       t.line   - 1,         // 1-based → 0-based
                    character:  t.column - 1,         // 1-based → 0-based
                    length:     t.value.len() as i32, // UTF-16 length (ASCII safe)
                    token_type: lsp_type.as_str().to_string(),
                    modifiers:  vec![],
                })
            })
            .collect();
        Some(Ok(sem))
    }

    // -----------------------------------------------------------------------
    // Optional: document_symbols
    // -----------------------------------------------------------------------

    /// Return one `DocumentSymbol` per node matching a `declaration_rules` rule.
    ///
    /// The symbol name is the first `NAME` token found inside the matched node.
    /// If no NAME token exists the rule name itself is used as a fallback label.
    fn document_symbols(&self, ast: &dyn Any) -> Option<Result<Vec<DocumentSymbol>, String>> {
        let ast = ast.downcast_ref::<GrammarASTNode>()?;

        if ast.rule_name == "<error>" {
            return Some(Ok(vec![]));
        }

        let mut symbols: Vec<DocumentSymbol> = Vec::new();

        for &rule_name in self.spec.declaration_rules {
            for node in find_nodes(ast, rule_name) {
                let sym_name = Self::first_name_token(&node)
                    .unwrap_or_else(|| rule_name.to_string());

                let sl = node.start_line.unwrap_or(1);
                let sc = node.start_column.unwrap_or(1);
                let el = node.end_line.unwrap_or(sl);
                let ec = node.end_column.unwrap_or(sc);

                let full_range      = Self::to_lsp_range(sl, sc, el, ec);
                let selection_range = Self::to_lsp_range(sl, sc, sl, sc + sym_name.len());

                symbols.push(DocumentSymbol {
                    name:             sym_name,
                    kind:             SymbolKind::Function,
                    range:            full_range,
                    selection_range,
                    children:         vec![],
                });
            }
        }

        Some(Ok(symbols))
    }

    // -----------------------------------------------------------------------
    // Optional: folding_ranges
    // -----------------------------------------------------------------------

    /// Emit a `FoldingRange` for every AST node that spans more than one line.
    ///
    /// This gives collapsible regions at the granularity of grammar rules —
    /// e.g. function bodies, `if` branches, `let` bindings.
    fn folding_ranges(&self, ast: &dyn Any) -> Option<Result<Vec<FoldingRange>, String>> {
        let ast = ast.downcast_ref::<GrammarASTNode>()?;
        let mut ranges: Vec<FoldingRange> = Vec::new();
        collect_folding(ast, &mut ranges);
        Some(Ok(ranges))
    }

    // -----------------------------------------------------------------------
    // Optional: hover
    // -----------------------------------------------------------------------

    /// Show the matched grammar rule name as hover documentation.
    ///
    /// This is a simple but universally applicable hover: it shows the parser's
    /// understanding of what syntactic construct sits at the cursor position,
    /// which is useful for learning and debugging grammars.
    fn hover(
        &self,
        ast: &dyn Any,
        pos: Position,
    ) -> Option<Result<Option<HoverResult>, String>> {
        let ast = ast.downcast_ref::<GrammarASTNode>()?;
        let node = Self::node_at_pos(ast, &pos)?;

        // Don't show hover for root or error nodes — not useful.
        if node.rule_name == "<error>" || node.rule_name == ast.rule_name {
            return Some(Ok(None));
        }

        let content = format!("**`{}`**", node.rule_name);
        Some(Ok(Some(HoverResult { contents: content, range: None })))
    }

    // -----------------------------------------------------------------------
    // Optional: completion
    // -----------------------------------------------------------------------

    /// Provide keyword completions + declaration-name completions.
    ///
    /// - Keywords: one `CompletionItem` per entry in `spec.keyword_names`.
    /// - Declarations: first `NAME` token from each node matching a
    ///   `declaration_rules` rule.
    fn completion(
        &self,
        ast: &dyn Any,
        _pos: Position,
    ) -> Option<Result<Vec<CompletionItem>, String>> {
        let mut items: Vec<CompletionItem> = Vec::new();

        // 1. Keywords — always offered regardless of cursor position.
        for &kw in self.spec.keyword_names {
            items.push(CompletionItem {
                label:              kw.to_string(),
                kind:               Some(CompletionItemKind::Keyword),
                detail:             None,
                documentation:      None,
                insert_text:        None,
                insert_text_format: None,
            });
        }

        // 2. Declarations extracted from the current AST.
        if let Some(ast_node) = ast.downcast_ref::<GrammarASTNode>() {
            if ast_node.rule_name != "<error>" {
                for &rule_name in self.spec.declaration_rules {
                    for node in find_nodes(ast_node, rule_name) {
                        if let Some(name) = Self::first_name_token(&node) {
                            items.push(CompletionItem {
                                label:              name,
                                kind:               Some(CompletionItemKind::Function),
                                detail:             Some(rule_name.to_string()),
                                documentation:      None,
                                insert_text:        None,
                                insert_text_format: None,
                            });
                        }
                    }
                }
            }
        }

        Some(Ok(items))
    }

    // -----------------------------------------------------------------------
    // Optional: format
    // -----------------------------------------------------------------------

    /// Delegate to `spec.format_fn` if present.
    ///
    /// Returns a single whole-file `TextEdit` that replaces the entire source
    /// with the formatted output.
    fn format(&self, source: &str) -> Option<Result<Vec<TextEdit>, String>> {
        let format_fn = self.spec.format_fn?;
        let formatted = match format_fn(source) {
            Ok(s)  => s,
            Err(e) => return Some(Err(e)),
        };
        let line_count = source.lines().count();
        let last_col   = source.lines().last().map(|l| l.len()).unwrap_or(0);
        let whole_file = Range {
            start: Position { line: 0, character: 0 },
            end:   Position {
                line:      line_count as i32,
                character: last_col   as i32,
            },
        };
        Some(Ok(vec![TextEdit { range: whole_file, new_text: formatted }]))
    }

    // -----------------------------------------------------------------------
    // Capability flags
    // -----------------------------------------------------------------------

    fn supports_semantic_tokens(&self)  -> bool { true }
    fn supports_document_symbols(&self) -> bool { true }
    fn supports_folding_ranges(&self)   -> bool { true }
    fn supports_hover(&self)            -> bool { true }
    fn supports_completion(&self)       -> bool { true }
    fn supports_format(&self)           -> bool { self.spec.format_fn.is_some() }
}

// ===========================================================================
// Folding range collector
// ===========================================================================

/// Recursively collect folding ranges for all multi-line AST nodes.
fn collect_folding(node: &GrammarASTNode, out: &mut Vec<FoldingRange>) {
    let start = node.start_line.unwrap_or(0);
    let end   = node.end_line.unwrap_or(0);

    if end > start && start > 0 {
        out.push(FoldingRange {
            start_line: (start as i32) - 1, // 1-based → 0-based
            end_line:   (end   as i32) - 1, // 1-based → 0-based
            kind:       Some("region".to_string()),
        });
    }

    for child in &node.children {
        if let ASTNodeOrToken::Node(child_node) = child {
            collect_folding(child_node, out);
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{LanguageSpec, LspSemanticTokenType};

    // -----------------------------------------------------------------------
    // Toy grammar for testing
    //
    // Tokens:
    //   LPAREN    = "("
    //   RPAREN    = ")"
    //   NUMBER    = /[0-9]+/
    //   NAME      = /[a-z][a-z0-9_]*/
    //   WHITESPACE (skip)
    //
    // Grammar (Lisp-like):
    //   program = { form } ;
    //   form    = list | atom ;
    //   list    = LPAREN { form } RPAREN ;
    //   atom    = NAME | NUMBER ;
    // -----------------------------------------------------------------------

    static TOY_TOKENS: &str = concat!(
        "LPAREN = \"(\"\n",
        "RPAREN = \")\"\n",
        "NUMBER = /[0-9]+/\n",
        "NAME   = /[a-z][a-z0-9_]*/\n",
        "skip:\n",
        "  WHITESPACE = /[ \\t\\n\\r]+/\n",
    );

    static TOY_GRAMMAR: &str = concat!(
        "program = { form } ;\n",
        "form    = list | atom ;\n",
        "list    = LPAREN { form } RPAREN ;\n",
        "atom    = NAME | NUMBER ;\n",
    );

    static TOY_SPEC: LanguageSpec = LanguageSpec {
        name:              "toy",
        file_extensions:   &["toy"],
        tokens_source:     TOY_TOKENS,
        grammar_source:    TOY_GRAMMAR,
        token_kind_map:    &[
            ("NUMBER", LspSemanticTokenType::Number),
            ("NAME",   LspSemanticTokenType::Variable),
        ],
        declaration_rules: &[],
        keyword_names:     &["define", "if", "let"],
        format_fn:         None,
        symbol_table_fn:   None,
    };

    fn bridge() -> GrammarLanguageBridge {
        GrammarLanguageBridge::new(&TOY_SPEC)
    }

    // -----------------------------------------------------------------------
    // tokenize
    // -----------------------------------------------------------------------

    #[test]
    fn tokenize_returns_tokens() {
        let b = bridge();
        let tokens = b.tokenize("(hello 42)").expect("tokenize ok");
        // Expected: LPAREN, NAME, NUMBER, RPAREN (WHITESPACE skipped).
        assert_eq!(tokens.len(), 4, "tokens: {:?}", tokens);
        assert_eq!(tokens[0].token_type, "LPAREN");
        assert_eq!(tokens[1].token_type, "NAME");
        assert_eq!(tokens[1].value,      "hello");
        assert_eq!(tokens[2].token_type, "NUMBER");
        assert_eq!(tokens[2].value,      "42");
        assert_eq!(tokens[3].token_type, "RPAREN");
    }

    #[test]
    fn tokenize_empty_source_returns_empty() {
        let b = bridge();
        let tokens = b.tokenize("").expect("tokenize ok");
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokenize_whitespace_skipped() {
        let b = bridge();
        let tokens = b.tokenize("  ( )  ").expect("tokenize ok");
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn tokenize_preserves_1based_line_col() {
        let b = bridge();
        let tokens = b.tokenize("(x)").expect("tokenize ok");
        assert_eq!(tokens[0].line,   1);
        assert_eq!(tokens[0].column, 1);
        assert_eq!(tokens[1].line,   1);
        assert_eq!(tokens[1].column, 2);
    }

    // -----------------------------------------------------------------------
    // parse
    // -----------------------------------------------------------------------

    #[test]
    fn parse_valid_source_returns_ast_no_diags() {
        let b = bridge();
        let (ast_box, diags) = b.parse("(hello 42)").expect("parse ok");
        assert!(diags.is_empty(), "no diagnostics for valid source");
        let ast = ast_box.downcast_ref::<GrammarASTNode>()
            .expect("AST is GrammarASTNode");
        assert_ne!(ast.rule_name, "<error>");
    }

    #[test]
    fn parse_empty_source_returns_program_node() {
        let b = bridge();
        let (ast_box, diags) = b.parse("").expect("parse ok");
        assert!(diags.is_empty());
        let ast = ast_box.downcast_ref::<GrammarASTNode>().expect("GrammarASTNode");
        // `program = { form }` matches zero forms.
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn parse_invalid_source_returns_diagnostic() {
        let b = bridge();
        // Unbalanced paren — the grammar parser should report an error.
        let (ast_box, diags) = b.parse("(hello").expect("no internal err");
        assert!(!diags.is_empty(), "unbalanced paren → diagnostic");
        let ast = ast_box.downcast_ref::<GrammarASTNode>().expect("GrammarASTNode");
        assert_eq!(ast.rule_name, "<error>");
    }

    #[test]
    fn parse_ast_has_position_info() {
        let b = bridge();
        let (ast_box, _) = b.parse("(abc 1)\n(def 2)").expect("parse ok");
        let ast = ast_box.downcast_ref::<GrammarASTNode>().expect("GrammarASTNode");
        if let Some(el) = ast.end_line {
            assert!(el >= 2, "root node ends at line ≥ 2, got {el}");
        }
    }

    // -----------------------------------------------------------------------
    // semantic_tokens
    // -----------------------------------------------------------------------

    #[test]
    fn semantic_tokens_maps_known_types() {
        let b = bridge();
        let tokens = b.tokenize("(abc 42)").expect("tokenize");
        let sem = b.semantic_tokens("", &tokens)
            .expect("Some").expect("Ok");
        let types: Vec<&str> = sem.iter().map(|s| s.token_type.as_str()).collect();
        assert!(types.contains(&"variable"), "variable in {:?}", types);
        assert!(types.contains(&"number"),   "number in {:?}", types);
    }

    #[test]
    fn semantic_tokens_skips_unmapped_types() {
        let b = bridge();
        // LPAREN / RPAREN are not in token_kind_map.
        let tokens = b.tokenize("(abc)").expect("tokenize");
        let sem = b.semantic_tokens("", &tokens)
            .expect("Some").expect("Ok");
        // Only "abc" (NAME → Variable).
        assert_eq!(sem.len(), 1, "one semantic token: {:?}", sem);
        assert_eq!(sem[0].token_type, "variable");
    }

    #[test]
    fn semantic_tokens_are_zero_based() {
        let b = bridge();
        let tokens = b.tokenize("(abc)").expect("tokenize");
        // "abc" is at line 1, col 2 (1-based) → (0, 1) 0-based.
        let sem = b.semantic_tokens("", &tokens).expect("Some").expect("Ok");
        assert_eq!(sem[0].line,      0);
        assert_eq!(sem[0].character, 1);
    }

    // -----------------------------------------------------------------------
    // folding_ranges
    // -----------------------------------------------------------------------

    #[test]
    fn folding_ranges_multi_line() {
        let b = bridge();
        let source = "(hello\n  world\n  42)";
        let (ast_box, _) = b.parse(source).expect("parse");
        let folds = b.folding_ranges(ast_box.as_ref())
            .expect("Some").expect("Ok");
        assert!(!folds.is_empty(), "expected folds for multi-line source");
        assert!(folds.iter().all(|f| f.end_line > f.start_line));
    }

    #[test]
    fn folding_ranges_single_line_no_folds() {
        let b = bridge();
        let (ast_box, _) = b.parse("(a b c)").expect("parse");
        let folds = b.folding_ranges(ast_box.as_ref())
            .expect("Some").expect("Ok");
        assert!(folds.is_empty(), "no folds for single-line: {:?}", folds);
    }

    // -----------------------------------------------------------------------
    // hover
    // -----------------------------------------------------------------------

    #[test]
    fn hover_returns_something_or_none() {
        let b = bridge();
        let source = "(hello\n  42)";
        let (ast_box, _) = b.parse(source).expect("parse");
        // Hover on line 2 (0-based: 1), col 2.
        let pos = Position { line: 1, character: 2 };
        let result = b.hover(ast_box.as_ref(), pos);
        // The important thing is that hover doesn't crash and returns a valid
        // shape.  The exact content is grammar-dependent.
        match result {
            Some(Ok(_)) | None => {} // all valid
            Some(Err(e)) => panic!("hover error: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // completion
    // -----------------------------------------------------------------------

    #[test]
    fn completion_includes_keywords() {
        let b = bridge();
        let (ast_box, _) = b.parse("(a b)").expect("parse");
        let pos = Position { line: 0, character: 0 };
        let items = b.completion(ast_box.as_ref(), pos)
            .expect("Some").expect("Ok");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"define"), "define in {:?}", labels);
        assert!(labels.contains(&"if"),     "if in {:?}", labels);
        assert!(labels.contains(&"let"),    "let in {:?}", labels);
    }

    #[test]
    fn completion_keyword_items_have_keyword_kind() {
        let b = bridge();
        let (ast_box, _) = b.parse("(x)").expect("parse");
        let pos = Position { line: 0, character: 0 };
        let items = b.completion(ast_box.as_ref(), pos)
            .expect("Some").expect("Ok");
        let kw: Vec<&CompletionItem> = items.iter()
            .filter(|i| i.kind == Some(CompletionItemKind::Keyword))
            .collect();
        assert!(!kw.is_empty(), "keyword items present");
    }

    // -----------------------------------------------------------------------
    // format
    // -----------------------------------------------------------------------

    #[test]
    fn format_returns_none_when_no_format_fn() {
        let b = bridge();
        assert!(b.format("(hello 42)").is_none());
    }

    #[test]
    fn format_delegates_to_format_fn() {
        static FMT_SPEC: LanguageSpec = LanguageSpec {
            name:              "toy-fmt",
            file_extensions:   &["toy"],
            tokens_source:     TOY_TOKENS,
            grammar_source:    TOY_GRAMMAR,
            token_kind_map:    &[],
            declaration_rules: &[],
            keyword_names:     &[],
            format_fn:         Some(|src| Ok(src.trim().to_string())),
            symbol_table_fn:   None,
        };
        let b = GrammarLanguageBridge::new(&FMT_SPEC);
        let edits = b.format("  (hello)  ").expect("Some").expect("Ok");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "(hello)");
    }

    // -----------------------------------------------------------------------
    // capability flags
    // -----------------------------------------------------------------------

    #[test]
    fn capability_flags_correct() {
        let b = bridge();
        assert!(b.supports_semantic_tokens());
        assert!(b.supports_document_symbols());
        assert!(b.supports_folding_ranges());
        assert!(b.supports_hover());
        assert!(b.supports_completion());
        assert!(!b.supports_format(), "no format_fn in TOY_SPEC");
    }

    // -----------------------------------------------------------------------
    // LspSemanticTokenType::as_str
    // -----------------------------------------------------------------------

    #[test]
    fn lsp_token_type_as_str() {
        use LspSemanticTokenType::*;
        assert_eq!(Keyword.as_str(),   "keyword");
        assert_eq!(Number.as_str(),    "number");
        assert_eq!(Variable.as_str(),  "variable");
        assert_eq!(Function.as_str(),  "function");
        assert_eq!(Parameter.as_str(), "parameter");
        assert_eq!(String.as_str(),    "string");
        assert_eq!(Comment.as_str(),   "comment");
        assert_eq!(Type.as_str(),      "type");
        assert_eq!(Property.as_str(),  "property");
        assert_eq!(Operator.as_str(),  "operator");
    }
}
