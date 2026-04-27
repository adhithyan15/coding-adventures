"""Tetrad lexer: tokenizes Tetrad source text into a flat token stream.

Tetrad is a small interpreted language whose bytecode runs on a register VM
small enough to execute on an Intel 4004 (128 bytes of usable RAM, 4 KB ROM).
This lexer is the first stage of the Tetrad pipeline:

    source text
        → [tetrad-lexer]  token stream
        → [tetrad-parser] AST
        → [tetrad-type-checker] typed AST
        → [tetrad-compiler] bytecode (CodeObject)
        → [tetrad-vm] execution + metrics
        → [tetrad-jit] native code for hot functions

The lexer is a single-pass scanner with one character of lookahead.  It uses
maximal munch (longest match) for two-character operators, and raises
``LexError`` on the first illegal character it encounters.

Public API
----------
``tokenize(source)``
    Lex a Tetrad source string.  Returns a list ending with ``Token(EOF)``.
    Raises ``LexError`` on illegal input.

``Token``
    A dataclass carrying ``type``, ``value``, ``line``, ``column``, ``offset``.

``TokenType``
    Enum of all token categories.

``LexError``
    Raised on illegal characters or malformed literals.
"""

from __future__ import annotations

import enum
from dataclasses import dataclass

# ---------------------------------------------------------------------------
# Token type enumeration
# ---------------------------------------------------------------------------


class TokenType(enum.Enum):
    """Every category of token the Tetrad lexer can produce."""

    # Literals
    INT = "INT"   # decimal integer, e.g. 42
    HEX = "HEX"  # hex integer,     e.g. 0xFF

    # Identifier
    IDENT = "IDENT"

    # Keywords
    KW_FN = "KW_FN"
    KW_LET = "KW_LET"
    KW_IF = "KW_IF"
    KW_ELSE = "KW_ELSE"
    KW_WHILE = "KW_WHILE"
    KW_RETURN = "KW_RETURN"
    KW_IN = "KW_IN"
    KW_OUT = "KW_OUT"
    KW_U8 = "KW_U8"  # type keyword

    # Arithmetic operators
    PLUS = "PLUS"        # +
    MINUS = "MINUS"      # -
    STAR = "STAR"        # *
    SLASH = "SLASH"      # /
    PERCENT = "PERCENT"  # %

    # Bitwise operators
    AMP = "AMP"      # &
    PIPE = "PIPE"    # |
    CARET = "CARET"  # ^
    TILDE = "TILDE"  # ~

    # Shift operators (two-char, maximal munch)
    SHL = "SHL"  # <<
    SHR = "SHR"  # >>

    # Comparison operators
    EQ_EQ = "EQ_EQ"    # ==
    BANG_EQ = "BANG_EQ"  # !=
    LT = "LT"          # <
    LT_EQ = "LT_EQ"    # <=
    GT = "GT"          # >
    GT_EQ = "GT_EQ"    # >=

    # Logical operators
    AMP_AMP = "AMP_AMP"    # &&
    PIPE_PIPE = "PIPE_PIPE"  # ||
    BANG = "BANG"            # !

    # Assignment and annotation
    EQ = "EQ"        # =  (assignment, NOT equality)
    ARROW = "ARROW"  # -> (return type annotation)
    COLON = "COLON"  # :  (type annotation separator)

    # Delimiters
    LPAREN = "LPAREN"  # (
    RPAREN = "RPAREN"  # )
    LBRACE = "LBRACE"  # {
    RBRACE = "RBRACE"  # }
    COMMA = "COMMA"    # ,
    SEMI = "SEMI"      # ;

    # Sentinel
    EOF = "EOF"


# ---------------------------------------------------------------------------
# Lookup tables
# ---------------------------------------------------------------------------

_RESERVED: dict[str, TokenType] = {
    "fn": TokenType.KW_FN,
    "let": TokenType.KW_LET,
    "if": TokenType.KW_IF,
    "else": TokenType.KW_ELSE,
    "while": TokenType.KW_WHILE,
    "return": TokenType.KW_RETURN,
    "in": TokenType.KW_IN,
    "out": TokenType.KW_OUT,
    "u8": TokenType.KW_U8,
}

# Two-char operators checked *before* their one-char prefixes (maximal munch).
_TWO_CHAR: dict[str, TokenType] = {
    "<<": TokenType.SHL,
    ">>": TokenType.SHR,
    "==": TokenType.EQ_EQ,
    "!=": TokenType.BANG_EQ,
    "<=": TokenType.LT_EQ,
    ">=": TokenType.GT_EQ,
    "&&": TokenType.AMP_AMP,
    "||": TokenType.PIPE_PIPE,
    "->": TokenType.ARROW,
}

_ONE_CHAR: dict[str, TokenType] = {
    "+": TokenType.PLUS,
    "-": TokenType.MINUS,
    "*": TokenType.STAR,
    "/": TokenType.SLASH,
    "%": TokenType.PERCENT,
    "&": TokenType.AMP,
    "|": TokenType.PIPE,
    "^": TokenType.CARET,
    "~": TokenType.TILDE,
    "!": TokenType.BANG,
    "=": TokenType.EQ,
    ":": TokenType.COLON,
    "<": TokenType.LT,
    ">": TokenType.GT,
    "(": TokenType.LPAREN,
    ")": TokenType.RPAREN,
    "{": TokenType.LBRACE,
    "}": TokenType.RBRACE,
    ",": TokenType.COMMA,
    ";": TokenType.SEMI,
}

_HEX_DIGITS = frozenset("0123456789abcdefABCDEF")
_WHITESPACE = frozenset(" \t\r\n")


# ---------------------------------------------------------------------------
# Token dataclass
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Token:
    """A single lexical token.

    Attributes
    ----------
    type:
        The token category.
    value:
        ``int`` for INT/HEX literals; ``str`` for IDENT; ``None`` for
        keywords, operators, punctuation, and EOF.
    line:
        1-based line number of the token's first character.
    column:
        1-based column number of the token's first character.
    offset:
        0-based byte offset into the source string.
    """

    type: TokenType
    value: str | int | None
    line: int
    column: int
    offset: int


# ---------------------------------------------------------------------------
# Error type
# ---------------------------------------------------------------------------


class LexError(Exception):
    """Raised when the lexer encounters an illegal character or malformed literal."""

    def __init__(self, message: str, line: int, column: int) -> None:
        super().__init__(f"{message} at line {line} col {column}")
        self.line = line
        self.column = column


# ---------------------------------------------------------------------------
# Lexer implementation
# ---------------------------------------------------------------------------


def tokenize(source: str) -> list[Token]:
    """Tokenize a Tetrad source string.

    Returns a list of tokens ending with a single ``Token(EOF)``.
    Raises ``LexError`` on the first illegal character or malformed literal.

    The lexer:
    - Skips all ASCII whitespace (space, tab, CR, LF).
    - Skips C-style line comments (``//`` to end of line).
    - Uses maximal munch for two-character operators.
    - Tracks 1-based line and column numbers for every token.

    Parameters
    ----------
    source:
        The Tetrad source code as a UTF-8 string.
    """
    tokens: list[Token] = []
    pos = 0
    line = 1
    col = 1
    n = len(source)

    def _ch() -> str:
        """Return the character at the current position (empty string at EOF)."""
        return source[pos] if pos < n else ""

    def _peek_at(offset: int) -> str:
        """Return the character at pos+offset (empty string past EOF)."""
        idx = pos + offset
        return source[idx] if idx < n else ""

    def _advance() -> str:
        """Consume and return the current character, updating line/col."""
        nonlocal pos, line, col
        ch = source[pos]
        pos += 1
        if ch == "\n":
            line += 1
            col = 1
        else:
            col += 1
        return ch

    while pos < n:
        # --- skip whitespace ---
        if _ch() in _WHITESPACE:
            _advance()
            continue

        # --- skip line comments ---
        if _ch() == "/" and _peek_at(1) == "/":
            while pos < n and _ch() != "\n":
                _advance()
            continue

        start_pos = pos
        start_line = line
        start_col = col
        ch = _ch()

        # --- number literals ---
        if ch.isdigit():
            if ch == "0" and _peek_at(1) in ("x", "X"):
                _advance()  # consume '0'
                _advance()  # consume 'x' / 'X'
                if _ch() not in _HEX_DIGITS:
                    raise LexError(
                        "empty hex literal", start_line, start_col
                    )
                while _ch() in _HEX_DIGITS:
                    _advance()
                tokens.append(
                    Token(
                        TokenType.HEX,
                        int(source[start_pos:pos], 16),
                        start_line,
                        start_col,
                        start_pos,
                    )
                )
            else:
                while pos < n and _ch().isdigit():
                    _advance()
                tokens.append(
                    Token(
                        TokenType.INT,
                        int(source[start_pos:pos], 10),
                        start_line,
                        start_col,
                        start_pos,
                    )
                )
            continue

        # --- identifiers and keywords ---
        if ch.isalpha() or ch == "_":
            while pos < n and (_ch().isalnum() or _ch() == "_"):
                _advance()
            text = source[start_pos:pos]
            tok_type = _RESERVED.get(text, TokenType.IDENT)
            tokens.append(
                Token(
                    tok_type,
                    text if tok_type is TokenType.IDENT else None,
                    start_line,
                    start_col,
                    start_pos,
                )
            )
            continue

        # --- two-char operators (maximal munch) ---
        two = source[pos : pos + 2]
        if two in _TWO_CHAR:
            _advance()
            _advance()
            tokens.append(
                Token(_TWO_CHAR[two], None, start_line, start_col, start_pos)
            )
            continue

        # --- one-char operators and punctuation ---
        if ch in _ONE_CHAR:
            _advance()
            tokens.append(
                Token(_ONE_CHAR[ch], None, start_line, start_col, start_pos)
            )
            continue

        raise LexError(f"unexpected character {ch!r}", start_line, start_col)

    tokens.append(Token(TokenType.EOF, None, line, col, pos))
    return tokens
