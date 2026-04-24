"""ISO/Core Prolog parser backed by ``code/grammars/prolog/iso.grammar``."""

from __future__ import annotations

from dataclasses import dataclass
from functools import cache
from pathlib import Path

from grammar_tools import ParserGrammar, parse_parser_grammar
from iso_prolog_lexer import tokenize_iso_prolog
from lang_parser import ASTNode, GrammarParseError, GrammarParser
from lexer import Token
from logic_engine import Clause, Program
from prolog_core import (
    OperatorTable,
    PredicateRegistry,
    PrologDirective,
    iso_operator_table,
)
from prolog_operator_parser import (
    parse_operator_program_tokens,
    parse_operator_query_tokens,
    parse_operator_source_tokens,
)
from prolog_parser import ParsedQuery, PrologParseError

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
ISO_PROLOG_GRAMMAR_PATH = GRAMMAR_DIR / "prolog" / "iso.grammar"


@dataclass(frozen=True, slots=True)
class ParsedIsoSource:
    """A parsed ISO/Core source file with shared directive/operator metadata."""

    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]
    directives: tuple[PrologDirective, ...]
    operator_table: OperatorTable
    predicate_registry: PredicateRegistry


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
    try:
        return GrammarParser(tokens, _iso_parser_grammar()).parse()
    except GrammarParseError as error:
        token = error.token if error.token is not None else tokens[-1]
        raise PrologParseError(token, str(error)) from error


def parse_iso_source(
    source: str,
    *,
    operator_table: OperatorTable | None = None,
) -> ParsedIsoSource:
    """Parse ISO/Core Prolog clauses, queries, and file-scoped directives."""

    tokens = tokenize_iso_prolog(source)
    active_operator_table = (
        iso_operator_table() if operator_table is None else operator_table
    )
    parsed = parse_operator_source_tokens(
        tokens,
        active_operator_table,
        allow_directives=True,
    )
    return ParsedIsoSource(
        program=parsed.program,
        clauses=parsed.clauses,
        queries=parsed.queries,
        directives=parsed.directives,
        operator_table=parsed.operator_table,
        predicate_registry=parsed.predicate_registry,
    )


def parse_iso_program(
    source: str,
    *,
    operator_table: OperatorTable | None = None,
) -> Program:
    """Parse an ISO/Core Prolog source containing only clauses and directives."""

    tokens = tokenize_iso_prolog(source)
    active_operator_table = (
        iso_operator_table() if operator_table is None else operator_table
    )
    return parse_operator_program_tokens(
        tokens,
        active_operator_table,
        allow_directives=True,
    )


def parse_iso_query(
    source: str,
    *,
    operator_table: OperatorTable | None = None,
) -> ParsedQuery:
    """Parse one ISO/Core Prolog top-level query statement."""

    tokens = tokenize_iso_prolog(source)
    first_token = next((token for token in tokens if token.type_name != "EOF"), None)
    if first_token is None or first_token.type_name != "QUERY":
        raise PrologParseError(
            first_token or Token("EOF", "", 1, 1),
            "expected only a query",
        )

    active_operator_table = (
        iso_operator_table() if operator_table is None else operator_table
    )
    return parse_operator_query_tokens(
        tokens,
        active_operator_table,
    )
