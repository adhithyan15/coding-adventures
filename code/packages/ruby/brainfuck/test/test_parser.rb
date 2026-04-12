# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Brainfuck Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with brainfuck.grammar, correctly builds Abstract Syntax Trees
# from Brainfuck token streams.
#
# The grammar-driven parser produces generic ASTNode objects:
#
#   ASTNode(rule_name: "program", children: [...])
#
# Each node records which grammar rule produced it. The tree shape
# mirrors the grammar's recursive structure:
#
#   program     = { instruction }
#   instruction = loop | command
#   loop        = LOOP_START { instruction } LOOP_END
#   command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT
#
# A key property of Brainfuck's grammar: loops are RECURSIVE.
# The parser's "loop" rule contains { instruction }, which can
# contain more loops. This creates an arbitrarily deep AST.
#
# We are not testing the parser engine itself (tested in the
# parser gem) — we are testing that brainfuck.grammar correctly
# describes Brainfuck's structural syntax.
# ================================================================

class TestBrainfuckParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  # Brainfuck token types
  RIGHT_TYPE      = "RIGHT"
  LEFT_TYPE       = "LEFT"
  INC_TYPE        = "INC"
  DEC_TYPE        = "DEC"
  OUTPUT_TYPE     = "OUTPUT"
  INPUT_TYPE      = "INPUT"
  LOOP_START_TYPE = "LOOP_START"
  LOOP_END_TYPE   = "LOOP_END"

  # ------------------------------------------------------------------
  # Helpers
  # ------------------------------------------------------------------

  def parse(source)
    CodingAdventures::Brainfuck::Parser.parse(source)
  end

  # Recursively collect all Token leaf objects from an AST.
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

  # Find all ASTNode descendants with a given rule_name (depth-first).
  def find_nodes(node, rule_name)
    results = []
    return results unless node.is_a?(ASTNode)

    results << node if node.rule_name == rule_name
    node.children.each do |child|
      results.concat(find_nodes(child, rule_name)) if child.is_a?(ASTNode)
    end
    results
  end

  # ------------------------------------------------------------------
  # Empty program
  # ------------------------------------------------------------------
  # An empty Brainfuck program is a valid "program" with no instructions.
  # The grammar rule is:  program = { instruction }
  # The { } means "zero or more", so empty is legal.

  def test_empty_program_returns_ast
    ast = parse("")
    assert_instance_of ASTNode, ast,
      "Parsing empty string should return an ASTNode"
  end

  def test_empty_program_rule_name
    ast = parse("")
    assert_equal "program", ast.rule_name,
      "Root node should have rule_name 'program'"
  end

  def test_empty_program_has_no_command_tokens
    ast = parse("")
    tokens = collect_tokens(ast)
    # Only EOF (which the parser may include) — no command tokens
    command_tokens = tokens.reject { |t| t.type == TT::EOF }
    assert_empty command_tokens, "Empty program should have no command tokens"
  end

  # ------------------------------------------------------------------
  # Simple command sequences
  # ------------------------------------------------------------------
  # A flat sequence of commands with no loops should parse into a
  # "program" node whose instructions each wrap a single command token.

  def test_single_increment
    ast = parse("+")
    assert_equal "program", ast.rule_name
    # There should be at least one "command" node in the tree
    commands = find_nodes(ast, "command")
    refute_empty commands, "'+' should produce at least one 'command' node"
  end

  def test_right_command
    ast = parse(">")
    commands = find_nodes(ast, "command")
    refute_empty commands
    # The command's leaf token should be RIGHT
    leaf_tokens = collect_tokens(ast).reject { |t| t.type == TT::EOF }
    assert_equal 1, leaf_tokens.length
    assert_equal RIGHT_TYPE, leaf_tokens[0].type
  end

  def test_all_six_non_bracket_commands
    # Parse all six non-loop commands in sequence.
    ast = parse("><+-.,")
    leaf_tokens = collect_tokens(ast).reject { |t| t.type == TT::EOF }
    types = leaf_tokens.map(&:type)
    assert_equal [RIGHT_TYPE, LEFT_TYPE, INC_TYPE, DEC_TYPE, OUTPUT_TYPE, INPUT_TYPE], types
  end

  def test_command_values_preserved
    # The token values must survive the parse unchanged.
    ast = parse("+-")
    leaf_tokens = collect_tokens(ast).reject { |t| t.type == TT::EOF }
    values = leaf_tokens.map(&:value)
    assert_equal ["+", "-"], values
  end

  # ------------------------------------------------------------------
  # Loop structure
  # ------------------------------------------------------------------
  # A loop must produce a "loop" node in the tree. The loop rule is:
  #   loop = LOOP_START { instruction } LOOP_END
  #
  # The LOOP_START and LOOP_END tokens appear as children of the
  # "loop" node (they are not stripped — the grammar includes them
  # to allow the parser to verify matching brackets).

  def test_empty_loop_parses
    # [] is a valid Brainfuck idiom — it clears a cell (no-op if zero).
    ast = parse("[]")
    assert_instance_of ASTNode, ast
    loops = find_nodes(ast, "loop")
    assert_equal 1, loops.length, "[] should produce exactly one loop node"
  end

  def test_simple_loop_with_body
    # [+] has one INC inside the loop.
    ast = parse("[+]")
    loops = find_nodes(ast, "loop")
    assert_equal 1, loops.length

    # The loop node should contain a command for "+"
    commands_in_loop = find_nodes(loops[0], "command")
    refute_empty commands_in_loop
  end

  def test_nested_loops
    # [[+]] has an outer loop containing an inner loop.
    ast = parse("[[+]]")
    loops = find_nodes(ast, "loop")
    # Should find at least 2 loop nodes (outer and inner)
    assert loops.length >= 2, "[[+]] should have at least 2 loop nodes"
  end

  def test_loop_tokens_in_tree
    # The LOOP_START and LOOP_END tokens must be present in the AST.
    # This lets consumers of the tree know where each loop begins/ends.
    ast = parse("[+]")
    all_tokens = collect_tokens(ast).reject { |t| t.type == TT::EOF }
    types = all_tokens.map(&:type)
    assert_includes types, LOOP_START_TYPE, "Loop should have LOOP_START token"
    assert_includes types, LOOP_END_TYPE, "Loop should have LOOP_END token"
  end

  # ------------------------------------------------------------------
  # Unmatched bracket handling
  # ------------------------------------------------------------------
  # The grammar requires that every "[" has a matching "]". If the
  # parser cannot find a matching bracket, parsing must fail. The
  # grammar-driven parser raises an error in this case.

  def test_unmatched_open_bracket_raises
    # An extra "[" with no matching "]" is a parse error.
    assert_raises(StandardError) do
      parse("[+")
    end
  end

  def test_unmatched_close_bracket_raises
    # A "]" without a preceding "[" is also a parse error.
    assert_raises(StandardError) do
      parse("+]")
    end
  end

  # ------------------------------------------------------------------
  # Comments are stripped before parsing
  # ------------------------------------------------------------------
  # The lexer removes all non-command characters before the parser
  # runs. The parser should never see or choke on comments.

  def test_comments_stripped_before_parse
    # "add two" is a comment, only "++" are commands.
    ast = parse("++ add two")
    leaf_tokens = collect_tokens(ast).reject { |t| t.type == TT::EOF }
    assert_equal 2, leaf_tokens.length
    assert_equal [INC_TYPE, INC_TYPE], leaf_tokens.map(&:type)
  end

  # ------------------------------------------------------------------
  # Canonical Brainfuck example: ++[>+<-]
  # ------------------------------------------------------------------
  # This is the standard "copy cell" idiom. It reads:
  #   "Increment cell 0 twice, then loop: move right, increment cell 1,
  #    move left, decrement cell 0. Exit when cell 0 is zero."
  #
  # Expected AST structure:
  #   program
  #     instruction → command (INC)
  #     instruction → command (INC)
  #     instruction → loop
  #       LOOP_START
  #       instruction → command (RIGHT)
  #       instruction → command (INC)
  #       instruction → command (LEFT)
  #       instruction → command (DEC)
  #       LOOP_END

  def test_canonical_plus_plus_loop_parses
    ast = parse("++[>+<-]")
    assert_equal "program", ast.rule_name
  end

  def test_canonical_plus_plus_loop_token_sequence
    ast = parse("++[>+<-]")
    leaf_tokens = collect_tokens(ast).reject { |t| t.type == TT::EOF }
    types = leaf_tokens.map(&:type)
    # The token sequence must match the original source order exactly.
    expected = [
      INC_TYPE, INC_TYPE,
      LOOP_START_TYPE,
        RIGHT_TYPE, INC_TYPE,
        LEFT_TYPE, DEC_TYPE,
      LOOP_END_TYPE
    ]
    assert_equal expected, types
  end

  def test_canonical_has_one_loop_node
    ast = parse("++[>+<-]")
    loops = find_nodes(ast, "loop")
    assert_equal 1, loops.length, "++[>+<-] should have exactly one loop"
  end

  def test_canonical_has_correct_command_count
    ast = parse("++[>+<-]")
    commands = find_nodes(ast, "command")
    # ++  (2 commands outside loop) + >+<- (4 commands inside loop) = 6
    assert_equal 6, commands.length, "++[>+<-] should have 6 command nodes"
  end

  def test_deeply_nested_loops_parse
    # A program with three levels of nesting to verify recursion works.
    # +[+[+[+]]]
    ast = parse("+[+[+[+]]]")
    assert_equal "program", ast.rule_name
    loops = find_nodes(ast, "loop")
    assert_equal 3, loops.length, "Three nested loops should produce 3 loop nodes"
  end

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::Brainfuck::Parser::BF_GRAMMAR_PATH),
      "brainfuck.grammar should exist at #{CodingAdventures::Brainfuck::Parser::BF_GRAMMAR_PATH}"
  end
end
