"""SWI-Prolog parser backed by ``code/grammars/prolog/swi.grammar``."""

from __future__ import annotations

from dataclasses import dataclass
from functools import cache
from pathlib import Path

from grammar_tools import ParserGrammar, parse_parser_grammar
from lang_parser import ASTNode, GrammarParseError, GrammarParser
from lexer import Token
from logic_engine import Clause, Program
from prolog_core import (
    OperatorTable,
    PredicateRegistry,
    PrologDirective,
    swi_operator_table,
)
from prolog_operator_parser import (
    parse_operator_program_tokens,
    parse_operator_query_tokens,
    parse_operator_source_tokens,
)
from prolog_parser import (
    ParsedQuery,
    PrologParseError,
)
from swi_prolog_lexer import tokenize_swi_prolog

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
SWI_PROLOG_GRAMMAR_PATH = GRAMMAR_DIR / "prolog" / "swi.grammar"


@dataclass(frozen=True, slots=True)
class ParsedSwiSource:
    """A parsed SWI-Prolog source file lowered to executable engine objects."""

    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]
    directives: tuple[PrologDirective, ...]
    operator_table: OperatorTable
    predicate_registry: PredicateRegistry


ParsedSwiDirective = PrologDirective


@cache
def _swi_parser_grammar() -> ParserGrammar:
    """Load and cache the SWI-Prolog parser grammar."""

    return parse_parser_grammar(SWI_PROLOG_GRAMMAR_PATH.read_text())


def create_swi_prolog_parser(source: str) -> GrammarParser:
    """Create a grammar-driven parser configured for SWI-Prolog."""

    return GrammarParser(tokenize_swi_prolog(source), _swi_parser_grammar())


def parse_swi_ast(source: str) -> ASTNode:
    """Parse SWI-Prolog source and return the grammar AST."""

    tokens = tokenize_swi_prolog(source)
    try:
        return GrammarParser(tokens, _swi_parser_grammar()).parse()
    except GrammarParseError as error:
        token = error.token if error.token is not None else tokens[-1]
        raise PrologParseError(token, str(error)) from error


def parse_swi_source(
    source: str,
    *,
    operator_table: OperatorTable | None = None,
) -> ParsedSwiSource:
    """Parse SWI-Prolog clauses, queries, and top-level directives."""

    tokens = tokenize_swi_prolog(source)
    active_operator_table = (
        swi_operator_table() if operator_table is None else operator_table
    )
    parsed = parse_operator_source_tokens(
        tokens,
        active_operator_table,
        allow_directives=True,
    )
    return ParsedSwiSource(
        program=parsed.program,
        clauses=parsed.clauses,
        queries=parsed.queries,
        directives=parsed.directives,
        operator_table=parsed.operator_table,
        predicate_registry=parsed.predicate_registry,
    )


def parse_swi_program(
    source: str,
    *,
    operator_table: OperatorTable | None = None,
) -> Program:
    """Parse a SWI-Prolog source containing only facts, rules, and directives."""

    tokens = tokenize_swi_prolog(source)
    active_operator_table = (
        swi_operator_table() if operator_table is None else operator_table
    )
    return parse_operator_program_tokens(
        tokens,
        active_operator_table,
        allow_directives=True,
    )


def parse_swi_query(
    source: str,
    *,
    operator_table: OperatorTable | None = None,
) -> ParsedQuery:
    """Parse one SWI-Prolog top-level query statement."""

    tokens = tokenize_swi_prolog(source)
    first_token = next((token for token in tokens if token.type_name != "EOF"), None)
    if first_token is None:
        raise PrologParseError(Token("EOF", "", 1, 1), "expected only a query")
    if first_token.type_name == "RULE":
        raise PrologParseError(first_token, "expected a query, not a directive")
    if first_token.type_name != "QUERY":
        raise PrologParseError(first_token, "expected only a query")

    active_operator_table = (
        swi_operator_table() if operator_table is None else operator_table
    )
    return parse_operator_query_tokens(
        tokens,
        active_operator_table,
    )
