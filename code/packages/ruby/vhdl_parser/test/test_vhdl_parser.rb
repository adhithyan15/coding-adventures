# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the VHDL Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with vhdl.grammar, correctly builds Abstract Syntax Trees
# from VHDL source code.
#
# VHDL describes hardware, not software. Each test corresponds
# to a different hardware construct:
#
#   - Empty entity:        a component with no ports and no logic
#   - Entity with ports:   a component with input/output connections
#   - Architecture:        the implementation body for an entity
#   - Signal assignment:   connecting signals with <=
#   - Process:             sequential region inside concurrent world
#   - If/elsif/else:       multiplexers and priority encoders
#   - Expressions:         arithmetic and logical operations on signals
#
# Key difference from Verilog: VHDL separates interface (entity)
# from implementation (architecture). A Verilog module contains
# both in a single declaration, but VHDL requires you to declare
# the entity first, then provide one or more architectures.
# ================================================================

class TestVhdlParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  # Helper: parse VHDL source and return the AST root.
  def parse(source)
    CodingAdventures::VhdlParser.parse(source)
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
    assert File.exist?(CodingAdventures::VhdlParser::VHDL_GRAMMAR_PATH),
      "vhdl.grammar file should exist at #{CodingAdventures::VhdlParser::VHDL_GRAMMAR_PATH}"
  end

  # ------------------------------------------------------------------
  # Empty entity: entity empty is end entity empty;
  # ------------------------------------------------------------------
  # The simplest possible VHDL design unit. An entity with no ports
  # and no generics. In hardware terms, this is a component with no
  # connections to the outside world — it declares an interface with
  # zero pins.
  #
  # Unlike Verilog's "module empty; endmodule", VHDL requires the
  # explicit "is" keyword and an "end" clause that can optionally
  # repeat the entity name (for readability and error checking).

  def test_empty_entity
    ast = parse("entity empty is end entity empty;")

    # The root should be design_file (the grammar's start rule).
    assert_equal "design_file", ast.rule_name

    # There should be an entity_declaration inside the tree.
    entity_decl = find_node(ast, "entity_declaration")
    refute_nil entity_decl, "Expected an entity_declaration node"

    # The entity name token should be present (lowercased by the lexer).
    tokens = collect_tokens(entity_decl)
    name_token = tokens.find { |t| t.type == TT::NAME && t.value == "empty" }
    refute_nil name_token, "Expected NAME token 'empty'"
  end

  # ------------------------------------------------------------------
  # Entity with ports
  # ------------------------------------------------------------------
  # An entity with ports has connections to the outside world. In VHDL,
  # each port must declare its direction (in, out, inout, buffer) and
  # its type. This is more verbose than Verilog but catches type
  # mismatches at compile time.
  #
  #   entity and_gate is
  #     port (
  #       a, b : in std_logic;
  #       y    : out std_logic
  #     );
  #   end entity and_gate;
  #
  # Note: VHDL uses semicolons between port elements (not commas),
  # and the last element has no trailing semicolon.

  def test_entity_with_ports
    source = <<~VHDL
      entity and_gate is
        port (
          a, b : in std_logic;
          y : out std_logic
        );
      end entity and_gate;
    VHDL
    ast = parse(source)

    entity_decl = find_node(ast, "entity_declaration")
    refute_nil entity_decl, "Expected an entity_declaration node"

    # The port_clause node should be present.
    port_clause = find_node(entity_decl, "port_clause")
    refute_nil port_clause, "Expected a port_clause node"

    # The interface_list should contain interface_elements for the ports.
    interface_list = find_node(port_clause, "interface_list")
    refute_nil interface_list, "Expected an interface_list node"

    interface_elements = find_all_nodes(interface_list, "interface_element")
    assert_equal 2, interface_elements.length,
      "Expected 2 interface_elements (a,b : in std_logic and y : out std_logic)"
  end

  # ------------------------------------------------------------------
  # Architecture body
  # ------------------------------------------------------------------
  # An architecture provides the IMPLEMENTATION of an entity. VHDL
  # requires a separate entity and architecture — they are distinct
  # design units. The architecture references the entity by name
  # with the "of" keyword.
  #
  # This is like having a separate header file (.h) and implementation
  # file (.c) in C, except VHDL enforces it at the language level.

  def test_architecture_body
    source = <<~VHDL
      entity top is
      end entity top;

      architecture rtl of top is
      begin
      end architecture rtl;
    VHDL
    ast = parse(source)

    arch = find_node(ast, "architecture_body")
    refute_nil arch, "Expected an architecture_body node"

    # The architecture name "rtl" and entity name "top" should be present.
    tokens = collect_tokens(arch)
    assert tokens.any? { |t| t.type == TT::NAME && t.value == "rtl" },
      "Expected NAME token 'rtl' (architecture name)"
    assert tokens.any? { |t| t.type == TT::NAME && t.value == "top" },
      "Expected NAME token 'top' (entity reference)"
  end

  # ------------------------------------------------------------------
  # Signal assignment (concurrent)
  # ------------------------------------------------------------------
  # A concurrent signal assignment describes combinational logic.
  # The right-hand side is continuously evaluated — whenever any
  # signal on the right changes, the left-hand signal updates.
  #
  #   y <= a and b;
  #
  # The <= operator in VHDL is the signal assignment operator (not
  # "less than or equal" — that's only in expression context).

  def test_signal_assignment
    source = <<~VHDL
      entity top is
        port (a, b : in std_logic; y : out std_logic);
      end entity top;

      architecture rtl of top is
      begin
        y <= a and b;
      end architecture rtl;
    VHDL
    ast = parse(source)

    sig_assign = find_node(ast, "signal_assignment_concurrent")
    refute_nil sig_assign, "Expected a signal_assignment_concurrent node"

    # The assignment target "y" should be present.
    tokens = collect_tokens(sig_assign)
    assert tokens.any? { |t| t.type == TT::NAME && t.value == "y" },
      "Expected NAME token 'y' as assignment target"
  end

  # ------------------------------------------------------------------
  # Process statement
  # ------------------------------------------------------------------
  # A process is a sequential region inside the concurrent world.
  # Inside a process, statements execute top to bottom (like software).
  # But the process itself is concurrent with everything outside it.
  #
  # The sensitivity list specifies which signals trigger the process:
  #   process (clk) — re-evaluate when clk changes
  #
  # This creates sequential logic (flip-flops) in hardware.

  def test_process_statement
    source = <<~VHDL
      entity ff is
        port (clk, d : in std_logic; q : out std_logic);
      end entity ff;

      architecture rtl of ff is
      begin
        process (clk)
        begin
          q <= d;
        end process;
      end architecture rtl;
    VHDL
    ast = parse(source)

    proc_stmt = find_node(ast, "process_statement")
    refute_nil proc_stmt, "Expected a process_statement node"

    # The sensitivity list should be present.
    sens_list = find_node(proc_stmt, "sensitivity_list")
    refute_nil sens_list, "Expected a sensitivity_list node"

    # There should be a sequential signal assignment inside the process.
    sig_assign = find_node(proc_stmt, "signal_assignment_seq")
    refute_nil sig_assign, "Expected a signal_assignment_seq inside process"
  end

  # ------------------------------------------------------------------
  # If/elsif/else statement
  # ------------------------------------------------------------------
  # If/elsif/else in VHDL creates multiplexers or priority encoders
  # in hardware. The condition selects which assignment takes effect.
  #
  # VHDL uses "elsif" (one word) rather than "else if" (two words),
  # and every if chain must end with "end if;" (with a space).
  #
  #   if sel = "00" then
  #     y <= a;
  #   elsif sel = "01" then
  #     y <= b;
  #   else
  #     y <= c;
  #   end if;

  def test_if_elsif_else
    source = <<~VHDL
      entity mux is
        port (sel : in std_logic; a, b, c : in std_logic; y : out std_logic);
      end entity mux;

      architecture rtl of mux is
      begin
        process (sel, a, b, c)
        begin
          if sel = '0' then
            y <= a;
          elsif sel = '1' then
            y <= b;
          else
            y <= c;
          end if;
        end process;
      end architecture rtl;
    VHDL
    ast = parse(source)

    if_stmt = find_node(ast, "if_statement")
    refute_nil if_stmt, "Expected an if_statement node"

    # There should be sequential signal assignments inside the if branches.
    seq_assigns = find_all_nodes(if_stmt, "signal_assignment_seq")
    assert seq_assigns.length >= 3,
      "Expected at least 3 signal_assignment_seq nodes (one per branch)"
  end

  # ------------------------------------------------------------------
  # Expressions: logical and arithmetic operators
  # ------------------------------------------------------------------
  # VHDL expressions map to hardware operators. Addition becomes an
  # adder circuit, logical AND becomes AND gates, etc.
  #
  # A key difference from Verilog: VHDL uses keyword operators for
  # logic (and, or, xor, not) rather than symbol operators (&, |, ^, ~).

  def test_expression_with_addition
    source = <<~VHDL
      entity adder is
        port (a, b : in std_logic; y : out std_logic);
      end entity adder;

      architecture rtl of adder is
      begin
        y <= a + b;
      end architecture rtl;
    VHDL
    ast = parse(source)

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == TT::PLUS },
      "Expected PLUS token in expression"
  end

  def test_expression_with_logical_and
    source = <<~VHDL
      entity gate is
        port (a, b : in std_logic; y : out std_logic);
      end entity gate;

      architecture rtl of gate is
      begin
        y <= a and b;
      end architecture rtl;
    VHDL
    ast = parse(source)

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == TT::KEYWORD && t.value == "and" },
      "Expected KEYWORD 'and' in expression"
  end

  # ------------------------------------------------------------------
  # Root is always 'design_file'
  # ------------------------------------------------------------------
  # The VHDL grammar's start rule is design_file, so the root
  # AST node should always have that rule_name.

  def test_root_is_design_file
    ast = parse("entity e is end entity e;")
    assert_equal "design_file", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Signal declaration inside architecture
  # ------------------------------------------------------------------
  # Signals in VHDL are declared in the declarative region of an
  # architecture (between "is" and "begin"). Every signal must
  # have an explicit type — there are no implicit declarations.
  #
  #   signal temp : std_logic;

  def test_signal_declaration
    source = <<~VHDL
      entity top is
      end entity top;

      architecture rtl of top is
        signal temp : std_logic;
      begin
      end architecture rtl;
    VHDL
    ast = parse(source)

    sig_decl = find_node(ast, "signal_declaration")
    refute_nil sig_decl, "Expected a signal_declaration node"

    tokens = collect_tokens(sig_decl)
    assert tokens.any? { |t| t.type == TT::NAME && t.value == "temp" },
      "Expected NAME token 'temp' in signal declaration"
  end

  # ------------------------------------------------------------------
  # Case insensitivity
  # ------------------------------------------------------------------
  # VHDL is case-insensitive. "ENTITY", "Entity", and "entity" are
  # identical. The lexer normalizes everything to lowercase, so the
  # parser should handle uppercase source without issues.

  def test_case_insensitive
    ast = parse("ENTITY upper IS END ENTITY upper;")
    assert_equal "design_file", ast.rule_name

    entity_decl = find_node(ast, "entity_declaration")
    refute_nil entity_decl, "Expected entity_declaration even with uppercase keywords"
  end

  # ------------------------------------------------------------------
  # Version constant
  # ------------------------------------------------------------------

  def test_version_exists
    refute_nil CodingAdventures::VhdlParser::VERSION
    assert_match(/\A\d+\.\d+\.\d+\z/, CodingAdventures::VhdlParser::VERSION)
  end

  # ------------------------------------------------------------------
  # Multiple design units
  # ------------------------------------------------------------------
  # A single VHDL file typically contains an entity followed by its
  # architecture. Both are separate design units.

  def test_multiple_design_units
    source = <<~VHDL
      entity a is end entity a;
      entity b is end entity b;
    VHDL
    ast = parse(source)

    entities = find_all_nodes(ast, "entity_declaration")
    assert_equal 2, entities.length, "Expected 2 entity_declaration nodes"
  end

  # ------------------------------------------------------------------
  # Entity with end shorthand (no "entity" keyword in end clause)
  # ------------------------------------------------------------------
  # VHDL allows several forms for the end clause:
  #   end entity foo;    -- most explicit
  #   end foo;           -- omit entity keyword
  #   end;               -- most minimal

  def test_entity_end_shorthand
    ast = parse("entity minimal is end;")
    assert_equal "design_file", ast.rule_name

    entity_decl = find_node(ast, "entity_declaration")
    refute_nil entity_decl, "Expected entity_declaration with minimal end clause"
  end

  def test_default_version_matches_explicit_2008
    default_ast = parse("entity empty is end entity empty;")
    explicit_ast = CodingAdventures::VhdlParser.parse(
      "entity empty is end entity empty;",
      version: "2008"
    )
    assert_equal default_ast.rule_name, explicit_ast.rule_name
  end

  def test_rejects_unknown_version
    error = assert_raises(ArgumentError) do
      CodingAdventures::VhdlParser.parse("entity empty is end entity empty;", version: "2099")
    end
    assert_match(/Unknown VHDL version/, error.message)
  end
end
