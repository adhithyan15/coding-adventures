"""
Tokenizer — Breaking Source Code into Tokens
=============================================

This module implements a *lexer* (also called a *tokenizer* or *scanner*), which is
the very first phase of understanding a programming language. Before a computer can
execute code like ``x = 1 + 2``, it needs to break that raw text into meaningful
chunks. Those chunks are called **tokens**.

Think of it like reading a sentence in English. When you see:

    The cat sat on the mat.

Your brain automatically groups the letters into words: "The", "cat", "sat", "on",
"the", "mat", and the period ".". You don't think about individual letters — you
think about *words* and *punctuation*. A lexer does the same thing for source code.

Given the input ``x = 1 + 2``, the lexer produces:

    NAME("x")  EQUALS("=")  NUMBER("1")  PLUS("+")  NUMBER("2")  EOF

Each of these is a **Token** — a small labeled piece of text. The label (like NAME
or NUMBER) is called the **token type**, and the text itself (like "x" or "1") is
called the **token value**.

Why is this useful?
-------------------

The lexer simplifies everything that comes after it. The *parser* (the next stage)
doesn't have to worry about whitespace, or whether a number is one digit or five
digits. It just sees a clean stream of tokens to work with. This separation of
concerns is a fundamental principle of compiler design, established in the earliest
days of computing.

Design: Language-Agnostic
-------------------------

This lexer is designed to be **language-agnostic**. The core logic — reading numbers,
reading names, recognizing operators — is the same across many programming languages.
The only thing that changes is *which words are keywords*. In Python, ``if`` is a
keyword. In Ruby, ``elsif`` is a keyword (instead of Python's ``elif``). By making
the keyword list configurable via ``LexerConfig``, we can reuse the same lexer for
multiple languages.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto


# ---------------------------------------------------------------------------
# Token Types
# ---------------------------------------------------------------------------

class TokenType(Enum):
    """Every possible kind of token our lexer can produce.

    Think of this like a catalog of "word types" in a language. In English,
    you have nouns, verbs, adjectives, and punctuation. In a programming
    language, you have names (identifiers), numbers, operators, and so on.

    We use Python's ``Enum`` class because each token type is a distinct,
    named constant. This prevents typos — if you write ``TokenType.NMAE``
    by accident, Python raises an error immediately, whereas a plain string
    ``"NMAE"`` would silently be wrong.

    The ``auto()`` function assigns each member a unique integer value
    automatically. We don't care what the integers are — we only care about
    the *names*.
    """

    # --- Literals -----------------------------------------------------------
    # These token types represent actual values in the source code.

    NAME = auto()
    """An identifier — a name that refers to something.

    Examples: ``x``, ``print``, ``hello_world``, ``myVariable``

    In most programming languages, identifiers must start with a letter or
    underscore, and can contain letters, digits, and underscores.
    """

    NUMBER = auto()
    """An integer literal — a sequence of digits.

    Examples: ``1``, ``42``, ``1000``, ``0``

    For simplicity, this lexer only handles whole numbers (integers).
    A production lexer would also handle floating-point numbers like ``3.14``.
    """

    STRING = auto()
    """A string literal — text enclosed in double quotes.

    Examples: ``"Hello, World!"``, ``""``, ``"abc 123"``

    The quotes themselves are *not* included in the token value. If the source
    code says ``"Hello"``, the token value is just ``Hello`` (without quotes).
    The lexer also handles escape sequences like ``\\n`` (newline) and ``\\t``
    (tab) inside strings.
    """

    KEYWORD = auto()
    """A reserved word that has special meaning in the language.

    Examples (Python): ``if``, ``else``, ``while``, ``def``, ``return``
    Examples (Ruby): ``if``, ``elsif``, ``end``, ``def``, ``puts``

    Keywords look exactly like identifiers syntactically, but they are
    reserved — you can't use them as variable names. The lexer distinguishes
    keywords from regular names by checking against a configurable list.
    """

    # --- Operators ----------------------------------------------------------
    # These token types represent mathematical and assignment operators.

    PLUS = auto()
    """The ``+`` operator, used for addition."""

    MINUS = auto()
    """The ``-`` operator, used for subtraction (or negation)."""

    STAR = auto()
    """The ``*`` operator, used for multiplication."""

    SLASH = auto()
    """The ``/`` operator, used for division."""

    EQUALS = auto()
    """The ``=`` operator, used for assignment.

    Example: ``x = 5`` assigns the value 5 to the variable x.

    Important: This is a *single* equals sign. Do not confuse it with
    ``==`` (double equals), which is used for comparison.
    """

    EQUALS_EQUALS = auto()
    """The ``==`` operator, used for equality comparison.

    Example: ``x == 5`` checks whether x is equal to 5.

    This is a *two-character* operator. When the lexer sees ``=``, it must
    "peek ahead" at the next character to decide whether it's a single ``=``
    (assignment) or a double ``==`` (comparison). This is one of the trickier
    parts of lexing.
    """

    # --- Delimiters ---------------------------------------------------------
    # These token types represent punctuation that structures the code.

    LPAREN = auto()
    """A left parenthesis ``(``."""

    RPAREN = auto()
    """A right parenthesis ``)``."""

    COMMA = auto()
    """A comma ``,``, typically used to separate arguments or items."""

    COLON = auto()
    """A colon ``:``, used in many languages for blocks, slices, or labels."""

    SEMICOLON = auto()
    """A semicolon ``;``, used in many languages to terminate statements."""

    LBRACE = auto()
    """A left curly brace ``{``, used in C-family languages for blocks."""

    RBRACE = auto()
    """A right curly brace ``}``, used to close blocks."""

    LBRACKET = auto()
    """A left square bracket ``[``, used for array indexing and literals."""

    RBRACKET = auto()
    """A right square bracket ``]``, used to close array access and literals."""

    DOT = auto()
    """A dot ``.``, used for member access in many languages."""

    BANG = auto()
    """An exclamation mark ``!``, used for logical negation."""

    # --- Structural ---------------------------------------------------------
    # These token types represent the structure of the source code itself.

    NEWLINE = auto()
    """A newline character, marking the end of a line.

    In many languages (like Python), newlines are significant — they mark
    the end of a statement. In others (like C or Java), newlines are treated
    as whitespace. Our lexer always emits NEWLINE tokens, and the parser can
    decide whether they matter.
    """

    EOF = auto()
    """End of file — a synthetic token marking the end of the input.

    This isn't a real character in the source code. The lexer adds it
    automatically when it reaches the end of the input. It's extremely
    useful for the parser, because it provides a clean "stop" signal.
    Without it, the parser would have to constantly check "am I past the
    end?" which makes the code more complex.
    """


# ---------------------------------------------------------------------------
# Token
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class Token:
    """A single token — the smallest meaningful unit of source code.

    A token pairs a **type** (what kind of thing it is) with a **value**
    (the actual text from the source code), plus position information for
    error reporting.

    Think of a token like a labeled sticky note attached to a piece of text:

        ┌──────────┐
        │ NAME     │  ← type (what kind of token)
        │ "x"      │  ← value (the actual text)
        │ line 1   │  ← where it appeared
        │ col 1    │
        └──────────┘

    Why ``frozen=True``?
        Tokens are *immutable* — once created, they never change. This is a
        good practice because tokens represent facts about the source code,
        and facts don't change. Making them frozen also means they can be
        used in sets and as dictionary keys.

    Why ``slots=True``?
        This is a performance optimization. It tells Python to use a more
        memory-efficient internal representation, which matters when you're
        creating thousands of tokens for a large source file.

    Attributes:
        type: The kind of token (e.g., ``TokenType.NAME``, ``TokenType.NUMBER``).
        value: The actual text from the source code that this token represents.
        line: The 1-based line number where this token starts.
        column: The 1-based column number where this token starts.
    """

    type: TokenType
    value: str
    line: int
    column: int

    def __repr__(self) -> str:
        """Return a concise, readable representation of the token.

        Example: ``Token(NAME, 'x', 1:1)`` means "a NAME token with value
        'x' at line 1, column 1".
        """
        return f"Token({self.type.name}, {self.value!r}, {self.line}:{self.column})"


# ---------------------------------------------------------------------------
# Lexer Configuration
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class LexerConfig:
    """Configuration that makes the lexer adaptable to different languages.

    The key insight is that most programming languages share the same *kinds*
    of tokens — numbers, strings, operators, identifiers — but differ in which
    words are **keywords**. By externalizing the keyword list into a config
    object, we can reuse the same lexer engine for Python, Ruby, JavaScript,
    or any other language.

    Example — Python configuration::

        python_config = LexerConfig(keywords=[
            "if", "else", "elif", "while", "for", "def", "return",
            "class", "import", "from", "as", "True", "False", "None",
        ])

    Example — Ruby configuration::

        ruby_config = LexerConfig(keywords=[
            "if", "else", "elsif", "end", "while", "for", "def", "return",
            "class", "require", "puts", "true", "false", "nil",
        ])

    Attributes:
        keywords: A list of words that should be classified as KEYWORD tokens
            instead of NAME tokens. The lexer checks every identifier against
            this list. If no config is provided to the Lexer, no words are
            treated as keywords (everything is a NAME).
    """

    keywords: list[str] = field(default_factory=list)

    @property
    def keyword_set(self) -> frozenset[str]:
        """Return the keywords as a frozen set for O(1) membership testing.

        Why a set instead of a list? Because checking "is this word in the
        list?" takes O(n) time with a list (you scan every element), but
        only O(1) time with a set (constant time, thanks to hashing).
        For a handful of keywords this doesn't matter much, but it's a
        good habit and makes the intent clear.

        We use ``frozenset`` (immutable set) because the config is frozen —
        we don't want anyone accidentally modifying the keyword set.
        """
        return frozenset(self.keywords)


# ---------------------------------------------------------------------------
# Lexer Error
# ---------------------------------------------------------------------------

class LexerError(Exception):
    """An error encountered during tokenization.

    When the lexer encounters something it doesn't understand — like an
    unterminated string ``"hello`` or an unexpected character ``@`` — it
    raises this exception with a helpful message that includes the line
    and column where the problem occurred.

    Attributes:
        message: A human-readable description of what went wrong.
        line: The 1-based line number where the error occurred.
        column: The 1-based column number where the error occurred.
    """

    def __init__(self, message: str, line: int, column: int) -> None:
        self.message = message
        self.line = line
        self.column = column
        super().__init__(f"Lexer error at {line}:{column}: {message}")


# ---------------------------------------------------------------------------
# The Lexer
# ---------------------------------------------------------------------------

class Lexer:
    """The main lexer — reads source code character by character and produces tokens.

    How It Works — The Big Picture
    ------------------------------

    Imagine you're reading a book one letter at a time, with your finger
    pointing at the current letter. The lexer works the same way:

    1. It maintains a **position** (like your finger) that points to the
       current character in the source code.
    2. It looks at the current character and decides what kind of token
       is starting here.
    3. It reads as many characters as needed to complete that token.
    4. It records the token and moves on.
    5. It repeats until it reaches the end of the input.

    For example, given ``x = 42``:

    - Position 0: sees ``x`` (a letter) → reads an identifier → emits NAME("x")
    - Position 1: sees `` `` (a space) → skips whitespace
    - Position 2: sees ``=`` → peeks ahead, next is `` `` (not ``=``) → emits EQUALS("=")
    - Position 3: sees `` `` → skips whitespace
    - Position 4: sees ``4`` (a digit) → reads all digits → emits NUMBER("42")
    - Position 6: end of input → emits EOF

    The "peek ahead" step for ``=`` is important. When the lexer sees ``=``,
    it doesn't know yet whether this is ``=`` (assignment) or ``==`` (comparison).
    It needs to look at the *next* character without consuming it. This is called
    **lookahead**, and it's one of the fundamental techniques in lexer design.

    Usage::

        source = 'x = 1 + 2'
        lexer = Lexer(source)
        tokens = lexer.tokenize()
        # tokens is a list of Token objects

    With a language-specific configuration::

        config = LexerConfig(keywords=["if", "else", "while"])
        lexer = Lexer('if x == 1', config)
        tokens = lexer.tokenize()
        # The word "if" will be a KEYWORD token, not a NAME token

    Attributes:
        _source: The complete source code string being tokenized.
        _config: The lexer configuration (keyword list, etc.).
        _pos: The current position (index) in the source string.
        _line: The current line number (1-based), for error reporting.
        _column: The current column number (1-based), for error reporting.
        _tokens: The list of tokens built up during tokenization.
        _keyword_set: A pre-computed set of keywords for fast lookup.
    """

    # -- A mapping from single characters to their token types ---------------
    # This table handles "simple" tokens — characters that always mean the
    # same thing regardless of context. We look up the character in this
    # dictionary to instantly know what token type it is.
    #
    # Note that ``=`` is NOT in this table because it requires lookahead
    # (it could be ``=`` or ``==``).
    _SIMPLE_TOKENS: dict[str, TokenType] = {
        "+": TokenType.PLUS,
        "-": TokenType.MINUS,
        "*": TokenType.STAR,
        "/": TokenType.SLASH,
        "(": TokenType.LPAREN,
        ")": TokenType.RPAREN,
        ",": TokenType.COMMA,
        ":": TokenType.COLON,
        ";": TokenType.SEMICOLON,
        "{": TokenType.LBRACE,
        "}": TokenType.RBRACE,
        "[": TokenType.LBRACKET,
        "]": TokenType.RBRACKET,
        ".": TokenType.DOT,
        "!": TokenType.BANG,
    }

    def __init__(self, source: str, config: LexerConfig | None = None) -> None:
        """Initialize the lexer with source code and optional configuration.

        Args:
            source: The raw source code text to tokenize.
            config: Optional configuration specifying language-specific keywords.
                If ``None``, no words will be treated as keywords.
        """
        self._source: str = source
        self._config: LexerConfig = config or LexerConfig()
        self._pos: int = 0
        self._line: int = 1
        self._column: int = 1
        self._tokens: list[Token] = []
        # Pre-compute the keyword set once, not on every identifier check.
        self._keyword_set: frozenset[str] = self._config.keyword_set

    # -- Core character-reading methods -------------------------------------
    #
    # These are the low-level "machinery" of the lexer. They move the
    # position forward and tell us what character we're looking at.

    def _current_char(self) -> str | None:
        """Return the character at the current position, or None if at end.

        This is how the lexer "sees" the current character. Returning
        ``None`` at the end (instead of raising an error) makes it easy
        to write loops like ``while self._current_char() is not None``.
        """
        if self._pos < len(self._source):
            return self._source[self._pos]
        return None

    def _peek(self) -> str | None:
        """Look at the *next* character without advancing the position.

        This is the "lookahead" operation. It's essential for distinguishing
        tokens that start the same way:
        - ``=`` vs ``==``

        The lexer sees ``=`` and thinks, "Is this assignment or comparison?"
        It peeks at the next character to decide, without moving forward.
        If the next character is ``=``, it's ``==``. Otherwise, it's just ``=``.

        Returns:
            The next character, or ``None`` if the current character is the last one.
        """
        peek_pos = self._pos + 1
        if peek_pos < len(self._source):
            return self._source[peek_pos]
        return None

    def _advance(self) -> str:
        """Consume the current character and move to the next one.

        This is the fundamental "step forward" operation. Every time the lexer
        reads a character that belongs to the current token, it calls
        ``_advance()`` to move past it.

        The method also updates the line and column counters. When it sees a
        newline character, it increments the line counter and resets the column
        to 1 (the start of a new line). Otherwise, it just increments the column.

        Returns:
            The character that was consumed.

        Raises:
            LexerError: If called when already at the end of input.
        """
        char = self._source[self._pos]
        self._pos += 1

        if char == "\n":
            self._line += 1
            self._column = 1
        else:
            self._column += 1

        return char

    def _skip_whitespace(self) -> None:
        """Skip over spaces and tabs (but NOT newlines).

        Whitespace between tokens is meaningless in most contexts — ``x=1`` and
        ``x = 1`` mean the same thing. The lexer skips over it silently.

        However, **newlines are NOT skipped** here. Newlines get their own token
        (``NEWLINE``) because in some languages (like Python), newlines are
        significant — they mark the end of a statement. The main tokenize loop
        handles newlines explicitly.
        """
        while self._current_char() is not None and self._current_char() in " \t\r":
            self._advance()

    # -- Token-reading methods -----------------------------------------------
    #
    # Each of these methods reads one specific kind of token. They are called
    # from the main tokenize() loop when it identifies what kind of token is
    # starting at the current position.

    def _read_number(self) -> Token:
        """Read an integer literal (a sequence of digits).

        When the main loop sees a digit character (0-9), it delegates to this
        method. We keep reading characters as long as they are digits, building
        up the number string.

        For example, if the source has ``42 + 3``, and we're at position 0:
        - Read '4', advance → "4"
        - Read '2', advance → "42"
        - See ' ' (space) — not a digit, so stop
        - Emit NUMBER("42")

        Returns:
            A Token of type NUMBER with the digit string as its value.
        """
        start_line = self._line
        start_column = self._column
        digits: list[str] = []

        while self._current_char() is not None and self._current_char().isdigit():  # type: ignore[union-attr]
            digits.append(self._advance())

        return Token(
            type=TokenType.NUMBER,
            value="".join(digits),
            line=start_line,
            column=start_column,
        )

    def _read_name(self) -> Token:
        """Read an identifier or keyword.

        Identifiers (names) follow the same rules in almost every language:
        - They start with a letter or underscore: ``a-z``, ``A-Z``, ``_``
        - They continue with letters, digits, or underscores: ``a-z``, ``A-Z``, ``0-9``, ``_``

        After reading the full name, we check whether it's a **keyword** —
        a reserved word with special meaning in the language. If it is, we
        emit a KEYWORD token; otherwise, we emit a NAME token.

        For example, with Python keywords configured:
        - ``x`` → NAME("x")
        - ``if`` → KEYWORD("if")
        - ``if_condition`` → NAME("if_condition")  (not a keyword — it has extra chars)

        Returns:
            A Token of type NAME or KEYWORD.
        """
        start_line = self._line
        start_column = self._column
        chars: list[str] = []

        # Read the first character (must be a letter or underscore).
        # Read subsequent characters (letters, digits, or underscores).
        while (
            self._current_char() is not None
            and (self._current_char().isalnum() or self._current_char() == "_")  # type: ignore[union-attr]
        ):
            chars.append(self._advance())

        name = "".join(chars)

        # Is this word a keyword in the configured language?
        token_type = TokenType.KEYWORD if name in self._keyword_set else TokenType.NAME

        return Token(
            type=token_type,
            value=name,
            line=start_line,
            column=start_column,
        )

    def _read_string(self) -> Token:
        r"""Read a double-quoted string literal.

        String literals are delimited by double quotes: ``"Hello, World!"``.
        The lexer reads everything between the opening and closing quotes,
        handling **escape sequences** along the way.

        Escape sequences let you include special characters inside a string:
        - ``\"`` → a literal double-quote (without ending the string)
        - ``\\`` → a literal backslash
        - ``\n`` → a newline character
        - ``\t`` → a tab character

        For example, ``"He said \"hi\""`` produces the value: ``He said "hi"``

        The opening quote has already been identified by the caller but NOT
        consumed — this method consumes it and everything through the closing
        quote.

        Returns:
            A Token of type STRING with the content between the quotes as value.

        Raises:
            LexerError: If the string is never closed (reaches end of input
                without a closing quote).
        """
        start_line = self._line
        start_column = self._column
        chars: list[str] = []

        # Consume the opening double quote.
        self._advance()

        while True:
            current = self._current_char()

            if current is None:
                # We reached the end of input without finding a closing quote.
                # This is an error — the programmer forgot to close the string.
                raise LexerError(
                    "Unterminated string literal",
                    line=start_line,
                    column=start_column,
                )

            if current == '"':
                # Found the closing quote. Consume it and stop.
                self._advance()
                break

            if current == "\\":
                # Escape sequence — the backslash says "the next character
                # is special, don't treat it normally."
                self._advance()  # consume the backslash
                escaped = self._current_char()

                if escaped is None:
                    raise LexerError(
                        "Unterminated string literal (ends with backslash)",
                        line=start_line,
                        column=start_column,
                    )

                # Map escape codes to their actual characters.
                escape_map: dict[str, str] = {
                    "n": "\n",
                    "t": "\t",
                    "\\": "\\",
                    '"': '"',
                }

                chars.append(escape_map.get(escaped, escaped))
                self._advance()
            else:
                # A regular character — just add it to the string.
                chars.append(current)
                self._advance()

        return Token(
            type=TokenType.STRING,
            value="".join(chars),
            line=start_line,
            column=start_column,
        )

    # -- Main tokenization loop ----------------------------------------------

    def tokenize(self) -> list[Token]:
        """Tokenize the entire source code and return a list of tokens.

        This is the main entry point. It loops through the source code,
        character by character, deciding what kind of token starts at each
        position and delegating to the appropriate reading method.

        The algorithm is a classic **dispatch on first character**:

        1. If the character is a space or tab → skip whitespace
        2. If the character is a newline → emit a NEWLINE token
        3. If the character is a digit → read a number
        4. If the character is a letter or underscore → read a name/keyword
        5. If the character is a double quote → read a string
        6. If the character is ``=`` → peek ahead to distinguish ``=`` from ``==``
        7. If the character is a simple operator/delimiter → look it up in the table
        8. Otherwise → raise an error (unexpected character)

        After all characters are processed, an EOF token is appended.

        Returns:
            A list of Token objects, always ending with an EOF token.

        Raises:
            LexerError: If an unexpected character is encountered, or if a
                string literal is not properly terminated.
        """
        self._tokens = []

        while self._current_char() is not None:
            char = self._current_char()
            assert char is not None  # for type checker; guaranteed by while condition

            # --- Whitespace (spaces and tabs) ---
            if char in " \t\r":
                self._skip_whitespace()
                continue

            # --- Newlines ---
            if char == "\n":
                token = Token(
                    type=TokenType.NEWLINE,
                    value="\\n",
                    line=self._line,
                    column=self._column,
                )
                self._advance()
                self._tokens.append(token)
                continue

            # --- Numbers ---
            if char.isdigit():
                self._tokens.append(self._read_number())
                continue

            # --- Names and keywords ---
            if char.isalpha() or char == "_":
                self._tokens.append(self._read_name())
                continue

            # --- String literals ---
            if char == '"':
                self._tokens.append(self._read_string())
                continue

            # --- The = and == operators (requires lookahead) ---
            if char == "=":
                start_line = self._line
                start_column = self._column
                self._advance()

                if self._current_char() == "=":
                    # It's ``==`` (comparison)
                    self._advance()
                    self._tokens.append(
                        Token(TokenType.EQUALS_EQUALS, "==", start_line, start_column)
                    )
                else:
                    # It's just ``=`` (assignment)
                    self._tokens.append(
                        Token(TokenType.EQUALS, "=", start_line, start_column)
                    )
                continue

            # --- Simple single-character tokens ---
            if char in self._SIMPLE_TOKENS:
                token = Token(
                    type=self._SIMPLE_TOKENS[char],
                    value=char,
                    line=self._line,
                    column=self._column,
                )
                self._advance()
                self._tokens.append(token)
                continue

            # --- Unexpected character ---
            # If we get here, we've encountered a character the lexer doesn't
            # know how to handle. This is an error in the source code.
            raise LexerError(
                f"Unexpected character: {char!r}",
                line=self._line,
                column=self._column,
            )

        # --- End of input ---
        # Append the EOF token so the parser knows the input is finished.
        self._tokens.append(
            Token(
                type=TokenType.EOF,
                value="",
                line=self._line,
                column=self._column,
            )
        )

        return self._tokens
