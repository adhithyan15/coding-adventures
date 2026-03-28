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
end
