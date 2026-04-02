"""Converter — bridges between ASTs, JsonValues, and native Python types.

This module provides the four conversion functions that connect the three
representations of JSON data:

::

    JSON text  --(lexer+parser)-->  ASTNode tree
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
                            (dict, list,    (dict, list,
                             str, int,       str, int,
                             float, bool,    float, bool,
                             None)           None)

Plus two convenience functions that go directly from text to values:

- ``parse(text)`` -- text --> JsonValue (via lexer + parser + from_ast)
- ``parse_native(text)`` -- text --> native Python types

Algorithm: from_ast(node)
-------------------------

The core of this module is ``from_ast()``, a recursive tree walker that
converts the generic ``ASTNode`` tree produced by ``json-parser`` into
a typed ``JsonValue`` tree.

The JSON grammar produces ASTs with these rule names:

- ``"value"`` -- the top-level wrapper; contains exactly one meaningful child
- ``"object"`` -- ``{ pair, pair, ... }``; children include LBRACE, pairs, RBRACE
- ``"pair"`` -- ``STRING : value``; children are STRING token, COLON token, value node
- ``"array"`` -- ``[ value, value, ... ]``; children include LBRACKET, values, RBRACKET

And these token types for leaf values:

- ``STRING`` -- already unescaped by the lexer (``"hello"`` becomes ``hello``)
- ``NUMBER`` -- the numeric text (``42``, ``3.14``, ``1e10``)
- ``TRUE`` / ``FALSE`` -- boolean literals
- ``NULL`` -- the null literal
- ``LBRACE``, ``RBRACE``, ``LBRACKET``, ``RBRACKET``, ``COLON``, ``COMMA`` --
  structural tokens (we skip these)
"""

from __future__ import annotations

from typing import Any

from lang_parser import ASTNode
from lexer import Token

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


# ---------------------------------------------------------------------------
# Helper: extract the type name from a Token
# ---------------------------------------------------------------------------


def _token_type_name(token: Token) -> str:
    """Get the string name of a token's type.

    Tokens can have either an ``Enum`` type (from the hand-written lexer)
    or a plain ``str`` type (from the grammar-driven lexer). This helper
    normalizes both to a string so we can use simple string comparisons.

    Example::

        _token_type_name(Token(TokenType.NUMBER, "42", 1, 1))  # "NUMBER"
        _token_type_name(Token("NUMBER", "42", 1, 1))          # "NUMBER"
    """
    return token.type if isinstance(token.type, str) else token.type.name


# ---------------------------------------------------------------------------
# The set of token types that represent meaningful JSON values (not structure)
# ---------------------------------------------------------------------------

# These are the token types that carry data. Everything else (LBRACE, RBRACE,
# LBRACKET, RBRACKET, COLON, COMMA) is structural punctuation that we skip.
_VALUE_TOKEN_TYPES = frozenset({"STRING", "NUMBER", "TRUE", "FALSE", "NULL"})


# ---------------------------------------------------------------------------
# JSON string escape processing
# ---------------------------------------------------------------------------


def _unescape_json_string(s: str) -> str:
    """Decode JSON escape sequences in a string value.

    The JSON grammar (json.tokens) uses ``escapes: none``, which tells the
    lexer to strip the surrounding quotes from STRING tokens but leave all
    escape sequences as raw character pairs (e.g. ``\\n`` stays as two
    characters: backslash + ``n``).  This function decodes those raw
    sequences into the corresponding Unicode characters, matching the
    behaviour of ``json.loads`` on the string content.

    Supported escape sequences::

        \\\"  --> "      (double quote)
        \\\\  --> \\      (backslash)
        \\/   --> /      (forward slash — valid but unusual)
        \\b   --> \\x08  (backspace)
        \\f   --> \\x0c  (form feed)
        \\n   --> \\n    (newline)
        \\r   --> \\r    (carriage return)
        \\t   --> \\t    (tab)
        \\uXXXX --> the corresponding Unicode code point

    Any other two-character sequence starting with ``\\`` is left unchanged
    so that malformed inputs do not silently lose data.
    """
    result: list[str] = []
    i = 0
    while i < len(s):
        if s[i] == "\\" and i + 1 < len(s):
            next_ch = s[i + 1]
            if next_ch == '"':
                result.append('"')
                i += 2
            elif next_ch == "\\":
                result.append("\\")
                i += 2
            elif next_ch == "/":
                result.append("/")
                i += 2
            elif next_ch == "b":
                result.append("\b")
                i += 2
            elif next_ch == "f":
                result.append("\f")
                i += 2
            elif next_ch == "n":
                result.append("\n")
                i += 2
            elif next_ch == "r":
                result.append("\r")
                i += 2
            elif next_ch == "t":
                result.append("\t")
                i += 2
            elif next_ch == "u" and i + 5 < len(s):
                hex_digits = s[i + 2 : i + 6]
                if all(c in "0123456789abcdefABCDEF" for c in hex_digits):
                    result.append(chr(int(hex_digits, 16)))
                    i += 6
                else:
                    result.append(s[i])
                    i += 1
            else:
                # Unknown escape — preserve as-is (be lenient with malformed input).
                result.append(s[i])
                i += 1
        else:
            result.append(s[i])
            i += 1
    return "".join(result)


# ---------------------------------------------------------------------------
# from_ast: ASTNode --> JsonValue
# ---------------------------------------------------------------------------


def from_ast(node: ASTNode | Token) -> JsonValue:
    """Convert a json-parser AST node into a typed JsonValue.

    This is a recursive tree walk. It dispatches on the node type:

    - **Token** (leaf node): Convert based on token type.
      - ``STRING`` --> ``JsonString(token.value)``
      - ``NUMBER`` --> ``JsonNumber(int or float)``
      - ``TRUE`` --> ``JsonBool(True)``
      - ``FALSE`` --> ``JsonBool(False)``
      - ``NULL`` --> ``JsonNull()``

    - **ASTNode** (interior node): Convert based on rule name.
      - ``"value"`` --> unwrap and recurse into the meaningful child
      - ``"object"`` --> collect pairs into ``JsonObject``
      - ``"array"`` --> collect elements into ``JsonArray``
      - ``"pair"`` --> should not be called directly (handled by object)

    Args:
        node: An ``ASTNode`` or ``Token`` from the json-parser.

    Returns:
        A ``JsonValue`` representing the parsed JSON data.

    Raises:
        JsonValueError: If the AST has an unexpected structure.

    Example::

        from json_parser import parse_json
        from json_value.converter import from_ast

        ast = parse_json('{"name": "Alice"}')
        value = from_ast(ast)
        # value == JsonObject({"name": JsonString("Alice")})
    """
    # ---- Case 1: Token (leaf node) ----
    # Tokens are the leaves of the AST tree. Each one represents a single
    # piece of data or punctuation from the JSON text.
    if isinstance(node, Token):
        return _convert_token(node)

    # ---- Case 2: ASTNode (interior node) ----
    # ASTNodes represent grammar rules. The rule_name tells us what kind
    # of JSON construct we're looking at.
    if isinstance(node, ASTNode):
        return _convert_ast_node(node)

    # ---- Case 3: Something unexpected ----
    msg = f"Expected ASTNode or Token, got {type(node).__name__}"
    raise JsonValueError(msg)


def _convert_token(token: Token) -> JsonValue:
    """Convert a single Token into a JsonValue.

    This handles the five JSON primitive token types:

    +----------+---------------------------+-----------------------------+
    | Token    | Example value             | JsonValue produced          |
    +----------+---------------------------+-----------------------------+
    | STRING   | ``hello`` (unescaped)     | ``JsonString("hello")``     |
    | NUMBER   | ``42`` or ``3.14``        | ``JsonNumber(42 or 3.14)``  |
    | TRUE     | ``true``                  | ``JsonBool(True)``          |
    | FALSE    | ``false``                 | ``JsonBool(False)``         |
    | NULL     | ``null``                  | ``JsonNull()``              |
    +----------+---------------------------+-----------------------------+

    For NUMBER tokens, we distinguish integers from floats:

    - If the text contains a ``.`` or ``e``/``E``, it's a float.
    - Otherwise, it's an integer.

    This matches ``json.loads`` behavior: ``42`` becomes ``int``,
    ``42.0`` becomes ``float``.
    """
    type_name = _token_type_name(token)

    if type_name == "STRING":
        # The JSON lexer uses "escapes: none" (in json.tokens), which means
        # it strips the surrounding quotes but leaves escape sequences as raw
        # two-character pairs (e.g. \n stays as backslash + n).  We decode
        # those sequences here so that JsonString values contain real Unicode
        # characters, matching the behaviour of json.loads.
        return JsonString(_unescape_json_string(token.value))

    if type_name == "NUMBER":
        return _parse_number(token.value)

    if type_name == "TRUE":
        return JsonBool(True)

    if type_name == "FALSE":
        return JsonBool(False)

    if type_name == "NULL":
        return JsonNull()

    # Structural tokens (LBRACE, RBRACE, etc.) should never be passed
    # to from_ast directly -- they're handled by the parent node's logic.
    msg = f"Unexpected token type: {type_name}"
    raise JsonValueError(msg)


def _parse_number(text: str) -> JsonNumber:
    """Parse a number string into a JsonNumber.

    JSON numbers follow this grammar::

        number = [ "-" ] int [ frac ] [ exp ]
        int    = "0" | ( digit1-9 { digit } )
        frac   = "." { digit }
        exp    = ( "e" | "E" ) [ "+" | "-" ] { digit }

    Our rule for integer vs. float:

    - Contains ``.`` or ``e`` or ``E`` --> float
    - Otherwise --> integer

    This means:
    - ``42`` --> ``JsonNumber(42)`` (int)
    - ``-17`` --> ``JsonNumber(-17)`` (int)
    - ``0`` --> ``JsonNumber(0)`` (int)
    - ``3.14`` --> ``JsonNumber(3.14)`` (float)
    - ``1e10`` --> ``JsonNumber(10000000000.0)`` (float)
    - ``-0.5e-2`` --> ``JsonNumber(-0.005)`` (float)
    """
    if "." in text or "e" in text or "E" in text:
        return JsonNumber(float(text))
    return JsonNumber(int(text))


def _convert_ast_node(node: ASTNode) -> JsonValue:
    """Convert an ASTNode (interior node) into a JsonValue.

    Dispatches on ``node.rule_name``:

    - ``"value"`` -- unwrap the single meaningful child
    - ``"object"`` -- collect pairs from child ``"pair"`` nodes
    - ``"array"`` -- collect elements from child ``"value"`` nodes
    """
    if node.rule_name == "value":
        return _convert_value_node(node)

    if node.rule_name == "object":
        return _convert_object_node(node)

    if node.rule_name == "array":
        return _convert_array_node(node)

    if node.rule_name == "pair":
        # Pairs are handled by _convert_object_node. If someone calls
        # from_ast on a pair node directly, we can still handle it by
        # returning the value part.
        _key, value = _extract_pair(node)
        return value

    msg = f"Unknown AST rule: {node.rule_name}"
    raise JsonValueError(msg)


def _convert_value_node(node: ASTNode) -> JsonValue:
    """Convert a ``"value"`` node.

    The ``value`` rule in the JSON grammar is::

        value = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;

    It always wraps exactly one meaningful child. The child is either:

    - An ``ASTNode`` with rule ``"object"`` or ``"array"``
    - A ``Token`` with type STRING, NUMBER, TRUE, FALSE, or NULL

    We scan the children and return the first meaningful one.
    """
    for child in node.children:
        # If it's an ASTNode, it must be "object" or "array" -- recurse.
        if isinstance(child, ASTNode):
            return _convert_ast_node(child)

        # If it's a value Token (not structural punctuation), convert it.
        if isinstance(child, Token):
            type_name = _token_type_name(child)
            if type_name in _VALUE_TOKEN_TYPES:
                return _convert_token(child)

    # If we get here, the AST is malformed -- no meaningful child found.
    msg = "value node has no meaningful child"
    raise JsonValueError(msg)


def _convert_object_node(node: ASTNode) -> JsonObject:
    """Convert an ``"object"`` node into a ``JsonObject``.

    An object node's children look like::

        LBRACE, [pair, COMMA, pair, COMMA, pair, ...], RBRACE

    We iterate through the children, looking for ``ASTNode`` children
    with ``rule_name="pair"``. Each pair gives us a key-value mapping.

    The ``LBRACE``, ``RBRACE``, and ``COMMA`` tokens are structural --
    we skip them.
    """
    pairs: dict[str, JsonValue] = {}

    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name == "pair":
            key, value = _extract_pair(child)
            pairs[key] = value

    return JsonObject(pairs)


def _extract_pair(pair_node: ASTNode) -> tuple[str, JsonValue]:
    """Extract a key-value pair from a ``"pair"`` node.

    A pair node's children look like::

        Token(STRING, key_text), Token(COLON, ':'), ASTNode(rule="value", ...)

    We find the STRING token (the key) and the "value" ASTNode (the value),
    then recursively convert the value.

    Returns:
        A tuple of (key_string, json_value).
    """
    key: str | None = None
    value: JsonValue | None = None

    for child in pair_node.children:
        if isinstance(child, Token) and _token_type_name(child) == "STRING":
            key = child.value
        elif isinstance(child, ASTNode) and child.rule_name == "value":
            value = from_ast(child)

    if key is None:
        msg = "pair node has no STRING key"
        raise JsonValueError(msg)

    if value is None:
        msg = "pair node has no value"
        raise JsonValueError(msg)

    return key, value


def _convert_array_node(node: ASTNode) -> JsonArray:
    """Convert an ``"array"`` node into a ``JsonArray``.

    An array node's children look like::

        LBRACKET, [value, COMMA, value, COMMA, value, ...], RBRACKET

    We iterate through the children, looking for ``ASTNode`` children
    with ``rule_name="value"``. Each value node is recursively converted.

    The ``LBRACKET``, ``RBRACKET``, and ``COMMA`` tokens are structural --
    we skip them.

    Edge case: some parser implementations might produce direct Token
    children (STRING, NUMBER, etc.) instead of wrapping them in a
    ``"value"`` ASTNode. We handle both cases.
    """
    elements: list[JsonValue] = []

    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name == "value":
            elements.append(from_ast(child))
        elif isinstance(child, Token):
            type_name = _token_type_name(child)
            if type_name in _VALUE_TOKEN_TYPES:
                elements.append(_convert_token(child))

    return JsonArray(elements)


# ---------------------------------------------------------------------------
# to_native: JsonValue --> native Python types
# ---------------------------------------------------------------------------


def to_native(
    value: JsonValue,
) -> dict[str, Any] | list[Any] | str | int | float | bool | None:
    """Convert a JsonValue tree into native Python types.

    This is the "I just want a dict" function. It recursively walks the
    JsonValue tree and produces the equivalent Python data structure:

    +---------------+-------------------+
    | JsonValue     | Python type       |
    +---------------+-------------------+
    | JsonObject    | dict              |
    | JsonArray     | list              |
    | JsonString    | str               |
    | JsonNumber    | int or float      |
    | JsonBool      | bool              |
    | JsonNull      | None              |
    +---------------+-------------------+

    The conversion is recursive -- nested JsonValues become nested dicts
    and lists.

    Args:
        value: A JsonValue to convert.

    Returns:
        The equivalent native Python value.

    Raises:
        JsonValueError: If the value is not a recognized JsonValue subclass.

    Example::

        obj = JsonObject({"name": JsonString("Alice"), "age": JsonNumber(30)})
        native = to_native(obj)
        # native == {"name": "Alice", "age": 30}
    """
    if isinstance(value, JsonNull):
        return None

    if isinstance(value, JsonBool):
        return value.value

    if isinstance(value, JsonNumber):
        return value.value

    if isinstance(value, JsonString):
        return value.value

    if isinstance(value, JsonArray):
        return [to_native(element) for element in value.elements]

    if isinstance(value, JsonObject):
        return {key: to_native(val) for key, val in value.pairs.items()}

    msg = f"Cannot convert {type(value).__name__} to native type"
    raise JsonValueError(msg)


# ---------------------------------------------------------------------------
# from_native: native Python types --> JsonValue
# ---------------------------------------------------------------------------


def from_native(
    value: dict[str, Any] | list[Any] | str | int | float | bool | None,
) -> JsonValue:
    """Convert native Python types into a JsonValue tree.

    This is the reverse of ``to_native()``. It takes a Python dict, list,
    string, number, boolean, or None and produces the equivalent JsonValue.

    +-------------------+---------------+
    | Python type       | JsonValue     |
    +-------------------+---------------+
    | dict              | JsonObject    |
    | list              | JsonArray     |
    | str               | JsonString    |
    | int               | JsonNumber    |
    | float             | JsonNumber    |
    | bool              | JsonBool      |
    | None              | JsonNull      |
    +-------------------+---------------+

    Important constraints:

    - Dict keys **must** be strings. JSON only supports string keys.
      Passing ``{1: "val"}`` raises ``JsonValueError``.
    - Only the types listed above are supported. Passing a set, function,
      or custom object raises ``JsonValueError``.

    Args:
        value: A native Python value to convert.

    Returns:
        The equivalent JsonValue.

    Raises:
        JsonValueError: If the value contains non-JSON-compatible types
            or non-string dict keys.

    Example::

        native = {"name": "Alice", "scores": [100, 95, 88]}
        json_val = from_native(native)
        # json_val == JsonObject({
        #     "name": JsonString("Alice"),
        #     "scores": JsonArray([JsonNumber(100), JsonNumber(95), JsonNumber(88)])
        # })
    """
    # ---- None --> JsonNull ----
    if value is None:
        return JsonNull()

    # ---- bool --> JsonBool ----
    # IMPORTANT: Check bool BEFORE int! In Python, bool is a subclass of int,
    # so isinstance(True, int) returns True. If we checked int first, True
    # would become JsonNumber(1) instead of JsonBool(True).
    if isinstance(value, bool):
        return JsonBool(value)

    # ---- int --> JsonNumber ----
    if isinstance(value, int):
        return JsonNumber(value)

    # ---- float --> JsonNumber ----
    if isinstance(value, float):
        return JsonNumber(value)

    # ---- str --> JsonString ----
    if isinstance(value, str):
        return JsonString(value)

    # ---- list --> JsonArray ----
    if isinstance(value, list):
        return JsonArray([from_native(item) for item in value])

    # ---- dict --> JsonObject ----
    if isinstance(value, dict):
        pairs: dict[str, JsonValue] = {}
        for key, val in value.items():
            if not isinstance(key, str):
                msg = (
                    f"JSON object keys must be strings, "
                    f"got {type(key).__name__}: {key!r}"
                )
                raise JsonValueError(msg)
            pairs[key] = from_native(val)
        return JsonObject(pairs)

    # ---- Unsupported type ----
    msg = (
        f"Cannot convert {type(value).__name__} to JsonValue. "
        f"Supported types: dict, list, str, int, float, bool, None"
    )
    raise JsonValueError(msg)


# ---------------------------------------------------------------------------
# Convenience: text --> JsonValue
# ---------------------------------------------------------------------------


def parse(text: str) -> JsonValue:
    """Parse JSON text into a JsonValue.

    This is the all-in-one convenience function. It chains together:

    1. ``json_parser.parse_json(text)`` -- tokenize and parse into AST
    2. ``from_ast(ast)`` -- convert AST to JsonValue

    Args:
        text: A string containing valid JSON.

    Returns:
        A ``JsonValue`` representing the parsed data.

    Raises:
        JsonValueError: If the text is not valid JSON (wraps the underlying
            lexer or parser error).

    Example::

        value = parse('{"name": "Alice", "age": 30}')
        # value == JsonObject({"name": JsonString("Alice"), ...})
    """
    try:
        from json_parser import parse_json

        ast = parse_json(text)
        return from_ast(ast)
    except Exception as exc:
        # If it's already a JsonValueError, re-raise as-is.
        if isinstance(exc, JsonValueError):
            raise
        # Wrap lexer/parser errors in JsonValueError so callers only need
        # to catch one exception type.
        msg = f"Failed to parse JSON: {exc}"
        raise JsonValueError(msg) from exc


# ---------------------------------------------------------------------------
# Convenience: text --> native Python types
# ---------------------------------------------------------------------------


def parse_native(
    text: str,
) -> dict[str, Any] | list[Any] | str | int | float | bool | None:
    """Parse JSON text directly into native Python types.

    This is the most common use case -- "give me a dict from this JSON string."

    Equivalent to ``to_native(parse(text))``.

    Args:
        text: A string containing valid JSON.

    Returns:
        A native Python value (dict, list, str, int, float, bool, or None).

    Raises:
        JsonValueError: If the text is not valid JSON.

    Example::

        data = parse_native('{"name": "Alice", "age": 30}')
        # data == {"name": "Alice", "age": 30}
        # data is a plain Python dict -- no JsonValue wrapper
    """
    return to_native(parse(text))
