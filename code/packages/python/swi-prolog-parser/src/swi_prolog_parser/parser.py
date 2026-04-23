"""SWI-Prolog parser backed by ``code/grammars/prolog/swi.grammar``."""

from __future__ import annotations

from dataclasses import dataclass
from functools import cache
from pathlib import Path

from grammar_tools import ParserGrammar, parse_parser_grammar
from lang_parser import ASTNode, GrammarParseError, GrammarParser
from lexer import Token
from logic_engine import Clause, Program
from prolog_parser import ParsedQuery, PrologParseError, lower_ast
from swi_prolog_lexer import tokenize_swi_prolog

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
SWI_PROLOG_GRAMMAR_PATH = GRAMMAR_DIR / "prolog" / "swi.grammar"


@dataclass(frozen=True, slots=True)
class ParsedSwiDirective:
    """A top-level SWI directive parsed from ``:- goal.`` syntax."""

    goal_ast: ASTNode


@dataclass(frozen=True, slots=True)
class ParsedSwiSource:
    """A parsed SWI-Prolog source file lowered to executable engine objects."""

    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]
    directives: tuple[ParsedSwiDirective, ...]


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
    for token in tokens:
        if token.type_name == "DCG":
            raise PrologParseError(
                token,
                "DCG rules are recognized by the SWI lexer but not parsed yet",
            )
    try:
        return GrammarParser(tokens, _swi_parser_grammar()).parse()
    except GrammarParseError as error:
        token = error.token if error.token is not None else tokens[-1]
        raise PrologParseError(token, str(error)) from error


def parse_swi_source(source: str) -> ParsedSwiSource:
    """Parse SWI-Prolog clauses, queries, and top-level directives."""

    ast = parse_swi_ast(source)
    executable_ast, directives = _split_directives(ast)
    parsed = lower_ast(executable_ast)
    return ParsedSwiSource(
        program=parsed.program,
        clauses=parsed.clauses,
        queries=parsed.queries,
        directives=directives,
    )


def parse_swi_program(source: str) -> Program:
    """Parse a SWI-Prolog source containing only facts, rules, and directives."""

    parsed = parse_swi_source(source)
    if parsed.queries:
        raise PrologParseError(
            tokenize_swi_prolog(source)[0],
            "expected only clauses and directives, but found "
            f"{len(parsed.queries)} query statement(s)",
        )
    return parsed.program


def parse_swi_query(source: str) -> ParsedQuery:
    """Parse one SWI-Prolog top-level query statement."""

    parsed = parse_swi_source(source)
    if parsed.directives:
        raise PrologParseError(
            tokenize_swi_prolog(source)[0],
            f"expected only a query, but found {len(parsed.directives)} directive(s)",
        )
    if parsed.clauses:
        raise PrologParseError(
            tokenize_swi_prolog(source)[0],
            f"expected only a query, but found {len(parsed.clauses)} clause(s)",
        )
    if len(parsed.queries) != 1:
        raise PrologParseError(
            tokenize_swi_prolog(source)[0],
            f"expected exactly one query, but found {len(parsed.queries)}",
        )
    return parsed.queries[0]


def _split_directives(ast: ASTNode) -> tuple[ASTNode, tuple[ParsedSwiDirective, ...]]:
    """Return a generic-compatible AST plus SWI directive metadata."""

    statements: list[ASTNode | Token] = []
    directives: list[ParsedSwiDirective] = []
    for statement in ast.children:
        if not isinstance(statement, ASTNode):
            statements.append(statement)
            continue

        body = _single_node_child(statement)
        if body.rule_name == "directive_statement":
            directives.append(
                ParsedSwiDirective(goal_ast=_single_node_child(body, "goal")),
            )
        else:
            statements.append(statement)

    executable_ast = ASTNode(
        rule_name=ast.rule_name,
        children=statements,
        start_line=ast.start_line,
        start_column=ast.start_column,
        end_line=ast.end_line,
        end_column=ast.end_column,
    )
    return executable_ast, tuple(directives)


def _single_node_child(node: ASTNode, rule_name: str | None = None) -> ASTNode:
    """Return exactly one AST child from ``node``."""

    children = [
        child
        for child in node.children
        if isinstance(child, ASTNode)
        and (rule_name is None or child.rule_name == rule_name)
    ]
    if len(children) != 1:
        token = _first_token(node) or Token("EOF", "", 1, 1)
        raise PrologParseError(
            token,
            f"expected one {rule_name or 'AST'} child, found {len(children)}",
        )
    return children[0]


def _first_token(node: ASTNode) -> Token | None:
    for child in node.children:
        if isinstance(child, Token):
            return child
        token = _first_token(child)
        if token is not None:
            return token
    return None
