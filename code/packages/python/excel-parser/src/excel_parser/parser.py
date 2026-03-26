"""Excel formula parser built on the shared grammar-driven parser."""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from lexer import Token

from excel_lexer import tokenize_excel_formula

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
EXCEL_GRAMMAR_PATH = GRAMMAR_DIR / "excel.grammar"


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
    grammar = parse_parser_grammar(EXCEL_GRAMMAR_PATH.read_text())
    parser = GrammarParser(tokens, grammar)
    parser.add_pre_parse(normalize_excel_reference_tokens)
    return parser


def parse_excel_formula(source: str) -> ASTNode:
    return create_excel_parser(source).parse()
