"""TOML Converter — walks an AST and builds a Python dictionary.

This is the second phase of the two-phase TOML parser. The first phase
(``parser.py``) produces a syntax tree. This module walks that tree and:

1. **Builds a nested dictionary** — following TOML's table semantics.
2. **Validates semantic constraints** — things a context-free grammar cannot
   express: key uniqueness, table path consistency, inline table immutability.
3. **Converts values** — strips quotes from strings, processes escape
   sequences, parses numbers (hex, octal, binary, underscores), and converts
   date/time literals to Python ``datetime`` objects.

Why a Separate Converter?
-------------------------

TOML's syntax is context-free, but its semantics are not. For example:

.. code-block:: toml

    [a]
    b = 1
    [a]        # ERROR: table [a] already defined
    b = 2

Both ``[a]`` lines are syntactically valid, but the second one is a semantic
error. The grammar parser happily produces an AST for this input — the
*converter* is what catches the error.

This two-phase approach (parse → validate + convert) is how real TOML parsers
work. It keeps the grammar clean and makes errors easier to report because we
have position information from the AST.

Architecture
------------

The converter maintains several pieces of state as it walks the AST:

- ``result`` — the root ``TOMLDocument`` being built.
- ``current_table`` — pointer to the table where key-value pairs are being
  added. Changes when ``[table]`` or ``[[array]]`` headers are encountered.
- ``implicit_tables`` — tables that were created implicitly by dotted keys
  or intermediate table paths. These can be extended later.
- ``defined_tables`` — tables that were explicitly defined with ``[table]``.
  Defining the same table twice is an error.
- ``inline_tables`` — tables created by inline syntax ``{...}``. These are
  immutable — no keys can be added after creation.
- ``array_tables`` — paths that are arrays of tables (``[[array]]``). Used
  to check for conflicts with regular tables.

Error Reporting
---------------

All errors are raised as ``TOMLConversionError`` with a descriptive message.
We don't try to recover from errors — the first semantic violation stops
conversion. This is simpler and matches the behavior of Python's ``tomllib``.
"""

from __future__ import annotations

import datetime
import re
from typing import Any

from lang_parser import ASTNode
from lexer import Token

from toml_parser.types import TOMLDocument, TOMLValue

# =============================================================================
# ERROR TYPE
# =============================================================================


class TOMLConversionError(Exception):
    """Raised when the AST contains a semantic error.

    Examples of semantic errors:

    - Defining the same key twice in a table.
    - Using ``[a.b]`` when ``a.b`` is already a non-table value.
    - Adding keys to an inline table after its definition.
    - Using ``[[a]]`` and ``[a]`` for the same path.
    """


# =============================================================================
# ESCAPE SEQUENCES
# =============================================================================
#
# TOML basic strings support these escape sequences:
#
# ============ =====================================
# Escape        Meaning
# ============ =====================================
# ``\\b``       Backspace (U+0008)
# ``\\t``       Tab (U+0009)
# ``\\n``       Linefeed (U+000A)
# ``\\f``       Form feed (U+000C)
# ``\\r``       Carriage return (U+000D)
# ``\\\"``      Quote (U+0022)
# ``\\\\``      Backslash (U+005C)
# ``\\uXXXX``   Unicode (4 hex digits)
# ``\\UXXXXXXXX`` Unicode (8 hex digits)
# ============ =====================================
#
# Literal strings (single-quoted) have NO escape processing at all.

ESCAPE_MAP: dict[str, str] = {
    "b": "\b",
    "t": "\t",
    "n": "\n",
    "f": "\f",
    "r": "\r",
    '"': '"',
    "\\": "\\",
}

# Regex for matching escape sequences in basic strings.
# Matches: \b, \t, \n, \f, \r, \", \\, \uXXXX, \UXXXXXXXX
_ESCAPE_RE = re.compile(
    r'\\(b|t|n|f|r|"|\\|u[0-9a-fA-F]{4}|U[0-9a-fA-F]{8})'
)


def _process_basic_escapes(text: str) -> str:
    """Process escape sequences in a TOML basic string.

    This handles all escape sequences defined in TOML v1.0.0. Invalid
    escape sequences raise ``TOMLConversionError``.

    Args:
        text: The string content (quotes already stripped).

    Returns:
        The string with escape sequences replaced by their values.
    """

    def _replace_escape(match: re.Match[str]) -> str:
        seq = match.group(1)
        if seq in ESCAPE_MAP:
            return ESCAPE_MAP[seq]
        if seq.startswith("u"):
            return chr(int(seq[1:], 16))
        if seq.startswith("U"):
            return chr(int(seq[1:], 16))
        msg = f"Invalid escape sequence: \\{seq}"
        raise TOMLConversionError(msg)

    return _ESCAPE_RE.sub(_replace_escape, text)


def _process_ml_basic_escapes(text: str) -> str:
    """Process escape sequences in a multi-line basic string.

    Multi-line basic strings support the same escapes as basic strings,
    plus "line ending backslash" — a backslash at end of line trims the
    newline and any leading whitespace on the next line::

        key = \"\"\"\\
          hello\"\"\"

    is equivalent to ``"hello"`` (the newline and leading spaces are trimmed).
    """
    # First, handle line-ending backslash: \ followed by newline and
    # optional whitespace is collapsed to nothing.
    text = re.sub(r"\\\n\s*", "", text)
    # Then process standard escapes.
    return _process_basic_escapes(text)


# =============================================================================
# STRING PROCESSING
# =============================================================================
#
# TOML has four string types with different quoting and escape rules:
#
# ==================== =========== ========= ===========
# Token Type            Quotes      Escapes   Multi-line
# ==================== =========== ========= ===========
# BASIC_STRING          " ... "     Yes       No
# ML_BASIC_STRING       """ ... """ Yes (+LEB) Yes
# LITERAL_STRING        ' ... '     No        No
# ML_LITERAL_STRING     ''' ... ''' No        Yes
# ==================== =========== ========= ===========
#
# LEB = Line Ending Backslash (backslash before newline trims whitespace)
#
# Because we use `escapes: none` in toml.tokens, the lexer gives us the raw
# token text including surrounding quotes. We strip quotes and process escapes
# here in the converter.


def _convert_string(token_type: str, raw_value: str) -> str:
    """Convert a raw string token to a Python string.

    Strips surrounding quotes and processes escape sequences according
    to the token type.

    Args:
        token_type: One of BASIC_STRING, ML_BASIC_STRING, LITERAL_STRING,
                    ML_LITERAL_STRING.
        raw_value: The raw token text including quotes.

    Returns:
        The processed Python string.
    """
    if token_type == "ML_BASIC_STRING":
        # Strip triple quotes: """..."""
        inner = raw_value[3:-3]
        # TOML spec: a newline immediately following the opening delimiter
        # is trimmed.
        if inner.startswith("\n"):
            inner = inner[1:]
        elif inner.startswith("\r\n"):
            inner = inner[2:]
        return _process_ml_basic_escapes(inner)

    if token_type == "ML_LITERAL_STRING":
        # Strip triple quotes: '''...'''
        inner = raw_value[3:-3]
        # Same trimming rule for opening newline.
        if inner.startswith("\n"):
            inner = inner[1:]
        elif inner.startswith("\r\n"):
            inner = inner[2:]
        return inner  # No escape processing for literal strings

    if token_type == "BASIC_STRING":
        # Strip double quotes: "..."
        inner = raw_value[1:-1]
        return _process_basic_escapes(inner)

    if token_type == "LITERAL_STRING":
        # Strip single quotes: '...'
        return raw_value[1:-1]  # No escape processing

    msg = f"Unknown string token type: {token_type}"
    raise TOMLConversionError(msg)


# =============================================================================
# NUMBER CONVERSION
# =============================================================================
#
# TOML numbers can contain underscores as visual separators (like Python):
#   1_000_000  →  1000000
#   0xFF_FF    →  65535
#   3.14_15    →  3.1415
#
# The lexer preserves underscores in the token value. We strip them here.


def _convert_integer(raw: str) -> int:
    """Convert a raw integer token to a Python int.

    Handles decimal, hexadecimal (0x), octal (0o), and binary (0b) formats,
    all with optional underscore separators and optional leading sign.

    Args:
        raw: The raw token text (e.g., "1_000", "0xFF", "+42").

    Returns:
        The integer value.
    """
    # Strip underscores — they're visual separators only.
    clean = raw.replace("_", "")

    # Detect base from prefix.
    if clean.startswith(("0x", "0X", "+0x", "-0x", "+0X", "-0X")):
        return int(clean, 16)
    if clean.startswith(("0o", "0O", "+0o", "-0o", "+0O", "-0O")):
        return int(clean, 8)
    if clean.startswith(("0b", "0B", "+0b", "-0b", "+0B", "-0B")):
        return int(clean, 2)
    return int(clean)


def _convert_float(raw: str) -> float:
    """Convert a raw float token to a Python float.

    Handles decimal, scientific notation, and special values (inf, nan),
    all with optional underscore separators and optional leading sign.

    Args:
        raw: The raw token text (e.g., "3.14", "1e10", "inf", "+nan").

    Returns:
        The float value.
    """
    clean = raw.replace("_", "")

    # Special values.
    if clean in ("inf", "+inf"):
        return float("inf")
    if clean == "-inf":
        return float("-inf")
    if clean in ("nan", "+nan", "-nan"):
        return float("nan")

    return float(clean)


# =============================================================================
# DATE/TIME CONVERSION
# =============================================================================
#
# TOML date/time types map directly to Python's datetime module:
#
# ==================== ================================
# TOML Type             Python Type
# ==================== ================================
# Offset Date-Time      datetime.datetime (with tzinfo)
# Local Date-Time       datetime.datetime (no tzinfo)
# Local Date            datetime.date
# Local Time            datetime.time
# ==================== ================================
#
# The T separator between date and time can also be a space:
#   1979-05-27T07:32:00Z
#   1979-05-27 07:32:00Z


def _convert_offset_datetime(raw: str) -> datetime.datetime:
    """Convert an offset datetime string to a Python datetime.

    Args:
        raw: e.g., "1979-05-27T07:32:00Z" or "1979-05-27 07:32:00+09:00"

    Returns:
        A timezone-aware ``datetime.datetime``.
    """
    # Normalize: replace space separator with T for fromisoformat.
    normalized = raw.replace(" ", "T", 1)
    # Python 3.11+ fromisoformat handles Z suffix.
    return datetime.datetime.fromisoformat(normalized)


def _convert_local_datetime(raw: str) -> datetime.datetime:
    """Convert a local datetime string to a Python datetime (no timezone).

    Args:
        raw: e.g., "1979-05-27T07:32:00" or "1979-05-27 07:32:00.999"

    Returns:
        A naive ``datetime.datetime`` (no tzinfo).
    """
    normalized = raw.replace(" ", "T", 1)
    return datetime.datetime.fromisoformat(normalized)


def _convert_local_date(raw: str) -> datetime.date:
    """Convert a local date string to a Python date.

    Args:
        raw: e.g., "1979-05-27"

    Returns:
        A ``datetime.date``.
    """
    return datetime.date.fromisoformat(raw)


def _convert_local_time(raw: str) -> datetime.time:
    """Convert a local time string to a Python time.

    Args:
        raw: e.g., "07:32:00" or "07:32:00.999999"

    Returns:
        A ``datetime.time``.
    """
    return datetime.time.fromisoformat(raw)


# =============================================================================
# AST WALKER — The Core Converter
# =============================================================================
#
# The converter walks the AST top-down. The document-level structure is:
#
#   document
#   ├── NEWLINE  (ignored)
#   ├── expression
#   │   └── keyval | table_header | array_table_header
#   ├── NEWLINE
#   ├── expression
#   │   └── ...
#   └── ...
#
# The key-value pairs are added to the "current table", which starts as the
# root document. Table headers ([section]) change the current table. Array
# table headers ([[section]]) append a new table to an array and set that
# as the current table.


class _Converter:
    """Internal state machine that walks a TOML AST and builds a document.

    This class is not part of the public API. Use ``convert_ast()`` instead.
    """

    def __init__(self) -> None:
        # The root document being built.
        self.result = TOMLDocument()

        # The table where key-value pairs are currently being added.
        # Starts as the root, changes on [table] and [[array]] headers.
        self.current_table: TOMLDocument = self.result

        # Track the current table path for error messages.
        self.current_path: list[str] = []

        # Tables explicitly defined with [table] headers. We track these
        # to detect duplicate definitions. The key is the dotted path as
        # a tuple of strings (e.g., ("server", "database")).
        self.defined_tables: set[tuple[str, ...]] = set()

        # Tables created implicitly — either as intermediate tables in a
        # dotted key (a.b.c = 1 creates implicit tables "a" and "a.b")
        # or as intermediate tables in a table path ([a.b.c] creates
        # implicit tables "a" and "a.b"). Implicit tables CAN be later
        # explicitly defined with [table].
        self.implicit_tables: set[tuple[str, ...]] = set()

        # Tables created by inline syntax { ... }. These are immutable:
        # once created, no keys can be added. The key is the tuple path.
        self.inline_tables: set[tuple[str, ...]] = set()

        # Paths that are arrays of tables (created by [[array]]).
        # Used to check for conflicts with regular [table] definitions.
        self.array_tables: set[tuple[str, ...]] = set()

    # -----------------------------------------------------------------
    # Public entry point
    # -----------------------------------------------------------------

    def convert(self, ast: ASTNode) -> TOMLDocument:
        """Walk the AST and build a TOMLDocument.

        Args:
            ast: The root ASTNode (rule_name="document").

        Returns:
            A ``TOMLDocument`` containing the parsed TOML data.

        Raises:
            TOMLConversionError: If a semantic constraint is violated.
        """
        for child in ast.children:
            if isinstance(child, Token):
                # Top-level tokens are NEWLINEs — skip them.
                continue
            # child is an ASTNode — must be an "expression" node.
            self._convert_expression(child)

        return self.result

    # -----------------------------------------------------------------
    # Expression dispatch
    # -----------------------------------------------------------------

    def _convert_expression(self, node: ASTNode) -> None:
        """Dispatch an expression node to the appropriate handler.

        An expression is one of:
        - ``keyval`` — a key-value pair (name = "value")
        - ``table_header`` — a table section ([server])
        - ``array_table_header`` — an array of tables ([[products]])
        """
        # The expression node wraps exactly one child node.
        inner = node.children[0]
        if isinstance(inner, Token):
            # Should not happen in well-formed AST, but handle gracefully.
            return

        if inner.rule_name == "keyval":
            self._convert_keyval(inner, self.current_table, self.current_path)
        elif inner.rule_name == "table_header":
            self._convert_table_header(inner)
        elif inner.rule_name == "array_table_header":
            self._convert_array_table_header(inner)

    # -----------------------------------------------------------------
    # Key extraction
    # -----------------------------------------------------------------

    def _extract_key_parts(self, key_node: ASTNode) -> list[str]:
        """Extract the dotted key parts from a ``key`` AST node.

        A TOML key like ``a.b.c`` is parsed as::

            key
            ├── simple_key (BARE_KEY "a")
            ├── DOT
            ├── simple_key (BARE_KEY "b")
            ├── DOT
            └── simple_key (BARE_KEY "c")

        This method extracts ["a", "b", "c"].

        Args:
            key_node: An ASTNode with rule_name="key".

        Returns:
            A list of key strings.
        """
        parts: list[str] = []
        for child in key_node.children:
            if isinstance(child, Token):
                # DOT tokens between key parts — skip.
                continue
            # child is a simple_key ASTNode.
            parts.append(self._extract_simple_key(child))
        return parts

    def _extract_simple_key(self, node: ASTNode) -> str:
        """Extract the string value from a ``simple_key`` AST node.

        A simple_key wraps a single token. The token type determines how
        the key text is extracted:

        - BARE_KEY, TRUE, FALSE, INTEGER, FLOAT, date/time tokens: use
          the raw token value as the key string.
        - BASIC_STRING, LITERAL_STRING: strip quotes and process escapes.

        Args:
            node: An ASTNode with rule_name="simple_key".

        Returns:
            The key as a Python string.
        """
        # simple_key always wraps exactly one token.
        token = node.children[0]
        if not isinstance(token, Token):
            msg = "Expected token in simple_key node"
            raise TOMLConversionError(msg)

        token_type = str(token.type)

        # String keys need quote stripping and escape processing.
        if token_type in ("BASIC_STRING", "LITERAL_STRING"):
            return _convert_string(token_type, token.value)

        # All other key types use the raw token value.
        # This includes BARE_KEY, TRUE, FALSE, INTEGER, FLOAT, and all
        # date/time types. In TOML, "true" as a key is the string "true",
        # not the boolean value true.
        return token.value

    # -----------------------------------------------------------------
    # Key-value pairs
    # -----------------------------------------------------------------

    def _convert_keyval(
        self,
        node: ASTNode,
        target: TOMLDocument,
        base_path: list[str],
    ) -> None:
        """Convert a ``keyval`` node and add it to the target table.

        A keyval node looks like::

            keyval
            ├── key
            │   ├── simple_key (BARE_KEY "name")
            │   ├── DOT
            │   └── simple_key (BARE_KEY "first")
            ├── EQUALS
            └── value
                └── BASIC_STRING "Tom"

        For dotted keys like ``name.first = "Tom"``, this creates intermediate
        tables as needed: ``{"name": {"first": "Tom"}}``.

        Args:
            node: An ASTNode with rule_name="keyval".
            target: The table to add the key-value pair to.
            base_path: The current table path (for error reporting).
        """
        # Extract key and value nodes from the keyval children.
        key_node = None
        value_node = None
        for child in node.children:
            if isinstance(child, ASTNode):
                if child.rule_name == "key":
                    key_node = child
                elif child.rule_name == "value":
                    value_node = child

        if key_node is None or value_node is None:
            msg = "Malformed keyval node"
            raise TOMLConversionError(msg)

        key_parts = self._extract_key_parts(key_node)
        value = self._convert_value(value_node)

        # Navigate through dotted key parts, creating intermediate tables.
        # For "a.b.c = 1", we need to:
        #   1. Get or create table "a" in target
        #   2. Get or create table "b" in "a"
        #   3. Set "c" = 1 in "b"
        table = target
        path = list(base_path)

        for part in key_parts[:-1]:
            path.append(part)
            path_tuple = tuple(path)

            # Check: can't extend an inline table.
            if path_tuple in self.inline_tables:
                msg = (
                    f"Cannot add keys to inline table "
                    f"'{'.'.join(path)}'"
                )
                raise TOMLConversionError(msg)

            if part in table:
                existing = table[part]
                if isinstance(existing, TOMLDocument):
                    table = existing
                elif isinstance(existing, list) and existing:
                    # For array of tables, target the last element.
                    last = existing[-1]
                    if isinstance(last, TOMLDocument):
                        table = last
                    else:
                        msg = (
                            f"Key '{'.'.join(path)}' is already defined "
                            f"as a non-table value"
                        )
                        raise TOMLConversionError(msg)
                else:
                    msg = (
                        f"Key '{'.'.join(path)}' is already defined "
                        f"as a non-table value"
                    )
                    raise TOMLConversionError(msg)
            else:
                new_table = TOMLDocument()
                table[part] = new_table
                self.implicit_tables.add(path_tuple)
                table = new_table

        # Set the final key.
        final_key = key_parts[-1]
        full_path = tuple([*path, final_key])

        # Check: can't extend an inline table.
        if tuple(path) in self.inline_tables:
            msg = (
                f"Cannot add keys to inline table "
                f"'{'.'.join(path)}'"
            )
            raise TOMLConversionError(msg)

        if final_key in table:
            msg = (
                f"Duplicate key: '{'.'.join([*path, final_key])}'"
            )
            raise TOMLConversionError(msg)

        table[final_key] = value

        # If the value is an inline table, mark it as immutable.
        if isinstance(value, TOMLDocument):
            self.inline_tables.add(full_path)
            # Also mark all nested tables within the inline table.
            self._mark_inline_tables(value, full_path)

    def _mark_inline_tables(
        self,
        table: TOMLDocument,
        path: tuple[str, ...],
    ) -> None:
        """Recursively mark all nested tables within an inline table.

        Inline tables and all their sub-tables are immutable. If an inline
        table contains nested inline tables, those are also marked.
        """
        for key, value in table.items():
            child_path = (*path, key)
            if isinstance(value, TOMLDocument):
                self.inline_tables.add(child_path)
                self._mark_inline_tables(value, child_path)

    # -----------------------------------------------------------------
    # Table headers: [table]
    # -----------------------------------------------------------------

    def _convert_table_header(self, node: ASTNode) -> None:
        """Process a ``[table]`` header.

        This changes the current table to the one named by the header.
        If the table doesn't exist yet, it is created. The grammar structure
        is::

            table_header
            ├── LBRACKET
            ├── key
            │   ├── simple_key (BARE_KEY "server")
            │   ├── DOT
            │   └── simple_key (BARE_KEY "database")
            └── RBRACKET

        Semantic rules:
        - A table cannot be defined twice (unless it was implicitly created).
        - A table path cannot conflict with an array of tables.
        - A table path cannot extend an inline table.
        """
        # Extract the key from between the brackets.
        key_node = None
        for child in node.children:
            if isinstance(child, ASTNode) and child.rule_name == "key":
                key_node = child
                break

        if key_node is None:
            msg = "Malformed table_header node"
            raise TOMLConversionError(msg)

        key_parts = self._extract_key_parts(key_node)
        path_tuple = tuple(key_parts)

        # Check: cannot redefine an explicitly defined table.
        if path_tuple in self.defined_tables:
            msg = f"Table '[{'.'.join(key_parts)}]' already defined"
            raise TOMLConversionError(msg)

        # Check: cannot use [table] for a path that's an array of tables.
        if path_tuple in self.array_tables:
            msg = (
                f"Cannot define '[{'.'.join(key_parts)}]' as a table — "
                f"it is already defined as an array of tables"
            )
            raise TOMLConversionError(msg)

        # Check: cannot extend an inline table.
        if path_tuple in self.inline_tables:
            msg = (
                f"Cannot extend inline table "
                f"'[{'.'.join(key_parts)}]'"
            )
            raise TOMLConversionError(msg)

        # Navigate/create the table path from the root.
        table = self.result
        for i, part in enumerate(key_parts):
            sub_path = tuple(key_parts[: i + 1])

            # Check: intermediate paths cannot be inline tables.
            if sub_path in self.inline_tables:
                msg = (
                    f"Cannot extend inline table "
                    f"'{'.'.join(key_parts[:i+1])}'"
                )
                raise TOMLConversionError(msg)

            if part in table:
                existing = table[part]
                if isinstance(existing, TOMLDocument):
                    table = existing
                elif isinstance(existing, list) and existing:
                    # For array of tables, target the last element.
                    last = existing[-1]
                    if isinstance(last, TOMLDocument):
                        table = last
                    else:
                        msg = (
                            f"Key '{'.'.join(key_parts[:i+1])}' is not a table"
                        )
                        raise TOMLConversionError(msg)
                else:
                    msg = (
                        f"Key '{'.'.join(key_parts[:i+1])}' is already "
                        f"defined as a non-table value"
                    )
                    raise TOMLConversionError(msg)
            else:
                new_table = TOMLDocument()
                table[part] = new_table
                # Intermediate tables are implicit.
                if i < len(key_parts) - 1:
                    self.implicit_tables.add(sub_path)
                table = new_table

        self.defined_tables.add(path_tuple)
        # Remove from implicit if it was previously implicit.
        self.implicit_tables.discard(path_tuple)
        self.current_table = table
        self.current_path = list(key_parts)

    # -----------------------------------------------------------------
    # Array-of-tables headers: [[array]]
    # -----------------------------------------------------------------

    def _convert_array_table_header(self, node: ASTNode) -> None:
        """Process a ``[[array]]`` header.

        Each ``[[array]]`` header appends a new table to the array and sets
        it as the current table. The grammar structure is::

            array_table_header
            ├── LBRACKET
            ├── LBRACKET
            ├── key
            ├── RBRACKET
            └── RBRACKET

        Semantic rules:
        - Cannot use [[array]] for a path already defined as a regular [table].
        - Cannot extend an inline table.
        """
        # Extract the key from between the double brackets.
        key_node = None
        for child in node.children:
            if isinstance(child, ASTNode) and child.rule_name == "key":
                key_node = child
                break

        if key_node is None:
            msg = "Malformed array_table_header node"
            raise TOMLConversionError(msg)

        key_parts = self._extract_key_parts(key_node)
        path_tuple = tuple(key_parts)

        # Check: cannot use [[array]] for a path that's a regular [table].
        if path_tuple in self.defined_tables:
            msg = (
                f"Cannot define '[[{'.'.join(key_parts)}]]' — "
                f"already defined as a regular table"
            )
            raise TOMLConversionError(msg)

        # Check: cannot extend an inline table.
        if path_tuple in self.inline_tables:
            msg = (
                f"Cannot extend inline table "
                f"'[[{'.'.join(key_parts)}]]'"
            )
            raise TOMLConversionError(msg)

        # Navigate/create intermediate tables from root.
        table = self.result
        for i, part in enumerate(key_parts[:-1]):
            sub_path = tuple(key_parts[: i + 1])

            if sub_path in self.inline_tables:
                msg = (
                    f"Cannot extend inline table "
                    f"'{'.'.join(key_parts[:i+1])}'"
                )
                raise TOMLConversionError(msg)

            if part in table:
                existing = table[part]
                if isinstance(existing, TOMLDocument):
                    table = existing
                elif isinstance(existing, list) and existing:
                    last = existing[-1]
                    if isinstance(last, TOMLDocument):
                        table = last
                    else:
                        msg = (
                            f"Key '{'.'.join(key_parts[:i+1])}' is not a table"
                        )
                        raise TOMLConversionError(msg)
                else:
                    msg = (
                        f"Key '{'.'.join(key_parts[:i+1])}' is already "
                        f"defined as a non-table value"
                    )
                    raise TOMLConversionError(msg)
            else:
                new_table = TOMLDocument()
                table[part] = new_table
                self.implicit_tables.add(sub_path)
                table = new_table

        # The last key part is the array itself.
        array_key = key_parts[-1]

        if array_key in table:
            existing = table[array_key]
            if not isinstance(existing, list):
                msg = (
                    f"Key '{'.'.join(key_parts)}' is already defined "
                    f"as a non-array value"
                )
                raise TOMLConversionError(msg)
            new_entry = TOMLDocument()
            existing.append(new_entry)
        else:
            new_entry = TOMLDocument()
            table[array_key] = [new_entry]

        self.array_tables.add(path_tuple)
        self.current_table = new_entry
        self.current_path = list(key_parts)

    # -----------------------------------------------------------------
    # Value conversion
    # -----------------------------------------------------------------

    def _convert_value(self, node: ASTNode) -> TOMLValue:
        """Convert a ``value`` AST node to a Python value.

        A value node wraps either:
        - A single token (string, number, boolean, date/time)
        - A sub-rule (array, inline_table)

        Args:
            node: An ASTNode with rule_name="value".

        Returns:
            The corresponding Python value.
        """
        child = node.children[0]

        # Token child — scalar value.
        if isinstance(child, Token):
            return self._convert_token_value(child)

        # ASTNode child — compound value (array or inline_table).
        if child.rule_name == "array":
            return self._convert_array(child)
        if child.rule_name == "inline_table":
            return self._convert_inline_table(child)

        msg = f"Unexpected value node: {child.rule_name}"
        raise TOMLConversionError(msg)

    def _convert_token_value(self, token: Token) -> TOMLValue:
        """Convert a scalar token to a Python value.

        The token type determines the conversion:

        ===================== ==========================
        Token Type             Python Result
        ===================== ==========================
        BASIC_STRING           str (escapes processed)
        ML_BASIC_STRING        str (escapes + ML rules)
        LITERAL_STRING         str (raw)
        ML_LITERAL_STRING      str (raw, ML trimming)
        INTEGER                int
        FLOAT                  float
        TRUE                   True
        FALSE                  False
        OFFSET_DATETIME        datetime.datetime (aware)
        LOCAL_DATETIME         datetime.datetime (naive)
        LOCAL_DATE             datetime.date
        LOCAL_TIME             datetime.time
        ===================== ==========================
        """
        token_type = str(token.type)

        # Strings
        if token_type in (
            "BASIC_STRING",
            "ML_BASIC_STRING",
            "LITERAL_STRING",
            "ML_LITERAL_STRING",
        ):
            return _convert_string(token_type, token.value)

        # Numbers
        if token_type == "INTEGER":
            return _convert_integer(token.value)
        if token_type == "FLOAT":
            return _convert_float(token.value)

        # Booleans
        if token_type == "TRUE":
            return True
        if token_type == "FALSE":
            return False

        # Date/time types
        if token_type == "OFFSET_DATETIME":
            return _convert_offset_datetime(token.value)
        if token_type == "LOCAL_DATETIME":
            return _convert_local_datetime(token.value)
        if token_type == "LOCAL_DATE":
            return _convert_local_date(token.value)
        if token_type == "LOCAL_TIME":
            return _convert_local_time(token.value)

        msg = f"Unknown token type in value position: {token_type}"
        raise TOMLConversionError(msg)

    # -----------------------------------------------------------------
    # Arrays
    # -----------------------------------------------------------------

    def _convert_array(self, node: ASTNode) -> list[Any]:
        """Convert an ``array`` AST node to a Python list.

        Array structure::

            array
            ├── LBRACKET
            ├── array_values
            │   ├── NEWLINE (optional)
            │   ├── value
            │   ├── COMMA
            │   ├── value
            │   └── NEWLINE (optional)
            └── RBRACKET

        TOML arrays can be heterogeneous (mixed types), which is valid per
        the TOML v1.0.0 spec.
        """
        result: list[Any] = []

        # Find the array_values node.
        for child in node.children:
            if isinstance(child, ASTNode) and child.rule_name == "array_values":
                self._collect_array_values(child, result)
                break

        return result

    def _collect_array_values(
        self, node: ASTNode, result: list[Any],
    ) -> None:
        """Collect values from an ``array_values`` node into a list.

        Walks the array_values children, skipping NEWLINEs and COMAs,
        and converting each value node.
        """
        for child in node.children:
            if isinstance(child, Token):
                # NEWLINE, COMMA — structural tokens, skip.
                continue
            if isinstance(child, ASTNode) and child.rule_name == "value":
                result.append(self._convert_value(child))

    # -----------------------------------------------------------------
    # Inline tables
    # -----------------------------------------------------------------

    def _convert_inline_table(self, node: ASTNode) -> TOMLDocument:
        """Convert an ``inline_table`` AST node to a TOMLDocument.

        Inline table structure::

            inline_table
            ├── LBRACE
            ├── keyval
            ├── COMMA
            ├── keyval
            └── RBRACE

        Inline tables are immutable — once created, no additional keys can
        be added via [table] headers or dotted keys. This immutability is
        tracked but enforced when later code tries to modify the table.
        """
        table = TOMLDocument()

        for child in node.children:
            if isinstance(child, ASTNode) and child.rule_name == "keyval":
                self._convert_keyval(child, table, [])

        return table


# =============================================================================
# PUBLIC API
# =============================================================================


def convert_ast(ast: ASTNode) -> TOMLDocument:
    """Convert a TOML AST into a Python dictionary.

    This is the second phase of the two-phase parser. The AST (from
    ``parse_toml_ast()``) captures syntax; this function validates
    semantics and produces a usable Python dictionary.

    Semantic constraints enforced:

    1. **Key uniqueness** — no duplicate keys within the same table.
    2. **Table path consistency** — ``[a.b]`` cannot overwrite a non-table.
    3. **Inline table immutability** — ``{ ... }`` tables cannot be extended.
    4. **Array-of-tables consistency** — ``[[a]]`` and ``[a]`` conflict.

    Args:
        ast: The root ASTNode from ``parse_toml_ast()``.

    Returns:
        A ``TOMLDocument`` containing the fully converted TOML data.
        All values are native Python types.

    Raises:
        TOMLConversionError: If any semantic constraint is violated.

    Example::

        from toml_parser.parser import parse_toml_ast
        from toml_parser.converter import convert_ast

        ast = parse_toml_ast('name = "TOML"\\nversion = "1.0.0"')
        doc = convert_ast(ast)
        # TOMLDocument({"name": "TOML", "version": "1.0.0"})
    """
    converter = _Converter()
    return converter.convert(ast)
