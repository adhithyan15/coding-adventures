# frozen_string_literal: true

require_relative "test_helper"

class TestPythonParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  def parse(source, version: "3.12")
    CodingAdventures::PythonParser.parse(source, version: version)
  end

  def test_default_version_is_3_12
    assert_equal "3.12", CodingAdventures::PythonParser::DEFAULT_VERSION
  end

  def test_supported_versions
    assert_equal %w[2.7 3.0 3.6 3.8 3.10 3.12], CodingAdventures::PythonParser::SUPPORTED_VERSIONS
  end

  def test_grammar_path_for_version
    path = CodingAdventures::PythonParser.grammar_path("3.12")
    assert path.end_with?("python/python3.12.grammar"),
      "Expected path to end with python/python3.12.grammar, got: #{path}"
  end

  def test_compiled_grammar_path_for_version
    path = CodingAdventures::PythonParser.compiled_grammar_path("3.12")
    assert path.end_with?("_grammar_3_12.rb"),
      "Expected compiled grammar path to end with _grammar_3_12.rb, got: #{path}"
  end

  def test_grammar_files_exist_for_all_versions
    CodingAdventures::PythonParser::SUPPORTED_VERSIONS.each do |version|
      assert File.exist?(CodingAdventures::PythonParser.grammar_path(version)),
        "Expected parser grammar file for version #{version}"
      assert File.exist?(CodingAdventures::PythonParser.compiled_grammar_path(version)),
        "Expected compiled parser grammar for version #{version}"
    end
  end

  def test_default_parse_uses_versioned_3_12_grammar
    ast = parse("x = 1 + 2")

    assert_equal "file", ast.rule_name
    refute_nil find_first_rule(ast, "assign_stmt"), "Expected assign_stmt in 3.12 parse"
  end

  def test_assignment_contains_expected_tokens
    ast = parse("x = 1 + 2")
    assignment = find_first_rule(ast, "assign_stmt")
    refute_nil assignment

    tokens = collect_tokens(assignment)
    assert tokens.any? { |t| t.type == TT::NAME && t.value == "x" }, "Expected NAME token"
    assert tokens.any? { |t| t.type == TT::EQUALS && t.value == "=" }, "Expected EQUALS token"
    assert_equal %w[1 2], tokens.select { |t| t.type == "INT" }.map(&:value)
    assert tokens.any? { |t| t.type == TT::PLUS && t.value == "+" }, "Expected PLUS token"
  end

  def test_multiple_statements
    ast = parse("x = 1\ny = 2")
    assert_equal 2, collect_rules(ast, "statement").length
  end

  def test_function_call_in_python_3_12
    ast = parse('print("hello")')
    refute_nil find_first_rule(ast, "suffix"), "Expected suffix node for function call"

    tokens = collect_tokens(ast)
    assert tokens.any? { |t| t.type == TT::NAME && t.value == "print" }
    assert tokens.any? { |t| t.type == TT::STRING && t.value == "hello" }
    assert tokens.any? { |t| t.type == TT::LPAREN }
    assert tokens.any? { |t| t.type == TT::RPAREN }
  end

  def test_python_2_7_print_statement_is_version_specific
    ast = parse('print "hello"', version: "2.7")
    print_stmt = find_first_rule(ast, "print_stmt")
    refute_nil print_stmt, "Expected print_stmt in Python 2.7 parse"

    tokens = collect_tokens(print_stmt)
    assert tokens.any? { |t| t.type == TT::KEYWORD && t.value == "print" }
    assert tokens.any? { |t| t.type == TT::STRING && t.value == "hello" }
  end

  def test_nil_version_defaults_to_3_12
    ast = CodingAdventures::PythonParser.parse("x = 1 + 2", version: nil)
    assert_equal "file", ast.rule_name
    refute_nil find_first_rule(ast, "assign_stmt")
  end

  def test_invalid_version_raises_argument_error
    error = assert_raises(ArgumentError) do
      CodingAdventures::PythonParser.parse("x = 1", version: "1.0")
    end

    assert_match(/Unsupported Python version/, error.message)
  end

  private

  def collect_tokens(node)
    return [] unless node.is_a?(ASTNode)

    node.children.flat_map do |child|
      if child.is_a?(CodingAdventures::Lexer::Token)
        [child]
      elsif child.is_a?(ASTNode)
        collect_tokens(child)
      else
        []
      end
    end
  end

  def find_first_rule(node, rule_name)
    return nil unless node.is_a?(ASTNode)
    return node if node.rule_name == rule_name

    node.children.each do |child|
      next unless child.is_a?(ASTNode)

      match = find_first_rule(child, rule_name)
      return match if match
    end

    nil
  end

  def collect_rules(node, rule_name, matches = [])
    return matches unless node.is_a?(ASTNode)

    matches << node if node.rule_name == rule_name
    node.children.each { |child| collect_rules(child, rule_name, matches) if child.is_a?(ASTNode) }
    matches
  end
end
