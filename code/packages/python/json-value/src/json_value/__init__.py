"""JSON Value -- typed representations of JSON data.

This package converts json-parser ASTs into typed ``JsonValue`` objects
and provides conversions between JsonValue and native Python types.

The JSON pipeline::

    JSON text  --> json-lexer --> tokens --> json-parser --> AST
                                                              |
                                                         from_ast()
                                                              |
                                                              v
                                                         JsonValue tree
                                                          /        \\
                                                  to_native()   from_native()
                                                        /            \\
                                                       v              v
                                                  Native Python   Native Python

Usage::

    from json_value import parse, parse_native, JsonObject, JsonString

    # Parse JSON text into typed JsonValue:
    value = parse('{"name": "Alice"}')
    assert isinstance(value, JsonObject)
    assert value.pairs["name"] == JsonString("Alice")

    # Parse JSON text into native Python dict:
    data = parse_native('{"name": "Alice"}')
    assert data == {"name": "Alice"}

    # Convert between JsonValue and native types:
    from json_value import from_native, to_native

    json_val = from_native({"x": 1, "y": [2, 3]})
    native = to_native(json_val)
"""

from json_value.converter import (
    from_ast,
    from_native,
    parse,
    parse_native,
    to_native,
)
from json_value.value import (
    JsonArray,
    JsonBool,
    JsonNull,
    JsonNumber,
    JsonObject,
    JsonString,
    JsonValue,
    JsonValueError,
)

__all__ = [
    # Value types
    "JsonArray",
    "JsonBool",
    "JsonNull",
    "JsonNumber",
    "JsonObject",
    "JsonString",
    "JsonValue",
    "JsonValueError",
    # Conversion functions
    "from_ast",
    "from_native",
    "parse",
    "parse_native",
    "to_native",
]
