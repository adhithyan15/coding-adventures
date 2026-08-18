"""Excel formula parser built on the shared grammar-driven parser.

The ``excel.grammar`` grammar is compiled ahead of time by the
``grammar-tools`` compiler into ``excel_parser/_grammar.py``, which embeds
the ``ParserGrammar`` as native Python data structures. This module
imports ``PARSER_GRAMMAR`` from it directly instead of reading and parsing
``excel.grammar`` from ``code/grammars/`` at runtime — no file I/O
happens, and the package works correctly when installed as a
site-package.

To regenerate after editing ``code/grammars/excel/excel.grammar``:

    grammar-tools compile-grammar code/grammars/excel/excel.grammar \\
        -o code/packages/python/excel-parser/src/excel_parser/_grammar.py
"""

from __future__ import annotations

from excel_lexer import tokenize_excel_formula
from lang_parser import ASTNode, GrammarParser
from lexer import Token

from excel_parser._grammar import PARSER_GRAMMAR


def _previous_significant_token(tokens: list[Token], index: int) -> Token | None:
    for i in range(index - 1, -1, -1):
        if tokens[i].type_name != "SPACE":
            return tokens[i]
    return None


def _next_significant_token(tokens: list[Token], index: int) -> Token | None:
    for i in range(index + 1, len(tokens)):
        if tokens[i].type_name != "SPACE":
            return tokens[i]
    return None


def normalize_excel_reference_tokens(tokens: list[Token]) -> list[Token]:
    normalized: list[Token] = []

    for index, token in enumerate(tokens):
        if token.type_name not in {"NAME", "NUMBER"}:
            normalized.append(token)
            continue

        previous = _previous_significant_token(tokens, index)
        next_token = _next_significant_token(tokens, index)
        adjacent_to_colon = (
            previous is not None
            and previous.type_name == "COLON"
            or next_token is not None
            and next_token.type_name == "COLON"
        )

        if token.type_name == "NAME" and adjacent_to_colon:
            normalized.append(Token("COLUMN_REF", token.value, token.line, token.column))
            continue

        if token.type_name == "NUMBER" and adjacent_to_colon:
            normalized.append(Token("ROW_REF", token.value, token.line, token.column))
            continue

        normalized.append(token)

    return normalized


def create_excel_parser(source: str) -> GrammarParser:
    tokens = tokenize_excel_formula(source)
    parser = GrammarParser(tokens, PARSER_GRAMMAR)
    parser.add_pre_parse(normalize_excel_reference_tokens)
    return parser


def parse_excel_formula(source: str) -> ASTNode:
    return create_excel_parser(source).parse()
