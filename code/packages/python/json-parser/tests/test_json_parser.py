"""Tests for the JSON parser thin wrapper.

These tests verify that the grammar-driven parser, configured with
``json.grammar``, correctly parses JSON text per RFC 8259 into ASTs.
"""

from __future__ import annotations

import pytest

from json_parser import create_json_parser, parse_json
from lang_parser import ASTNode, GrammarParser, GrammarParseError
from lexer import Token


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def get_type_name(token: Token) -> str:
    """Extract the type name from a token (handles both enum and string)."""
    return token.type if isinstance(token.type, str) else token.type.name


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all nodes with a given rule_name."""
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def child_tokens(node: ASTNode) -> list[Token]:
    """Extract all Token children from a node (not ASTNode children)."""
    return [c for c in node.children if isinstance(c, Token)]


def child_nodes(node: ASTNode) -> list[ASTNode]:
    """Extract all ASTNode children from a node (not Token children)."""
    return [c for c in node.children if isinstance(c, ASTNode)]


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_json_parser factory function."""

    def test_returns_grammar_parser(self) -> None:
        """create_json_parser should return a GrammarParser instance."""
        parser = create_json_parser("42")
        assert isinstance(parser, GrammarParser)

    def test_factory_produces_ast(self) -> None:
        """The factory-created parser should produce a valid AST."""
        parser = create_json_parser('"hello"')
        ast = parser.parse()
        assert isinstance(ast, ASTNode)
        assert ast.rule_name == "value"


# ---------------------------------------------------------------------------
# Primitive value tests
# ---------------------------------------------------------------------------


class TestPrimitiveValues:
    """Tests for parsing JSON primitive values."""

    def test_string(self) -> None:
        """A string value parses to a value node wrapping a STRING token."""
        ast = parse_json('"hello"')
        assert ast.rule_name == "value"
        # The value node should contain a STRING token
        tokens = child_tokens(ast)
        assert len(tokens) == 1
        assert get_type_name(tokens[0]) == "STRING"
        assert tokens[0].value == "hello"

    def test_number_integer(self) -> None:
        """An integer value."""
        ast = parse_json("42")
        tokens = child_tokens(ast)
        assert len(tokens) == 1
        assert get_type_name(tokens[0]) == "NUMBER"
        assert tokens[0].value == "42"

    def test_number_negative(self) -> None:
        """A negative number."""
        ast = parse_json("-7")
        tokens = child_tokens(ast)
        assert tokens[0].value == "-7"

    def test_number_decimal(self) -> None:
        """A decimal number."""
        ast = parse_json("3.14")
        tokens = child_tokens(ast)
        assert tokens[0].value == "3.14"

    def test_number_exponent(self) -> None:
        """A number with exponent."""
        ast = parse_json("1e10")
        tokens = child_tokens(ast)
        assert tokens[0].value == "1e10"

    def test_true(self) -> None:
        """The literal true."""
        ast = parse_json("true")
        tokens = child_tokens(ast)
        assert len(tokens) == 1
        assert get_type_name(tokens[0]) == "TRUE"

    def test_false(self) -> None:
        """The literal false."""
        ast = parse_json("false")
        tokens = child_tokens(ast)
        assert get_type_name(tokens[0]) == "FALSE"

    def test_null(self) -> None:
        """The literal null."""
        ast = parse_json("null")
        tokens = child_tokens(ast)
        assert get_type_name(tokens[0]) == "NULL"


# ---------------------------------------------------------------------------
# Object tests
# ---------------------------------------------------------------------------


class TestObjects:
    """Tests for parsing JSON objects."""

    def test_empty_object(self) -> None:
        """An empty object {}."""
        ast = parse_json("{}")
        assert ast.rule_name == "value"
        # The value's child should be an object node
        nodes = child_nodes(ast)
        assert len(nodes) == 1
        assert nodes[0].rule_name == "object"
        # The object should have LBRACE and RBRACE tokens
        tokens = child_tokens(nodes[0])
        type_names = [get_type_name(t) for t in tokens]
        assert "LBRACE" in type_names
        assert "RBRACE" in type_names

    def test_single_pair(self) -> None:
        """An object with a single key-value pair."""
        ast = parse_json('{"name": "Ada"}')
        obj_nodes = find_nodes(ast, "object")
        assert len(obj_nodes) == 1

        pair_nodes = find_nodes(ast, "pair")
        assert len(pair_nodes) == 1

        # The pair should have STRING (key), COLON, and a value child
        pair = pair_nodes[0]
        pair_tokens = child_tokens(pair)
        key_token = pair_tokens[0]
        assert get_type_name(key_token) == "STRING"
        assert key_token.value == "name"

    def test_multiple_pairs(self) -> None:
        """An object with multiple key-value pairs."""
        ast = parse_json('{"a": 1, "b": 2, "c": 3}')
        pair_nodes = find_nodes(ast, "pair")
        assert len(pair_nodes) == 3

    def test_nested_object(self) -> None:
        """An object nested inside another object."""
        ast = parse_json('{"outer": {"inner": 42}}')
        obj_nodes = find_nodes(ast, "object")
        assert len(obj_nodes) == 2  # outer and inner

    def test_object_with_all_value_types(self) -> None:
        """An object with all JSON value types as values."""
        ast = parse_json(
            '{"str": "hello", "num": 42, "t": true, "f": false, "n": null}'
        )
        pair_nodes = find_nodes(ast, "pair")
        assert len(pair_nodes) == 5


# ---------------------------------------------------------------------------
# Array tests
# ---------------------------------------------------------------------------


class TestArrays:
    """Tests for parsing JSON arrays."""

    def test_empty_array(self) -> None:
        """An empty array []."""
        ast = parse_json("[]")
        nodes = child_nodes(ast)
        assert len(nodes) == 1
        assert nodes[0].rule_name == "array"

    def test_single_element(self) -> None:
        """An array with one element."""
        ast = parse_json("[42]")
        arr_nodes = find_nodes(ast, "array")
        assert len(arr_nodes) == 1
        # Should have LBRACKET, value, RBRACKET
        value_nodes = find_nodes(arr_nodes[0], "value")
        assert len(value_nodes) == 1

    def test_multiple_elements(self) -> None:
        """An array with multiple elements."""
        ast = parse_json("[1, 2, 3]")
        arr_nodes = find_nodes(ast, "array")
        assert len(arr_nodes) == 1
        value_nodes = find_nodes(arr_nodes[0], "value")
        assert len(value_nodes) == 3

    def test_nested_array(self) -> None:
        """An array nested inside another array."""
        ast = parse_json("[[1, 2], [3, 4]]")
        arr_nodes = find_nodes(ast, "array")
        assert len(arr_nodes) == 3  # outer + 2 inner

    def test_mixed_types(self) -> None:
        """An array with mixed value types."""
        ast = parse_json('[1, "two", true, null]')
        arr_nodes = find_nodes(ast, "array")
        assert len(arr_nodes) == 1
        value_nodes = find_nodes(arr_nodes[0], "value")
        assert len(value_nodes) == 4


# ---------------------------------------------------------------------------
# Nested structure tests
# ---------------------------------------------------------------------------


class TestNestedStructures:
    """Tests for deeply nested and complex JSON structures."""

    def test_object_in_array(self) -> None:
        """Objects inside an array."""
        ast = parse_json('[{"a": 1}, {"b": 2}]')
        obj_nodes = find_nodes(ast, "object")
        assert len(obj_nodes) == 2

    def test_array_in_object(self) -> None:
        """An array inside an object."""
        ast = parse_json('{"list": [1, 2, 3]}')
        arr_nodes = find_nodes(ast, "array")
        assert len(arr_nodes) == 1

    def test_deeply_nested(self) -> None:
        """A deeply nested structure."""
        ast = parse_json('{"a": {"b": {"c": [1]}}}')
        obj_nodes = find_nodes(ast, "object")
        assert len(obj_nodes) == 3
        arr_nodes = find_nodes(ast, "array")
        assert len(arr_nodes) == 1

    def test_rfc8259_example(self) -> None:
        """A realistic JSON document."""
        source = """{
            "name": "Ada Lovelace",
            "born": 1815,
            "contributions": ["first algorithm", "Analytical Engine notes"],
            "isAlive": false,
            "deathYear": null,
            "details": {
                "nationality": "British",
                "parents": ["Lord Byron", "Anne Isabella Milbanke"]
            }
        }"""
        ast = parse_json(source)
        assert ast.rule_name == "value"
        obj_nodes = find_nodes(ast, "object")
        assert len(obj_nodes) == 2  # root + details
        arr_nodes = find_nodes(ast, "array")
        assert len(arr_nodes) == 2  # contributions + parents


# ---------------------------------------------------------------------------
# Whitespace handling tests
# ---------------------------------------------------------------------------


class TestWhitespace:
    """Tests for whitespace handling in parsed JSON."""

    def test_multiline_object(self) -> None:
        """An object spanning multiple lines parses correctly."""
        source = '{\n  "key": "value"\n}'
        ast = parse_json(source)
        assert ast.rule_name == "value"
        pair_nodes = find_nodes(ast, "pair")
        assert len(pair_nodes) == 1

    def test_extra_whitespace(self) -> None:
        """Extra whitespace between tokens is ignored."""
        source = '  {  "a"  :  1  }  '
        ast = parse_json(source)
        pair_nodes = find_nodes(ast, "pair")
        assert len(pair_nodes) == 1


# ---------------------------------------------------------------------------
# Error case tests
# ---------------------------------------------------------------------------


class TestErrors:
    """Tests for JSON parse errors."""

    def test_missing_colon(self) -> None:
        """Missing colon between key and value should raise an error."""
        with pytest.raises(GrammarParseError):
            parse_json('{"key" "value"}')

    def test_unclosed_brace(self) -> None:
        """Unclosed brace should raise an error."""
        with pytest.raises(GrammarParseError):
            parse_json('{"key": "value"')

    def test_unclosed_bracket(self) -> None:
        """Unclosed bracket should raise an error."""
        with pytest.raises(GrammarParseError):
            parse_json("[1, 2, 3")

    def test_trailing_comma_in_object(self) -> None:
        """Trailing comma in object should raise an error (JSON disallows it)."""
        with pytest.raises(GrammarParseError):
            parse_json('{"a": 1,}')

    def test_trailing_comma_in_array(self) -> None:
        """Trailing comma in array should raise an error."""
        with pytest.raises(GrammarParseError):
            parse_json("[1, 2,]")

    def test_empty_input(self) -> None:
        """Empty input should raise an error (no value to parse)."""
        with pytest.raises(GrammarParseError):
            parse_json("")
