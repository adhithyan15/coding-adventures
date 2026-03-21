//! # Starlark Parser — parsing Starlark source code into an AST.
//!
//! This crate is the second half of the Starlark front-end pipeline. Where
//! the `starlark-lexer` crate breaks source text into tokens, this crate
//! arranges those tokens into a tree that reflects the **structure** of the
//! code — an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing Starlark requires four cooperating components:
//!
//! ```text
//! Source code  ("def f(x):\n    return x + 1\n")
//!       |
//!       v
//! starlark-lexer        → Vec<Token>
//!       |                  [KEYWORD("def"), NAME("f"), LPAREN, NAME("x"),
//!       |                   RPAREN, COLON, NEWLINE, INDENT,
//!       |                   KEYWORD("return"), NAME("x"), PLUS, INT("1"),
//!       |                   NEWLINE, DEDENT, EOF]
//!       v
//! starlark.grammar      → ParserGrammar (rules like "def_stmt = ...")
//!       |
//!       v
//! GrammarParser         → GrammarASTNode tree
//!       |
//!       |                  file
//!       |                    └── statement
//!       |                          └── def_stmt
//!       |                                ├── KEYWORD("def")
//!       |                                ├── NAME("f")
//!       |                                ├── LPAREN
//!       |                                ├── parameters
//!       |                                │     └── NAME("x")
//!       |                                ├── RPAREN
//!       |                                ├── COLON
//!       |                                └── suite
//!       |                                      └── return_stmt
//!       v
//! [future stages: type checking, evaluation]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the `starlark.grammar` file and provides two
//! public entry points.
//!
//! # Grammar-driven parsing
//!
//! The `GrammarParser` is a **recursive descent parser with backtracking and
//! packrat memoization**. It reads grammar rules from the `.grammar` file
//! and interprets them at runtime:
//!
//! - **Sequence** (`a b c`): match all elements in order.
//! - **Alternation** (`a | b`): try each choice until one succeeds.
//! - **Repetition** (`{ a }`): match zero or more times.
//! - **Optional** (`[ a ]`): match zero or one time.
//! - **Literals** (`"def"`): match a token with exactly that value.
//! - **Token references** (`NAME`, `INT`): match a token of that type.
//! - **Rule references** (`expression`): recursively parse a named rule.
//!
//! Packrat memoization caches the result of every (rule, position) attempt,
//! preventing the exponential backtracking that would otherwise occur with
//! Starlark's ~40-rule grammar.
//!
//! # Why Starlark?
//!
//! Starlark is an ideal language for a parser project because:
//!
//! 1. **Real-world usage** — it powers Bazel, Buck, and other build systems.
//! 2. **Manageable complexity** — ~40 grammar rules (vs. Python's ~100+).
//! 3. **Deterministic** — no `while`, no recursion, always terminates.
//! 4. **Significant whitespace** — exercises the INDENT/DEDENT mechanism.
//! 5. **Rich expressions** — 15 precedence levels, comprehensions, lambdas.

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_starlark_lexer::tokenize_starlark;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `starlark.grammar` file.
///
/// Uses the same strategy as the starlark-lexer crate: `env!("CARGO_MANIFEST_DIR")`
/// gives us the compile-time path to this crate's directory, and we navigate
/// up to the shared `grammars/` directory.
///
/// ```text
/// code/
///   grammars/
///     starlark.grammar      <-- target file
///   packages/
///     rust/
///       starlark-parser/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/starlark.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for Starlark source code.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_starlark` from the starlark-lexer crate
///    to break the source into tokens. This includes indentation tracking,
///    keyword promotion, and reserved keyword rejection.
///
/// 2. **Grammar loading** — reads and parses the `starlark.grammar` file,
///    which defines ~40 rules covering statements, expressions, comprehensions,
///    function definitions, and more.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `starlark.grammar` file cannot be read or parsed.
/// - The source code fails tokenization (reserved keyword, unexpected char).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_starlark_parser::create_starlark_parser;
///
/// let mut parser = create_starlark_parser("x = 1\n");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_starlark_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the starlark-lexer.
    //
    // This produces a Vec<Token> with all the Starlark token types:
    // NAME, KEYWORD, INT, FLOAT, STRING, operators, delimiters,
    // INDENT, DEDENT, NEWLINE, and EOF.
    let tokens = tokenize_starlark(source);

    // Step 2: Read the parser grammar from disk.
    //
    // The grammar file defines the syntactic structure of Starlark in EBNF
    // notation. It has rules like:
    //   file = { NEWLINE | statement } ;
    //   statement = compound_stmt | simple_stmt ;
    //   ...
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read starlark.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    //
    // The ParserGrammar contains a list of GrammarRule objects, each with
    // a name and a body (a tree of GrammarElement nodes representing the
    // EBNF structure).
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse starlark.grammar: {e}"));

    // Step 4: Create the parser.
    //
    // The GrammarParser takes ownership of both the tokens and the grammar.
    // It builds internal indexes (rule lookup, memo cache) for efficient
    // parsing.
    GrammarParser::new(tokens, grammar)
}

/// Parse Starlark source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"file"` (the
/// start symbol of the Starlark grammar) and children corresponding
/// to the statements in the source.
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source code has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_starlark_parser::parse_starlark;
///
/// let ast = parse_starlark("x = 1 + 2\n");
/// assert_eq!(ast.rule_name, "file");
/// ```
pub fn parse_starlark(source: &str) -> GrammarASTNode {
    // Create a parser wired to the Starlark grammar and tokens.
    let mut starlark_parser = create_starlark_parser(source);

    // Parse and unwrap — any GrammarParseError becomes a panic.
    //
    // In a production tool, you would propagate the error via Result.
    // For this educational codebase, panicking with a descriptive message
    // is sufficient.
    starlark_parser
        .parse()
        .unwrap_or_else(|e| panic!("Starlark parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helper: check that the root node has the expected rule name.
    // -----------------------------------------------------------------------

    /// All Starlark programs parse to a root node with rule_name "file",
    /// since that is the start symbol of the grammar.
    fn assert_file_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "file",
            "Expected root rule 'file', got '{}'",
            ast.rule_name
        );
    }

    /// Count how many statement nodes are direct or indirect children of
    /// the given AST. This is a rough measure of how many top-level
    /// statements the parser found.
    fn count_statements(ast: &GrammarASTNode) -> usize {
        ast.children.iter().filter(|child| {
            matches!(child, ASTNodeOrToken::Node(n) if n.rule_name == "statement")
        }).count()
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple assignment
    // -----------------------------------------------------------------------

    /// The simplest Starlark program: a single assignment statement.
    /// This exercises the full pipeline: lexer -> parser -> AST.
    #[test]
    fn test_parse_simple_assignment() {
        let ast = parse_starlark("x = 1\n");
        assert_file_root(&ast);

        // The file should contain at least one statement.
        let stmt_count = count_statements(&ast);
        assert!(stmt_count >= 1, "Expected at least 1 statement, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 2: Arithmetic expression
    // -----------------------------------------------------------------------

    /// An expression statement with binary arithmetic. This tests that
    /// the parser correctly handles operator precedence through the
    /// expression -> or_expr -> ... -> arith -> term -> factor -> power
    /// chain of grammar rules.
    #[test]
    fn test_parse_expression() {
        let ast = parse_starlark("1 + 2\n");
        assert_file_root(&ast);

        // Should parse without error. The expression `1 + 2` becomes an
        // assign_stmt (the grammar's catch-all for expression statements)
        // inside a simple_stmt inside a statement.
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 3: Function definition
    // -----------------------------------------------------------------------

    /// A function definition with parameters and a return statement.
    /// This exercises compound_stmt -> def_stmt, suite with INDENT/DEDENT,
    /// and the parameters rule.
    #[test]
    fn test_parse_function_def() {
        let source = "def add(x, y):\n    return x + y\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        // The file should contain exactly one statement (the def).
        let stmt_count = count_statements(&ast);
        assert_eq!(stmt_count, 1, "Expected 1 statement (def), got {}", stmt_count);

        // Verify that somewhere in the tree there is a "def_stmt" node.
        let has_def = find_rule(&ast, "def_stmt");
        assert!(has_def, "Expected to find a def_stmt rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 4: If/else
    // -----------------------------------------------------------------------

    /// An if/else statement with indented blocks. This tests:
    /// - compound_stmt -> if_stmt rule
    /// - The "else" clause
    /// - Multiple suite blocks with INDENT/DEDENT
    #[test]
    fn test_parse_if_else() {
        let source = "if x:\n    y = 1\nelse:\n    y = 2\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        // Should find an if_stmt node in the tree.
        let has_if = find_rule(&ast, "if_stmt");
        assert!(has_if, "Expected to find an if_stmt rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 5: For loop
    // -----------------------------------------------------------------------

    /// A for loop iterating over a collection. This tests:
    /// - compound_stmt -> for_stmt rule
    /// - loop_vars rule (the iteration variable)
    /// - The "in" keyword as part of the for statement
    #[test]
    fn test_parse_for_loop() {
        let source = "for x in items:\n    print(x)\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        let has_for = find_rule(&ast, "for_stmt");
        assert!(has_for, "Expected to find a for_stmt rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 6: BUILD file pattern (function call with keyword args)
    // -----------------------------------------------------------------------

    /// Starlark is primarily used in BUILD files, which consist of function
    /// calls with keyword arguments. This test verifies that the parser
    /// handles the most common BUILD file pattern.
    ///
    /// Note: brackets suppress INDENT/DEDENT, so the multi-line argument
    /// list is parsed as a flat sequence of tokens without indentation.
    #[test]
    fn test_parse_build_file() {
        let source = "cc_library(\n    name = \"foo\",\n)\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        // This is an expression statement (function call), so it should
        // be parsed as an assign_stmt containing a primary with a call suffix.
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 7: Multiple statements
    // -----------------------------------------------------------------------

    /// A file with multiple top-level statements. The grammar's file rule
    /// is `file = { NEWLINE | statement } ;` so it should handle any number
    /// of statements separated by newlines.
    #[test]
    fn test_parse_multiple_statements() {
        let source = "x = 1\ny = 2\nz = x + y\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        let stmt_count = count_statements(&ast);
        assert_eq!(stmt_count, 3, "Expected 3 statements, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 8: Factory function
    // -----------------------------------------------------------------------

    /// The `create_starlark_parser` factory function should return a
    /// working `GrammarParser` that can successfully parse source code.
    #[test]
    fn test_create_parser() {
        let mut parser = create_starlark_parser("x = 1\n");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "file");
    }

    // -----------------------------------------------------------------------
    // Test 9: Lambda expression
    // -----------------------------------------------------------------------

    /// Lambda expressions are anonymous functions. This tests the
    /// lambda_expr grammar rule.
    #[test]
    fn test_parse_lambda() {
        let source = "f = lambda x: x + 1\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        let has_lambda = find_rule(&ast, "lambda_expr");
        assert!(has_lambda, "Expected to find a lambda_expr rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 10: List literal
    // -----------------------------------------------------------------------

    /// List literals are a common Starlark construct, especially in BUILD
    /// files (e.g., srcs = ["a.cc", "b.cc"]).
    #[test]
    fn test_parse_list_literal() {
        let source = "items = [1, 2, 3]\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        let has_list = find_rule(&ast, "list_expr");
        assert!(has_list, "Expected to find a list_expr rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Helper: recursively search for a rule name in the AST
    // -----------------------------------------------------------------------

    /// Recursively search the AST for a node with the given rule name.
    /// Returns true if found anywhere in the tree.
    fn find_rule(node: &GrammarASTNode, target_rule: &str) -> bool {
        if node.rule_name == target_rule {
            return true;
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                if find_rule(child_node, target_rule) {
                    return true;
                }
            }
        }
        false
    }
}
