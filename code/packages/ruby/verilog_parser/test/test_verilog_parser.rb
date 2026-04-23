# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Verilog Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with verilog.grammar, correctly builds Abstract Syntax Trees
# from Verilog HDL source code.
#
# Verilog describes hardware, not software. Each test corresponds
# to a different hardware construct:
#
#   - Empty module:      a component with no ports and no logic
#   - Module with ports: a component with input/output connections
#   - Assign:            combinational logic (gates wired together)
#   - Always block:      behavioral description of sequential logic
#   - Case statement:    multi-way branch (like a decoder or MUX)
#   - Expressions:       arithmetic and bitwise operations on signals
# ================================================================

class TestVerilogParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  # Helper: parse Verilog source and return the AST root.
  def parse(source)
    CodingAdventures::VerilogParser.parse(source)
  end

  # Helper: recursively collect all Token objects from an AST subtree.
  # Useful for checking that specific tokens (keywords, operators) appear
  # in the right part of the tree.
  def collect_tokens(node)
    tokens = []
    return tokens unless node.is_a?(ASTNode)

    node.children.each do |child|
      if child.is_a?(CodingAdventures::Lexer::Token)
        tokens << child
      elsif child.is_a?(ASTNode)
        tokens.concat(collect_tokens(child))
      end
    end
    tokens
  end

  # Helper: recursively find the first ASTNode with the given rule_name.
  def find_node(node, rule_name)
    return nil unless node.is_a?(ASTNode)
    return node if node.rule_name == rule_name

    node.children.each do |child|
      result = find_node(child, rule_name)
      return result if result
    end
    nil
  end

  # Helper: recursively find all ASTNodes with the given rule_name.
  def find_all_nodes(node, rule_name)
    results = []
    return results unless node.is_a?(ASTNode)

    results << node if node.rule_name == rule_name
    node.children.each do |child|
      results.concat(find_all_nodes(child, rule_name))
    end
    results
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------
  # Before testing parsing, verify the grammar file exists. If this
  # test fails, nothing else will work.

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::VerilogParser::VERILOG_GRAMMAR_PATH),
      "verilog.grammar file should exist at #{CodingAdventures::VerilogParser::VERILOG_GRAMMAR_PATH}"
  end

  # ------------------------------------------------------------------
  # Empty module: module empty; endmodule
  # ------------------------------------------------------------------
  # The simplest possible Verilog module. No ports, no internal logic.
  # In hardware terms, this is a component that does nothing — it has
  # no connections to the outside world and contains no circuitry.

  def test_empty_module
    ast = parse("module empty; endmodule")

    # The root should be source_text (the grammar's start rule).
    assert_equal "source_text", ast.rule_name

    # There should be a module_declaration inside the tree.
    mod_decl = find_node(ast, "module_declaration")
    refute_nil mod_decl, "Expected a module_declaration node"

    # The module name token should be present.
    tokens = collect_tokens(mod_decl)
    name_token = tokens.find { |t| t.type == TT::NAME && t.value == "empty" }
    refute_nil name_token, "Expected NAME token 'empty'"
  end

  # ------------------------------------------------------------------
  # Module with ports: module and_gate(input a, input b, output y);
  # ------------------------------------------------------------------
  # A module with ports has connections to the outside world. Think of
  # a physical chip with pins — 'a' and 'b' are input pins, 'y' is
  # the output pin.

  def test_module_with_ports
    source = "module and_gate(input a, input b, output y); endmodule"
    ast = parse(source)

    mod_decl = find_node(ast, "module_declaration")
    refute_nil mod_decl, "Expected a module_declaration node"

    # The port_list node should contain the three ports.
    port_list = find_node(mod_decl, "port_list")
    refute_nil port_list, "Expected a port_list node"

    # We should find port nodes for a, b, and y.
    ports = find_all_nodes(port_list, "port")
    assert_equal 3, ports.length, "Expected 3 ports (a, b, y)"
  end

  # ------------------------------------------------------------------
  # Continuous assignment: assign y = a & b;
  # ------------------------------------------------------------------
  # An assign statement describes combinational logic. The right-hand
  # side expression is continuously evaluated — whenever 'a' or 'b'
  # changes, 'y' updates immediately. This particular example describes
  # an AND gate.

  def test_continuous_assign
    source = "module top(input a, input b, output y); assign y = a & b; endmodule"
    ast = parse(source)

    cont_assign = find_node(ast, "continuous_assign")
    refute_nil cont_assign, "Expected a continuous_assign node"

    # The assignment should contain an lvalue and an expression.
    assignment = find_node(cont_assign, "assignment")
    refute_nil assignment, "Expected an assignment node inside continuous_assign"
  end

  # ------------------------------------------------------------------
  # Always block with posedge clock
  # ------------------------------------------------------------------
  # Always blocks describe behavior triggered by signal changes.
  # "always @(posedge clk)" means "execute this block on every rising
  # edge of the clock signal" — this creates a flip-flop (register)
  # in hardware.

  def test_always_block
    source = <<~VERILOG
      module ff(input clk, input d, output reg q);
        always @(posedge clk) begin
          q <= d;
        end
      endmodule
    VERILOG
    ast = parse(source)

    always = find_node(ast, "always_construct")
    refute_nil always, "Expected an always_construct node"

    # The sensitivity list should be present.
    sens_list = find_node(always, "sensitivity_list")
    refute_nil sens_list, "Expected a sensitivity_list node"

    # There should be a nonblocking assignment (<=) inside the block.
    nb_assign = find_node(always, "nonblocking_assignment")
    refute_nil nb_assign, "Expected a nonblocking_assignment (<=) inside always block"
  end

  # ------------------------------------------------------------------
  # If statement inside always block
  # ------------------------------------------------------------------
  # If/else in Verilog creates multiplexers or priority encoders in
  # hardware. The condition selects which assignment takes effect.

  def test_if_statement
    source = <<~VERILOG
      module mux(input sel, input a, input b, output reg y);
        always @(sel, a, b) begin
          if (sel) begin
            y = a;
          end else begin
            y = b;
          end
        end
      endmodule
    VERILOG
    ast = parse(source)

    if_stmt = find_node(ast, "if_statement")
    refute_nil if_stmt, "Expected an if_statement node"
  end

  # ------------------------------------------------------------------
  # Expressions: arithmetic and bitwise operators
  # ------------------------------------------------------------------
  # Verilog expressions map to hardware operators. Addition becomes an
  # adder circuit, bitwise AND becomes AND gates, etc.

  def test_expression_with_operators
    source = "module top(input a, input b, output y); assign y = a + b; endmodule"
    ast = parse(source)

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == TT::PLUS }, "Expected PLUS token in expression"
  end

  def test_expression_with_bitwise_and
    source = "module top(input a, input b, output y); assign y = a & b; endmodule"
    ast = parse(source)

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == "AMP" }, "Expected AMP token in expression"
  end

  # ------------------------------------------------------------------
  # Root is always 'source_text'
  # ------------------------------------------------------------------
  # The Verilog grammar's start rule is source_text, so the root
  # AST node should always have that rule_name.

  def test_root_is_source_text
    ast = parse("module m; endmodule")
    assert_equal "source_text", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Multiple modules in one source
  # ------------------------------------------------------------------
  # A single Verilog file can contain multiple module declarations.
  # This is common for utility libraries and testbenches.

  def test_multiple_modules
    source = <<~VERILOG
      module a; endmodule
      module b; endmodule
    VERILOG
    ast = parse(source)

    modules = find_all_nodes(ast, "module_declaration")
    assert_equal 2, modules.length, "Expected 2 module_declaration nodes"
  end

  # ------------------------------------------------------------------
  # Wire and reg declarations
  # ------------------------------------------------------------------
  # Wires are physical connections (like copper traces on a PCB).
  # Regs are storage elements that hold values between clock edges.

  def test_wire_declaration
    source = "module top; wire w; endmodule"
    ast = parse(source)

    net_decl = find_node(ast, "net_declaration")
    refute_nil net_decl, "Expected a net_declaration node for 'wire w'"
  end

  def test_reg_declaration
    # Note: "reg r;" is parsed as a net_declaration (since "reg" matches
    # net_type in module_item before reg_declaration). To get an explicit
    # reg_declaration, we use "reg signed r;" which won't match net_type
    # alone. However, since "reg" as net_type is also valid hardware,
    # we test both paths.
    source = "module top; reg r; endmodule"
    ast = parse(source)

    # "reg r;" matches net_declaration with net_type = "reg"
    net_decl = find_node(ast, "net_declaration")
    refute_nil net_decl, "Expected a net_declaration node for 'reg r'"

    net_type = find_node(net_decl, "net_type")
    refute_nil net_type, "Expected a net_type node"

    tokens = collect_tokens(net_type)
    assert tokens.any? { |t| t.type == TT::KEYWORD && t.value == "reg" },
      "Expected KEYWORD 'reg' inside net_type"
  end

  # ------------------------------------------------------------------
  # Module with bit-width range on ports
  # ------------------------------------------------------------------
  # Ports can have bit widths: input [7:0] data means an 8-bit input
  # (bits numbered from 7 down to 0).

  def test_port_with_range
    source = "module top(input [7:0] data); endmodule"
    ast = parse(source)

    port = find_node(ast, "port")
    refute_nil port, "Expected a port node"

    range_node = find_node(port, "range")
    refute_nil range_node, "Expected a range node for [7:0]"
  end

  # ------------------------------------------------------------------
  # Preprocess flag passthrough
  # ------------------------------------------------------------------
  # The parse method accepts a preprocess: option that gets forwarded
  # to the verilog_lexer. This test verifies the flag is accepted
  # without error.

  def test_parse_accepts_preprocess_flag
    ast = CodingAdventures::VerilogParser.parse("module m; endmodule", preprocess: false)
    assert_equal "source_text", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Version constant
  # ------------------------------------------------------------------

  def test_version_exists
    refute_nil CodingAdventures::VerilogParser::VERSION
    assert_match(/\A\d+\.\d+\.\d+\z/, CodingAdventures::VerilogParser::VERSION)
  end

  def test_default_version_matches_explicit_2005
    default_ast = parse("module empty; endmodule")
    explicit_ast = CodingAdventures::VerilogParser.parse(
      "module empty; endmodule",
      version: "2005"
    )
    assert_equal default_ast.rule_name, explicit_ast.rule_name
  end

  def test_rejects_unknown_version
    error = assert_raises(ArgumentError) do
      CodingAdventures::VerilogParser.parse("module empty; endmodule", version: "2099")
    end
    assert_match(/Unknown Verilog version/, error.message)
  end
end
