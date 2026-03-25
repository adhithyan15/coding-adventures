//! # Verilog Parser -- parsing Verilog HDL source code into an AST.
//!
//! This crate is the second half of the Verilog front-end pipeline. Where
//! the `verilog-lexer` crate breaks source text into tokens, this crate
//! arranges those tokens into a tree that reflects the **structure** of the
//! hardware description -- an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing Verilog requires four cooperating components:
//!
//! ```text
//! Source code  ("module top; endmodule")
//!       |
//!       v
//! verilog-lexer        -> Vec<Token>
//!       |                [KEYWORD("module"), NAME("top"),
//!       |                 SEMICOLON(";"), KEYWORD("endmodule"), EOF]
//!       v
//! verilog.grammar      -> ParserGrammar (rules like "source_text = ...")
//!       |
//!       v
//! GrammarParser        -> GrammarASTNode tree
//!       |
//!       |                source_text
//!       |                  +-- description
//!       |                        +-- module_declaration
//!       |                              +-- KEYWORD("module")
//!       |                              +-- NAME("top")
//!       |                              +-- SEMICOLON(";")
//!       |                              +-- KEYWORD("endmodule")
//!       v
//! [future stages: synthesis, simulation]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the `verilog.grammar` file and provides two
//! public entry points.

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
use coding_adventures_verilog_lexer::tokenize_verilog;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `verilog.grammar` file.
///
/// Uses the same strategy as the verilog-lexer crate:
/// `env!("CARGO_MANIFEST_DIR")` gives us the compile-time path to this
/// crate's directory, and we navigate up to the shared `grammars/` directory.
///
/// ```text
/// code/
///   grammars/
///     verilog.grammar       <-- target file
///   packages/
///     rust/
///       verilog-parser/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/verilog.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for Verilog source code.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** -- uses `tokenize_verilog` from the verilog-lexer
///    crate to break the source into tokens.
///
/// 2. **Grammar loading** -- reads and parses the `verilog.grammar` file,
///    which defines rules for modules, ports, assignments, always blocks,
///    case statements, expressions, and more.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `verilog.grammar` file cannot be read or parsed.
/// - The source code fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_verilog_parser::create_verilog_parser;
///
/// let mut parser = create_verilog_parser("module top; endmodule");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_verilog_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the verilog-lexer.
    //
    // The lexer reads verilog.tokens and produces tokens like:
    //   KEYWORD("module"), NAME("top"), SEMICOLON(";"), KEYWORD("endmodule"), EOF
    let tokens = tokenize_verilog(source);

    // Step 2: Read the parser grammar from disk.
    //
    // The verilog.grammar file defines the syntactic structure of Verilog
    // in EBNF notation. It covers everything from module declarations to
    // full expression precedence with ternary, logical, bitwise, shift,
    // arithmetic, and unary operators.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read verilog.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    //
    // The ParserGrammar contains rule definitions like:
    //   source_text = { description }
    //   module_declaration = "module" NAME [ port_list ] SEMICOLON ...
    //   expression = ternary_expr
    //   etc.
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse verilog.grammar: {e}"));

    // Step 4: Create the parser, ready to produce an AST.
    GrammarParser::new(tokens, grammar)
}

/// Parse Verilog source code into an AST.
///
/// This is the most convenient entry point -- it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"source_text"` (the
/// start symbol of the Verilog grammar) with children corresponding
/// to the module declarations in the source.
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source code has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_verilog_parser::parse_verilog;
///
/// let ast = parse_verilog("module top; endmodule");
/// assert_eq!(ast.rule_name, "source_text");
/// ```
pub fn parse_verilog(source: &str) -> GrammarASTNode {
    let mut verilog_parser = create_verilog_parser(source);

    verilog_parser
        .parse()
        .unwrap_or_else(|e| panic!("Verilog parse failed: {e}"))
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

    /// Assert that the root node of the AST is the `source_text` rule.
    ///
    /// Every valid Verilog file parses to a `source_text` node at the top.
    /// This is the grammar's start symbol, analogous to `program` in
    /// programming language grammars.
    fn assert_source_text_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "source_text",
            "Expected root rule 'source_text', got '{}'",
            ast.rule_name
        );
    }

    /// Count the number of `description` children (each wraps a module).
    ///
    /// In the Verilog grammar:
    ///   source_text = { description }
    ///   description = module_declaration
    ///
    /// So counting `description` nodes tells us how many modules were parsed.
    fn count_descriptions(ast: &GrammarASTNode) -> usize {
        ast.children
            .iter()
            .filter(|child| {
                matches!(child, ASTNodeOrToken::Node(n) if n.rule_name == "description")
            })
            .count()
    }

    /// Recursively search the AST for a node with the given rule name.
    ///
    /// This is useful for verifying that a particular grammar construct
    /// (like `always_construct` or `continuous_assign`) was recognized
    /// somewhere in the tree, without caring about the exact tree shape.
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

    /// Recursively search the AST for a token with the given value.
    ///
    /// Useful for checking that a specific keyword or identifier appears
    /// in the parsed tree.
    fn find_token_value(node: &GrammarASTNode, target_value: &str) -> bool {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == target_value => return true,
                ASTNodeOrToken::Node(n) if find_token_value(n, target_value) => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    // -----------------------------------------------------------------------
    // Test 1: Empty module
    // -----------------------------------------------------------------------

    /// The simplest possible Verilog module: no ports, no body.
    ///
    /// ```verilog
    /// module empty; endmodule
    /// ```
    ///
    /// This should parse to:
    ///   source_text
    ///     +-- description
    ///           +-- module_declaration
    ///                 +-- KEYWORD("module")
    ///                 +-- NAME("empty")
    ///                 +-- SEMICOLON(";")
    ///                 +-- KEYWORD("endmodule")
    #[test]
    fn test_parse_empty_module() {
        let ast = parse_verilog("module empty; endmodule");
        assert_source_text_root(&ast);

        let desc_count = count_descriptions(&ast);
        assert_eq!(desc_count, 1, "Expected 1 module, got {}", desc_count);

        // The module name "empty" should appear as a token in the tree.
        assert!(
            find_token_value(&ast, "empty"),
            "Expected to find module name 'empty' in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Module with ports
    // -----------------------------------------------------------------------

    /// A module with input and output ports exercises the port_list and
    /// port grammar rules.
    ///
    /// ```verilog
    /// module adder(input a, input b, output sum);
    /// endmodule
    /// ```
    ///
    /// The parser must recognize:
    ///   - port_list containing three ports
    ///   - port_direction keywords (input, output)
    ///   - port names (a, b, sum)
    #[test]
    fn test_parse_module_with_ports() {
        let ast = parse_verilog(
            "module adder(input a, input b, output sum); endmodule",
        );
        assert_source_text_root(&ast);

        // Verify the module_declaration and port_list rules are present.
        assert!(
            find_rule(&ast, "module_declaration"),
            "Expected module_declaration rule in AST"
        );
        assert!(
            find_rule(&ast, "port_list"),
            "Expected port_list rule in AST"
        );

        // Port names should be in the tree.
        assert!(find_token_value(&ast, "adder"), "Expected module name 'adder'");
        assert!(find_token_value(&ast, "a"), "Expected port 'a'");
        assert!(find_token_value(&ast, "b"), "Expected port 'b'");
        assert!(find_token_value(&ast, "sum"), "Expected port 'sum'");
    }

    // -----------------------------------------------------------------------
    // Test 3: Continuous assign statement
    // -----------------------------------------------------------------------

    /// A continuous assignment models combinational logic -- the output
    /// is always a function of the current inputs.
    ///
    /// ```verilog
    /// module and_gate(input a, input b, output y);
    ///   assign y = a & b;
    /// endmodule
    /// ```
    ///
    /// The parser should produce a `continuous_assign` node containing
    /// an `assignment` with lvalue `y` and an expression.
    #[test]
    fn test_parse_assign() {
        let source = "\
module and_gate(input a, input b, output y);
  assign y = a & b;
endmodule";

        let ast = parse_verilog(source);
        assert_source_text_root(&ast);

        assert!(
            find_rule(&ast, "continuous_assign"),
            "Expected continuous_assign rule in AST"
        );
        assert!(
            find_rule(&ast, "assignment"),
            "Expected assignment rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Always block
    // -----------------------------------------------------------------------

    /// An always block with a sensitivity list describes sequential or
    /// combinational behavior. This test uses `posedge clk` which models
    /// a flip-flop -- logic that triggers on the rising edge of the clock.
    ///
    /// ```verilog
    /// module ff(input clk, input d, output reg q);
    ///   always @(posedge clk)
    ///     q <= d;
    /// endmodule
    /// ```
    #[test]
    fn test_parse_always_block() {
        let source = "\
module ff(input clk, input d, output reg q);
  always @(posedge clk)
    q <= d;
endmodule";

        let ast = parse_verilog(source);
        assert_source_text_root(&ast);

        assert!(
            find_rule(&ast, "always_construct"),
            "Expected always_construct rule in AST"
        );
        assert!(
            find_rule(&ast, "sensitivity_list"),
            "Expected sensitivity_list rule in AST"
        );
        assert!(
            find_rule(&ast, "nonblocking_assignment"),
            "Expected nonblocking_assignment rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Case statement
    // -----------------------------------------------------------------------

    /// A case statement is Verilog's multi-way branch, commonly used for
    /// instruction decoders and multiplexers.
    ///
    /// ```verilog
    /// module decoder(input [1:0] sel, output reg [3:0] y);
    ///   always @(*)
    ///     case (sel)
    ///       2'b00: y = 4'b0001;
    ///       2'b01: y = 4'b0010;
    ///       default: y = 4'b0000;
    ///     endcase
    /// endmodule
    /// ```
    #[test]
    fn test_parse_case_statement() {
        let source = "\
module decoder(input wire sel, output reg y);
  always @(*)
    case (sel)
      1: y = 1;
      0: y = 0;
      default: y = 0;
    endcase
endmodule";

        let ast = parse_verilog(source);
        assert_source_text_root(&ast);

        assert!(
            find_rule(&ast, "case_statement"),
            "Expected case_statement rule in AST"
        );
        assert!(
            find_rule(&ast, "case_item"),
            "Expected case_item rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Expressions
    // -----------------------------------------------------------------------

    /// Verify that expressions with various operators parse correctly.
    /// The grammar defines a full operator precedence tower from ternary
    /// at the top down through logical, bitwise, equality, relational,
    /// shift, additive, multiplicative, power, and unary operators.
    #[test]
    fn test_parse_expressions() {
        let source = "\
module expr_test(input a, input b, output y);
  assign y = a + b;
endmodule";

        let ast = parse_verilog(source);
        assert_source_text_root(&ast);

        // The expression should be parsed into the precedence tower.
        // At minimum, additive_expr should appear for the `+` operator.
        assert!(
            find_rule(&ast, "expression"),
            "Expected expression rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Multiple modules
    // -----------------------------------------------------------------------

    /// A Verilog source file can contain multiple modules. Each becomes
    /// a separate `description` child under `source_text`.
    #[test]
    fn test_parse_multiple_modules() {
        let source = "\
module mod_a; endmodule
module mod_b; endmodule
module mod_c; endmodule";

        let ast = parse_verilog(source);
        assert_source_text_root(&ast);

        let desc_count = count_descriptions(&ast);
        assert_eq!(desc_count, 3, "Expected 3 modules, got {}", desc_count);
    }

    // -----------------------------------------------------------------------
    // Test 8: Empty source
    // -----------------------------------------------------------------------

    /// An empty source file should parse to a source_text node with
    /// no children. This is valid Verilog (a file with only comments,
    /// for example).
    #[test]
    fn test_parse_empty_source() {
        let ast = parse_verilog("");
        assert_source_text_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 9: Factory function
    // -----------------------------------------------------------------------

    /// The `create_verilog_parser` factory function should return a
    /// working `GrammarParser` that can be called manually.
    #[test]
    fn test_create_parser() {
        let mut parser = create_verilog_parser("module top; endmodule");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "source_text");
    }

    // -----------------------------------------------------------------------
    // Test 10: Wire and reg declarations
    // -----------------------------------------------------------------------

    /// Wire and reg declarations as module items should be recognized.
    ///
    /// ```verilog
    /// module decl_test;
    ///   wire a;
    ///   reg b;
    /// endmodule
    /// ```
    ///
    /// Note: in the Verilog grammar, `net_type` includes both `wire` and
    /// `reg`, so `reg b;` is parsed as a `net_declaration` (since that
    /// alternative comes before `reg_declaration` in `module_item`).
    /// Both declarations are structurally equivalent for parsing purposes.
    #[test]
    fn test_parse_wire_and_reg_declarations() {
        let source = "\
module decl_test;
  wire a;
  reg b;
endmodule";

        let ast = parse_verilog(source);
        assert_source_text_root(&ast);

        // Both wire and reg are parsed as net_declaration because net_type
        // includes "wire" | "reg" | "tri" | "supply0" | "supply1", and
        // net_declaration comes before reg_declaration in the module_item
        // alternation order.
        assert!(
            find_rule(&ast, "net_declaration"),
            "Expected net_declaration rule"
        );
        assert!(
            find_token_value(&ast, "a"),
            "Expected identifier 'a'"
        );
        assert!(
            find_token_value(&ast, "b"),
            "Expected identifier 'b'"
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: If/else inside always block
    // -----------------------------------------------------------------------

    /// An if/else statement inside an always block models a multiplexer
    /// or conditional logic.
    #[test]
    fn test_parse_if_else() {
        let source = "\
module mux(input a, input b, input sel, output reg y);
  always @(*)
    if (sel)
      y = b;
    else
      y = a;
endmodule";

        let ast = parse_verilog(source);
        assert_source_text_root(&ast);

        assert!(
            find_rule(&ast, "if_statement"),
            "Expected if_statement rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 12: Begin/end block
    // -----------------------------------------------------------------------

    /// A begin/end block groups multiple statements, like curly braces
    /// in C.
    #[test]
    fn test_parse_begin_end_block() {
        let source = "\
module blk(input clk, input d, output reg q);
  always @(posedge clk) begin
    q <= d;
  end
endmodule";

        let ast = parse_verilog(source);
        assert_source_text_root(&ast);

        assert!(
            find_rule(&ast, "block_statement"),
            "Expected block_statement rule in AST"
        );
    }
}
