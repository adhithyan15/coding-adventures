// parser — Brainfuck parser using the grammar-driven parser infrastructure
// ==========================================================================
//
// This module is the parsing layer for Brainfuck source code. It sits above
// the lexer in the three-stage front-end pipeline:
//
//   brainfuck.grammar    (grammar file on disk — rules for the parser)
//          |
//          v
//   grammar-tools        (parses .grammar → ParserGrammar struct)
//          |
//          v
//   parser::GrammarParser (builds an AST from tokens using ParserGrammar)
//          |
//          v
//   GrammarASTNode       (the Abstract Syntax Tree)
//
// # What does parsing add over tokenization?
//
// The lexer turns a flat string into a flat list of typed tokens:
//
//   "++[>+<-]"  →  INC INC LOOP_START RIGHT INC LEFT DEC LOOP_END EOF
//
// The parser turns that flat list into a tree that captures structure:
//
//   program
//     instruction → command(INC)
//     instruction → command(INC)
//     instruction → loop
//       LOOP_START
//       instruction → command(RIGHT)
//       instruction → command(INC)
//       instruction → command(LEFT)
//       instruction → command(DEC)
//       LOOP_END
//
// This tree is the input to the semantic layer (interpreter / code generator).
// Working with the tree is much simpler than manually tracking bracket nesting
// during interpretation.
//
// # Grammar rules (from brainfuck.grammar)
//
//   program     = { instruction } ;
//   instruction = loop | command ;
//   loop        = LOOP_START { instruction } LOOP_END ;
//   command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
//
// The grammar is purely left-to-right with no ambiguity. Every token
// uniquely determines which rule applies:
//   - LOOP_START → loop rule
//   - Any of RIGHT/LEFT/INC/DEC/OUTPUT/INPUT → command rule
//   - LOOP_END → end of enclosing loop
//   - EOF → end of program
//
// # Path navigation
//
// This module lives inside the `brainfuck` crate. CARGO_MANIFEST_DIR is
// `code/packages/rust/brainfuck/`. The grammar file is at:
//   `code/packages/rust/brainfuck/../../../grammars/brainfuck.grammar`
//   = `code/grammars/brainfuck.grammar`

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `brainfuck.grammar` file.
///
/// Uses the same strategy as the lexer module: `env!("CARGO_MANIFEST_DIR")`
/// gives us the compile-time path to this crate's directory, and we navigate
/// up to the shared `grammars/` directory.
///
/// ```text
/// code/
///   grammars/
///     brainfuck.grammar         <-- target file
///   packages/
///     rust/
///       brainfuck/
///         Cargo.toml            <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/brainfuck.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for Brainfuck source text.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — calls `super::lexer::tokenize_brainfuck` to break the
///    source into command tokens (comment characters are silently discarded).
///
/// 2. **Grammar loading** — reads and parses `brainfuck.grammar`, which defines
///    4 rules: program, instruction, loop, command.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Why separate create from parse?
///
/// Separating creation from parsing allows you to:
/// - Inspect the token stream before parsing.
/// - Retry parsing with different grammar options.
/// - Use trace mode for debugging.
///
/// For most use cases, `parse_brainfuck` is simpler.
///
/// # Panics
///
/// Panics if:
/// - The `brainfuck.grammar` file cannot be read or parsed.
/// - (tokenization never panics for Brainfuck — all non-command chars are skipped)
///
/// # Example
///
/// ```no_run
/// use brainfuck::parser::create_brainfuck_parser;
///
/// let mut parser = create_brainfuck_parser("++[>+<-]");
/// let ast = parser.parse().expect("parse failed");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn create_brainfuck_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the brainfuck lexer.
    //
    // We use the sibling lexer module in the same crate — no external
    // crate import needed.  This produces a Vec<Token> with types:
    // RIGHT, LEFT, INC, DEC, OUTPUT, INPUT, LOOP_START, LOOP_END, EOF.
    // Comment characters are silently consumed by the lexer's skip patterns.
    let tokens = super::lexer::tokenize_brainfuck(source);

    // Step 2: Read the parser grammar from disk.
    //
    // The grammar file defines Brainfuck's syntactic structure in EBNF:
    //   program     = { instruction } ;
    //   instruction = loop | command ;
    //   loop        = LOOP_START { instruction } LOOP_END ;
    //   command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read brainfuck.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    //
    // The ParserGrammar contains a list of GrammarRule objects, each with
    // a name and a body (a tree of GrammarElement nodes encoding the EBNF).
    // The rules are: program, instruction, loop, command.
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse brainfuck.grammar: {e}"));

    // Step 4: Create and return the parser.
    //
    // GrammarParser takes ownership of both tokens and grammar. It builds
    // internal indexes (rule lookup, memo cache) for efficient recursive
    // descent parsing with packrat memoization.
    GrammarParser::new(tokens, grammar)
}

/// Parse Brainfuck source text into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the Brainfuck grammar). Its children are `instruction`
/// nodes, each of which is either a `loop` node (for `[...]`) or a `command`
/// node (for the six non-bracket commands).
///
/// # Panics
///
/// Panics if the grammar file is missing/invalid, or if the source has
/// unmatched brackets. Well-formed Brainfuck (all brackets matched) will
/// always parse successfully.
///
/// # Example
///
/// ```no_run
/// use brainfuck::parser::parse_brainfuck;
///
/// let ast = parse_brainfuck("++[>+<-]").expect("parse failed");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_brainfuck(source: &str) -> Result<GrammarASTNode, String> {
    // Create a parser wired to the Brainfuck grammar and tokens.
    let mut bf_parser = create_brainfuck_parser(source);

    // Parse and return Result — GrammarParseError is converted to String.
    bf_parser
        .parse()
        .map_err(|e| format!("Brainfuck parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helper: check that the root node has rule_name "program".
    //
    // Every Brainfuck document parses to a root node with rule_name "program"
    // because that is the start symbol of the grammar.
    // -----------------------------------------------------------------------

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

    /// Count nodes with the given rule name anywhere in the tree.
    fn count_rule(node: &GrammarASTNode, target_rule: &str) -> usize {
        let mut count = if node.rule_name == target_rule { 1 } else { 0 };
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                count += count_rule(child_node, target_rule);
            }
        }
        count
    }

    // -----------------------------------------------------------------------
    // Test 1: Empty program
    // -----------------------------------------------------------------------

    /// An empty source string is a valid Brainfuck program — it does nothing.
    /// The grammar's `{ instruction }` production allows zero instructions.
    ///
    /// AST: program (with no instruction children)
    #[test]
    fn test_parse_empty() {
        let ast = parse_brainfuck("").expect("empty program should parse");
        assert_program_root(&ast);
        // No instruction children expected.
        let instruction_count = count_rule(&ast, "instruction");
        assert_eq!(instruction_count, 0, "Empty program should have no instructions");
    }

    // -----------------------------------------------------------------------
    // Test 2: Single command
    // -----------------------------------------------------------------------

    /// A single "+" command should parse to program → instruction → command.
    #[test]
    fn test_parse_single_command() {
        let ast = parse_brainfuck("+").expect("+  should parse");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "command"), "Expected a 'command' node");
        assert!(find_rule(&ast, "instruction"), "Expected an 'instruction' node");
    }

    // -----------------------------------------------------------------------
    // Test 3: All six command types
    // -----------------------------------------------------------------------

    /// Parse each of the six non-bracket commands. Each should produce
    /// a program with one instruction → command node.
    #[test]
    fn test_parse_all_commands() {
        for src in &[">", "<", "+", "-", ".", ","] {
            let ast = parse_brainfuck(src).unwrap_or_else(|e| panic!("'{}' failed: {}", src, e));
            assert_program_root(&ast);
            assert!(find_rule(&ast, "command"), "Expected 'command' for '{}'", src);
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: Multiple commands
    // -----------------------------------------------------------------------

    /// "++>" should parse to three instruction nodes.
    /// Each instruction wraps a single command.
    #[test]
    fn test_parse_multiple_commands() {
        let ast = parse_brainfuck("++>").expect("++> should parse");
        assert_program_root(&ast);
        let instruction_count = count_rule(&ast, "instruction");
        assert_eq!(instruction_count, 3, "Expected 3 instruction nodes for '++>'");
    }

    // -----------------------------------------------------------------------
    // Test 5: Simple loop
    // -----------------------------------------------------------------------

    /// "[+]" is a simple loop: enter if cell != 0, execute +, repeat.
    /// The AST should contain a loop node.
    #[test]
    fn test_parse_simple_loop() {
        let ast = parse_brainfuck("[+]").expect("[+] should parse");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "loop"), "Expected a 'loop' node in AST");
    }

    // -----------------------------------------------------------------------
    // Test 6: Empty loop
    // -----------------------------------------------------------------------

    /// "[]" is a legal empty loop in Brainfuck. If the current cell is nonzero,
    /// it loops forever (infinite loop). If zero, it's a no-op.
    /// The parser must accept this as a valid loop.
    #[test]
    fn test_parse_empty_loop() {
        let ast = parse_brainfuck("[]").expect("[] should parse");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "loop"), "Expected a 'loop' node for '[]'");
    }

    // -----------------------------------------------------------------------
    // Test 7: Nested loops
    // -----------------------------------------------------------------------

    /// "[[+]]" exercises nested loops. The outer loop contains an inner loop.
    /// Both loop nodes should appear in the AST.
    #[test]
    fn test_parse_nested_loops() {
        let ast = parse_brainfuck("[[+]]").expect("[[+]] should parse");
        assert_program_root(&ast);
        let loop_count = count_rule(&ast, "loop");
        assert_eq!(loop_count, 2, "Expected 2 loop nodes for '[[+]]'");
    }

    // -----------------------------------------------------------------------
    // Test 8: Canonical program "++[>+<-]"
    // -----------------------------------------------------------------------

    /// The canonical "copy-and-move" loop. This is one of the most common
    /// Brainfuck patterns:
    ///
    ///   ++      set cell 0 to 2
    ///   [       while cell 0 != 0:
    ///     >+      increment cell 1
    ///     <-      decrement cell 0
    ///   ]       end loop: cell 0 = 0, cell 1 = 2
    ///
    /// Expected AST: program with 3 instructions (cmd, cmd, loop).
    #[test]
    fn test_parse_canonical() {
        let ast = parse_brainfuck("++[>+<-]").expect("++[>+<-] should parse");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "loop"), "Expected a 'loop' node");
        assert!(find_rule(&ast, "command"), "Expected 'command' nodes");
    }

    // -----------------------------------------------------------------------
    // Test 9: Comments are ignored
    // -----------------------------------------------------------------------

    /// Brainfuck allows arbitrary text as comments. The lexer's skip patterns
    /// consume all non-command characters, so the parser sees only commands.
    /// "++ add two [loop body >+<-] done" should parse identically to "++[>+<-]".
    #[test]
    fn test_parse_with_comments() {
        let commented = parse_brainfuck("++ add two [loop body >+<-] done").expect("should parse");
        let uncommented = parse_brainfuck("++[>+<-]").expect("should parse");
        // Both should have the same rule structure.
        assert_eq!(
            count_rule(&commented, "instruction"),
            count_rule(&uncommented, "instruction"),
            "Comment stripping should produce identical instruction count"
        );
        assert_eq!(
            count_rule(&commented, "loop"),
            count_rule(&uncommented, "loop"),
            "Comment stripping should produce identical loop count"
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: Unmatched [ returns an error
    // -----------------------------------------------------------------------

    /// "[+" has an unmatched opening bracket. The parser should return an error
    /// rather than panic. Well-formed programs have matched brackets; ill-formed
    /// programs should be rejected with a descriptive error message.
    #[test]
    fn test_parse_unmatched_open_bracket() {
        let result = parse_brainfuck("[+");
        assert!(result.is_err(), "Unmatched '[' should return an error");
    }

    // -----------------------------------------------------------------------
    // Test 11: Unmatched ] returns an error
    // -----------------------------------------------------------------------

    /// "+]" has an unmatched closing bracket. Same expectation as above.
    #[test]
    fn test_parse_unmatched_close_bracket() {
        let result = parse_brainfuck("+]");
        assert!(result.is_err(), "Unmatched ']' should return an error");
    }

    // -----------------------------------------------------------------------
    // Test 12: Factory function returns working parser
    // -----------------------------------------------------------------------

    /// `create_brainfuck_parser` should return a GrammarParser that can
    /// successfully parse a simple Brainfuck program.
    #[test]
    fn test_create_parser() {
        let mut p = create_brainfuck_parser("+");
        let result = p.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());
        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 13: Deeply nested loops
    // -----------------------------------------------------------------------

    /// "[[[+]]]" has 3 levels of nesting. The parser must handle arbitrary
    /// depth without stack overflow (within Rust's default stack limit).
    #[test]
    fn test_parse_deeply_nested() {
        let ast = parse_brainfuck("[[[+]]]").expect("[[[+]]] should parse");
        assert_program_root(&ast);
        let loop_count = count_rule(&ast, "loop");
        assert_eq!(loop_count, 3, "Expected 3 nested loops");
    }

    // -----------------------------------------------------------------------
    // Test 14: Complex program — Hello World structure
    // -----------------------------------------------------------------------

    /// A structurally complex program (not full Hello World — that's very long —
    /// but a multi-loop, multi-command snippet that exercises the grammar).
    #[test]
    fn test_parse_complex_program() {
        // Set cell to 5, loop 5 times (decrement and move right), end
        let ast = parse_brainfuck("+++++[->+<]").expect("complex program should parse");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "loop"), "Expected at least one loop");
    }
}
