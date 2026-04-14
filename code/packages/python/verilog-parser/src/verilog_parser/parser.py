"""Verilog parser backed by compiled, versioned grammars."""

from __future__ import annotations

from pathlib import Path

from lang_parser import ASTNode, GrammarParser
from verilog_lexer import tokenize_verilog
from verilog_lexer.tokenizer import DEFAULT_VERSION, resolve_version
from verilog_parser import _grammar_1995, _grammar_2001, _grammar_2005

GRAMMAR_DIR = Path(__file__).resolve().parents[5] / "grammars"
VERILOG_GRAMMAR_PATH = GRAMMAR_DIR / "verilog" / "verilog2005.grammar"
SUPPORTED_VERSIONS = frozenset({"1995", "2001", "2005"})

_PARSER_GRAMMARS = {
    "1995": _grammar_1995.PARSER_GRAMMAR,
    "2001": _grammar_2001.PARSER_GRAMMAR,
    "2005": _grammar_2005.PARSER_GRAMMAR,
}


def resolve_grammar_path(version: str | None = None) -> Path:
    """Return the canonical source grammar path for a supported edition."""
    resolved = resolve_version(version)
    return GRAMMAR_DIR / "verilog" / f"verilog{resolved}.grammar"


def create_verilog_parser(
    source: str, *, preprocess: bool = True, version: str | None = None
) -> GrammarParser:
    """Create a Verilog parser for a specific supported edition."""
    resolved = resolve_version(version)
    tokens = tokenize_verilog(source, preprocess=preprocess, version=resolved)
    grammar = _PARSER_GRAMMARS[resolved]
    return GrammarParser(tokens, grammar)


def parse_verilog(
    source: str, *, preprocess: bool = True, version: str | None = None
) -> ASTNode:
    """Parse Verilog source code with an optional edition override."""
    parser = create_verilog_parser(source, preprocess=preprocess, version=version)
    return parser.parse()
