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
   ``Token`` dataclass and ``TokenType`` enum as the hand-written lexer.

Because both lexers produce identical ``Token`` objects, downstream
consumers (the parser, the evaluator) do not care which lexer generated
the tokens. You can swap one for the other freely.

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

    Both produce the same ``Token`` objects, so the parser does not care
    which lexer generated them.

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
        _patterns: Compiled regex patterns paired with their token names,
            in the order they should be tried (first match wins).
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

        # Compile token patterns into regex objects.
        # -------------------------------------------
        # Order matters here — patterns are tried in the order they appear
        # in the .tokens file. This is the "first match wins" rule that
        # Lex/Flex use. For example, if "==" is defined before "=", then
        # at a position where the source has "==", the "==" pattern will
        # match first, and we will never even try "=".
        #
        # For regex patterns (is_regex=True), we compile the pattern as-is.
        # For literal patterns (is_regex=False), we escape the pattern so
        # that characters like + and * are treated literally.
        self._patterns: list[tuple[str, re.Pattern[str]]] = []
        for defn in grammar.definitions:
            if defn.is_regex:
                pattern = re.compile(defn.pattern)
            else:
                # Literal: escape so special chars are treated literally.
                # For example, "+" becomes r"\+", which matches a literal +.
                pattern = re.compile(re.escape(defn.pattern))
            self._patterns.append((defn.name, pattern))

    # -- Main tokenization loop ----------------------------------------------

    def tokenize(self) -> list[Token]:
        """Tokenize the source code using the grammar's token definitions.

        The algorithm is straightforward:

        1. While there are characters left to process:
           a. Skip whitespace (spaces, tabs, carriage returns).
           b. If the current character is a newline, emit a NEWLINE token.
           c. Otherwise, try each compiled pattern against the remaining
              source text. The first pattern that matches wins.
           d. If no pattern matches, raise a ``LexerError``.
        2. Append an EOF token at the end.

        This is simpler than the hand-written lexer's dispatch logic because
        we do not need separate methods for reading numbers, names, strings,
        etc. The regex patterns handle all of that.

        Returns:
            A list of ``Token`` objects, always ending with an EOF token.

        Raises:
            LexerError: If an unexpected character is encountered that does
                not match any token pattern.
        """
        tokens: list[Token] = []

        while self._pos < len(self._source):
            char = self._source[self._pos]

            # --- Skip whitespace (spaces, tabs, carriage returns) ---
            # Just like the hand-written lexer, we skip horizontal whitespace
            # silently. Newlines are NOT whitespace here — they get their own
            # token because languages like Python care about line endings.
            if char in " \t\r":
                self._advance()
                continue

            # --- Newlines become NEWLINE tokens ---
            # We handle newlines specially (outside the pattern-matching loop)
            # because newlines are structural — they mark line boundaries.
            # The hand-written lexer does the same thing.
            if char == "\n":
                tokens.append(Token(
                    type=TokenType.NEWLINE,
                    value="\\n",
                    line=self._line,
                    column=self._column,
                ))
                self._advance()
                continue

            # --- Try each pattern in priority order (first match wins) ---
            # This is the core of the grammar-driven approach. We take a
            # slice of the source from the current position to the end,
            # and try to match each pattern at the START of that slice
            # (using regex's match(), not search()).
            matched = False
            remaining = self._source[self._pos:]

            for token_name, pattern in self._patterns:
                match = pattern.match(remaining)
                if match:
                    value = match.group(0)
                    start_line = self._line
                    start_column = self._column

                    # Determine the TokenType for this match.
                    token_type = self._resolve_token_type(token_name, value)

                    # Handle STRING tokens specially: strip surrounding quotes
                    # and process escape sequences, so the token value contains
                    # the actual string content (matching the hand-written lexer).
                    if token_name == "STRING":
                        inner = value[1:-1]  # strip quotes
                        inner = self._process_escapes(inner)
                        tokens.append(Token(
                            type=token_type,
                            value=inner,
                            line=start_line,
                            column=start_column,
                        ))
                    else:
                        tokens.append(Token(
                            type=token_type,
                            value=value,
                            line=start_line,
                            column=start_column,
                        ))

                    # Advance position by the number of characters matched.
                    # We advance one character at a time so that line/column
                    # tracking stays accurate (newlines inside strings, etc.).
                    for _ in range(len(value)):
                        self._advance()

                    matched = True
                    break

            if not matched:
                raise LexerError(
                    f"Unexpected character: {char!r}",
                    line=self._line,
                    column=self._column,
                )

        # --- Append the EOF sentinel ---
        # Just like the hand-written lexer, we always end with EOF so the
        # parser has a clean stop signal.
        tokens.append(Token(
            type=TokenType.EOF,
            value="",
            line=self._line,
            column=self._column,
        ))

        return tokens

    # -- Internal helpers ----------------------------------------------------

    def _advance(self) -> None:
        """Move position forward by one character, tracking line and column.

        This is identical in spirit to the hand-written lexer's ``_advance``
        method. When we encounter a newline character, we increment the line
        counter and reset the column to 1. For all other characters, we just
        increment the column.
        """
        if self._pos < len(self._source):
            if self._source[self._pos] == "\n":
                self._line += 1
                self._column = 1
            else:
                self._column += 1
            self._pos += 1

    def _resolve_token_type(self, token_name: str, value: str) -> TokenType:
        """Map a token name from the grammar to a ``TokenType`` enum value.

        This method handles two things:

        1. **Keyword detection**: If the grammar token name is ``NAME`` and the
           matched value is in the keyword set, we return ``TokenType.KEYWORD``
           instead of ``TokenType.NAME``. This is how ``if`` becomes a keyword
           while ``iffy`` stays a name.

        2. **Name-to-enum mapping**: We try to look up the token name as a
           member of the ``TokenType`` enum. For example, ``"PLUS"`` maps to
           ``TokenType.PLUS``. If there is no matching enum member (which
           shouldn't happen with a well-formed ``.tokens`` file), we fall back
           to ``TokenType.NAME``.

        Args:
            token_name: The token name from the grammar definition
                (e.g., ``"NAME"``, ``"PLUS"``, ``"NUMBER"``).
            value: The actual matched text from the source code.

        Returns:
            The appropriate ``TokenType`` enum member.
        """
        # Check if it's a NAME that should be reclassified as a KEYWORD.
        if token_name == "NAME" and value in self._keyword_set:
            return TokenType.KEYWORD

        # Map grammar token names to TokenType enum members.
        try:
            return TokenType[token_name]
        except KeyError:
            # If no direct mapping exists, default to NAME.
            # This provides a safe fallback for custom token names that
            # don't have a corresponding enum member.
            return TokenType.NAME

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
                escape_map = {"n": "\n", "t": "\t", "\\": "\\", '"': '"'}
                next_char = s[i + 1]
                result.append(escape_map.get(next_char, next_char))
                i += 2
            else:
                result.append(s[i])
                i += 1
        return "".join(result)
