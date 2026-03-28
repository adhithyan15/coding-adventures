# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_css_parser"

class TestCssParser < Minitest::Test
  def parse(source) = CodingAdventures::CssParser.parse(source)

  def find_nodes(node, rule_name)
    results = []
    results << node if node.respond_to?(:rule_name) && node.rule_name == rule_name
    children = node.respond_to?(:children) ? node.children : []
    children.each { |c| results.concat(find_nodes(c, rule_name)) }
    results
  end

  def test_version_exists
    refute_nil CodingAdventures::CssParser::VERSION
  end

  def test_stylesheet_root
    ast = parse("h1 { color: red; }")
    assert_equal "stylesheet", ast.rule_name
  end

  def test_simple_rule_has_children
    ast = parse("h1 { color: red; }")
    refute_empty ast.children
  end

  def test_empty_source
    ast = parse("")
    assert_equal "stylesheet", ast.rule_name
  end

  def test_multiple_rules
    ast = parse("h1 { color: red; } p { font-size: 16px; }")
    assert_equal "stylesheet", ast.rule_name
    refute_empty ast.children
  end
end
