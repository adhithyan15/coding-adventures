"""Layout-aware lexer for ``.warrant`` source (VER04 section 2, VER04j section 1.2).

This is the first half of the oracle's *reader*.  It shares nothing with the
Rust generator's ``layout.rs``/``source.rs``: it is written from the prose of
VER04 only, because the whole point of the oracle is to be a second, independent
reading of the same specification.

How the text becomes tokens
===========================
Warrant is indentation structured, like Python::

    warrant ct_compare            <- NAME NAME NEWLINE
      error E = A | B             <- INDENT ... NEWLINE
      claim c:
        forall a: U8. f(a) == 1u8 <- INDENT ... NEWLINE DEDENT DEDENT

The lexer therefore produces, besides ordinary tokens, three *synthetic*
layout tokens: ``NEWLINE`` (end of a logical line), ``INDENT`` and ``DEDENT``.
Inside ``(`` ``)`` or ``[`` ``]`` layout is suppressed, so an expression may
continue over physical lines; that is how long model bodies are written.

Three small decisions are worth knowing because they are easy to get wrong:

* ``--`` starts an end-of-line comment.  Subtraction of a negative is written
  ``a - -b`` (with a space), exactly as VER04 section 2 says.
* A ``.`` followed by whitespace is the quantifier separator (``DOTSEP``); a
  ``.`` glued to the next character is ``DOT`` (qualified name or projection).
* Integer literals carry an optional lowercase type suffix (``5u8``, ``-`` is
  never part of a literal), hexadecimal literals are ``0x`` plus lowercase digits.

Hostile input
=============
The admission limits of VER04 section 2 are enforced *before* anything large is
allocated: 1 MiB of source, 100,000 tokens, 128 bytes per identifier and 4,096
decimal digits per integer.  An exceeded limit is an error, never a truncation.
"""

from __future__ import annotations

from dataclasses import dataclass

from .errors import OracleError

MAX_SOURCE_BYTES = 1 << 20
MAX_TOKENS = 100_000
MAX_IDENT_BYTES = 128
MAX_INT_DIGITS = 4096

#: Reserved words (VER04 section 2).  They can never be identifiers.
RESERVED = frozenset(
    [
        "warrant",
        "model",
        "error",
        "op",
        "example",
        "claim",
        "assume",
        "platform",
        "environment",
        "none",
        "labels",
        "public",
        "secret",
        "input",
        "hostile",
        "rejects",
        "with",
        "ensures",
        "bound",
        "fresh",
        "if",
        "then",
        "else",
        "match",
        "forall",
        "exists",
        "not",
        "and",
        "or",
        "true",
        "false",
    ]
)

INT_SUFFIXES = ("u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64")

_TWO_CHAR = ("==", "!=", "<=", ">=", "->", "=>")
_ONE_CHAR = "()[],:|=<>+-*"
_NAME_START = frozenset("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_")
_NAME_REST = _NAME_START | frozenset("0123456789")
_DIGITS = frozenset("0123456789")
_HEX = frozenset("0123456789abcdef")


@dataclass(frozen=True, slots=True)
class Token:
    """One lexical token with its one-based source position."""

    kind: str  # NAME INT BYTES OP DOT DOTSEP NEWLINE INDENT DEDENT EOF
    text: str
    line: int
    col: int


def decode_source(raw: bytes) -> str:
    """Admit source bytes (VER04 section 2) and normalise CRLF to LF."""
    if len(raw) > MAX_SOURCE_BYTES:
        raise OracleError("source", "source exceeds the 1 MiB admission limit")
    if raw.startswith(b"\xef\xbb\xbf"):
        raise OracleError("source", "source must not begin with a byte order mark")
    if b"\x00" in raw:
        raise OracleError("source", "source must not contain NUL")
    try:
        text = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise OracleError("source", "source is not valid UTF-8") from exc
    text = text.replace("\r\n", "\n")
    if "\r" in text:
        raise OracleError("source", "a bare carriage return is not a line ending")
    return text


def split_int(text: str) -> tuple[int, str | None]:
    """Split an ``INT`` token into ``(value, suffix)``; suffix is ``None`` if bare."""
    suffix = None
    for candidate in INT_SUFFIXES:
        if text.endswith(candidate):
            suffix = candidate
            text = text[: -len(candidate)]
            break
    if text.startswith("0x"):
        return int(text[2:], 16), suffix
    return int(text), suffix


def tokenize(source: str) -> list[Token]:
    """Turn admitted source text into a layout-annotated token list."""
    tokens: list[Token] = []
    indents = [0]
    depth = 0  # open bracket count; layout is suppressed while positive
    line_has_tokens = False

    def emit(kind: str, text: str, line: int, col: int) -> None:
        if len(tokens) >= MAX_TOKENS:
            raise OracleError("limit", "source exceeds the 100,000 token limit")
        tokens.append(Token(kind, text, line, col))

    lines = source.split("\n")
    for number, line in enumerate(lines, start=1):
        pos = 0
        if depth == 0:
            while pos < len(line) and line[pos] == " ":
                pos += 1
            if pos < len(line) and line[pos] == "\t":
                raise OracleError("lex", f"line {number}: tab used for indentation")
            rest = line[pos:]
            if rest == "" or rest.startswith("--"):
                continue  # blank or comment-only lines make no layout tokens
            if line_has_tokens:
                emit("NEWLINE", "", number - 1, 0)
                line_has_tokens = False
            if pos > indents[-1]:
                indents.append(pos)
                emit("INDENT", "", number, 1)
            else:
                while pos < indents[-1]:
                    indents.pop()
                    emit("DEDENT", "", number, 1)
                if pos != indents[-1]:
                    raise OracleError("lex", f"line {number}: inconsistent dedent")
        while pos < len(line):
            ch = line[pos]
            col = pos + 1
            if ch == " ":
                pos += 1
                continue
            if ch == "\t":
                pos += 1
                continue
            if ch == "-" and line.startswith("--", pos):
                break
            if ch in _NAME_START:
                if ch == "b" and line.startswith('b"', pos):
                    end = line.find('"', pos + 2)
                    if end < 0:
                        raise OracleError("lex", f"line {number}: unterminated bytes literal")
                    digits = line[pos + 2 : end]
                    if len(digits) % 2 or any(c not in _HEX for c in digits):
                        raise OracleError(
                            "lex", f"line {number}: bytes literal is not even lowercase hex"
                        )
                    emit("BYTES", digits, number, col)
                    pos = end + 1
                    line_has_tokens = True
                    continue
                end = pos + 1
                while end < len(line) and line[end] in _NAME_REST:
                    end += 1
                word = line[pos:end]
                if len(word.encode("utf-8")) > MAX_IDENT_BYTES:
                    raise OracleError("limit", f"line {number}: identifier exceeds 128 bytes")
                emit("NAME", word, number, col)
                pos = end
                line_has_tokens = True
                continue
            if ch in _DIGITS:
                end = pos
                if line.startswith("0x", pos):
                    end = pos + 2
                    while end < len(line) and line[end] in _HEX:
                        end += 1
                    if end == pos + 2:
                        raise OracleError("lex", f"line {number}: hex literal has no digits")
                    digits_len = end - pos - 2
                else:
                    while end < len(line) and line[end] in _DIGITS:
                        end += 1
                    body = line[pos:end]
                    if len(body) > 1 and body[0] == "0":
                        raise OracleError("lex", f"line {number}: leading zero in integer")
                    digits_len = len(body)
                if digits_len > MAX_INT_DIGITS:
                    raise OracleError("limit", f"line {number}: integer exceeds 4,096 digits")
                for suffix in INT_SUFFIXES:
                    if line.startswith(suffix, end):
                        end += len(suffix)
                        break
                if end < len(line) and line[end] in _NAME_REST:
                    raise OracleError("lex", f"line {number}: malformed integer literal")
                emit("INT", line[pos:end], number, col)
                pos = end
                line_has_tokens = True
                continue
            two = line[pos : pos + 2]
            if two in _TWO_CHAR:
                emit("OP", two, number, col)
                pos += 2
                line_has_tokens = True
                continue
            if ch == ".":
                follows = line[pos + 1] if pos + 1 < len(line) else " "
                emit("DOTSEP" if follows == " " else "DOT", ".", number, col)
                pos += 1
                line_has_tokens = True
                continue
            if ch in _ONE_CHAR:
                if ch in "([":
                    depth += 1
                elif ch in ")]":
                    depth -= 1
                    if depth < 0:
                        raise OracleError("lex", f"line {number}: unbalanced closing bracket")
                emit("OP", ch, number, col)
                pos += 1
                line_has_tokens = True
                continue
            raise OracleError("lex", f"line {number}: unexpected character")
    if depth != 0:
        raise OracleError("lex", "unbalanced brackets at end of file")
    if line_has_tokens:
        emit("NEWLINE", "", len(lines), 0)
    while len(indents) > 1:
        indents.pop()
        emit("DEDENT", "", len(lines), 0)
    emit("EOF", "", len(lines), 0)
    return tokens
