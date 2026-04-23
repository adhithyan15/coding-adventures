"""Verilog lexer backed by compiled, versioned grammars."""

from __future__ import annotations

from lexer import GrammarLexer, Token

from verilog_lexer import _grammar_1995, _grammar_2001, _grammar_2005
from verilog_lexer.preprocessor import verilog_preprocess

DEFAULT_VERSION = "2005"
SUPPORTED_VERSIONS = frozenset({"1995", "2001", "2005"})

_TOKEN_GRAMMARS = {
    "1995": _grammar_1995.TOKEN_GRAMMAR,
    "2001": _grammar_2001.TOKEN_GRAMMAR,
    "2005": _grammar_2005.TOKEN_GRAMMAR,
}


def resolve_version(version: str | None = None) -> str:
    """Normalize a Verilog edition string and reject unknown values."""
    resolved = DEFAULT_VERSION if not version else version
    if resolved not in SUPPORTED_VERSIONS:
        valid = ", ".join(sorted(SUPPORTED_VERSIONS))
        raise ValueError(
            f"Unknown Verilog version {resolved!r}. Valid versions: {valid}"
        )
    return resolved


def create_verilog_lexer(
    source: str, *, preprocess: bool = True, version: str | None = None
) -> GrammarLexer:
    """Create a Verilog lexer for a specific supported edition."""
    if preprocess:
        source = verilog_preprocess(source)
    grammar = _TOKEN_GRAMMARS[resolve_version(version)]
    return GrammarLexer(source, grammar)


def tokenize_verilog(
    source: str, *, preprocess: bool = True, version: str | None = None
) -> list[Token]:
    """Tokenize Verilog source code with an optional edition override."""
    lexer = create_verilog_lexer(source, preprocess=preprocess, version=version)
    return lexer.tokenize()
