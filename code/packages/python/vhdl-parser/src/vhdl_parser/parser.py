"""VHDL parser backed by compiled, versioned grammars."""

from __future__ import annotations

from pathlib import Path

from lang_parser import ASTNode, GrammarParser
from vhdl_lexer import tokenize_vhdl
from vhdl_lexer.tokenizer import DEFAULT_VERSION, resolve_version
from vhdl_parser import (
    _grammar_1987,
    _grammar_1993,
    _grammar_2002,
    _grammar_2008,
    _grammar_2019,
)

GRAMMAR_DIR = Path(__file__).resolve().parents[5] / "grammars"
VHDL_GRAMMAR_PATH = GRAMMAR_DIR / "vhdl" / "vhdl2008.grammar"
SUPPORTED_VERSIONS = frozenset({"1987", "1993", "2002", "2008", "2019"})

_PARSER_GRAMMARS = {
    "1987": _grammar_1987.PARSER_GRAMMAR,
    "1993": _grammar_1993.PARSER_GRAMMAR,
    "2002": _grammar_2002.PARSER_GRAMMAR,
    "2008": _grammar_2008.PARSER_GRAMMAR,
    "2019": _grammar_2019.PARSER_GRAMMAR,
}


def resolve_grammar_path(version: str | None = None) -> Path:
    """Return the canonical source grammar path for a supported edition."""
    resolved = resolve_version(version)
    return GRAMMAR_DIR / "vhdl" / f"vhdl{resolved}.grammar"


def create_vhdl_parser(source: str, version: str | None = None) -> GrammarParser:
    """Create a VHDL parser for a specific supported edition."""
    resolved = resolve_version(version)
    tokens = tokenize_vhdl(source, version=resolved)
    grammar = _PARSER_GRAMMARS[resolved]
    return GrammarParser(tokens, grammar)


def parse_vhdl(source: str, version: str | None = None) -> ASTNode:
    """Parse VHDL source code with an optional edition override."""
    parser = create_vhdl_parser(source, version=version)
    return parser.parse()
