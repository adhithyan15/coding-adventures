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
//! # How it works
//!
//! 1. A `.grammar` file defines the language's syntax in EBNF notation:
//!
//!    ```text
//!    program    = { statement } ;
//!    statement  = assignment | expression ;
//!    assignment = NAME EQUALS expression ;
//!    expression = term { PLUS term } ;
//!    term       = NUMBER | NAME ;
//!    ```
//!
//! 2. The `grammar_tools` crate parses this file into a `ParserGrammar` —
//!    a data structure of `GrammarElement` nodes (Sequence, Alternation,
//!    Repetition, Optional, RuleReference, TokenReference, Literal).
//!
//! 3. This module's `GrammarParser` walks the `GrammarElement` tree while
//!    consuming tokens. Each grammar element type has a matching strategy:
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
//!
//! 4. The result is a generic AST made of `GrammarASTNode` nodes, where
//!    each node carries the rule name and a list of children (which can
//!    be either more AST nodes or raw tokens).
//!
//! # Backtracking
//!
//! Grammar-driven parsing needs backtracking: when an alternative fails,
//! we restore the token position and try the next one. This is implemented
//! by saving `self.pos` before each attempt and restoring it on failure.
//!
//! Backtracking is simple but potentially slow for ambiguous grammars. For
//! our educational purposes, it works perfectly.
//!
//! # Comparison with the hand-written parser
//!
//! | Aspect          | Hand-written          | Grammar-driven           |
//! |-----------------|-----------------------|--------------------------|
//! | Flexibility     | One language only     | Any language via grammar  |
//! | AST type        | Typed (`ASTNode` enum)| Generic (`GrammarASTNode`)|
//! | Error messages  | Very specific         | More generic             |
//! | Performance     | No backtracking       | Backtracks on alternation|
//! | Maintenance     | Change Rust code      | Change `.grammar` file   |

use lexer::token::{Token, TokenType, string_to_token_type};
use grammar_tools::parser_grammar::{GrammarElement, ParserGrammar, GrammarRule};
use std::collections::HashMap;
use std::fmt;

// ===========================================================================
// AST types for grammar-driven parsing
// ===========================================================================

/// A child of a grammar AST node — either a nested node or a raw token.
///
/// This enum is the grammar-driven parser's equivalent of Go's
/// `[]interface{}` (which can hold `*ASTNode` or `Token`). In Rust, we use
/// an enum to make this type-safe: every child is clearly labeled as either
/// a `Node` or a `Token`.
///
/// # Why not just use `ASTNode` from `crate::ast`?
///
/// The hand-written parser produces typed AST nodes (`Number`, `BinaryOp`,
/// etc.) because it knows the language it is parsing. The grammar-driven
/// parser does not know the language — it just knows grammar rules. So its
/// AST is generic: each node has a rule name and a list of children.
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
///
/// # Example
///
/// For the grammar rule `expression = term { PLUS term }` and the input
/// `1 + 2`, the AST would be:
///
/// ```text
/// GrammarASTNode {
///     rule_name: "expression",
///     children: [
///         Node(GrammarASTNode { rule_name: "term", children: [Token(NUMBER "1")] }),
///         Token(PLUS "+"),
///         Node(GrammarASTNode { rule_name: "term", children: [Token(NUMBER "2")] }),
///     ]
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GrammarASTNode {
    pub rule_name: String,
    pub children: Vec<ASTNodeOrToken>,
}

impl GrammarASTNode {
    /// Check if this node is a "leaf" — a node with exactly one child that
    /// is a raw token.
    ///
    /// Leaf nodes typically correspond to terminal grammar rules like
    /// `term = NUMBER ;`. They wrap a single token.
    pub fn is_leaf(&self) -> bool {
        if self.children.len() == 1 {
            matches!(&self.children[0], ASTNodeOrToken::Token(_))
        } else {
            false
        }
    }

    /// If this is a leaf node, return a reference to its token.
    ///
    /// Returns `None` if the node has multiple children or if its single
    /// child is a nested node rather than a token.
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
// Grammar parser
// ===========================================================================

/// A parser that uses a `ParserGrammar` (from a `.grammar` file) to parse
/// a token stream into a generic AST.
///
/// # Fields
///
/// - `tokens` — The token stream to parse.
/// - `grammar` — The grammar rules to follow.
/// - `pos` — Current position in the token stream.
/// - `rules` — Lookup table mapping rule names to their definitions.
///   Built from the grammar's rule list for O(1) access during parsing.
pub struct GrammarParser {
    tokens: Vec<Token>,
    grammar: ParserGrammar,
    pos: usize,
    rules: HashMap<String, GrammarRule>,
}

impl GrammarParser {
    /// Create a new grammar-driven parser.
    ///
    /// Builds an internal lookup table from rule names to rule definitions
    /// for efficient access during recursive descent.
    pub fn new(tokens: Vec<Token>, grammar: ParserGrammar) -> Self {
        let rules: HashMap<String, GrammarRule> = grammar
            .rules
            .iter()
            .map(|r| (r.name.clone(), r.clone()))
            .collect();

        GrammarParser {
            tokens,
            grammar,
            pos: 0,
            rules,
        }
    }

    /// Get the current token without consuming it.
    fn current(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    /// Parse the token stream according to the grammar.
    ///
    /// Uses the first rule in the grammar as the entry point (start symbol).
    /// After parsing, verifies that all tokens have been consumed (except
    /// trailing newlines and EOF).
    pub fn parse(&mut self) -> Result<GrammarASTNode, GrammarParseError> {
        if self.grammar.rules.is_empty() {
            return Err(GrammarParseError {
                message: "Grammar has no rules".to_string(),
                token: self.current().clone(),
            });
        }

        // The first rule is the entry point / start symbol.
        let entry_rule_name = self.grammar.rules[0].name.clone();
        let result = self.parse_rule(&entry_rule_name);

        match result {
            None => Err(GrammarParseError {
                message: "Failed to parse using grammar".to_string(),
                token: self.current().clone(),
            }),
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
                    return Err(GrammarParseError {
                        message: format!(
                            "Unexpected token: {:?}",
                            self.current().value
                        ),
                        token: self.current().clone(),
                    });
                }

                Ok(node)
            }
        }
    }

    /// Try to match a named grammar rule.
    ///
    /// Looks up the rule by name and attempts to match its body against
    /// the current token stream. Returns `Some(GrammarASTNode)` on success
    /// or `None` on failure.
    fn parse_rule(&mut self, rule_name: &str) -> Option<GrammarASTNode> {
        let rule = match self.rules.get(rule_name) {
            Some(r) => r.clone(),
            None => return None,
        };

        let children = self.match_element(&rule.body)?;

        Some(GrammarASTNode {
            rule_name: rule_name.to_string(),
            children,
        })
    }

    /// The core matching engine: try to match a grammar element against
    /// the token stream.
    ///
    /// This is where the magic happens. Each variant of `GrammarElement`
    /// has its own matching strategy:
    ///
    /// - **Sequence**: Match all sub-elements in order. If any fails,
    ///   backtrack to the starting position and fail.
    ///
    /// - **Alternation**: Try each choice in order. The first one that
    ///   succeeds wins. If all fail, backtrack and fail.
    ///
    /// - **Repetition**: Match the inner element zero or more times.
    ///   Always succeeds (zero matches is valid).
    ///
    /// - **Optional**: Try to match the inner element. If it fails,
    ///   succeed with an empty result (zero matches is valid).
    ///
    /// - **Group**: Just delegate to the inner element (grouping is
    ///   purely syntactic in the grammar).
    ///
    /// - **RuleReference**: Recursively parse the named rule. For token
    ///   references, match directly against the current token's type.
    ///
    /// - **Literal**: Match if the current token's value equals the literal.
    fn match_element(&mut self, element: &GrammarElement) -> Option<Vec<ASTNodeOrToken>> {
        let save_pos = self.pos;

        match element {
            // ----- Sequence: match all children in order (AND) -----
            //
            // A sequence like `NAME EQUALS expression` requires all three
            // elements to match consecutively. If the second element fails,
            // we undo all progress (backtrack) and report failure.
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

            // ----- Alternation: try each choice until one works (OR) -----
            //
            // An alternation like `assignment | expression` tries the first
            // option. If it fails, we backtrack and try the second. The
            // first match wins (ordered choice).
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

            // ----- Repetition: zero or more matches (Kleene star) -----
            //
            // `{ statement }` matches statements until it cannot match any
            // more. Zero matches is perfectly valid — an empty program has
            // no statements.
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

            // ----- Optional: zero or one match -----
            //
            // `[ ELSE block ]` matches the ELSE clause if present, or
            // succeeds with nothing if it is absent.
            GrammarElement::Optional { element: inner } => {
                match self.match_element(inner) {
                    Some(result) => Some(result),
                    None => Some(Vec::new()),
                }
            }

            // ----- Group: just delegate (parentheses in EBNF) -----
            GrammarElement::Group { element: inner } => {
                self.match_element(inner)
            }

            // ----- RuleReference: either a rule name or a token type -----
            //
            // UPPERCASE names (like `NUMBER`, `PLUS`) are token references.
            // Lowercase names (like `expression`, `statement`) are rule
            // references that trigger recursive parsing.
            GrammarElement::RuleReference { name } => {
                // Is this an uppercase token reference?
                let is_token = name.chars().all(|c| c.is_uppercase() || c == '_');

                if is_token {
                    // Skip newlines unless we are specifically looking for NEWLINE.
                    while self.current().type_ == TokenType::Newline && name != "NEWLINE" {
                        self.pos += 1;
                    }

                    let expected_type = string_to_token_type(name);
                    if self.current().type_ == expected_type {
                        let tok = self.current().clone();
                        self.pos += 1;
                        Some(vec![ASTNodeOrToken::Token(tok)])
                    } else {
                        None
                    }
                } else {
                    // Lowercase: recursively parse the named rule.
                    match self.parse_rule(name) {
                        Some(node) => Some(vec![ASTNodeOrToken::Node(node)]),
                        None => {
                            self.pos = save_pos;
                            None
                        }
                    }
                }
            }

            // ----- TokenReference: match by token type name -----
            GrammarElement::TokenReference { name } => {
                // Skip newlines unless specifically matching NEWLINE.
                while self.current().type_ == TokenType::Newline && name != "NEWLINE" {
                    self.pos += 1;
                }

                let expected_type = string_to_token_type(name);
                if self.current().type_ == expected_type {
                    let tok = self.current().clone();
                    self.pos += 1;
                    Some(vec![ASTNodeOrToken::Token(tok)])
                } else {
                    None
                }
            }

            // ----- Literal: match by exact token value -----
            //
            // A literal like `"+"` in the grammar matches a token whose
            // value is exactly "+".
            GrammarElement::Literal { value } => {
                if self.current().value == *value {
                    let tok = self.current().clone();
                    self.pos += 1;
                    Some(vec![ASTNodeOrToken::Token(tok)])
                } else {
                    None
                }
            }
        }
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
        }
    }

    /// Build a simple test grammar:
    ///
    /// ```text
    /// expression = term { PLUS term } ;
    /// term       = NUMBER ;
    /// ```
    ///
    /// This grammar recognizes expressions like `1`, `1 + 2`, `1 + 2 + 3`.
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

    /// Test parsing a single number with the grammar-driven parser.
    ///
    /// Input: `42`
    /// Grammar: expression = term { PLUS term } ; term = NUMBER ;
    /// Expected: expression(term(42))
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
        // Should have one child: a "term" node
        assert_eq!(result.children.len(), 1);
        match &result.children[0] {
            ASTNodeOrToken::Node(term) => {
                assert_eq!(term.rule_name, "term");
                assert!(term.is_leaf());
                assert_eq!(term.token().unwrap().value, "42");
            }
            other => panic!("Expected Node, got {:?}", other),
        }
    }

    /// Test parsing addition: `1 + 2`.
    ///
    /// Expected structure:
    /// expression(term(1), PLUS, term(2))
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
        // Should have: term, PLUS token, term = 3 children
        assert_eq!(result.children.len(), 3);

        // First child: term node with "1"
        match &result.children[0] {
            ASTNodeOrToken::Node(term) => {
                assert_eq!(term.rule_name, "term");
                assert_eq!(term.token().unwrap().value, "1");
            }
            other => panic!("Expected Node for first term, got {:?}", other),
        }

        // Second child: PLUS token
        match &result.children[1] {
            ASTNodeOrToken::Token(tok) => {
                assert_eq!(tok.type_, TokenType::Plus);
            }
            other => panic!("Expected Token for PLUS, got {:?}", other),
        }

        // Third child: term node with "2"
        match &result.children[2] {
            ASTNodeOrToken::Node(term) => {
                assert_eq!(term.rule_name, "term");
                assert_eq!(term.token().unwrap().value, "2");
            }
            other => panic!("Expected Node for second term, got {:?}", other),
        }
    }

    /// Test chained addition: `1 + 2 + 3`.
    ///
    /// The repetition `{ PLUS term }` should match twice, producing:
    /// expression(term(1), PLUS, term(2), PLUS, term(3))
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

        assert_eq!(result.rule_name, "expression");
        // term, PLUS, term, PLUS, term = 5 children
        assert_eq!(result.children.len(), 5);
    }

    /// Test that an empty grammar produces an error.
    #[test]
    fn test_grammar_parse_empty_grammar() {
        let tokens = vec![tok(TokenType::Eof, "")];
        let grammar = ParserGrammar { rules: vec![] };
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse();

        assert!(result.is_err());
    }

    /// Test parsing with alternation.
    ///
    /// Grammar:
    /// ```text
    /// value = NUMBER | NAME ;
    /// ```
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

        // Test with a number
        let tokens = vec![
            tok(TokenType::Number, "42"),
            tok(TokenType::Eof, ""),
        ];
        let mut parser = GrammarParser::new(tokens, grammar.clone());
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "value");

        // Test with a name
        let tokens = vec![
            tok(TokenType::Name, "x"),
            tok(TokenType::Eof, ""),
        ];
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "value");
    }

    /// Test optional element matching.
    ///
    /// Grammar:
    /// ```text
    /// maybe_number = [ NUMBER ] ;
    /// ```
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

        // With a number present
        let tokens = vec![
            tok(TokenType::Number, "42"),
            tok(TokenType::Eof, ""),
        ];
        let mut parser = GrammarParser::new(tokens, grammar.clone());
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 1);

        // Without a number (just EOF)
        let tokens = vec![tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 0);
    }

    /// Test literal matching.
    ///
    /// Grammar:
    /// ```text
    /// greeting = "hello" ;
    /// ```
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

        let tokens = vec![
            tok(TokenType::Name, "hello"),
            tok(TokenType::Eof, ""),
        ];
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "greeting");
        assert_eq!(result.children.len(), 1);
    }

    /// Test that the leaf/token helper methods work correctly.
    #[test]
    fn test_ast_node_helpers() {
        // A leaf node (single token child)
        let leaf = GrammarASTNode {
            rule_name: "number".to_string(),
            children: vec![ASTNodeOrToken::Token(tok(TokenType::Number, "42"))],
        };
        assert!(leaf.is_leaf());
        assert_eq!(leaf.token().unwrap().value, "42");

        // A non-leaf node (nested node child)
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

    /// Test parsing with a group element.
    ///
    /// Grammar:
    /// ```text
    /// expr = NUMBER ( PLUS | MINUS ) NUMBER ;
    /// ```
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

        // Test with plus
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];
        let mut parser = GrammarParser::new(tokens, grammar.clone());
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 3);

        // Test with minus
        let tokens = vec![
            tok(TokenType::Number, "5"),
            tok(TokenType::Minus, "-"),
            tok(TokenType::Number, "3"),
            tok(TokenType::Eof, ""),
        ];
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 3);
    }

    /// Integration test: use the grammar-driven parser with real lexer output.
    #[test]
    fn test_grammar_parser_with_lexer() {
        let source = "1 + 2";
        let mut lexer = lexer::tokenizer::Lexer::new(source, None);
        let tokens = lexer.tokenize().unwrap();

        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();

        assert_eq!(result.rule_name, "expression");
        assert_eq!(result.children.len(), 3); // term, PLUS, term
    }
}
