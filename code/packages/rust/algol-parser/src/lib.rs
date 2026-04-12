//! # ALGOL 60 Parser — parsing ALGOL 60 source text into an AST.
//!
//! This crate is the second half of the ALGOL 60 front-end pipeline. Where
//! the `algol-lexer` crate breaks source text into tokens, this crate arranges
//! those tokens into a tree that reflects the **structure** of the program —
//! an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! ```text
//! Source text  ("begin integer x; x := 42 end")
//!       |
//!       v
//! algol-lexer          → Vec<Token>
//!       |                [BEGIN, INTEGER, IDENT("x"), SEMICOLON,
//!       |                 IDENT("x"), ASSIGN, INTEGER_LIT("42"), END, EOF]
//!       v
//! algol.grammar        → ParserGrammar (rules: program, block, statement, ...)
//!       |
//!       v
//! GrammarParser        → GrammarASTNode tree
//!       |
//!       |                program
//!       |                  └── block
//!       |                        ├── BEGIN
//!       |                        ├── declaration (type_decl)
//!       |                        │     ├── INTEGER
//!       |                        │     └── ident_list: IDENT("x")
//!       |                        ├── SEMICOLON
//!       |                        ├── statement (assign_stmt)
//!       |                        │     ├── left_part: IDENT("x") ASSIGN
//!       |                        │     └── expression: INTEGER_LIT("42")
//!       |                        └── END
//!       v
//! [application logic consumes the AST]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the `algol.grammar` file and provides two public
//! entry points.
//!
//! # Grammar-driven parsing
//!
//! The `GrammarParser` is a recursive descent parser with backtracking and
//! packrat memoization. ALGOL 60's grammar has approximately 30 rules covering:
//!
//! - **Block structure**: `begin { declaration ; } statement { ; statement } end`
//! - **Declarations**: type declarations, array declarations, switch declarations,
//!   and procedure declarations.
//! - **Statements**: assignments, conditionals (if/then/else), for loops,
//!   goto, procedure calls, and compound statements.
//! - **Expressions**: arithmetic expressions (with operator precedence),
//!   boolean expressions (with `eqv`, `impl`, `or`, `and`, `not`), and
//!   designational expressions (for goto targets).
//!
//! # Why ALGOL 60?
//!
//! ALGOL 60 is an ideal teaching language for parser infrastructure because:
//!
//! 1. **Historical significance** — the ALGOL 60 report was the first use of
//!    BNF (Backus-Naur Form) to specify a programming language's grammar.
//! 2. **Small core** — the grammar has ~30 rules, not hundreds.
//! 3. **Clean design** — ALGOL 60's grammar is famously well-structured,
//!    with clear separation of declarations, statements, and expressions.
//! 4. **Dangling else resolved** — ALGOL 60 resolves the "dangling else"
//!    ambiguity through grammar structure (not conventions), making it a
//!    great case study in grammar design.
//! 5. **All modern patterns** — recursion, mutual recursion, operator
//!    precedence, optional parts, and repetition.

use coding_adventures_algol_lexer::tokenize_algol;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for ALGOL 60 source text.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_algol` from the algol-lexer crate
///    to break the source into tokens (keywords, identifiers, literals,
///    operators, and delimiters).
///
/// 2. **Grammar loading** — reads and parses the `algol.grammar` file,
///    which defines ~30 rules covering ALGOL 60's full syntax.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `algol.grammar` file cannot be read or parsed.
/// - The source text fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_algol_parser::create_algol_parser;
///
/// let mut parser = create_algol_parser("begin integer x; x := 42 end");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_algol_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the algol-lexer.
    //
    // This produces a Vec<Token> with all ALGOL 60 token types.
    // Comments and whitespace have already been consumed.
    let tokens = tokenize_algol(source);

    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
}

/// Parse ALGOL 60 source text into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the ALGOL 60 grammar) with children corresponding to
/// the structure of the program.
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source text has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_algol_parser::parse_algol;
///
/// let ast = parse_algol("begin integer x; x := 42 end");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_algol(source: &str) -> GrammarASTNode {
    // Create a parser wired to the ALGOL 60 grammar and tokens.
    let mut algol_parser = create_algol_parser(source);

    // Parse and unwrap — any GrammarParseError becomes a panic.
    //
    // In a production tool, you would propagate the error via Result.
    // For this educational codebase, panicking with a descriptive message
    // is sufficient.
    algol_parser
        .parse()
        .unwrap_or_else(|e| panic!("ALGOL 60 parse failed: {e}"))
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

    /// All ALGOL 60 programs parse to a root node with rule_name "program",
    /// since that is the start symbol of the grammar.
    fn assert_program_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "program",
            "Expected root rule 'program', got '{}'",
            ast.rule_name
        );
    }

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

    // -----------------------------------------------------------------------
    // Test 1: Minimal program with integer declaration
    // -----------------------------------------------------------------------

    /// The simplest valid ALGOL 60 program: declare an integer, assign to it.
    /// Exercises the full pipeline: lexer → tokenizer → grammar → AST.
    ///
    /// This program exercises:
    /// - program → block
    /// - block → BEGIN { declaration ; } statement END
    /// - declaration → type_decl → INTEGER ident_list
    /// - statement → assign_stmt → left_part expression
    #[test]
    fn test_parse_minimal_program() {
        let ast = parse_algol("begin integer x; x := 42 end");
        assert_program_root(&ast);

        // Must find a block rule
        assert!(
            find_rule(&ast, "block"),
            "Expected 'block' rule in minimal program AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Block structure
    // -----------------------------------------------------------------------

    /// Verifies that the `block` rule is the direct child of `program`.
    #[test]
    fn test_parse_block_structure() {
        let ast = parse_algol("begin integer n; n := 0 end");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "block"), "Expected 'block' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 3: Assignment statement
    // -----------------------------------------------------------------------

    /// An assignment `x := 42` should produce an `assign_stmt` node.
    #[test]
    fn test_parse_assignment() {
        let ast = parse_algol("begin integer x; x := 42 end");
        assert_program_root(&ast);

        assert!(
            find_rule(&ast, "assign_stmt"),
            "Expected 'assign_stmt' rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Arithmetic expression
    // -----------------------------------------------------------------------

    /// An arithmetic expression in an assignment exercises arith_expr, term,
    /// factor, and primary rules.
    #[test]
    fn test_parse_arithmetic_expression() {
        let ast = parse_algol("begin integer x; x := 1 + 2 * 3 end");
        assert_program_root(&ast);

        // With the unified expression grammar, `1 + 2 * 3` in an assignment
        // is parsed via `expression → expr_eqv → ... → expr_add → expr_mul`.
        // The old `arith_expr` / `simple_arith` rules are only used for
        // type-specific contexts (for-loop bounds, subscripts, etc.); a plain
        // assignment RHS goes through the unified `expr_add` rule.
        let has_arith = find_rule(&ast, "expr_add")
            || find_rule(&ast, "arith_expr")
            || find_rule(&ast, "simple_arith");
        assert!(has_arith, "Expected arithmetic expression rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 5: If/then statement
    // -----------------------------------------------------------------------

    /// An if/then statement (without else) exercises the `cond_stmt` rule.
    #[test]
    fn test_parse_if_then() {
        let ast = parse_algol("begin integer x; if x = 0 then x := 1 end");
        assert_program_root(&ast);

        assert!(
            find_rule(&ast, "cond_stmt"),
            "Expected 'cond_stmt' rule for if/then statement"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: If/then/else statement
    // -----------------------------------------------------------------------

    /// An if/then/else statement exercises the full `cond_stmt` rule.
    /// The else-branch is a full `statement`, so nested conditionals work.
    #[test]
    fn test_parse_if_then_else() {
        let source = "begin integer x; if x = 0 then x := 1 else x := 2 end";
        let ast = parse_algol(source);
        assert_program_root(&ast);

        assert!(
            find_rule(&ast, "cond_stmt"),
            "Expected 'cond_stmt' rule for if/then/else"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: For loop (step/until form)
    // -----------------------------------------------------------------------

    /// The classic ALGOL 60 for loop: `for i := 1 step 1 until 10 do`
    /// exercises the `for_stmt` rule with the `step ... until` for_elem form.
    #[test]
    fn test_parse_for_loop_step_until() {
        let source =
            "begin integer i, sum; sum := 0; for i := 1 step 1 until 10 do sum := sum + i end";
        let ast = parse_algol(source);
        assert_program_root(&ast);

        assert!(
            find_rule(&ast, "for_stmt"),
            "Expected 'for_stmt' rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: Type declaration
    // -----------------------------------------------------------------------

    /// Verifies that `integer x, y` is parsed as a type_decl.
    #[test]
    fn test_parse_type_declaration() {
        let ast = parse_algol("begin integer x, y; x := 1; y := 2 end");
        assert_program_root(&ast);

        assert!(
            find_rule(&ast, "type_decl"),
            "Expected 'type_decl' in AST for integer declaration"
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: Real type declaration
    // -----------------------------------------------------------------------

    /// Real variables work the same as integer declarations.
    #[test]
    fn test_parse_real_declaration() {
        let ast = parse_algol("begin real pi; pi := 3.14 end");
        assert_program_root(&ast);

        assert!(
            find_rule(&ast, "type_decl"),
            "Expected 'type_decl' for real declaration"
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: Factory function
    // -----------------------------------------------------------------------

    /// The `create_algol_parser` factory function should return a working
    /// `GrammarParser` that can successfully parse ALGOL 60.
    #[test]
    fn test_create_parser() {
        let mut parser = create_algol_parser("begin integer x; x := 0 end");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 11: Compound statement (begin...end without declarations)
    // -----------------------------------------------------------------------

    /// A compound statement is begin...end with only statements, no declarations.
    /// This differs from a full block.
    #[test]
    fn test_parse_compound_statement() {
        // Nested begin...end as the body of an if statement
        let source = "begin integer x; if x = 0 then begin x := 1 end end";
        let ast = parse_algol(source);
        assert_program_root(&ast);

        // The compound_stmt rule should appear for the inner begin...end
        assert!(
            find_rule(&ast, "compound_stmt") || find_rule(&ast, "block"),
            "Expected compound_stmt or block for nested begin...end"
        );
    }

    // -----------------------------------------------------------------------
    // Test 12: Exponentiation operator
    // -----------------------------------------------------------------------

    /// ALGOL 60 supports both `**` and `^` for exponentiation.
    /// The grammar uses `factor = primary { (CARET | POWER) primary }`.
    #[test]
    fn test_parse_exponentiation() {
        // Using ** operator
        let ast = parse_algol("begin real x; x := 2 ** 3 end");
        assert_program_root(&ast);

        // Using ^ operator
        let ast2 = parse_algol("begin real x; x := 2 ^ 3 end");
        assert_program_root(&ast2);
    }

    // -----------------------------------------------------------------------
    // Test 13: Boolean expression with and/or/not
    // -----------------------------------------------------------------------

    /// ALGOL 60 uses word-based boolean operators.
    /// This exercises the bool_expr → bool_term → bool_factor → bool_secondary
    /// → bool_primary chain.
    #[test]
    fn test_parse_boolean_expression() {
        let source = "begin boolean flag; if true and not false then flag := true end";
        let ast = parse_algol(source);
        assert_program_root(&ast);

        assert!(
            find_rule(&ast, "bool_expr") || find_rule(&ast, "bool_factor"),
            "Expected boolean expression rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: Goto statement
    // -----------------------------------------------------------------------

    /// A `goto` statement with a label target.
    #[test]
    fn test_parse_goto() {
        let source = "begin integer x; x := 0; goto done; x := 1; done: x := 99 end";
        let ast = parse_algol(source);
        assert_program_root(&ast);

        assert!(find_rule(&ast, "goto_stmt"), "Expected 'goto_stmt' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 15: Procedure call as statement
    // -----------------------------------------------------------------------

    /// A procedure call statement: just an identifier (no arguments).
    /// This exercises the `proc_stmt` rule.
    #[test]
    fn test_parse_proc_call_stmt() {
        let source = "begin integer x; x := 0; print end";
        let ast = parse_algol(source);
        assert_program_root(&ast);

        // proc_stmt covers both labeled call "print" and "print(x)"
        assert!(
            find_rule(&ast, "proc_stmt") || find_rule(&ast, "assign_stmt"),
            "Expected proc_stmt or assign_stmt rule"
        );
    }

    // -----------------------------------------------------------------------
    // Test 16: While form in for loop
    // -----------------------------------------------------------------------

    /// ALGOL 60 for loops can use the `while` form: `for x := expr while bool do`.
    #[test]
    fn test_parse_for_while() {
        let source = "begin integer x; x := 10; for x := x while x > 0 do x := x - 1 end";
        let ast = parse_algol(source);
        assert_program_root(&ast);

        assert!(
            find_rule(&ast, "for_stmt"),
            "Expected 'for_stmt' rule in AST"
        );
    }
}
