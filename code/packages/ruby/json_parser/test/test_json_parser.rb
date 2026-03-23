# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the JSON Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with json.grammar, correctly builds Abstract Syntax Trees from
# JSON text (RFC 8259).
#
# The grammar-driven parser produces generic ASTNode objects:
#
#   ASTNode(rule_name: "value", children: [...])
#
# Each node records which grammar rule produced it and its matched
# children (which can be tokens or other ASTNodes). This is different
# from a hand-written parser's typed nodes (JsonObject, JsonArray),
# but it captures the same structural information.
#
# JSON's grammar (json.grammar) has just four rules:
#   - value:  object | array | STRING | NUMBER | TRUE | FALSE | NULL
#   - object: LBRACE [ pair { COMMA pair } ] RBRACE
#   - pair:   STRING COLON value
#   - array:  LBRACKET [ value { COMMA value } ] RBRACKET
# ================================================================

class TestJsonParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  # JSON-specific token type strings (not in TokenType::ALL)
  NUMBER_TYPE = "NUMBER"
  TRUE_TYPE   = "TRUE"
  FALSE_TYPE  = "FALSE"
  NULL_TYPE   = "NULL"

  # ------------------------------------------------------------------
  # Helper
  # ------------------------------------------------------------------

  def parse(source)
    CodingAdventures::JsonParser.parse(source)
  end

  # Recursively collect all Token objects from an AST.
  # Walks the entire tree depth-first, gathering every leaf token.
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

  # Find all ASTNode descendants with a given rule_name.
  # Useful for asserting that specific grammar rules were matched.
  def find_nodes_by_rule(node, rule_name)
    results = []
    return results unless node.is_a?(ASTNode)

    results << node if node.rule_name == rule_name
    node.children.each do |child|
      results.concat(find_nodes_by_rule(child, rule_name)) if child.is_a?(ASTNode)
    end
    results
  end

  # ------------------------------------------------------------------
  # Primitives: STRING, NUMBER, TRUE, FALSE, NULL
  # ------------------------------------------------------------------
  # JSON primitives are the leaf values. Each should parse into a
  # "value" node containing a single token.

  def test_parse_string
    ast = parse('"hello"')
    assert_equal "value", ast.rule_name

    all_tokens = collect_tokens(ast)
    string_token = all_tokens.find { |t| t.type == TT::STRING }
    refute_nil string_token, "Expected STRING token"
    assert_equal "hello", string_token.value
  end

  def test_parse_number
    ast = parse("42")
    assert_equal "value", ast.rule_name

    all_tokens = collect_tokens(ast)
    number_token = all_tokens.find { |t| t.type == NUMBER_TYPE }
    refute_nil number_token, "Expected NUMBER token"
    assert_equal "42", number_token.value
  end

  def test_parse_negative_number
    ast = parse("-3.14")
    assert_equal "value", ast.rule_name

    all_tokens = collect_tokens(ast)
    number_token = all_tokens.find { |t| t.type == NUMBER_TYPE }
    refute_nil number_token, "Expected NUMBER token for -3.14"
    assert_equal "-3.14", number_token.value
  end

  def test_parse_exponent_number
    ast = parse("1e10")
    assert_equal "value", ast.rule_name

    all_tokens = collect_tokens(ast)
    number_token = all_tokens.find { |t| t.type == NUMBER_TYPE }
    refute_nil number_token, "Expected NUMBER token for 1e10"
    assert_equal "1e10", number_token.value
  end

  def test_parse_true
    ast = parse("true")
    assert_equal "value", ast.rule_name

    all_tokens = collect_tokens(ast)
    true_token = all_tokens.find { |t| t.type == TRUE_TYPE }
    refute_nil true_token, "Expected TRUE token"
  end

  def test_parse_false
    ast = parse("false")
    assert_equal "value", ast.rule_name

    all_tokens = collect_tokens(ast)
    false_token = all_tokens.find { |t| t.type == FALSE_TYPE }
    refute_nil false_token, "Expected FALSE token"
  end

  def test_parse_null
    ast = parse("null")
    assert_equal "value", ast.rule_name

    all_tokens = collect_tokens(ast)
    null_token = all_tokens.find { |t| t.type == NULL_TYPE }
    refute_nil null_token, "Expected NULL token"
  end

  # ------------------------------------------------------------------
  # Empty object: {}
  # ------------------------------------------------------------------
  # The simplest compound structure. The grammar rule is:
  #   object = LBRACE [ pair { COMMA pair } ] RBRACE
  # With no pairs, it matches LBRACE RBRACE.

  def test_parse_empty_object
    ast = parse("{}")
    assert_equal "value", ast.rule_name

    # Should contain an object node
    object_nodes = find_nodes_by_rule(ast, "object")
    assert object_nodes.length >= 1, "Expected an object node"

    # Should have LBRACE and RBRACE tokens
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == TT::LBRACE }, "Expected LBRACE"
    assert all_tokens.any? { |t| t.type == TT::RBRACE }, "Expected RBRACE"
  end

  # ------------------------------------------------------------------
  # Non-empty object: {"key": "value"}
  # ------------------------------------------------------------------
  # An object with one key-value pair. The grammar path is:
  #   value -> object -> pair -> STRING COLON value -> STRING

  def test_parse_object_with_one_pair
    ast = parse('{"name": "Alice"}')
    assert_equal "value", ast.rule_name

    # Should have an object node containing a pair node
    object_nodes = find_nodes_by_rule(ast, "object")
    assert object_nodes.length >= 1, "Expected an object node"

    pair_nodes = find_nodes_by_rule(ast, "pair")
    assert_equal 1, pair_nodes.length, "Expected exactly one pair node"

    # The pair should contain two STRING tokens: the key and the value
    all_tokens = collect_tokens(ast)
    string_tokens = all_tokens.select { |t| t.type == TT::STRING }
    assert_equal 2, string_tokens.length, "Expected 2 STRING tokens (key + value)"
    assert_equal "name", string_tokens[0].value
    assert_equal "Alice", string_tokens[1].value
  end

  def test_parse_object_with_multiple_pairs
    ast = parse('{"a": 1, "b": 2, "c": 3}')
    assert_equal "value", ast.rule_name

    pair_nodes = find_nodes_by_rule(ast, "pair")
    assert_equal 3, pair_nodes.length, "Expected 3 pair nodes"

    all_tokens = collect_tokens(ast)
    number_tokens = all_tokens.select { |t| t.type == NUMBER_TYPE }
    assert_equal 3, number_tokens.length, "Expected 3 NUMBER tokens"
  end

  # ------------------------------------------------------------------
  # Empty array: []
  # ------------------------------------------------------------------
  # The grammar rule is:
  #   array = LBRACKET [ value { COMMA value } ] RBRACKET

  def test_parse_empty_array
    ast = parse("[]")
    assert_equal "value", ast.rule_name

    array_nodes = find_nodes_by_rule(ast, "array")
    assert array_nodes.length >= 1, "Expected an array node"

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.type == TT::LBRACKET }, "Expected LBRACKET"
    assert all_tokens.any? { |t| t.type == TT::RBRACKET }, "Expected RBRACKET"
  end

  # ------------------------------------------------------------------
  # Non-empty array: [1, 2, 3]
  # ------------------------------------------------------------------

  def test_parse_array_with_elements
    ast = parse("[1, 2, 3]")
    assert_equal "value", ast.rule_name

    array_nodes = find_nodes_by_rule(ast, "array")
    assert array_nodes.length >= 1, "Expected an array node"

    # Should contain 3 value nodes inside the array (one per element)
    all_tokens = collect_tokens(ast)
    number_tokens = all_tokens.select { |t| t.type == NUMBER_TYPE }
    assert_equal 3, number_tokens.length, "Expected 3 NUMBER tokens"
    assert_equal %w[1 2 3], number_tokens.map(&:value)
  end

  def test_parse_array_with_mixed_types
    ast = parse('[1, "two", true, null]')
    assert_equal "value", ast.rule_name

    all_tokens = collect_tokens(ast)
    types = all_tokens.map(&:type)
    assert_includes types, NUMBER_TYPE
    assert_includes types, TT::STRING
    assert_includes types, TRUE_TYPE
    assert_includes types, NULL_TYPE
  end

  # ------------------------------------------------------------------
  # Nested structures
  # ------------------------------------------------------------------
  # JSON's power comes from mutual recursion: values can contain
  # objects and arrays, which can contain values. This allows
  # arbitrarily deep nesting.

  def test_parse_nested_object_in_object
    ast = parse('{"outer": {"inner": 42}}')
    assert_equal "value", ast.rule_name

    object_nodes = find_nodes_by_rule(ast, "object")
    assert object_nodes.length >= 2, "Expected at least 2 object nodes (outer + inner)"

    all_tokens = collect_tokens(ast)
    number_token = all_tokens.find { |t| t.type == NUMBER_TYPE }
    refute_nil number_token
    assert_equal "42", number_token.value
  end

  def test_parse_array_in_object
    ast = parse('{"items": [1, 2, 3]}')
    assert_equal "value", ast.rule_name

    object_nodes = find_nodes_by_rule(ast, "object")
    assert object_nodes.length >= 1, "Expected object node"

    array_nodes = find_nodes_by_rule(ast, "array")
    assert array_nodes.length >= 1, "Expected array node inside object"
  end

  def test_parse_object_in_array
    ast = parse('[{"a": 1}, {"b": 2}]')
    assert_equal "value", ast.rule_name

    array_nodes = find_nodes_by_rule(ast, "array")
    assert array_nodes.length >= 1, "Expected array node"

    object_nodes = find_nodes_by_rule(ast, "object")
    assert_equal 2, object_nodes.length, "Expected 2 object nodes inside array"
  end

  def test_parse_deeply_nested
    ast = parse('[{"a": [1, {"b": 2}]}]')
    assert_equal "value", ast.rule_name

    # Should have: outer array, object, inner array, inner object
    array_nodes = find_nodes_by_rule(ast, "array")
    assert array_nodes.length >= 2, "Expected at least 2 array nodes"

    object_nodes = find_nodes_by_rule(ast, "object")
    assert object_nodes.length >= 2, "Expected at least 2 object nodes"
  end

  # ------------------------------------------------------------------
  # RFC 8259 example
  # ------------------------------------------------------------------
  # A realistic JSON document that exercises all value types and
  # nesting patterns.

  def test_parse_rfc8259_example
    source = <<~JSON
      {
        "name": "John Doe",
        "age": 42,
        "active": true,
        "address": null,
        "scores": [95, 87, 91],
        "metadata": {
          "created": "2024-01-01",
          "tags": ["admin", "user"]
        }
      }
    JSON
    ast = parse(source)
    assert_equal "value", ast.rule_name

    # Should have object nodes for root and metadata
    object_nodes = find_nodes_by_rule(ast, "object")
    assert object_nodes.length >= 2, "Expected root object + metadata object"

    # Should have array nodes for scores and tags
    array_nodes = find_nodes_by_rule(ast, "array")
    assert array_nodes.length >= 2, "Expected scores array + tags array"

    # Should have pair nodes for all key-value pairs
    pair_nodes = find_nodes_by_rule(ast, "pair")
    # Root has 6 pairs: name, age, active, address, scores, metadata
    # Metadata has 2 pairs: created, tags
    assert_equal 8, pair_nodes.length, "Expected 8 total pair nodes"

    # Verify key tokens contain expected values
    all_tokens = collect_tokens(ast)
    string_values = all_tokens.select { |t| t.type == TT::STRING }.map(&:value)
    assert_includes string_values, "name"
    assert_includes string_values, "John Doe"
    assert_includes string_values, "age"
    assert_includes string_values, "active"
    assert_includes string_values, "address"
    assert_includes string_values, "scores"
    assert_includes string_values, "metadata"
    assert_includes string_values, "created"
    assert_includes string_values, "tags"
    assert_includes string_values, "admin"
    assert_includes string_values, "user"

    # Verify number tokens
    number_values = all_tokens.select { |t| t.type == NUMBER_TYPE }.map(&:value)
    assert_includes number_values, "42"
    assert_includes number_values, "95"
    assert_includes number_values, "87"
    assert_includes number_values, "91"

    # Verify boolean and null tokens
    assert all_tokens.any? { |t| t.type == TRUE_TYPE }, "Expected TRUE token"
    assert all_tokens.any? { |t| t.type == NULL_TYPE }, "Expected NULL token"
  end

  # ------------------------------------------------------------------
  # Error cases
  # ------------------------------------------------------------------
  # The parser should raise errors on invalid JSON input.

  def test_error_on_empty_input
    assert_raises(StandardError) { parse("") }
  end

  def test_error_on_trailing_comma_in_object
    assert_raises(StandardError) { parse('{"a": 1,}') }
  end

  def test_error_on_trailing_comma_in_array
    assert_raises(StandardError) { parse("[1, 2,]") }
  end

  def test_error_on_missing_colon_in_pair
    assert_raises(StandardError) { parse('{"key" "value"}') }
  end

  def test_error_on_missing_value_in_pair
    assert_raises(StandardError) { parse('{"key":}') }
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::JsonParser::JSON_GRAMMAR_PATH),
      "json.grammar file should exist at #{CodingAdventures::JsonParser::JSON_GRAMMAR_PATH}"
  end

  # ------------------------------------------------------------------
  # Root node is always 'value'
  # ------------------------------------------------------------------

  def test_root_is_value
    ast = parse("42")
    assert_equal "value", ast.rule_name
  end

  def test_root_is_value_for_object
    ast = parse("{}")
    assert_equal "value", ast.rule_name
  end

  def test_root_is_value_for_array
    ast = parse("[]")
    assert_equal "value", ast.rule_name
  end
end
