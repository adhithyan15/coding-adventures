"""JSON Serializer — converts JsonValue trees or native types to JSON text.

This package is the final stage of the JSON pipeline built in
coding-adventures. It takes a ``JsonValue`` tree (produced by
``json-value``) or native Python types (dict, list, str, etc.) and
produces valid JSON text in either compact or pretty-printed format.

Two modes of operation
----------------------

**Compact mode** (``serialize``, ``stringify``):
    Produces the smallest valid JSON — no unnecessary whitespace.
    Ideal for wire transmission, APIs, and storage.

    >>> from json_serializer import stringify
    >>> stringify({"name": "Alice", "age": 30})
    '{"name":"Alice","age":30}'

**Pretty mode** (``serialize_pretty``, ``stringify_pretty``):
    Produces human-readable JSON with configurable indentation.
    Ideal for config files, debugging, and display.

    >>> from json_serializer import stringify_pretty
    >>> stringify_pretty({"name": "Alice"})
    '{\\n  "name": "Alice"\\n}'

Formatting is controlled by ``SerializerConfig``:

    >>> from json_serializer import SerializerConfig, stringify_pretty
    >>> config = SerializerConfig(indent_size=4, sort_keys=True)
    >>> stringify_pretty({"b": 2, "a": 1}, config)
    '{\\n    "a": 1,\\n    "b": 2\\n}'
"""

from json_serializer.config import SerializerConfig
from json_serializer.serializer import (
    JsonSerializerError,
    serialize,
    serialize_pretty,
    stringify,
    stringify_pretty,
)

__all__ = [
    "JsonSerializerError",
    "SerializerConfig",
    "serialize",
    "serialize_pretty",
    "stringify",
    "stringify_pretty",
]
