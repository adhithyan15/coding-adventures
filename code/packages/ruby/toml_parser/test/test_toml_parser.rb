# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the TOML Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with toml.grammar, correctly parses TOML v1.0.0 text into ASTs.
#
# This tests the syntax phase only -- semantic validation (key
# uniqueness, table consistency) is not yet implemented in Ruby.
# The Python implementation has the full converter.
#
# We test that:
#   1. All expression types produce correct AST nodes
#   2. All value types are recognized
#   3. Table and array-of-tables headers parse correctly
#   4. Arrays and inline tables parse correctly
#   5. Whitespace and comments are handled properly
#   6. Syntax errors are detected
# ================================================================

class TestTomlParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode

  def parse(source)
    CodingAdventures::TomlParser.parse(source)
  end

  # Helper: get all expression nodes from a document AST
  def expressions(ast)
    ast.children.select { |c| c.is_a?(ASTNode) && c.rule_name == "expression" }
  end

  # Helper: get inner node from expression
  def inner(expr)
    expr.children.find { |c| c.is_a?(ASTNode) }
  end

  # Helper: recursively collect all tokens
  def collect_tokens(node)
    result = []
    node.children.each do |child|
      if child.is_a?(ASTNode)
        result.concat(collect_tokens(child))
      else
        result << child
      end
    end
    result
  end

  # ------------------------------------------------------------------
  # Document structure
  # ------------------------------------------------------------------

  def test_root_is_document
    ast = parse('key = "value"')
    assert_equal "document", ast.rule_name
  end

  def test_empty_document
    ast = parse("")
    assert_equal "document", ast.rule_name
  end

  def test_only_comments
    ast = parse("# just a comment\n# another")
    exprs = expressions(ast)
    assert_equal 0, exprs.length
  end

  def test_only_newlines
    ast = parse("\n\n\n")
    exprs = expressions(ast)
    assert_equal 0, exprs.length
  end

  # ------------------------------------------------------------------
  # Key-value pairs
  # ------------------------------------------------------------------

  def test_simple_keyval
    ast = parse('name = "TOML"')
    exprs = expressions(ast)
    assert_equal 1, exprs.length
    assert_equal "keyval", inner(exprs[0]).rule_name
  end

  def test_integer_value
    ast = parse("port = 8080")
    exprs = expressions(ast)
    assert_equal "keyval", inner(exprs[0]).rule_name
  end

  def test_multiple_keyvals
    ast = parse("a = 1\nb = 2\nc = 3")
    exprs = expressions(ast)
    assert_equal 3, exprs.length
  end

  def test_dotted_key
    ast = parse('server.host = "localhost"')
    exprs = expressions(ast)
    tokens = collect_tokens(inner(exprs[0]))
    types = tokens.map(&:type)
    assert types.any? { |t| t == :DOT || t.to_s == "DOT" }, "Expected DOT token"
  end

  def test_quoted_key
    ast = parse('"key with spaces" = "value"')
    exprs = expressions(ast)
    assert_equal 1, exprs.length
  end

  # ------------------------------------------------------------------
  # Table headers
  # ------------------------------------------------------------------

  def test_table_header
    ast = parse("[server]")
    exprs = expressions(ast)
    assert_equal "table_header", inner(exprs[0]).rule_name
  end

  def test_dotted_table_header
    ast = parse("[server.database]")
    exprs = expressions(ast)
    assert_equal "table_header", inner(exprs[0]).rule_name
  end

  def test_table_with_keyvals
    ast = parse("[server]\nhost = \"localhost\"\nport = 8080")
    exprs = expressions(ast)
    assert_equal 3, exprs.length
    assert_equal "table_header", inner(exprs[0]).rule_name
    assert_equal "keyval", inner(exprs[1]).rule_name
    assert_equal "keyval", inner(exprs[2]).rule_name
  end

  # ------------------------------------------------------------------
  # Array-of-tables headers
  # ------------------------------------------------------------------

  def test_array_table_header
    ast = parse("[[products]]")
    exprs = expressions(ast)
    assert_equal "array_table_header", inner(exprs[0]).rule_name
  end

  def test_multiple_array_tables
    ast = parse("[[p]]\nname = \"a\"\n[[p]]\nname = \"b\"")
    exprs = expressions(ast)
    assert_equal 4, exprs.length
    assert_equal "array_table_header", inner(exprs[0]).rule_name
    assert_equal "array_table_header", inner(exprs[2]).rule_name
  end

  # ------------------------------------------------------------------
  # Value types
  # ------------------------------------------------------------------

  def test_basic_string_value
    ast = parse('key = "hello"')
    tokens = collect_tokens(ast)
    types = tokens.map { |t| t.type.to_s }
    assert types.include?("BASIC_STRING")
  end

  def test_literal_string_value
    ast = parse("key = 'hello'")
    tokens = collect_tokens(ast)
    types = tokens.map { |t| t.type.to_s }
    assert types.include?("LITERAL_STRING")
  end

  def test_integer_value_type
    ast = parse("key = 42")
    tokens = collect_tokens(ast)
    types = tokens.map { |t| t.type.to_s }
    assert types.include?("INTEGER")
  end

  def test_float_value_type
    ast = parse("key = 3.14")
    tokens = collect_tokens(ast)
    types = tokens.map { |t| t.type.to_s }
    assert types.include?("FLOAT")
  end

  def test_boolean_true
    ast = parse("key = true")
    tokens = collect_tokens(ast)
    types = tokens.map { |t| t.type.to_s }
    assert types.include?("TRUE")
  end

  def test_boolean_false
    ast = parse("key = false")
    tokens = collect_tokens(ast)
    types = tokens.map { |t| t.type.to_s }
    assert types.include?("FALSE")
  end

  def test_offset_datetime
    ast = parse("key = 1979-05-27T07:32:00Z")
    tokens = collect_tokens(ast)
    types = tokens.map { |t| t.type.to_s }
    assert types.include?("OFFSET_DATETIME")
  end

  def test_local_date
    ast = parse("key = 1979-05-27")
    tokens = collect_tokens(ast)
    types = tokens.map { |t| t.type.to_s }
    assert types.include?("LOCAL_DATE")
  end

  def test_local_time
    ast = parse("key = 07:32:00")
    tokens = collect_tokens(ast)
    types = tokens.map { |t| t.type.to_s }
    assert types.include?("LOCAL_TIME")
  end

  # ------------------------------------------------------------------
  # Arrays
  # ------------------------------------------------------------------

  def test_empty_array
    ast = parse("key = []")
    exprs = expressions(ast)
    assert_equal 1, exprs.length
  end

  def test_simple_array
    ast = parse("key = [1, 2, 3]")
    exprs = expressions(ast)
    assert_equal 1, exprs.length
  end

  def test_multiline_array
    ast = parse("key = [\n  1,\n  2,\n  3,\n]")
    exprs = expressions(ast)
    assert_equal 1, exprs.length
  end

  def test_nested_array
    ast = parse("key = [[1, 2], [3, 4]]")
    exprs = expressions(ast)
    assert_equal 1, exprs.length
  end

  # ------------------------------------------------------------------
  # Inline tables
  # ------------------------------------------------------------------

  def test_empty_inline_table
    ast = parse("key = {}")
    exprs = expressions(ast)
    assert_equal 1, exprs.length
  end

  def test_simple_inline_table
    ast = parse("point = { x = 1, y = 2 }")
    exprs = expressions(ast)
    assert_equal 1, exprs.length
  end

  # ------------------------------------------------------------------
  # Whitespace and comments
  # ------------------------------------------------------------------

  def test_blank_lines_between_expressions
    ast = parse("a = 1\n\n\nb = 2")
    exprs = expressions(ast)
    assert_equal 2, exprs.length
  end

  def test_inline_comment
    ast = parse("a = 1 # comment")
    exprs = expressions(ast)
    assert_equal 1, exprs.length
  end

  # ------------------------------------------------------------------
  # Syntax errors
  # ------------------------------------------------------------------

  def test_missing_equals_raises
    assert_raises(StandardError) { parse("key value") }
  end

  def test_unclosed_bracket_raises
    assert_raises(StandardError) { parse("[server") }
  end

  def test_unclosed_array_raises
    assert_raises(StandardError) { parse("key = [1, 2") }
  end

  # ------------------------------------------------------------------
  # Complex documents
  # ------------------------------------------------------------------

  def test_realistic_document
    source = <<~TOML
      [package]
      name = "my-project"
      version = "0.1.0"

      [dependencies]
      serde = "1.0"
    TOML
    ast = parse(source)
    exprs = expressions(ast)
    # 2 table headers + 3 key-value pairs
    assert_equal 5, exprs.length
  end

  def test_array_of_tables_document
    source = <<~TOML
      [[products]]
      name = "Hammer"

      [[products]]
      name = "Nail"
    TOML
    ast = parse(source)
    exprs = expressions(ast)
    assert_equal 4, exprs.length
  end
end
