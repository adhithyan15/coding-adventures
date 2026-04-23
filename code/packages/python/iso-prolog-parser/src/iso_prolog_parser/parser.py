"""ISO/Core Prolog parser backed by ``code/grammars/prolog/iso.grammar``."""

from __future__ import annotations

from functools import cache
from pathlib import Path

from grammar_tools import ParserGrammar, parse_parser_grammar
from iso_prolog_lexer import tokenize_iso_prolog
from lang_parser import ASTNode, GrammarParseError, GrammarParser
from logic_engine import Program
from prolog_parser import ParsedQuery, ParsedSource, PrologParseError, lower_ast

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
ISO_PROLOG_GRAMMAR_PATH = GRAMMAR_DIR / "prolog" / "iso.grammar"


@cache
def _iso_parser_grammar() -> ParserGrammar:
    """Load and cache the ISO/Core Prolog parser grammar."""

    return parse_parser_grammar(ISO_PROLOG_GRAMMAR_PATH.read_text())


def create_iso_prolog_parser(source: str) -> GrammarParser:
    """Create a grammar-driven parser configured for ISO/Core Prolog."""

    return GrammarParser(tokenize_iso_prolog(source), _iso_parser_grammar())


def parse_iso_ast(source: str) -> ASTNode:
    """Parse ISO/Core Prolog source and return the grammar AST."""

    tokens = tokenize_iso_prolog(source)
    for token in tokens:
        if token.type_name == "DCG":
            raise PrologParseError(
                token,
                "DCG rules are recognized by the ISO lexer but not parsed yet",
            )
    try:
        return GrammarParser(tokens, _iso_parser_grammar()).parse()
    except GrammarParseError as error:
        token = error.token if error.token is not None else tokens[-1]
        raise PrologParseError(token, str(error)) from error


def parse_iso_source(source: str) -> ParsedSource:
    """Parse ISO/Core Prolog clauses and queries."""

    return lower_ast(parse_iso_ast(source))


def parse_iso_program(source: str) -> Program:
    """Parse an ISO/Core Prolog source containing only facts and rules."""

    parsed = parse_iso_source(source)
    if parsed.queries:
        raise PrologParseError(
            tokenize_iso_prolog(source)[0],
            "expected only clauses, but found "
            f"{len(parsed.queries)} query statement(s)",
        )
    return parsed.program


def parse_iso_query(source: str) -> ParsedQuery:
    """Parse one ISO/Core Prolog top-level query statement."""

    parsed = parse_iso_source(source)
    if parsed.clauses:
        raise PrologParseError(
            tokenize_iso_prolog(source)[0],
            f"expected only a query, but found {len(parsed.clauses)} clause(s)",
        )
    if len(parsed.queries) != 1:
        raise PrologParseError(
            tokenize_iso_prolog(source)[0],
            f"expected exactly one query, but found {len(parsed.queries)}",
        )
    return parsed.queries[0]
