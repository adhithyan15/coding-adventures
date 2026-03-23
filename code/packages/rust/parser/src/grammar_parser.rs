//! # Grammar-driven parser — parsing any language from a `.grammar` file.
//!
//! The hand-written parser in [`crate::parser`] can only parse one language:
//! our Python subset. If we wanted to parse JavaScript, Ruby, or any other
//! language, we would need to write another parser from scratch.
//!
//! This module takes a different approach: **grammar-driven parsing**. Instead
//! of hard-coding grammar rules as Rust functions, we read the rules from a
//! `.grammar` file (parsed by the `grammar_tools` crate) and use them to
//! drive the parse at runtime.
//!
//! # Extensions for Starlark-like languages
//!
//! This parser supports several extensions beyond basic EBNF interpretation:
//!
//! - **Packrat memoization**: Caches parse results for each (rule, position) pair,
//!   avoiding exponential backtracking. Essential for grammars with ~40 rules.
//!
//! - **Significant newlines**: If the grammar references NEWLINE tokens, they are
//!   treated as significant (not auto-skipped). Otherwise, NEWLINEs are transparent.
//!
//! - **Furthest failure tracking**: When parsing fails, the error message reports
//!   what was expected at the furthest position reached, not just the first failure.
//!
//! - **String-based token matching**: Tokens with a `type_name` field are matched
//!   by their string name, allowing grammars with custom token types beyond the
//!   fixed `TokenType` enum.
//!
//! # How it works
//!
//! 1. A `.grammar` file defines the language's syntax in EBNF notation.
//! 2. The `grammar_tools` crate parses this file into a `ParserGrammar`.
//! 3. This module's `GrammarParser` walks the grammar rule tree while
//!    consuming tokens. Each EBNF element type has a natural interpretation:
//!
//!    | Element       | Strategy                                    |
//!    |---------------|---------------------------------------------|
//!    | Sequence      | Match all children in order (AND)           |
//!    | Alternation   | Try each choice until one matches (OR)      |
//!    | Repetition    | Match zero or more times (loop)             |
//!    | Optional      | Match zero or one time                      |
//!    | Group         | Delegate to inner element                   |
//!    | RuleReference | Recursively parse the named rule            |
//!    | TokenReference| Match if current token has the right type   |
//!    | Literal       | Match if current token has the right value  |

use lexer::token::{Token, TokenType, string_to_token_type};
use grammar_tools::parser_grammar::{GrammarElement, ParserGrammar, GrammarRule};
use std::collections::HashMap;
use std::fmt;

// ===========================================================================
// AST types for grammar-driven parsing
// ===========================================================================

/// A child of a grammar AST node — either a nested node or a raw token.
#[derive(Debug, Clone, PartialEq)]
pub enum ASTNodeOrToken {
    /// A nested AST node produced by matching a grammar rule.
    Node(GrammarASTNode),
    /// A raw token that was matched directly (token reference or literal).
    Token(Token),
}

/// A node in the grammar-driven AST.
///
/// Each node corresponds to a successfully matched grammar rule. The
/// `rule_name` says which rule matched, and `children` contains the
/// sub-matches (either deeper rule matches or individual tokens).
#[derive(Debug, Clone, PartialEq)]
pub struct GrammarASTNode {
    pub rule_name: String,
    pub children: Vec<ASTNodeOrToken>,
}

impl GrammarASTNode {
    /// Check if this node is a "leaf" — a node with exactly one child that
    /// is a raw token.
    pub fn is_leaf(&self) -> bool {
        if self.children.len() == 1 {
            matches!(&self.children[0], ASTNodeOrToken::Token(_))
        } else {
            false
        }
    }

    /// If this is a leaf node, return a reference to its token.
    pub fn token(&self) -> Option<&Token> {
        if self.is_leaf() {
            match &self.children[0] {
                ASTNodeOrToken::Token(tok) => Some(tok),
                _ => None,
            }
        } else {
            None
        }
    }
}

// ===========================================================================
// Error type
// ===========================================================================

/// An error encountered during grammar-driven parsing.
#[derive(Debug, Clone)]
pub struct GrammarParseError {
    pub message: String,
    pub token: Token,
}

impl fmt::Display for GrammarParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at {}:{}: {}",
            self.token.line, self.token.column, self.message
        )
    }
}

impl std::error::Error for GrammarParseError {}

// ===========================================================================
// Memo entry — packrat memoization cache entry
// ===========================================================================

/// A cached result from parsing a rule at a specific position.
///
/// Packrat memoization stores the result of every (rule, position) attempt
/// so that re-parsing the same rule at the same position is O(1). This is
/// essential for grammars with ~40 rules that would otherwise cause
/// exponential backtracking.
struct MemoEntry {
    /// The matched children, or None if the rule failed.
    children: Option<Vec<ASTNodeOrToken>>,
    /// The position after the match (or where we gave up).
    end_pos: usize,
    /// Whether the match succeeded.
    ok: bool,
}

// ===========================================================================
// Grammar parser
// ===========================================================================

/// A parser that uses a `ParserGrammar` (from a `.grammar` file) to parse
/// a token stream into a generic AST.
///
/// Includes packrat memoization, significant newline detection, and
/// furthest failure tracking for better error messages.
pub struct GrammarParser {
    tokens: Vec<Token>,
    grammar: ParserGrammar,
    pos: usize,
    rules: HashMap<String, GrammarRule>,

    /// Index of each rule name for memo key generation.
    rule_index: HashMap<String, usize>,

    /// Whether newlines are significant in this grammar.
    newlines_significant: bool,

    /// Packrat memoization cache: "rule_idx,pos" -> MemoEntry.
    memo: HashMap<String, MemoEntry>,

    /// Furthest position reached during parsing.
    furthest_pos: usize,

    /// What was expected at the furthest position.
    furthest_expected: Vec<String>,

    /// When true, emit a `[TRACE]` line to stderr for every rule attempt.
    ///
    /// Trace mode is invaluable when debugging why a grammar does not match
    /// a particular input. Instead of reading the grammar rules and mentally
    /// simulating the parse, you can see exactly which rule was attempted at
    /// which token position and whether it succeeded or failed.
    ///
    /// Example output:
    /// ```text
    /// [TRACE] rule 'expression' at token 0 (Name "x") → match
    /// [TRACE] rule 'term' at token 0 (Name "x") → match
    /// [TRACE] rule 'factor' at token 2 (Plus "+") → fail
    /// ```
    trace: bool,

    /// Set of (rule_index, pos) pairs currently being parsed.
    /// Used to detect and break left recursion: if we try to parse a rule
    /// at a position where we're already inside that same rule (but haven't
    /// cached the result yet), we know it's left recursion and should fail.
    in_progress: std::collections::HashSet<String>,
}

impl GrammarParser {
    /// Create a new grammar-driven parser (trace disabled).
    pub fn new(tokens: Vec<Token>, grammar: ParserGrammar) -> Self {
        Self::new_with_trace(tokens, grammar, false)
    }

    /// Create a new grammar-driven parser with optional trace mode.
    ///
    /// When `trace` is `true`, every rule attempt emits a `[TRACE]` line to
    /// stderr showing the rule name, the token position, the current token
    /// type and value, and whether the rule matched or failed.
    ///
    /// This is intended for debugging grammar issues. Keep it off in
    /// production because the output can be voluminous.
    ///
    /// # Format
    ///
    /// ```text
    /// [TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match
    /// [TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → fail
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// use parser::grammar_parser::GrammarParser;
    /// use grammar_tools::parser_grammar::parse_parser_grammar;
    /// use lexer::token::{Token, TokenType};
    ///
    /// let grammar = parse_parser_grammar("value = NUMBER ;").unwrap();
    /// let tokens = vec![
    ///     Token { type_: TokenType::Number, value: "42".into(), line: 1, column: 1, type_name: None },
    ///     Token { type_: TokenType::Eof,    value: "".into(),   line: 1, column: 3, type_name: None },
    /// ];
    /// let mut parser = GrammarParser::new_with_trace(tokens, grammar, true);
    /// let result = parser.parse();
    /// assert!(result.is_ok());
    /// ```
    pub fn new_with_trace(tokens: Vec<Token>, grammar: ParserGrammar, trace: bool) -> Self {
        let mut rules = HashMap::new();
        let mut rule_index = HashMap::new();

        for (i, rule) in grammar.rules.iter().enumerate() {
            rules.insert(rule.name.clone(), rule.clone());
            rule_index.insert(rule.name.clone(), i);
        }

        let newlines_significant = grammar_references_newline(&grammar);

        GrammarParser {
            tokens,
            grammar,
            pos: 0,
            rules,
            rule_index,
            newlines_significant,
            memo: HashMap::new(),
            furthest_pos: 0,
            furthest_expected: Vec::new(),
            trace,
            in_progress: std::collections::HashSet::new(),
        }
    }

    /// Whether newlines are treated as significant tokens in this grammar.
    pub fn is_newlines_significant(&self) -> bool {
        self.newlines_significant
    }

    /// Get the current token without consuming it.
    fn current(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    /// Record a failed expectation at the current position for error reporting.
    fn record_failure(&mut self, expected: &str) {
        if self.pos > self.furthest_pos {
            self.furthest_pos = self.pos;
            self.furthest_expected = vec![expected.to_string()];
        } else if self.pos == self.furthest_pos {
            if !self.furthest_expected.contains(&expected.to_string()) {
                self.furthest_expected.push(expected.to_string());
            }
        }
    }

    /// Parse the token stream according to the grammar.
    ///
    /// Uses the first rule in the grammar as the entry point (start symbol).
    pub fn parse(&mut self) -> Result<GrammarASTNode, GrammarParseError> {
        if self.grammar.rules.is_empty() {
            return Err(GrammarParseError {
                message: "Grammar has no rules".to_string(),
                token: self.current().clone(),
            });
        }

        let entry_rule_name = self.grammar.rules[0].name.clone();
        let result = self.parse_rule(&entry_rule_name);

        match result {
            None => {
                let tok = self.current().clone();
                if !self.furthest_expected.is_empty() {
                    let expected = self.furthest_expected.join(" or ");
                    let furthest_tok = if self.furthest_pos < self.tokens.len() {
                        self.tokens[self.furthest_pos].clone()
                    } else {
                        tok.clone()
                    };
                    Err(GrammarParseError {
                        message: format!(
                            "Expected {}, got {:?}",
                            expected, furthest_tok.value
                        ),
                        token: furthest_tok,
                    })
                } else {
                    Err(GrammarParseError {
                        message: "Failed to parse using grammar".to_string(),
                        token: tok,
                    })
                }
            }
            Some(node) => {
                // Skip trailing newlines.
                while self.pos < self.tokens.len()
                    && self.current().type_ == TokenType::Newline
                {
                    self.pos += 1;
                }

                // Check that we consumed all tokens.
                if self.pos < self.tokens.len()
                    && self.current().type_ != TokenType::Eof
                {
                    let tok = self.current().clone();
                    if !self.furthest_expected.is_empty() && self.furthest_pos > self.pos {
                        let expected = self.furthest_expected.join(" or ");
                        let furthest_tok = if self.furthest_pos < self.tokens.len() {
                            self.tokens[self.furthest_pos].clone()
                        } else {
                            tok.clone()
                        };
                        return Err(GrammarParseError {
                            message: format!(
                                "Expected {}, got {:?}",
                                expected, furthest_tok.value
                            ),
                            token: furthest_tok,
                        });
                    }
                    return Err(GrammarParseError {
                        message: format!(
                            "Unexpected token: {:?}",
                            tok.value
                        ),
                        token: tok,
                    });
                }

                Ok(node)
            }
        }
    }

    // =========================================================================
    // Rule parsing (with packrat memoization)
    // =========================================================================

    /// Try to match a named grammar rule with memoization.
    fn parse_rule(&mut self, rule_name: &str) -> Option<GrammarASTNode> {
        let rule = match self.rules.get(rule_name) {
            Some(r) => r.clone(),
            None => return None,
        };

        // Check memo cache.
        if let Some(&idx) = self.rule_index.get(rule_name) {
            let key = format!("{},{}", idx, self.pos);
            if let Some(entry) = self.memo.get(&key) {
                let end_pos = entry.end_pos;
                let ok = entry.ok;
                let children = entry.children.clone();
                self.pos = end_pos;
                if !ok {
                    return None;
                }
                return Some(GrammarASTNode {
                    rule_name: rule_name.to_string(),
                    children: children.unwrap(),
                });
            }

            // Left-recursion guard: if we're already trying to parse this
            // rule at this position (but haven't finished and cached the
            // result yet), then we've hit left recursion. Return None to
            // break the cycle. This handles grammars with rules like:
            //   primary = ... | primary LBRACKET expression RBRACKET
            // where `primary` appears as the first element of an alternative.
            if !self.in_progress.insert(key.clone()) {
                // key was already present — left recursion detected
                return None;
            }
        }

        let start_pos = self.pos;

        // Capture trace info BEFORE mutating self.pos via match_element.
        // We snapshot the token at start_pos so the trace line shows the
        // token the rule is attempting to match at the moment of the attempt.
        let trace_token_info = if self.trace {
            let tok = if start_pos < self.tokens.len() {
                &self.tokens[start_pos]
            } else {
                &self.tokens[self.tokens.len() - 1]
            };
            // Prefer the string type_name (grammar-driven tokens like "IDENT",
            // "NUMBER") over the enum variant name for readability.
            let type_label = if let Some(ref tn) = tok.type_name {
                tn.clone()
            } else {
                format!("{}", tok.type_)
            };
            Some((start_pos, type_label, tok.value.clone()))
        } else {
            None
        };

        let children = self.match_element(&rule.body);

        // Emit [TRACE] line to stderr now that we know success/failure.
        // The arrow character → (U+2192) mirrors the task spec exactly.
        if let Some((idx, type_label, value)) = trace_token_info {
            let outcome = if children.is_some() { "match" } else { "fail" };
            eprintln!(
                "[TRACE] rule '{}' at token {} ({} \"{}\") \u{2192} {}",
                rule_name, idx, type_label, value, outcome
            );
        }

        // Cache result and remove from in_progress set.
        if let Some(&idx) = self.rule_index.get(rule_name) {
            let key = format!("{},{}", idx, start_pos);
            self.in_progress.remove(&key);
            if let Some(ref result) = children {
                self.memo.insert(key, MemoEntry {
                    children: Some(result.clone()),
                    end_pos: self.pos,
                    ok: true,
                });
            } else {
                self.memo.insert(key, MemoEntry {
                    children: None,
                    end_pos: self.pos,
                    ok: false,
                });
            }
        }

        match children {
            Some(c) => Some(GrammarASTNode {
                rule_name: rule_name.to_string(),
                children: c,
            }),
            None => {
                self.pos = start_pos;
                self.record_failure(rule_name);
                None
            }
        }
    }

    // =========================================================================
    // Element matching
    // =========================================================================

    fn match_element(&mut self, element: &GrammarElement) -> Option<Vec<ASTNodeOrToken>> {
        let save_pos = self.pos;

        match element {
            GrammarElement::Sequence { elements } => {
                let mut children = Vec::new();
                for sub in elements {
                    match self.match_element(sub) {
                        Some(mut result) => children.append(&mut result),
                        None => {
                            self.pos = save_pos;
                            return None;
                        }
                    }
                }
                Some(children)
            }

            GrammarElement::Alternation { choices } => {
                for choice in choices {
                    self.pos = save_pos;
                    if let Some(result) = self.match_element(choice) {
                        return Some(result);
                    }
                }
                self.pos = save_pos;
                None
            }

            GrammarElement::Repetition { element: inner } => {
                let mut children = Vec::new();
                loop {
                    let save_rep = self.pos;
                    match self.match_element(inner) {
                        Some(mut result) => children.append(&mut result),
                        None => {
                            self.pos = save_rep;
                            break;
                        }
                    }
                }
                Some(children)
            }

            GrammarElement::Optional { element: inner } => {
                match self.match_element(inner) {
                    Some(result) => Some(result),
                    None => Some(Vec::new()),
                }
            }

            GrammarElement::Group { element: inner } => {
                self.match_element(inner)
            }

            GrammarElement::RuleReference { name } => {
                // Is this an uppercase token reference?
                let is_token = name.chars().all(|c| c.is_uppercase() || c == '_');

                if is_token {
                    self.match_token_reference(name)
                } else {
                    match self.parse_rule(name) {
                        Some(node) => Some(vec![ASTNodeOrToken::Node(node)]),
                        None => {
                            self.pos = save_pos;
                            None
                        }
                    }
                }
            }

            GrammarElement::TokenReference { name } => {
                self.match_token_reference(name)
            }

            GrammarElement::Literal { value } => {
                // Skip insignificant newlines before literal matching.
                if !self.newlines_significant {
                    while self.current().type_ == TokenType::Newline {
                        self.pos += 1;
                    }
                }

                if self.current().value == *value {
                    let tok = self.current().clone();
                    self.pos += 1;
                    Some(vec![ASTNodeOrToken::Token(tok)])
                } else {
                    self.record_failure(&format!("\"{}\"", value));
                    None
                }
            }
        }
    }

    // =========================================================================
    // Token reference matching
    // =========================================================================

    /// Match a token reference, handling string-based type names and
    /// newline skipping.
    fn match_token_reference(&mut self, expected_type: &str) -> Option<Vec<ASTNodeOrToken>> {
        // Skip newlines when matching non-NEWLINE tokens (if insignificant).
        if !self.newlines_significant && expected_type != "NEWLINE" {
            while self.current().type_ == TokenType::Newline {
                self.pos += 1;
            }
        }

        let token = self.current();

        // First, check string-based type_name for custom token types.
        if let Some(ref type_name) = token.type_name {
            if type_name == expected_type {
                let tok = token.clone();
                self.pos += 1;
                return Some(vec![ASTNodeOrToken::Token(tok)]);
            }
        }

        // Fall back to enum-based matching.
        let expected = string_to_token_type(expected_type);

        // If the expected type maps to `Name` but is not literally "NAME", it
        // is a custom grammar-defined token type (e.g. AT_KEYWORD, VARIABLE,
        // FUNCTION, IDENT). In that case we must NOT match a token that already
        // has a *different* type_name set — e.g. an IDENT token must not match
        // a VARIABLE reference just because both have TokenType::Name.
        //
        // A token with type_name = None and type_ = Name is a "bare" name token
        // (e.g. a keyword or identifier produced by a grammar that didn't assign
        // a named type), and we allow it to match any custom Name-based type.
        if expected == TokenType::Name && expected_type != "NAME" {
            if token.type_name.is_some() {
                // Token has a specific custom type that didn't match above.
                self.record_failure(expected_type);
                return None;
            }
        }

        if token.type_ == expected {
            let tok = token.clone();
            self.pos += 1;
            Some(vec![ASTNodeOrToken::Token(tok)])
        } else {
            self.record_failure(expected_type);
            None
        }
    }
}

// ===========================================================================
// Newline detection — scan grammar for NEWLINE references
// ===========================================================================

/// Check if any rule in the grammar references the NEWLINE token.
fn grammar_references_newline(grammar: &ParserGrammar) -> bool {
    grammar.rules.iter().any(|rule| element_references_newline(&rule.body))
}

/// Recursively check if a grammar element references NEWLINE.
fn element_references_newline(element: &GrammarElement) -> bool {
    match element {
        GrammarElement::TokenReference { name } => name == "NEWLINE",
        GrammarElement::RuleReference { name } => name == "NEWLINE",
        GrammarElement::Sequence { elements } => {
            elements.iter().any(|e| element_references_newline(e))
        }
        GrammarElement::Alternation { choices } => {
            choices.iter().any(|c| element_references_newline(c))
        }
        GrammarElement::Repetition { element: inner }
        | GrammarElement::Optional { element: inner }
        | GrammarElement::Group { element: inner } => {
            element_references_newline(inner)
        }
        GrammarElement::Literal { .. } => false,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    /// Helper: create a token with default position.
    fn tok(type_: TokenType, value: &str) -> Token {
        Token {
            type_,
            value: value.to_string(),
            line: 1,
            column: 1,
            type_name: None,
        }
    }

    /// Helper: create a token with a string type name.
    fn tok_named(type_: TokenType, value: &str, type_name: &str) -> Token {
        Token {
            type_,
            value: value.to_string(),
            line: 1,
            column: 1,
            type_name: Some(type_name.to_string()),
        }
    }

    /// Build a simple test grammar:
    ///
    /// ```text
    /// expression = term { PLUS term } ;
    /// term       = NUMBER ;
    /// ```
    fn simple_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
                GrammarRule {
                    name: "expression".to_string(),
                    body: GrammarElement::Sequence {
                        elements: vec![
                            GrammarElement::RuleReference { name: "term".to_string() },
                            GrammarElement::Repetition {
                                element: Box::new(GrammarElement::Sequence {
                                    elements: vec![
                                        GrammarElement::TokenReference { name: "PLUS".to_string() },
                                        GrammarElement::RuleReference { name: "term".to_string() },
                                    ],
                                }),
                            },
                        ],
                    },
                    line_number: 1,
                },
                GrammarRule {
                    name: "term".to_string(),
                    body: GrammarElement::TokenReference { name: "NUMBER".to_string() },
                    line_number: 2,
                },
            ],
        }
    }

    // -----------------------------------------------------------------------
    // Basic parsing
    // -----------------------------------------------------------------------

    #[test]
    fn test_grammar_parse_single_number() {
        let tokens = vec![
            tok(TokenType::Number, "42"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expression");
        assert_eq!(result.children.len(), 1);
    }

    #[test]
    fn test_grammar_parse_addition() {
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expression");
        assert_eq!(result.children.len(), 3);
    }

    #[test]
    fn test_grammar_parse_chained_addition() {
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "3"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 5);
    }

    #[test]
    fn test_grammar_parse_empty_grammar() {
        let tokens = vec![tok(TokenType::Eof, "")];
        let grammar = ParserGrammar { rules: vec![] };
        let mut parser = GrammarParser::new(tokens, grammar);
        assert!(parser.parse().is_err());
    }

    // -----------------------------------------------------------------------
    // Alternation, Optional, Literal, Group
    // -----------------------------------------------------------------------

    #[test]
    fn test_grammar_parse_alternation() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "value".to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference { name: "NUMBER".to_string() },
                        GrammarElement::TokenReference { name: "NAME".to_string() },
                    ],
                },
                line_number: 1,
            }],
        };

        let tokens = vec![tok(TokenType::Number, "42"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar.clone());
        assert!(parser.parse().is_ok());

        let tokens = vec![tok(TokenType::Name, "x"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar);
        assert!(parser.parse().is_ok());
    }

    #[test]
    fn test_grammar_parse_optional() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "maybe_number".to_string(),
                body: GrammarElement::Optional {
                    element: Box::new(GrammarElement::TokenReference {
                        name: "NUMBER".to_string(),
                    }),
                },
                line_number: 1,
            }],
        };

        let tokens = vec![tok(TokenType::Number, "42"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar.clone());
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 1);

        let tokens = vec![tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 0);
    }

    #[test]
    fn test_grammar_parse_literal() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "greeting".to_string(),
                body: GrammarElement::Literal {
                    value: "hello".to_string(),
                },
                line_number: 1,
            }],
        };
        let tokens = vec![tok(TokenType::Name, "hello"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "greeting");
    }

    #[test]
    fn test_grammar_parse_group() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "expr".to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference { name: "NUMBER".to_string() },
                        GrammarElement::Group {
                            element: Box::new(GrammarElement::Alternation {
                                choices: vec![
                                    GrammarElement::TokenReference { name: "PLUS".to_string() },
                                    GrammarElement::TokenReference { name: "MINUS".to_string() },
                                ],
                            }),
                        },
                        GrammarElement::TokenReference { name: "NUMBER".to_string() },
                    ],
                },
                line_number: 1,
            }],
        };

        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];
        let mut parser = GrammarParser::new(tokens, grammar);
        assert_eq!(parser.parse().unwrap().children.len(), 3);
    }

    // -----------------------------------------------------------------------
    // AST node helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_ast_node_helpers() {
        let leaf = GrammarASTNode {
            rule_name: "number".to_string(),
            children: vec![ASTNodeOrToken::Token(tok(TokenType::Number, "42"))],
        };
        assert!(leaf.is_leaf());
        assert_eq!(leaf.token().unwrap().value, "42");

        let non_leaf = GrammarASTNode {
            rule_name: "expr".to_string(),
            children: vec![
                ASTNodeOrToken::Node(leaf.clone()),
                ASTNodeOrToken::Token(tok(TokenType::Plus, "+")),
            ],
        };
        assert!(!non_leaf.is_leaf());
        assert!(non_leaf.token().is_none());
    }

    // -----------------------------------------------------------------------
    // Integration: parser with lexer output
    // -----------------------------------------------------------------------

    #[test]
    fn test_grammar_parser_with_lexer() {
        let source = "1 + 2";
        let mut lexer = lexer::tokenizer::Lexer::new(source, None);
        let tokens = lexer.tokenize().unwrap();
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expression");
        assert_eq!(result.children.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Packrat memoization
    // -----------------------------------------------------------------------

    #[test]
    fn test_packrat_memoization() {
        // Parse the same input twice to exercise memo cache.
        // The grammar has alternation that can cause re-parsing of the same
        // rule at the same position.
        let grammar = ParserGrammar {
            rules: vec![
                GrammarRule {
                    name: "start".to_string(),
                    body: GrammarElement::Alternation {
                        choices: vec![
                            // First alternative: NUMBER PLUS NUMBER
                            GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::RuleReference { name: "atom".to_string() },
                                    GrammarElement::TokenReference { name: "PLUS".to_string() },
                                    GrammarElement::RuleReference { name: "atom".to_string() },
                                ],
                            },
                            // Second alternative: just an atom
                            GrammarElement::RuleReference { name: "atom".to_string() },
                        ],
                    },
                    line_number: 1,
                },
                GrammarRule {
                    name: "atom".to_string(),
                    body: GrammarElement::TokenReference { name: "NUMBER".to_string() },
                    line_number: 2,
                },
            ],
        };

        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "start");
        // Memo should have been populated.
        assert!(!parser.memo.is_empty());
    }

    // -----------------------------------------------------------------------
    // String-based token types
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_token_types() {
        // Use custom token type names that don't map to TokenType variants.
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "expr".to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference { name: "INT".to_string() },
                        GrammarElement::TokenReference { name: "PLUS".to_string() },
                        GrammarElement::TokenReference { name: "INT".to_string() },
                    ],
                },
                line_number: 1,
            }],
        };

        let tokens = vec![
            tok_named(TokenType::Name, "1", "INT"),
            tok(TokenType::Plus, "+"),
            tok_named(TokenType::Name, "2", "INT"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expr");
        assert_eq!(result.children.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Significant newlines
    // -----------------------------------------------------------------------

    #[test]
    fn test_significant_newlines_detected() {
        // A grammar that references NEWLINE should be detected as
        // newlines-significant.
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "file".to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference { name: "NAME".to_string() },
                        GrammarElement::TokenReference { name: "NEWLINE".to_string() },
                    ],
                },
                line_number: 1,
            }],
        };

        let parser = GrammarParser::new(vec![], grammar);
        assert!(parser.is_newlines_significant());
    }

    #[test]
    fn test_insignificant_newlines_detected() {
        // A grammar without NEWLINE references should not be significant.
        let parser = GrammarParser::new(vec![], simple_grammar());
        assert!(!parser.is_newlines_significant());
    }

    // -----------------------------------------------------------------------
    // Furthest failure tracking
    // -----------------------------------------------------------------------

    #[test]
    fn test_furthest_failure_error_message() {
        // When parsing fails, the error should report what was expected
        // at the furthest position reached.
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Name, "x"),  // Invalid: expected PLUS or EOF
            tok(TokenType::Eof, ""),
        ];

        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let err = parser.parse().unwrap_err();
        // The error should mention what was expected.
        assert!(err.message.contains("Expected") || err.message.contains("Unexpected"));
    }

    // -----------------------------------------------------------------------
    // Starlark-like pipeline test
    // -----------------------------------------------------------------------

    #[test]
    fn test_starlark_pipeline() {
        // End-to-end: lex with grammar lexer, then parse with grammar parser.
        use grammar_tools::token_grammar::parse_token_grammar;
        use grammar_tools::parser_grammar::parse_parser_grammar;
        use lexer::grammar_lexer::GrammarLexer;

        let token_source = r#"
NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER = /[0-9]+/
EQUALS = "="
PLUS = "+"
"#;
        let grammar_source = r#"
program    = { statement } ;
statement  = assignment ;
assignment = NAME EQUALS expression ;
expression = term { PLUS term } ;
term       = NUMBER | NAME ;
"#;

        let token_grammar = parse_token_grammar(token_source).unwrap();
        let parser_grammar = parse_parser_grammar(grammar_source).unwrap();

        let tokens = GrammarLexer::new("x = 1 + 2", &token_grammar)
            .tokenize()
            .unwrap();

        let mut parser = GrammarParser::new(tokens, parser_grammar);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Trace mode
    // -----------------------------------------------------------------------

    #[test]
    fn test_trace_mode_parse_succeeds() {
        // new_with_trace(trace=true) must parse correctly — the same result
        // as new() with trace=false. Trace output goes to stderr so it does
        // not affect the return value.
        let tokens = vec![
            tok(TokenType::Number, "7"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new_with_trace(tokens, grammar, true);
        let result = parser.parse();
        assert!(result.is_ok(), "trace mode must not affect parse correctness");
        assert_eq!(result.unwrap().rule_name, "expression");
    }

    #[test]
    fn test_trace_mode_no_panic_on_failure() {
        // When the input does not match the grammar, trace mode must not panic.
        // The error is the same as without trace mode.
        let tokens = vec![
            tok(TokenType::Plus, "+"), // Does not match `NUMBER`
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new_with_trace(tokens, grammar, true);
        let result = parser.parse();
        assert!(result.is_err(), "invalid input should still produce an error in trace mode");
    }

    #[test]
    fn test_trace_mode_addition() {
        // Trace mode works correctly for a multi-token sequence.
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new_with_trace(tokens, grammar, true);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expression");
        // expression expands to: term + term = NUMBER "+" NUMBER
        // children: the NUMBER node, the Plus token, the NUMBER node.
        assert_eq!(result.children.len(), 3);
    }

    #[test]
    fn test_trace_false_same_as_new() {
        // new_with_trace(trace=false) is identical in behaviour to new().
        let tokens = vec![
            tok(TokenType::Number, "99"),
            tok(TokenType::Eof, ""),
        ];
        let g1 = simple_grammar();
        let g2 = simple_grammar();
        let mut p1 = GrammarParser::new(tokens.clone(), g1);
        let mut p2 = GrammarParser::new_with_trace(tokens, g2, false);
        let r1 = p1.parse().unwrap();
        let r2 = p2.parse().unwrap();
        assert_eq!(r1.rule_name, r2.rule_name);
        assert_eq!(r1.children.len(), r2.children.len());
    }
}
