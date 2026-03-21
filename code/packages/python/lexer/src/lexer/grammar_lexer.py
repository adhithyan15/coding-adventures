"""
Grammar-Driven Lexer — Tokenization from .tokens Files
=======================================================

The hand-written ``Lexer`` in ``tokenizer.py`` hardcodes which characters map
to which tokens. That works well for a single language, but what if you want
to tokenize Python *and* Ruby *and* JavaScript with the same codebase? You
would need to rewrite the character-dispatching logic for each language.

This module takes a different approach, inspired by classic tools like
`Lex <https://en.wikipedia.org/wiki/Lex_(software)>`_ and
`Flex <https://en.wikipedia.org/wiki/Flex_(lexical_analyser_generator)>`_.
Instead of hardcoding patterns in Python, we read token definitions from a
``.tokens`` file (parsed by the ``grammar_tools`` package) and use those
definitions to drive tokenization at runtime.

How It Works — The Big Picture
------------------------------

A ``.tokens`` file looks like this::

    NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
    NUMBER = /[0-9]+/
    PLUS   = "+"
    MINUS  = "-"

    keywords:
      if
      else

Each line defines a token: a name and a pattern. The pattern is either a
regex (``/.../.``) or a literal string (``"..."``). The ``grammar_tools``
package parses this file into a ``TokenGrammar`` object — a structured list
of ``TokenDefinition`` objects plus a keyword list.

The ``GrammarLexer`` takes that ``TokenGrammar`` and does the following:

1. **Compile** each token definition into a Python ``re.Pattern`` object.
   Literal patterns are escaped so that characters like ``+`` and ``*`` are
   treated as literal characters, not regex operators.

2. **At each position** in the source code, try each compiled pattern in
   order (first match wins). This is the "priority" mechanism — if two
   patterns could match at the same position, the one that appears first
   in the ``.tokens`` file wins.

3. **Emit a Token** with the matched type and value, using the same
   ``Token`` dataclass as the hand-written lexer. The token type is either
   a ``TokenType`` enum member (for backward compatibility with simple
   grammars) or a string (for extended grammars with custom token names
   like Starlark's 45+ token types).

Extended Format Support
-----------------------

Beyond simple token definitions, this lexer handles four extensions from
the ``.tokens`` format:

1. **Skip patterns** (``skip:`` section): Patterns that are matched and
   consumed without producing tokens. Used for comments and inline
   whitespace. These replace the hardcoded whitespace-skipping logic
   when present.

2. **Type aliases** (``-> TYPE`` suffix): When a token definition has an
   alias (e.g., ``STRING_DQ = /.../ -> STRING``), the emitted token uses
   the alias as its type. This lets the grammar reference ``STRING``
   while the tokens file handles the lexical complexity.

3. **Reserved keywords** (``reserved:`` section): Identifiers that are
   syntax errors. When the lexer matches a NAME and its value is in the
   reserved set, it raises a ``LexerError`` immediately.

4. **Indentation mode** (``mode: indentation``): Python-style significant
   whitespace. The lexer maintains an indentation stack and emits synthetic
   INDENT, DEDENT, and NEWLINE tokens. Brackets suppress indentation
   processing (implicit line joining).

Why Two Lexers?
---------------

The hand-written ``Lexer`` is the **reference implementation** — clear,
well-documented, and easy to step through in a debugger. The
``GrammarLexer`` is the **grammar-driven alternative** — flexible,
language-agnostic, and data-driven. Having both lets us:

- Verify correctness by comparing their outputs
- Demonstrate two fundamentally different approaches to the same problem
- Use the hand-written lexer for teaching and the grammar-driven one
  for production grammar work
"""

from __future__ import annotations

import re

from grammar_tools import TokenGrammar

from lexer.tokenizer import LexerError, Token, TokenType

# ---------------------------------------------------------------------------
# The Grammar-Driven Lexer
# ---------------------------------------------------------------------------


class GrammarLexer:
    """A lexer driven by a ``TokenGrammar`` (parsed from a ``.tokens`` file).

    Instead of hardcoded character-matching logic, this lexer:

    1. Compiles each token definition's pattern into a regex
    2. At each position, tries each regex in definition order (first match wins)
    3. Emits a ``Token`` with the matched type and value

    This is fundamentally different from the hand-written ``Lexer``:

    - **Hand-written**: dispatch on first character, custom read methods
    - **Grammar-driven**: regex matching in priority order

    Both produce ``Token`` objects, so the parser does not care which lexer
    generated them.

    Usage::

        from grammar_tools import parse_token_grammar
        from lexer import GrammarLexer

        grammar = parse_token_grammar(open("python.tokens").read())
        tokens = GrammarLexer("x = 1 + 2", grammar).tokenize()

    Attributes:
        _source: The complete source code string being tokenized.
        _grammar: The ``TokenGrammar`` that defines which tokens to recognize.
        _pos: The current position (index) in the source string.
        _line: The current line number (1-based), for error reporting.
        _column: The current column number (1-based), for error reporting.
        _keyword_set: A pre-computed frozenset of keywords for O(1) lookup.
        _reserved_set: Reserved keywords that cause lex errors.
        _patterns: Compiled regex patterns paired with their definition names
            and aliases, in the order they should be tried.
        _skip_patterns: Compiled patterns for text that should be consumed
            silently (no token emitted). Used for comments and whitespace.
        _alias_map: Maps definition names to their aliases. When a token
            matches a definition with an alias, the alias is used as the
            emitted token type.
        _indentation_mode: Whether Python-style INDENT/DEDENT tracking is on.
    """

    def __init__(self, source: str, grammar: TokenGrammar) -> None:
        """Initialize the grammar-driven lexer.

        Args:
            source: The raw source code text to tokenize.
            grammar: A ``TokenGrammar`` object (typically parsed from a
                ``.tokens`` file using ``grammar_tools.parse_token_grammar``).
        """
        self._source = source
        self._grammar = grammar
        self._pos = 0
        self._line = 1
        self._column = 1

        # Pre-compute keyword set for fast membership testing.
        # When the lexer matches a NAME token, it checks this set to decide
        # whether the value should be reclassified as a KEYWORD.
        self._keyword_set = frozenset(grammar.keywords)

        # Reserved keywords cause immediate lex errors. In Starlark, words
        # like "class" and "import" are reserved — using them is a syntax
        # error at lex time rather than a confusing parse error later.
        self._reserved_set = frozenset(grammar.reserved_keywords)

        # Indentation mode flag. When active, the lexer maintains an
        # indentation stack and emits INDENT/DEDENT/NEWLINE tokens.
        self._indentation_mode = grammar.mode == "indentation"

        # Whether the grammar has skip patterns defined. When skip patterns
        # exist, they replace the default whitespace-skipping behavior.
        self._has_skip_patterns = len(grammar.skip_definitions) > 0

        # Escape mode controls STRING token processing.
        # None = standard (strip quotes + process escapes).
        # "none" = strip quotes only, no escape processing.
        self._escape_mode = grammar.escape_mode

        # Build alias map: definition name → alias name.
        # For example, STRING_DQ → STRING. When we match STRING_DQ, we
        # emit the token type as STRING (the alias).
        self._alias_map: dict[str, str] = {}
        for defn in grammar.definitions:
            if defn.alias:
                self._alias_map[defn.name] = defn.alias

        # Compile token patterns into regex objects.
        # -------------------------------------------
        # Order matters — patterns are tried in the order they appear in the
        # .tokens file. This is the "first match wins" rule from Lex/Flex.
        self._patterns: list[tuple[str, re.Pattern[str]]] = []
        for defn in grammar.definitions:
            if defn.is_regex:
                pattern = re.compile(defn.pattern)
            else:
                pattern = re.compile(re.escape(defn.pattern))
            self._patterns.append((defn.name, pattern))

        # Compile skip patterns (comments, whitespace, etc.).
        # These are tried before token patterns at each position.
        self._skip_patterns: list[re.Pattern[str]] = []
        for defn in grammar.skip_definitions:
            if defn.is_regex:
                self._skip_patterns.append(re.compile(defn.pattern))
            else:
                self._skip_patterns.append(re.compile(re.escape(defn.pattern)))

        # Compile error patterns (error recovery tokens).
        # These are tried as a last resort when no normal token matches.
        # Error tokens allow graceful degradation for malformed inputs —
        # for example, CSS emits BAD_STRING for unclosed strings instead
        # of crashing with a LexerError.
        self._error_patterns: list[tuple[str, re.Pattern[str]]] = []
        for defn in grammar.error_definitions:
            if defn.is_regex:
                pattern = re.compile(defn.pattern)
            else:
                pattern = re.compile(re.escape(defn.pattern))
            self._error_patterns.append((defn.name, pattern))

    # -- Main tokenization entry point ----------------------------------------

    def tokenize(self) -> list[Token]:
        """Tokenize the source code using the grammar's token definitions.

        Dispatches to the appropriate tokenization method based on whether
        indentation mode is active.

        Returns:
            A list of ``Token`` objects, always ending with an EOF token.

        Raises:
            LexerError: If an unexpected character is encountered, a reserved
                keyword is used, or indentation is inconsistent.
        """
        if self._indentation_mode:
            return self._tokenize_indentation()
        return self._tokenize_standard()

    # -- Standard (non-indentation) tokenization ------------------------------

    def _tokenize_standard(self) -> list[Token]:
        """Tokenize without indentation tracking.

        The algorithm:

        1. While there are characters left:
           a. If skip patterns exist, try them (consume silently).
           b. If no skip patterns, use default whitespace skip.
           c. If the current character is a newline, emit NEWLINE.
           d. Try each token pattern (first match wins).
           e. If nothing matches, raise LexerError.
        2. Append EOF.
        """
        tokens: list[Token] = []

        while self._pos < len(self._source):
            char = self._source[self._pos]

            # --- Skip patterns (grammar-defined) ---
            # When the grammar has skip patterns, they take over whitespace
            # handling. The lexer tries each skip pattern before token patterns.
            if self._has_skip_patterns:
                if self._try_skip():
                    continue
            else:
                # --- Default whitespace skip ---
                # Without skip patterns, use the hardcoded behavior: skip
                # spaces, tabs, carriage returns silently.
                if char in " \t\r":
                    self._advance()
                    continue

            # --- Newlines become NEWLINE tokens ---
            # Newlines are structural — they mark line boundaries.
            if char == "\n":
                tokens.append(Token(
                    type=TokenType.NEWLINE,
                    value="\\n",
                    line=self._line,
                    column=self._column,
                ))
                self._advance()
                continue

            # --- Try each token pattern (first match wins) ---
            token = self._try_match_token()
            if token is not None:
                tokens.append(token)
                continue

            # --- Try error patterns as fallback ---
            # Error patterns allow graceful degradation: instead of crashing,
            # the lexer emits an error token (e.g., BAD_STRING for unclosed
            # strings). The parser can then handle or report these tokens.
            error_token = self._try_match_error_token()
            if error_token is not None:
                tokens.append(error_token)
                continue

            raise LexerError(
                f"Unexpected character: {char!r}",
                line=self._line,
                column=self._column,
            )

        # --- Append EOF sentinel ---
        tokens.append(Token(
            type=TokenType.EOF,
            value="",
            line=self._line,
            column=self._column,
        ))

        return tokens

    # -- Indentation mode tokenization ----------------------------------------
    #
    # Indentation-sensitive languages like Python and Starlark use whitespace
    # to delimit blocks. Instead of braces or ``end`` keywords, the amount
    # of leading whitespace on each line determines the block structure.
    #
    # The algorithm is based on CPython's tokenizer (documented in PEP 7 and
    # the Python Language Reference, section 2.1.8). It works as follows:
    #
    # 1. Maintain an **indent stack** — a stack of indentation levels. It
    #    starts with [0] (no indentation).
    #
    # 2. At the beginning of each **logical line**:
    #    a. Count the number of leading spaces.
    #    b. If it's greater than the top of the stack → push and emit INDENT.
    #    c. If it's less → pop until we find a matching level, emitting DEDENT
    #       for each pop. If no level matches, it's an indentation error.
    #    d. If it's equal → same block, nothing to emit.
    #
    # 3. **Implicit line joining**: Inside brackets (parentheses, square
    #    brackets, curly braces), newlines are ignored. This means:
    #      f(1,
    #        2,
    #        3)
    #    is a single logical line. We track bracket depth to know when to
    #    suppress NEWLINE/INDENT/DEDENT.
    #
    # 4. **Blank lines** (lines containing only whitespace or comments) do
    #    not emit NEWLINE tokens and don't affect indentation.
    #
    # 5. At **EOF**, emit DEDENTs for every remaining indent level on the
    #    stack (plus a final NEWLINE if the file didn't end with one).

    def _tokenize_indentation(self) -> list[Token]:
        """Tokenize with Python-style indentation tracking.

        This method implements the full indentation algorithm described above.
        It processes the source line-by-line conceptually, but character-by-
        character in practice.

        Returns:
            A list of tokens with synthetic INDENT/DEDENT/NEWLINE tokens.
        """
        tokens: list[Token] = []

        # The indent stack tracks nesting levels. Each entry is the number
        # of spaces at that indentation level. Starts at 0 (top-level).
        indent_stack: list[int] = [0]

        # Bracket depth for implicit line joining. When > 0, newlines are
        # silently consumed and indentation changes are ignored.
        bracket_depth = 0

        # Track whether we're at the start of a logical line (need to
        # process indentation).
        at_line_start = True

        while self._pos < len(self._source):
            # --- Beginning of a logical line: process indentation ---
            if at_line_start and bracket_depth == 0:
                indent_tokens = self._process_line_start(indent_stack)
                if indent_tokens is None:
                    # Blank/comment-only line — was consumed, continue
                    continue
                tokens.extend(indent_tokens)
                at_line_start = False
                continue

            char = self._source[self._pos]

            # --- Skip patterns (comments, inline whitespace) ---
            if self._try_skip():
                continue

            # --- Newline handling ---
            if char == "\n":
                if bracket_depth > 0:
                    # Inside brackets: implicit line joining.
                    # Consume the newline silently — no NEWLINE token.
                    self._advance()
                else:
                    # End of a logical line: emit NEWLINE.
                    tokens.append(Token(
                        type="NEWLINE",
                        value="\\n",
                        line=self._line,
                        column=self._column,
                    ))
                    self._advance()
                    at_line_start = True
                continue

            # --- Track bracket depth for implicit line joining ---
            # Opening brackets increase depth; closing brackets decrease it.
            # This is how Python allows multi-line expressions inside brackets:
            #   x = (1 +
            #        2 +
            #        3)
            if char in "([{":
                bracket_depth += 1
            elif char in ")]}":
                bracket_depth = max(0, bracket_depth - 1)

            # --- Try each token pattern ---
            token = self._try_match_token()
            if token is not None:
                tokens.append(token)
                continue

            # --- Try error patterns as fallback ---
            error_token = self._try_match_error_token()
            if error_token is not None:
                tokens.append(error_token)
                continue

            raise LexerError(
                f"Unexpected character: {char!r}",
                line=self._line,
                column=self._column,
            )

        # --- EOF cleanup ---
        # If we didn't end with a newline, emit one.
        if tokens and tokens[-1].type != "NEWLINE":
            tokens.append(Token(
                type="NEWLINE",
                value="\\n",
                line=self._line,
                column=self._column,
            ))

        # Emit DEDENTs for each remaining indentation level.
        while len(indent_stack) > 1:
            indent_stack.pop()
            tokens.append(Token(
                type="DEDENT",
                value="",
                line=self._line,
                column=self._column,
            ))

        # Append EOF.
        tokens.append(Token(
            type="EOF",
            value="",
            line=self._line,
            column=self._column,
        ))

        return tokens

    def _process_line_start(
        self, indent_stack: list[int],
    ) -> list[Token] | None:
        """Process the beginning of a logical line for indentation.

        Counts leading spaces, skips blank/comment-only lines, and emits
        INDENT or DEDENT tokens as needed.

        Args:
            indent_stack: The current indentation stack (modified in place).

        Returns:
            A list of INDENT/DEDENT tokens to emit, or None if the line
            was blank/comment-only and was fully consumed.
        """
        # Count leading spaces. Tabs are not allowed in leading whitespace
        # for indentation-sensitive languages (they cause ambiguity).
        indent = 0
        while self._pos < len(self._source):
            char = self._source[self._pos]
            if char == " ":
                indent += 1
                self._advance()
            elif char == "\t":
                raise LexerError(
                    "Tab character in leading whitespace is not allowed "
                    "in indentation mode (use spaces)",
                    line=self._line,
                    column=self._column,
                )
            else:
                break

        # Check if this is a blank line or a comment-only line.
        # These don't affect indentation and don't emit NEWLINE.
        if self._pos >= len(self._source):
            return []  # EOF after whitespace
        char = self._source[self._pos]
        if char == "\n":
            # Blank line — consume and continue
            self._advance()
            return None
        if char == "#":
            # Comment-only line — consume everything up to newline
            while self._pos < len(self._source) and self._source[self._pos] != "\n":
                self._advance()
            if self._pos < len(self._source):
                self._advance()  # consume the \n
            return None

        # This is a real line with content. Compare indentation to the stack.
        tokens: list[Token] = []
        current_indent = indent_stack[-1]

        if indent > current_indent:
            # Deeper indentation → new block. Push and emit INDENT.
            indent_stack.append(indent)
            tokens.append(Token(
                type="INDENT",
                value="",
                line=self._line,
                column=1,
            ))
        elif indent < current_indent:
            # Shallower indentation → closing one or more blocks.
            # Pop the stack until we find the matching level.
            while indent_stack[-1] > indent:
                indent_stack.pop()
                tokens.append(Token(
                    type="DEDENT",
                    value="",
                    line=self._line,
                    column=1,
                ))
            # After popping, the top of the stack must equal the current
            # indent. If not, the indentation is inconsistent.
            if indent_stack[-1] != indent:
                raise LexerError(
                    f"Indentation level {indent} does not match any "
                    f"outer indentation level (stack: {indent_stack})",
                    line=self._line,
                    column=1,
                )

        return tokens

    # -- Shared helpers -------------------------------------------------------

    def _try_skip(self) -> bool:
        """Try to match and consume a skip pattern at the current position.

        Skip patterns are defined in the ``skip:`` section of a .tokens file.
        They match text that should be consumed without emitting a token —
        typically comments and inline whitespace.

        Returns:
            True if a skip pattern matched (text was consumed), False otherwise.
        """
        remaining = self._source[self._pos:]
        for pattern in self._skip_patterns:
            match = pattern.match(remaining)
            if match:
                for _ in range(len(match.group(0))):
                    self._advance()
                return True
        return False

    def _try_match_token(self) -> Token | None:
        """Try to match a token pattern at the current position.

        Tries each compiled pattern in priority order (first match wins).
        Handles keyword detection, reserved word checking, aliases, and
        string escape processing.

        Returns:
            A Token if a pattern matched, None otherwise.
        """
        remaining = self._source[self._pos:]

        for token_name, pattern in self._patterns:
            match = pattern.match(remaining)
            if match:
                value = match.group(0)
                start_line = self._line
                start_column = self._column

                # Determine the token type for this match.
                token_type = self._resolve_token_type(token_name, value)

                # Handle STRING tokens: strip quotes and process escapes.
                # We check the effective type (after alias resolution) so
                # that STRING_DQ -> STRING still gets escape processing.
                #
                # Escape mode controls what happens to STRING values:
                # - None (default): strip quotes AND process escape sequences
                # - "none": strip quotes only, leave escapes as-is
                effective_name = self._alias_map.get(token_name, token_name)
                if effective_name == "STRING" or token_name == "STRING":
                    inner = value[1:-1]
                    if self._escape_mode != "none":
                        inner = self._process_escapes(inner)
                    token = Token(
                        type=token_type,
                        value=inner,
                        line=start_line,
                        column=start_column,
                    )
                else:
                    token = Token(
                        type=token_type,
                        value=value,
                        line=start_line,
                        column=start_column,
                    )

                # Advance position by the number of characters matched.
                for _ in range(len(value)):
                    self._advance()

                return token

        return None

    def _try_match_error_token(self) -> Token | None:
        """Try to match an error pattern at the current position.

        Error patterns are a fallback — they are only tried when no normal
        token pattern matches. This allows graceful degradation for malformed
        inputs. For example, CSS can emit a ``BAD_STRING`` token for an
        unclosed string instead of crashing with a ``LexerError``.

        Error tokens are emitted with the pattern name as the token type
        (e.g., ``"BAD_STRING"``). Downstream consumers can check for these
        error token types and report or recover accordingly.

        Returns:
            A Token if an error pattern matched, None otherwise.
        """
        remaining = self._source[self._pos:]

        for error_name, pattern in self._error_patterns:
            match = pattern.match(remaining)
            if match:
                value = match.group(0)
                start_line = self._line
                start_column = self._column

                token = Token(
                    type=error_name,
                    value=value,
                    line=start_line,
                    column=start_column,
                )

                for _ in range(len(value)):
                    self._advance()

                return token

        return None

    def _advance(self) -> None:
        """Move position forward by one character, tracking line and column.

        When we encounter a newline character, we increment the line counter
        and reset the column to 1. For all other characters, we just increment
        the column.
        """
        if self._pos < len(self._source):
            if self._source[self._pos] == "\n":
                self._line += 1
                self._column = 1
            else:
                self._column += 1
            self._pos += 1

    def _resolve_token_type(
        self, token_name: str, value: str,
    ) -> TokenType | str:
        """Map a token definition name to a token type for emission.

        The resolution order:

        1. **Reserved keyword check**: If the effective type is NAME and the
           value is a reserved keyword, raise a LexerError immediately.

        2. **Keyword detection**: If the effective type is NAME and the value
           is a keyword, return KEYWORD (or the string "KEYWORD" in extended
           mode).

        3. **Alias resolution**: If the definition has an alias, use the alias
           name as the token type (e.g., STRING_DQ → STRING).

        4. **Enum mapping**: Try to find a matching TokenType enum member.
           This provides backward compatibility with simple grammars.

        5. **String fallback**: If no enum member exists, use the token name
           as a string. This handles Starlark's 45+ custom token names.

        Args:
            token_name: The definition name from the grammar
                (e.g., ``"NAME"``, ``"STRING_DQ"``, ``"FLOOR_DIV"``).
            value: The actual matched text from the source code.

        Returns:
            A TokenType enum member or a string token type name.
        """
        # Resolve alias: STRING_DQ → STRING, FLOOR_DIV_EQUALS → FLOOR_DIV_EQUALS
        effective_name = self._alias_map.get(token_name, token_name)

        # Reserved keyword check — error on reserved identifiers.
        # In Starlark, "class", "import", etc. are reserved. Using them
        # is a lex error rather than a confusing parse error.
        if effective_name == "NAME" and value in self._reserved_set:
            raise LexerError(
                f"Reserved keyword '{value}' cannot be used as an identifier",
                line=self._line,
                column=self._column,
            )

        # Keyword detection — reclassify NAME → KEYWORD when the value
        # matches a known keyword.
        if effective_name == "NAME" and value in self._keyword_set:
            try:
                return TokenType.KEYWORD
            except (KeyError, AttributeError):
                return "KEYWORD"

        # Try to map to a TokenType enum member (backward compatibility).
        try:
            return TokenType[effective_name]
        except KeyError:
            # No enum member — use the string name. This is the normal
            # path for extended grammars (Starlark, etc.) that define
            # token names beyond the basic TokenType enum.
            return effective_name

    @staticmethod
    def _process_escapes(s: str) -> str:
        r"""Process escape sequences in a string value.

        This handles the same escape sequences as the hand-written lexer:

        - ``\n`` becomes a newline character
        - ``\t`` becomes a tab character
        - ``\\`` becomes a literal backslash
        - ``\"`` becomes a literal double quote
        - Any other ``\X`` becomes just ``X`` (unknown escapes pass through)

        This ensures that ``GrammarLexer`` produces identical string values
        to the hand-written ``Lexer``.

        Args:
            s: The raw string content (after removing surrounding quotes).

        Returns:
            The string with escape sequences resolved.
        """
        result: list[str] = []
        i = 0
        while i < len(s):
            if s[i] == "\\" and i + 1 < len(s):
                next_char = s[i + 1]

                # Standard escape sequences. This map covers all escapes
                # defined by JSON (RFC 8259 section 7) and most programming
                # languages. Previously only \n, \t, \\, and \" were handled;
                # \b, \f, \r, and \/ were added for JSON support.
                escape_map = {
                    "n": "\n",      # line feed
                    "t": "\t",      # tab
                    "r": "\r",      # carriage return
                    "b": "\b",      # backspace
                    "f": "\f",      # form feed
                    "\\": "\\",     # literal backslash
                    '"': '"',       # literal double quote
                    "/": "/",       # solidus (JSON allows \/ as an escape)
                }

                if next_char in escape_map:
                    result.append(escape_map[next_char])
                    i += 2
                elif next_char == "u" and i + 5 < len(s):
                    # Unicode escape: \uXXXX where XXXX is exactly 4 hex digits.
                    # This is required by JSON (RFC 8259) and supported by most
                    # programming languages. We validate that the 4 characters
                    # after \u are valid hex digits, then convert to the
                    # corresponding Unicode character.
                    hex_str = s[i + 2 : i + 6]
                    if len(hex_str) == 4 and all(
                        c in "0123456789abcdefABCDEF" for c in hex_str
                    ):
                        result.append(chr(int(hex_str, 16)))
                        i += 6
                    else:
                        # Invalid hex digits — pass through as-is.
                        result.append(next_char)
                        i += 2
                else:
                    # Unknown escape — pass through the escaped character.
                    result.append(next_char)
                    i += 2
            else:
                result.append(s[i])
                i += 1
        return "".join(result)
