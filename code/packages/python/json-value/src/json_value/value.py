"""JsonValue — typed representations of the six JSON value types.

JSON (RFC 8259) defines exactly six value types:

    1. Object  -- an ordered collection of key-value pairs
    2. Array   -- an ordered sequence of values
    3. String  -- a sequence of Unicode characters
    4. Number  -- an integer or floating-point number
    5. Boolean -- true or false
    6. Null    -- the absence of a value

This module provides a Python class for each type, all inheriting from
a common ``JsonValue`` base class. This gives us a **typed intermediate
representation** -- a discriminated union where each variant carries
exactly the data appropriate for its JSON type.

Why not just use native Python types (dict, list, str, etc.)?
-------------------------------------------------------------

Native types work great for *consuming* JSON data ("give me a dict").
But they lose type information:

    - You can't distinguish "this is a JSON number" from "this is a
      Python int that happened to come from JSON."
    - You can't attach methods specific to JSON values (like pretty-printing
      or path-based access) without monkey-patching built-in types.
    - Pattern matching (Python 3.10+) works much better with distinct classes.

JsonValue preserves the JSON type while still being easy to convert to
native types via ``to_native()``.

Class hierarchy
---------------

::

    JsonValue  (abstract base -- you never instantiate this directly)
      |-- JsonObject(pairs: dict[str, JsonValue])   -- {"key": value, ...}
      |-- JsonArray(elements: list[JsonValue])       -- [value, value, ...]
      |-- JsonString(value: str)                     -- "hello"
      |-- JsonNumber(value: int | float)             -- 42 or 3.14
      |-- JsonBool(value: bool)                      -- true / false
      |-- JsonNull()                                 -- null

All classes use ``@dataclass`` for automatic ``__init__``, ``__eq__``,
and ``__repr__`` generation. This means two ``JsonNumber(42)`` instances
are equal by value, not by identity -- exactly the semantics we want.
"""

from __future__ import annotations

from dataclasses import dataclass, field


# ---------------------------------------------------------------------------
# Exception
# ---------------------------------------------------------------------------


class JsonValueError(Exception):
    """Raised when a JSON value operation fails.

    This covers:
    - AST nodes that don't conform to the expected structure
    - Native values that can't be converted to JSON (e.g., functions, sets)
    - Non-string keys in dicts passed to ``from_native()``
    - Parse failures when using the convenience ``parse()`` function
    """


# ---------------------------------------------------------------------------
# Base class
# ---------------------------------------------------------------------------


@dataclass
class JsonValue:
    """Abstract base class for all JSON value types.

    You never create a ``JsonValue`` directly. Instead, use one of the six
    concrete subclasses: ``JsonObject``, ``JsonArray``, ``JsonString``,
    ``JsonNumber``, ``JsonBool``, or ``JsonNull``.

    This base class exists so you can write type annotations like::

        def process(value: JsonValue) -> None:
            match value:
                case JsonString(s):
                    print(f"Got string: {s}")
                case JsonNumber(n):
                    print(f"Got number: {n}")
                case _:
                    print("Got something else")
    """


# ---------------------------------------------------------------------------
# Concrete value types
# ---------------------------------------------------------------------------


@dataclass
class JsonObject(JsonValue):
    """A JSON object -- an ordered collection of string-keyed pairs.

    JSON objects are written as ``{"key": value, "key2": value2}``.

    Why ``dict`` for pairs?
        Python 3.7+ guarantees that ``dict`` preserves insertion order.
        This gives us the ordered-map semantics we need without importing
        any special collection types. RFC 8259 says JSON objects are
        "unordered," but preserving order is important for:

        - Human readability (keys stay where the author put them)
        - Round-trip fidelity (parse then serialize gives same order)
        - Deterministic output (same input always produces same output)

    Example::

        obj = JsonObject({"name": JsonString("Alice"), "age": JsonNumber(30)})
        obj.pairs["name"]  # JsonString("Alice")
    """

    pairs: dict[str, JsonValue] = field(default_factory=dict)


@dataclass
class JsonArray(JsonValue):
    """A JSON array -- an ordered sequence of values.

    JSON arrays are written as ``[value1, value2, value3]``.

    The elements can be any mix of JSON types -- arrays are heterogeneous
    in JSON (unlike typed arrays in languages like Go or Rust).

    Example::

        arr = JsonArray([JsonNumber(1), JsonString("two"), JsonBool(True)])
        arr.elements[0]  # JsonNumber(1)
    """

    elements: list[JsonValue] = field(default_factory=list)


@dataclass
class JsonString(JsonValue):
    """A JSON string value.

    JSON strings are sequences of Unicode characters enclosed in double
    quotes. By the time we create a ``JsonString``, all escape sequences
    (``\\n``, ``\\t``, ``\\uXXXX``, etc.) have already been resolved by
    the lexer. The ``value`` field contains the *actual* string content,
    not the JSON-encoded form.

    Example::

        s = JsonString("hello\\nworld")
        # s.value contains a real newline character, not backslash-n
    """

    value: str = ""


@dataclass
class JsonNumber(JsonValue):
    """A JSON number -- either integer or floating-point.

    JSON itself doesn't distinguish between integers and floats -- the
    spec just says "number." But practically, it matters:

    - ``42`` has no decimal point or exponent -- store as ``int``
    - ``3.14`` has a decimal point -- store as ``float``
    - ``1e10`` has an exponent -- store as ``float``

    This matches the behavior of Python's ``json.loads``, Ruby's
    ``JSON.parse``, and most other JSON libraries.

    The ``value`` field's type is ``int | float``, which Python's type
    system handles natively. At runtime, you can check with
    ``isinstance(n.value, int)`` or ``isinstance(n.value, float)``.

    Example::

        n1 = JsonNumber(42)       # integer
        n2 = JsonNumber(3.14)     # float
        n3 = JsonNumber(1e10)     # float (scientific notation)
    """

    value: int | float = 0


@dataclass
class JsonBool(JsonValue):
    """A JSON boolean -- true or false.

    JSON booleans map directly to Python's ``True`` and ``False``.

    Example::

        b = JsonBool(True)
        b.value  # True
    """

    value: bool = False


@dataclass
class JsonNull(JsonValue):
    """A JSON null -- the explicit absence of a value.

    JSON null maps to Python's ``None``. Unlike the other JsonValue types,
    ``JsonNull`` carries no data -- it's a singleton-like marker that says
    "this value is intentionally empty."

    Example::

        n = JsonNull()
        # No .value attribute -- null IS the value
    """
