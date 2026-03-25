"""Excel formula lexer built on the shared grammar-driven lexer."""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, LexerContext, Token

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
EXCEL_TOKENS_PATH = GRAMMAR_DIR / "excel.tokens"


def _next_non_space_char(ctx: LexerContext) -> str:
    offset = 1
    while True:
        ch = ctx.peek(offset)
        if ch == "" or ch != " ":
            return ch
        offset += 1


def excel_on_token(token: Token, ctx: LexerContext) -> None:
    if token.type_name != "NAME":
        return

    next_char = _next_non_space_char(ctx)
    if next_char == "(":
        ctx.suppress()
        ctx.emit(Token("FUNCTION_NAME", token.value, token.line, token.column))
        return

    if next_char == "[":
        ctx.suppress()
        ctx.emit(Token("TABLE_NAME", token.value, token.line, token.column))


def create_excel_lexer(source: str) -> GrammarLexer:
    grammar = parse_token_grammar(EXCEL_TOKENS_PATH.read_text())
    lexer = GrammarLexer(source, grammar)
    lexer.set_on_token(excel_on_token)
    return lexer


def tokenize_excel_formula(source: str) -> list[Token]:
    return create_excel_lexer(source).tokenize()
