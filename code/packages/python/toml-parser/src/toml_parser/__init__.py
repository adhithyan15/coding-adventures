"""TOML Parser — parses TOML text into Python dictionaries.

This package implements a complete TOML v1.0.0 parser in two phases:

1. **Syntax phase** (``parser.py``) — tokenizes the input and builds a generic
   ``ASTNode`` tree using the grammar-driven infrastructure. This is identical
   to how the JSON and CSS parsers work.

2. **Semantic phase** (``converter.py``) — walks the AST, validates
   context-sensitive constraints (key uniqueness, table consistency), and
   converts values to native Python types (strings, integers, floats,
   booleans, datetimes, lists, and nested dicts).

The result is a ``TOMLDocument`` (a ``dict`` subclass) that behaves exactly
like a Python dictionary.

Quick Start::

    from toml_parser import parse_toml

    doc = parse_toml('''
    [server]
    host = "localhost"
    port = 8080

    [database]
    name = "mydb"
    enabled = true
    ''')

    print(doc["server"]["host"])  # "localhost"
    print(doc["database"]["port"])  # KeyError — "port" is in [server]!

For advanced use cases (inspecting the AST, custom conversion), use the
lower-level functions::

    from toml_parser import parse_toml_ast, convert_ast

    ast = parse_toml_ast('name = "TOML"')
    # Inspect the syntax tree...

    doc = convert_ast(ast)
    # Apply semantic validation and value conversion...
"""

from toml_parser.converter import TOMLConversionError, convert_ast
from toml_parser.parser import create_toml_parser, parse_toml_ast
from toml_parser.types import TOMLDocument, TOMLValue


def parse_toml(source: str) -> TOMLDocument:
    """Parse a TOML string and return a Python dictionary.

    This is the main entry point — the function most callers want.
    It combines the syntax phase (tokenize + parse) and the semantic
    phase (validate + convert) into a single call.

    Args:
        source: A string containing valid TOML text.

    Returns:
        A ``TOMLDocument`` (dict subclass) containing the parsed data.
        All values are native Python types.

    Raises:
        LexerError: If the source contains invalid characters.
        GrammarParseError: If the source has syntax errors.
        TOMLConversionError: If the source violates TOML semantic rules
            (duplicate keys, table conflicts, etc.).

    Example::

        doc = parse_toml('name = "TOML"\\nversion = "1.0.0"')
        assert doc["name"] == "TOML"
        assert doc["version"] == "1.0.0"
    """
    ast = parse_toml_ast(source)
    return convert_ast(ast)


__all__ = [
    "TOMLConversionError",
    "TOMLDocument",
    "TOMLValue",
    "convert_ast",
    "create_toml_parser",
    "parse_toml",
    "parse_toml_ast",
]
