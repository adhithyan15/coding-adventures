//! Verilog parser backed by compiled parser grammar.

use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

pub const DEFAULT_VERSION: &str = coding_adventures_verilog_lexer::DEFAULT_VERSION;
pub const SUPPORTED_VERSIONS: &[&str] = _grammar::SUPPORTED_VERSIONS;

fn validate_version(version: &str) -> Result<&str, String> {
    if SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown Verilog version '{version}'. Valid values: {}",
            SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_verilog_parser(source: &str) -> GrammarParser {
    create_verilog_parser_with_version(source, DEFAULT_VERSION)
        .expect("compiled Verilog parser grammar missing default version")
}

pub fn create_verilog_parser_with_version(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = coding_adventures_verilog_lexer::tokenize_verilog_with_version(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled Verilog parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar))
}

pub fn parse_verilog(source: &str) -> GrammarASTNode {
    parse_verilog_with_version(source, DEFAULT_VERSION)
        .expect("compiled Verilog parser grammar missing default version")
}

pub fn parse_verilog_with_version(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_verilog_parser_with_version(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("Verilog parse failed: {e}"))
}

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

    #[test]
    fn test_default_version_matches_explicit_2005() {
        let default_ast = parse_verilog("module empty; endmodule");
        let explicit_ast =
            parse_verilog_with_version("module empty; endmodule", "2005").unwrap();
        assert_eq!(default_ast.rule_name, explicit_ast.rule_name);
    }

    #[test]
    fn test_unknown_version_rejected() {
        let err = parse_verilog_with_version("module empty; endmodule", "2099")
            .expect_err("unknown versions should be rejected");
        assert!(err.contains("Unknown Verilog version"));
    }
}

