# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_ruby_parser"

# ================================================================
# Tests for the Ruby Parser
# ================================================================
#
# The Ruby parser transforms a token stream into an AST using
# the grammar-driven parser engine loaded with ruby.grammar.
# ================================================================

class TestRubyParser < Minitest::Test
  def parse(source) = CodingAdventures::RubyParser.parse(source)

  def find_nodes(node, rule_name)
    results = []
    results << node if node.respond_to?(:rule_name) && node.rule_name == rule_name
    children = node.respond_to?(:children) ? node.children : []
    children.each { |c| results.concat(find_nodes(c, rule_name)) }
    results
  end

  def find_tokens(node, type_str)
    results = []
    if node.respond_to?(:type) && node.type.to_s == type_str
      results << node
    elsif node.respond_to?(:children)
      node.children.each { |c| results.concat(find_tokens(c, type_str)) }
    end
    results
  end

  def test_version_exists
    refute_nil CodingAdventures::RubyParser::VERSION
  end

  def test_program_root
    ast = parse("x = 1 + 2")
    assert_equal "program", ast.rule_name
  end

  def test_simple_assignment
    ast = parse("x = 42")
    assert_equal "program", ast.rule_name
    refute_empty ast.children
  end

  def test_arithmetic
    ast = parse("1 + 2")
    assert_equal "program", ast.rule_name
  end

  def test_method_call
    ast = parse("puts 42")
    assert_equal "program", ast.rule_name
  end

  def test_multiple_statements
    ast = parse("x = 1\ny = 2")
    assert_equal "program", ast.rule_name
    refute_empty ast.children
  end

  def test_empty_program
    ast = parse("")
    assert_equal "program", ast.rule_name
  end

  # Regression tests for statement types that were silently unparseable
  # under a stale compiled grammar (_grammar.rb had drifted far behind
  # ruby.grammar -- def/class/if/while/case/begin were all missing rule
  # references). These lock in that the compiled grammar stays in sync.
  def test_def_statement
    ast = parse("def greet(name)\n  puts name\nend")
    assert_equal "program", ast.rule_name
    refute_empty find_nodes(ast, "def_statement")
  end

  def test_class_statement
    ast = parse("class Foo\n  def bar\n  end\nend")
    assert_equal "program", ast.rule_name
    refute_empty find_nodes(ast, "class_statement")
  end

  def test_if_statement
    ast = parse("if x\n  y = 1\nend")
    assert_equal "program", ast.rule_name
    refute_empty find_nodes(ast, "if_statement")
  end

  def test_while_statement
    ast = parse("while x\n  y = 1\nend")
    assert_equal "program", ast.rule_name
    refute_empty find_nodes(ast, "while_statement")
  end

  def test_case_statement
    ast = parse("case x\nwhen 1\n  y = 1\nend")
    assert_equal "program", ast.rule_name
    refute_empty find_nodes(ast, "case_statement")
  end

  def test_begin_statement
    ast = parse("begin\n  x = 1\nend")
    assert_equal "program", ast.rule_name
    refute_empty find_nodes(ast, "begin_statement")
  end
end
