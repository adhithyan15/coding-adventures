# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_lisp_parser"

# ================================================================
# Tests for the Lisp Parser
# ================================================================
#
# Parsing verifies that the 6-rule Lisp grammar correctly structures
# tokens into an AST. The key invariant: every parse produces a
# "program" root node containing one or more "sexpr" children.
# ================================================================

class TestLispParser < Minitest::Test
  def parse(source) = CodingAdventures::LispParser.parse(source)

  def find_nodes(node, rule_name)
    results = []
    results << node if node.respond_to?(:rule_name) && node.rule_name == rule_name
    children = node.respond_to?(:children) ? node.children : []
    children.each { |c| results.concat(find_nodes(c, rule_name)) }
    results
  end

  def test_version_exists
    refute_nil CodingAdventures::LispParser::VERSION
  end

  def test_program_root
    ast = parse("(+ 1 2)")
    assert_equal "program", ast.rule_name
  end

  def test_simple_list
    ast = parse("(+ 1 2)")
    lists = find_nodes(ast, "list")
    refute_empty lists
  end

  def test_atom_number
    ast = parse("42")
    atoms = find_nodes(ast, "atom")
    refute_empty atoms
    nums = atoms.select { |a| a.children.any? { |c| c.respond_to?(:value) && c.value == "42" } }
    refute_empty nums
  end

  def test_atom_symbol
    ast = parse("define")
    atoms = find_nodes(ast, "atom")
    refute_empty atoms
  end

  def test_nested_list
    ast = parse("(define x (+ 1 2))")
    lists = find_nodes(ast, "list")
    assert lists.size >= 2
  end

  def test_quoted_form
    ast = parse("'foo")
    quoted = find_nodes(ast, "quoted")
    refute_empty quoted
  end

  def test_multiple_sexprs
    ast = parse("(define x 1) (define y 2)")
    sexprs = find_nodes(ast, "sexpr")
    assert sexprs.size >= 2
  end

  def test_empty_list
    ast = parse("()")
    assert_equal "program", ast.rule_name
    lists = find_nodes(ast, "list")
    refute_empty lists
  end

  def test_program_with_no_sexprs
    ast = parse("")
    assert_equal "program", ast.rule_name
  end
end
