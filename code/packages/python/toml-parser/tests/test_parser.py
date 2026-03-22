"""Tests for the TOML grammar parser (syntax phase).

These tests verify that TOML source text produces the expected AST
structure. They do NOT test semantic validation — that is in
test_converter.py. These tests focus on:

- The grammar correctly recognizes all TOML constructs.
- AST nodes have the expected rule names and token types.
- All 11 grammar rules produce correct parse trees.
"""

from __future__ import annotations

import pytest
from lang_parser import ASTNode, GrammarParseError
from lexer import LexerError, Token

from toml_parser.parser import create_toml_parser, parse_toml_ast

# =============================================================================
# Helper Functions
# =============================================================================


def get_expressions(ast: ASTNode) -> list[ASTNode]:
    """Extract all expression nodes from a document AST."""
    return [
        child for child in ast.children
        if isinstance(child, ASTNode) and child.rule_name == "expression"
    ]


def get_inner(expr: ASTNode) -> ASTNode:
    """Get the inner node (keyval, table_header, etc.) from an expression."""
    for child in expr.children:
        if isinstance(child, ASTNode):
            return child
    msg = "No inner ASTNode found"
    raise ValueError(msg)


def collect_tokens(node: ASTNode) -> list[Token]:
    """Recursively collect all tokens from an AST node."""
    tokens: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            tokens.append(child)
        elif isinstance(child, ASTNode):
            tokens.extend(collect_tokens(child))
    return tokens


def token_types(node: ASTNode) -> list[str]:
    """Get all token type names from an AST node.

    Token types can be either plain strings (e.g., "BARE_KEY") for grammar-
    defined tokens, or TokenType enum members (e.g., TokenType.DOT) for
    delimiter tokens. We normalize both to plain strings by extracting just
    the name portion.
    """
    result = []
    for t in collect_tokens(node):
        type_str = str(t.type)
        # Normalize "TokenType.DOT" → "DOT"
        if type_str.startswith("TokenType."):
            type_str = type_str.split(".", 1)[1]
        result.append(type_str)
    return result


def token_values(node: ASTNode) -> list[str]:
    """Get all token values from an AST node."""
    return [t.value for t in collect_tokens(node)]


# =============================================================================
# Factory Tests
# =============================================================================


class TestFactory:
    """Test the parser factory function."""

    def test_create_parser_returns_grammar_parser(self) -> None:
        """create_toml_parser returns a GrammarParser instance."""
        parser = create_toml_parser('name = "TOML"')
        assert hasattr(parser, "parse")

    def test_parse_returns_ast_node(self) -> None:
        """parse_toml_ast returns an ASTNode."""
        ast = parse_toml_ast('name = "TOML"')
        assert isinstance(ast, ASTNode)

    def test_root_is_document(self) -> None:
        """The root AST node has rule_name 'document'."""
        ast = parse_toml_ast('key = "value"')
        assert ast.rule_name == "document"


# =============================================================================
# Key-Value Pair Tests
# =============================================================================


class TestKeyValuePairs:
    """Test parsing of key-value pairs."""

    def test_bare_key_string_value(self) -> None:
        """Parse: name = 'TOML'"""
        ast = parse_toml_ast('name = "TOML"')
        exprs = get_expressions(ast)
        assert len(exprs) == 1
        inner = get_inner(exprs[0])
        assert inner.rule_name == "keyval"

    def test_bare_key_integer_value(self) -> None:
        """Parse: port = 8080"""
        ast = parse_toml_ast("port = 8080")
        exprs = get_expressions(ast)
        assert len(exprs) == 1
        inner = get_inner(exprs[0])
        assert inner.rule_name == "keyval"
        types = token_types(inner)
        assert "BARE_KEY" in types
        assert "INTEGER" in types

    def test_bare_key_float_value(self) -> None:
        """Parse: pi = 3.14"""
        ast = parse_toml_ast("pi = 3.14")
        exprs = get_expressions(ast)
        inner = get_inner(exprs[0])
        types = token_types(inner)
        assert "FLOAT" in types

    def test_bare_key_boolean_value(self) -> None:
        """Parse: enabled = true"""
        ast = parse_toml_ast("enabled = true")
        exprs = get_expressions(ast)
        inner = get_inner(exprs[0])
        types = token_types(inner)
        assert "TRUE" in types

    def test_bare_key_false_value(self) -> None:
        """Parse: enabled = false"""
        ast = parse_toml_ast("enabled = false")
        types = token_types(
            get_inner(get_expressions(ast)[0])
        )
        assert "FALSE" in types

    def test_multiple_kvs_separated_by_newlines(self) -> None:
        """Multiple key-value pairs separated by newlines."""
        ast = parse_toml_ast('name = "TOML"\nversion = "1.0"')
        exprs = get_expressions(ast)
        assert len(exprs) == 2

    def test_dotted_key(self) -> None:
        """Parse: server.host = 'localhost'"""
        ast = parse_toml_ast('server.host = "localhost"')
        exprs = get_expressions(ast)
        inner = get_inner(exprs[0])
        types = token_types(inner)
        assert "DOT" in types

    def test_quoted_key(self) -> None:
        """Parse: 'key with spaces' = 'value'"""
        ast = parse_toml_ast('"key with spaces" = "value"')
        exprs = get_expressions(ast)
        assert len(exprs) == 1

    def test_literal_string_key(self) -> None:
        """Parse: 'key' = 'value'"""
        ast = parse_toml_ast("'key' = \"value\"")
        exprs = get_expressions(ast)
        assert len(exprs) == 1


# =============================================================================
# Table Header Tests
# =============================================================================


class TestTableHeaders:
    """Test parsing of table headers."""

    def test_simple_table_header(self) -> None:
        """Parse: [server]"""
        ast = parse_toml_ast("[server]")
        exprs = get_expressions(ast)
        inner = get_inner(exprs[0])
        assert inner.rule_name == "table_header"

    def test_dotted_table_header(self) -> None:
        """Parse: [server.database]"""
        ast = parse_toml_ast("[server.database]")
        exprs = get_expressions(ast)
        inner = get_inner(exprs[0])
        assert inner.rule_name == "table_header"
        types = token_types(inner)
        assert "DOT" in types

    def test_table_with_keyvals(self) -> None:
        """Parse a table with key-value pairs underneath."""
        source = '[server]\nhost = "localhost"\nport = 8080'
        ast = parse_toml_ast(source)
        exprs = get_expressions(ast)
        assert len(exprs) == 3
        assert get_inner(exprs[0]).rule_name == "table_header"
        assert get_inner(exprs[1]).rule_name == "keyval"
        assert get_inner(exprs[2]).rule_name == "keyval"


# =============================================================================
# Array-of-Tables Tests
# =============================================================================


class TestArrayOfTables:
    """Test parsing of array-of-tables headers."""

    def test_array_table_header(self) -> None:
        """Parse: [[products]]"""
        ast = parse_toml_ast("[[products]]")
        exprs = get_expressions(ast)
        inner = get_inner(exprs[0])
        assert inner.rule_name == "array_table_header"

    def test_array_table_with_keyvals(self) -> None:
        """Parse [[products]] with entries."""
        source = '[[products]]\nname = "Hammer"\n[[products]]\nname = "Nail"'
        ast = parse_toml_ast(source)
        exprs = get_expressions(ast)
        assert len(exprs) == 4
        assert get_inner(exprs[0]).rule_name == "array_table_header"
        assert get_inner(exprs[1]).rule_name == "keyval"
        assert get_inner(exprs[2]).rule_name == "array_table_header"
        assert get_inner(exprs[3]).rule_name == "keyval"


# =============================================================================
# Value Type Tests
# =============================================================================


class TestValueTypes:
    """Test parsing of all TOML value types."""

    def test_basic_string(self) -> None:
        """Basic double-quoted string."""
        ast = parse_toml_ast('key = "hello"')
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "BASIC_STRING" in types

    def test_literal_string(self) -> None:
        """Literal single-quoted string."""
        ast = parse_toml_ast("key = 'hello'")
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "LITERAL_STRING" in types

    def test_ml_basic_string(self) -> None:
        """Multi-line basic string."""
        ast = parse_toml_ast('key = """hello\nworld"""')
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "ML_BASIC_STRING" in types

    def test_ml_literal_string(self) -> None:
        """Multi-line literal string."""
        ast = parse_toml_ast("key = '''hello\nworld'''")
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "ML_LITERAL_STRING" in types

    def test_integer(self) -> None:
        """Integer value."""
        ast = parse_toml_ast("key = 42")
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "INTEGER" in types

    def test_float(self) -> None:
        """Float value."""
        ast = parse_toml_ast("key = 3.14")
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "FLOAT" in types

    def test_offset_datetime(self) -> None:
        """Offset datetime value."""
        ast = parse_toml_ast("key = 1979-05-27T07:32:00Z")
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "OFFSET_DATETIME" in types

    def test_local_datetime(self) -> None:
        """Local datetime value."""
        ast = parse_toml_ast("key = 1979-05-27T07:32:00")
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "LOCAL_DATETIME" in types

    def test_local_date(self) -> None:
        """Local date value."""
        ast = parse_toml_ast("key = 1979-05-27")
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "LOCAL_DATE" in types

    def test_local_time(self) -> None:
        """Local time value."""
        ast = parse_toml_ast("key = 07:32:00")
        types = token_types(get_inner(get_expressions(ast)[0]))
        assert "LOCAL_TIME" in types


# =============================================================================
# Array Tests
# =============================================================================


class TestArrays:
    """Test parsing of TOML arrays."""

    def test_empty_array(self) -> None:
        """Parse: key = []"""
        ast = parse_toml_ast("key = []")
        exprs = get_expressions(ast)
        assert len(exprs) == 1

    def test_simple_array(self) -> None:
        """Parse: key = [1, 2, 3]"""
        ast = parse_toml_ast("key = [1, 2, 3]")
        exprs = get_expressions(ast)
        assert len(exprs) == 1

    def test_multiline_array(self) -> None:
        """Multi-line array with trailing comma."""
        source = "key = [\n  1,\n  2,\n  3,\n]"
        ast = parse_toml_ast(source)
        exprs = get_expressions(ast)
        assert len(exprs) == 1

    def test_nested_array(self) -> None:
        """Parse: key = [[1, 2], [3, 4]]"""
        ast = parse_toml_ast("key = [[1, 2], [3, 4]]")
        exprs = get_expressions(ast)
        assert len(exprs) == 1


# =============================================================================
# Inline Table Tests
# =============================================================================


class TestInlineTables:
    """Test parsing of inline tables."""

    def test_empty_inline_table(self) -> None:
        """Parse: key = {}"""
        ast = parse_toml_ast("key = {}")
        exprs = get_expressions(ast)
        assert len(exprs) == 1

    def test_simple_inline_table(self) -> None:
        """Parse: point = { x = 1, y = 2 }"""
        ast = parse_toml_ast("point = { x = 1, y = 2 }")
        exprs = get_expressions(ast)
        assert len(exprs) == 1

    def test_nested_inline_table(self) -> None:
        """Parse: key = { inner = { a = 1 } }"""
        ast = parse_toml_ast("key = { inner = { a = 1 } }")
        exprs = get_expressions(ast)
        assert len(exprs) == 1


# =============================================================================
# Whitespace and Comment Tests
# =============================================================================


class TestWhitespaceAndComments:
    """Test handling of whitespace and comments."""

    def test_blank_lines(self) -> None:
        """Blank lines between expressions."""
        source = 'a = 1\n\n\nb = 2'
        ast = parse_toml_ast(source)
        exprs = get_expressions(ast)
        assert len(exprs) == 2

    def test_comments_ignored(self) -> None:
        """Comments are skipped by the lexer."""
        source = '# This is a comment\na = 1\n# Another comment\nb = 2'
        ast = parse_toml_ast(source)
        exprs = get_expressions(ast)
        assert len(exprs) == 2

    def test_inline_comment(self) -> None:
        """Inline comments after values."""
        source = 'a = 1 # inline comment'
        ast = parse_toml_ast(source)
        exprs = get_expressions(ast)
        assert len(exprs) == 1

    def test_empty_document(self) -> None:
        """Empty input produces an empty document AST."""
        ast = parse_toml_ast("")
        assert ast.rule_name == "document"

    def test_only_comments(self) -> None:
        """Document with only comments."""
        ast = parse_toml_ast("# just a comment\n# another one")
        exprs = get_expressions(ast)
        assert len(exprs) == 0

    def test_only_newlines(self) -> None:
        """Document with only blank lines."""
        ast = parse_toml_ast("\n\n\n")
        exprs = get_expressions(ast)
        assert len(exprs) == 0


# =============================================================================
# Error Tests
# =============================================================================


class TestSyntaxErrors:
    """Test that syntax errors are detected."""

    def test_missing_equals(self) -> None:
        """Missing = in key-value pair."""
        with pytest.raises((LexerError, GrammarParseError)):
            parse_toml_ast("key value")

    def test_unclosed_bracket(self) -> None:
        """Unclosed table header bracket."""
        with pytest.raises((LexerError, GrammarParseError)):
            parse_toml_ast("[server")

    def test_unclosed_array(self) -> None:
        """Unclosed array bracket."""
        with pytest.raises((LexerError, GrammarParseError)):
            parse_toml_ast("key = [1, 2")

    def test_unclosed_inline_table(self) -> None:
        """Unclosed inline table brace."""
        with pytest.raises((LexerError, GrammarParseError)):
            parse_toml_ast("key = { a = 1")
