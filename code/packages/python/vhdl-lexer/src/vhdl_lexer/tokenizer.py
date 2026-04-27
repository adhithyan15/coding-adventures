"""VHDL lexer backed by compiled, versioned grammars."""

from __future__ import annotations

from lexer import GrammarLexer, Token, TokenType

from vhdl_lexer import (
    _grammar_1987,
    _grammar_1993,
    _grammar_2002,
    _grammar_2008,
    _grammar_2019,
)

DEFAULT_VERSION = "2008"
SUPPORTED_VERSIONS = frozenset({"1987", "1993", "2002", "2008", "2019"})

_TOKEN_GRAMMARS = {
    "1987": _grammar_1987.TOKEN_GRAMMAR,
    "1993": _grammar_1993.TOKEN_GRAMMAR,
    "2002": _grammar_2002.TOKEN_GRAMMAR,
    "2008": _grammar_2008.TOKEN_GRAMMAR,
    "2019": _grammar_2019.TOKEN_GRAMMAR,
}

_VHDL_KEYWORDS = {
    version: set(grammar.keywords)
    for version, grammar in _TOKEN_GRAMMARS.items()
}


def resolve_version(version: str | None = None) -> str:
    """Normalize a VHDL edition string and reject unknown values."""
    resolved = DEFAULT_VERSION if not version else version
    if resolved not in SUPPORTED_VERSIONS:
        valid = ", ".join(sorted(SUPPORTED_VERSIONS))
        raise ValueError(f"Unknown VHDL version {resolved!r}. Valid versions: {valid}")
    return resolved


def _normalize_case(tokens: list[Token], keywords: set[str]) -> list[Token]:
    """Lowercase the value of NAME and KEYWORD tokens, and reclassify keywords."""
    result: list[Token] = []
    for token in tokens:
        type_name = token.type.name if hasattr(token.type, "name") else token.type
        if type_name in ("NAME", "KEYWORD"):
            lowered = token.value.lower()
            new_type = TokenType.KEYWORD if lowered in keywords else token.type
            result.append(Token(new_type, lowered, token.line, token.column))
        else:
            result.append(token)
    return result


def create_vhdl_lexer(source: str, version: str | None = None) -> GrammarLexer:
    """Create a VHDL lexer for a specific supported edition."""
    grammar = _TOKEN_GRAMMARS[resolve_version(version)]
    return GrammarLexer(source, grammar)


def tokenize_vhdl(source: str, version: str | None = None) -> list[Token]:
    """Tokenize VHDL source code with an optional edition override."""
    resolved = resolve_version(version)
    lexer = create_vhdl_lexer(source, version=resolved)
    raw_tokens = lexer.tokenize()
    return _normalize_case(raw_tokens, _VHDL_KEYWORDS[resolved])
