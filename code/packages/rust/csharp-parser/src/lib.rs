//! # C# Parser — parsing C# source code into an AST.
//!
//! This crate is the second half of the C# front-end pipeline. Where
//! the `csharp-lexer` crate breaks source text into tokens, this crate
//! arranges those tokens into a tree that reflects the **structure** of the
//! code — an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing C# requires four cooperating components:
//!
//! ```text
//! Source code  ("class Hello { }")
//!       |
//!       v
//! csharp-lexer          -> Vec<Token>
//!       |                 [KEYWORD("class"), NAME("Hello"), LBRACE("{"),
//!       |                  RBRACE("}"), EOF]
//!       v
//! csharp{v}.grammar     -> ParserGrammar (rules like "program = ...")
//!       |
//!       v
//! GrammarParser         -> GrammarASTNode tree
//!       |
//!       |                 program
//!       |                   └── statement
//!       |                         └── class_declaration
//!       |                               ├── KEYWORD("class")
//!       |                               ├── NAME("Hello")
//!       |                               ├── LBRACE("{")
//!       |                               └── RBRACE("}")
//!       v
//! [future stages: interpretation, compilation]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the grammar files and provides two public entry
//! points.
//!
//! # Version-Aware API
//!
//! Both entry points accept a `version` parameter that selects which grammar
//! file to use. C# uses a `major.minor` versioning scheme:
//!
//! | `version` | grammar file loaded |
//! |---|---|
//! | `"1.0"` | `grammars/csharp/csharp1.0.grammar` |
//! | `"2.0"` | `grammars/csharp/csharp2.0.grammar` |
//! | `"3.0"` | `grammars/csharp/csharp3.0.grammar` |
//! | `"4.0"` | `grammars/csharp/csharp4.0.grammar` |
//! | `"5.0"` | `grammars/csharp/csharp5.0.grammar` |
//! | `"6.0"` | `grammars/csharp/csharp6.0.grammar` |
//! | `"7.0"` | `grammars/csharp/csharp7.0.grammar` |
//! | `"8.0"` | `grammars/csharp/csharp8.0.grammar` |
//! | `"9.0"` | `grammars/csharp/csharp9.0.grammar` |
//! | `"10.0"` | `grammars/csharp/csharp10.0.grammar` |
//! | `"11.0"` | `grammars/csharp/csharp11.0.grammar` |
//! | `"12.0"` | `grammars/csharp/csharp12.0.grammar` |
//!
//! An unknown version string returns `Err(String)`.
//!
//! # Grammar rules
//!
//! The C# grammar covers:
//! - **program** — the top-level rule, a sequence of statements (or a
//!   namespace/using block at the file level)
//! - **statement** — variable declarations, expression statements,
//!   if/else, while, for, foreach, return, class declarations, namespace
//!   declarations, using directives
//! - **expression** — arithmetic, comparison, logical, assignment, method
//!   calls, member access, null coalescing (`??`), null conditional (`?.`)
//! - **class_declaration** — class definitions with access modifiers, base
//!   class, interfaces, and body
//! - **var_declaration** — variable declarations with type annotations and
//!   optional initializers

use std::fs;
use std::path::PathBuf;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_csharp_lexer::tokenize_csharp;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Returns the root `grammars/` directory by navigating up from this crate.
///
/// ```text
/// code/
///   grammars/            <-- returned by this function
///   packages/
///     rust/
///       csharp-parser/
///         Cargo.toml     <-- env!("CARGO_MANIFEST_DIR")
/// ```
///
/// This relative navigation is baked in at compile time and works regardless
/// of where the repository is checked out, because `CARGO_MANIFEST_DIR` is
/// always the absolute path to the crate's directory at compile time.
fn grammar_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
}

/// Validate the C# version string and return the path to the corresponding
/// `.grammar` file.
///
/// The version string must be one of the 12 officially supported C# versions.
/// Each version maps to a dedicated `.grammar` file that describes the
/// syntactic rules for that edition of C#.
///
/// Using a version-specific grammar file (rather than a single "latest"
/// grammar) ensures that code written for an older C# version is parsed
/// according to the rules of that version. For example, C# 9.0's `record`
/// keyword is not valid in a C# 7.0 grammar, and C# 8.0's switch expressions
/// are not valid in a C# 6.0 grammar.
///
/// Returns `Err(String)` for any unrecognised version string.
fn grammar_path(version: &str) -> Result<PathBuf, String> {
    let root = grammar_root();

    match version {
        // All 12 C# versions — each maps to its own .grammar file.
        "1.0" | "2.0" | "3.0" | "4.0" | "5.0" | "6.0" |
        "7.0" | "8.0" | "9.0" | "10.0" | "11.0" | "12.0" => {
            Ok(root.join("csharp").join(format!("csharp{version}.grammar")))
        }

        // Anything else is an error — we'd rather fail loudly than silently
        // fall back to a default grammar and produce confusing results.
        other => Err(format!(
            "Unknown C# version '{other}'. \
             Valid values: \"1.0\", \"2.0\", \"3.0\", \"4.0\", \
             \"5.0\", \"6.0\", \"7.0\", \"8.0\", \
             \"9.0\", \"10.0\", \"11.0\", \"12.0\""
        )),
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for C# source code.
///
/// The `version` parameter selects which grammar file to load. Pass `"12.0"`
/// for the latest C# grammar, or an earlier version for legacy code.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_csharp` from the `csharp-lexer` crate
///    to break the source into tokens (also with the same `version`). This
///    ensures that the token stream and the grammar rules are always from the
///    same C# edition — you can't accidentally mix C# 12.0 tokens with a
///    C# 5.0 grammar.
///
/// 2. **Grammar loading** — reads and parses the appropriate `.grammar` file,
///    which defines rules for programs, statements, expressions, and class
///    definitions in C#-specific syntax.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - The `version` string is not recognised.
/// - The grammar file cannot be read or parsed.
/// - The source code fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_csharp_parser::create_csharp_parser;
///
/// // C# 12.0 (latest):
/// let mut parser = create_csharp_parser("class Hello { }", "12.0").unwrap();
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
///
/// // C# 8.0 (nullable reference types era):
/// let mut parser_8 = create_csharp_parser("int x = 42;", "8.0").unwrap();
/// ```
pub fn create_csharp_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    // Step 1: Tokenize the source using the csharp-lexer (same version).
    // Using the same version for both tokenization and parsing is critical:
    // for example, `record` is a keyword in C# 9.0+ but a valid identifier
    // in older versions. The lexer must agree with the parser on this.
    let tokens = tokenize_csharp(source, version)?;

    // Step 2: Resolve the parser grammar file path.
    let path = grammar_path(version)?;

    // Step 3: Read the parser grammar from disk.
    let grammar_text = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    // Step 4: Parse the grammar text into a structured ParserGrammar.
    //
    // The ParserGrammar defines rules like:
    //   program = statement*
    //   statement = var_declaration | expression_statement | ...
    //   expression = term (('+' | '-') term)*
    //   ...
    let grammar = parse_parser_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    // Step 5: Create the parser with the token stream and grammar.
    Ok(GrammarParser::new(tokens, grammar))
}

/// Parse C# source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The `version` parameter is the same as for [`create_csharp_parser`]:
/// pass a version like `"12.0"` for the latest C#, or `"8.0"` for code
/// targeting .NET Core 3.x.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the C# grammar) with children corresponding to the
/// top-level declarations and statements in the source file.
///
/// # Errors
///
/// Returns `Err(String)` if the version is unknown, the grammar file is
/// missing or malformed, or the source has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_csharp_parser::parse_csharp;
///
/// // C# 12.0 (latest):
/// let ast = parse_csharp("class Hello { }", "12.0").unwrap();
/// assert_eq!(ast.rule_name, "program");
///
/// // C# 9.0 (records era):
/// let ast_9 = parse_csharp("int x = 1;", "9.0").unwrap();
/// ```
pub fn parse_csharp(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut csharp_parser = create_csharp_parser(source, version)?;

    csharp_parser
        .parse()
        .map_err(|e| format!("C# parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Assert that the AST root has the rule name "compilation_unit".
    ///
    /// Every C# grammar has `compilation_unit` as its start symbol. This invariant
    /// must hold for every successful parse, regardless of version.
    fn assert_program_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "compilation_unit",
            "Expected root rule 'compilation_unit', got '{}'",
            ast.rule_name
        );
    }

    fn count_rules(node: &GrammarASTNode, target_rule: &str) -> usize {
        let mut count = usize::from(node.rule_name == target_rule);
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                count += count_rules(child_node, target_rule);
            }
        }
        count
    }

    fn count_statements(ast: &GrammarASTNode) -> usize {
        count_rules(ast, "statement")
    }

    /// Recursively search for a node with a given rule name anywhere in the
    /// AST.
    ///
    /// This is useful when you want to verify that a certain construct was
    /// parsed without caring exactly where in the tree it appears. For example,
    /// `find_rule(&ast, "class_declaration")` verifies a class was parsed
    /// somewhere in the tree.
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

    // -----------------------------------------------------------------------
    // Test 1: Simple class declaration (C# 12.0 grammar)
    // -----------------------------------------------------------------------

    /// The simplest C# program: a class with an empty body.
    ///
    /// In C#, everything lives inside a class, struct, or interface. Even
    /// "top-level statements" (introduced in C# 9.0) are syntactic sugar —
    /// the compiler implicitly wraps them in a hidden `Program` class with
    /// a synthesized `Main` method. The class is therefore the foundational
    /// building block of C# code organization.
    #[test]
    fn test_parse_class_declaration() {
        let ast = parse_csharp("class Hello {}", "12.0").unwrap();
        assert_program_root(&ast);
        assert!(find_rule(&ast, "class_declaration"));
    }

    // -----------------------------------------------------------------------
    // Test 2: Arithmetic expression
    // -----------------------------------------------------------------------

    /// An expression statement with binary arithmetic.
    ///
    /// C# uses left-to-right evaluation for arithmetic expressions of equal
    /// precedence, following standard mathematical convention. The parser
    /// must produce a left-associative tree for `1 + 2 + 3`.
    #[test]
    fn test_parse_expression() {
        let ast = parse_csharp("1 + 2;", "12.0").unwrap();
        assert_program_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 3: Multiple statements
    // -----------------------------------------------------------------------

    /// A program with multiple variable declarations.
    ///
    /// C# variable declarations have the form `type name = value;` or
    /// `var name = value;` (where `var` lets the compiler infer the type).
    /// The `int` type is a 32-bit signed integer, equivalent to `System.Int32`.
    #[test]
    fn test_parse_multiple_statements() {
        let source = "namespace Demo { public class A {} public class B {} public class C {} }";
        let ast = parse_csharp(source, "12.0").unwrap();
        assert_program_root(&ast);

        let class_count = count_rules(&ast, "class_declaration");
        assert_eq!(class_count, 3, "Expected 3 class declarations, got {}", class_count);
    }

    // -----------------------------------------------------------------------
    // Test 4: Empty program
    // -----------------------------------------------------------------------

    /// An empty program should parse to a program node with no children.
    ///
    /// The empty string is valid C# — it represents a file with no
    /// declarations, which is uncommon but syntactically permissible
    /// (though the compiler would complain about missing entry points
    /// unless the project uses top-level statements).
    #[test]
    fn test_parse_empty_program() {
        let ast = parse_csharp("", "12.0").unwrap();
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 5: Factory function
    // -----------------------------------------------------------------------

    /// The `create_csharp_parser` factory function should return a working
    /// `GrammarParser`.
    ///
    /// The factory function is useful when you want direct access to the
    /// parser object — for example, to inspect intermediate state during
    /// error recovery, or to run the parser in a loop.
    #[test]
    fn test_create_parser() {
        let mut parser = create_csharp_parser("public class Foo {}", "12.0").unwrap();
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "compilation_unit");
    }

    // -----------------------------------------------------------------------
    // Test 6: Versioned grammar — C# 8.0
    // -----------------------------------------------------------------------

    /// The C# 8.0 versioned grammar should parse a basic int declaration.
    ///
    /// C# 8.0 is a particularly important version: it introduced nullable
    /// reference types (`string?`), which fundamentally changed how C#
    /// programmers reason about null safety. It also added switch expressions,
    /// ranges and indices (`^1`, `1..3`), async streams, and default interface
    /// implementations.
    #[test]
    fn test_versioned_csharp_8() {
        let ast = parse_csharp("public class Foo {}", "8.0").unwrap();
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 7: All 12 versioned grammars parse an empty program
    // -----------------------------------------------------------------------

    /// Every versioned C# grammar should successfully parse an empty
    /// program (the simplest valid input).
    ///
    /// This smoke test verifies that all 12 grammar files are present,
    /// parseable, and produce the correct root node. It exercises the full
    /// version history from the original .NET Framework release through the
    /// modern .NET 8 era.
    #[test]
    fn test_all_versioned_grammars() {
        let versions = [
            "1.0", "2.0", "3.0", "4.0", "5.0", "6.0",
            "7.0", "8.0", "9.0", "10.0", "11.0", "12.0",
        ];
        for v in &versions {
            let result = parse_csharp("public class Foo {}", v);
            assert!(
                result.is_ok(),
                "Version '{v}' should parse successfully: {:?}",
                result.err()
            );
            assert_eq!(result.unwrap().rule_name, "compilation_unit");
        }
    }

    // -----------------------------------------------------------------------
    // Test 8: Unknown version returns Err
    // -----------------------------------------------------------------------

    /// Passing an unrecognised version string should return Err, not panic.
    ///
    /// This test exercises the version validation logic in both the lexer
    /// (called first) and the parser grammar path resolution. The error
    /// message should contain the bad version string so users know exactly
    /// what configuration value is wrong.
    #[test]
    fn test_unknown_version_returns_err() {
        let result = parse_csharp("int x = 1;", "99.0");
        assert!(result.is_err(), "Expected Err for unknown version '99.0'");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("99.0"),
            "Error message should mention the bad version: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: create_csharp_parser with unknown version returns Err
    // -----------------------------------------------------------------------

    /// The factory function should also return Err for unknown versions.
    #[test]
    fn test_create_parser_unknown_version() {
        let result = create_csharp_parser("int x = 1;", "bad-version");
        assert!(
            result.is_err(),
            "Expected Err from create_csharp_parser with bad version"
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: Versioned grammar — C# 5.0 (async/await era)
    // -----------------------------------------------------------------------

    /// The C# 5.0 versioned grammar should parse a basic int declaration.
    ///
    /// C# 5.0 introduced `async` and `await`, which transformed how C#
    /// programmers write asynchronous code. Before C# 5.0, async programming
    /// required explicit callbacks or continuation-passing style. After 5.0,
    /// you could write async code that looks sequential.
    #[test]
    fn test_versioned_csharp_5() {
        let ast = parse_csharp("public class Foo {}", "5.0").unwrap();
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 11: Versioned grammar — C# 3.0 (LINQ era)
    // -----------------------------------------------------------------------

    /// The C# 3.0 versioned grammar should parse a basic expression.
    ///
    /// C# 3.0 introduced LINQ (Language Integrated Query), which revolutionized
    /// how .NET programmers query and transform data. The `from x in collection
    /// where x > 0 select x` query syntax compiles to method calls using
    /// extension methods — another C# 3.0 innovation. Anonymous types and
    /// lambda expressions were also introduced here.
    #[test]
    fn test_versioned_csharp_3() {
        let ast = parse_csharp("public class Foo {}", "3.0").unwrap();
        assert_program_root(&ast);
        assert!(find_rule(&ast, "class_declaration"));
    }

    // -----------------------------------------------------------------------
    // Test 12: Versioned grammar — C# 12.0 (latest)
    // -----------------------------------------------------------------------

    /// The C# 12.0 versioned grammar should parse multiple statements.
    ///
    /// C# 12.0 (shipped with .NET 8 LTS in November 2023) introduced:
    /// - Primary constructors for all classes and structs
    /// - Collection expressions (`[1, 2, 3]` syntax)
    /// - `ref readonly` parameters
    /// - Default lambda parameters
    /// - Inline arrays
    /// - Experimental `interceptors` (preview)
    #[test]
    fn test_versioned_csharp_12() {
        let source = "namespace Demo { public class A {} public class B {} }";
        let ast = parse_csharp(source, "12.0").unwrap();
        assert_program_root(&ast);

        let class_count = count_rules(&ast, "class_declaration");
        assert_eq!(class_count, 2, "Expected 2 class declarations, got {}", class_count);
    }
}
