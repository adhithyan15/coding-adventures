"""Excel formula lexer built on the shared grammar-driven lexer.

The ``excel.tokens`` grammar is compiled ahead of time by the
``grammar-tools`` compiler into ``excel_lexer/_grammar.py``, which embeds
the ``TokenGrammar`` as native Python data structures. This module imports
``TOKEN_GRAMMAR`` from it directly instead of reading and parsing
``excel.tokens`` from ``code/grammars/`` at runtime — no file I/O happens,
and the package works correctly when installed as a site-package.

To regenerate after editing ``code/grammars/excel/excel.tokens``:

    grammar-tools compile-tokens code/grammars/excel/excel.tokens \\
        -o code/packages/python/excel-lexer/src/excel_lexer/_grammar.py
"""

from __future__ import annotations

from lexer import GrammarLexer, LexerContext, Token

from excel_lexer._grammar import TOKEN_GRAMMAR


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
    lexer = GrammarLexer(source, TOKEN_GRAMMAR)
    lexer.set_on_token(excel_on_token)
    return lexer


def tokenize_excel_formula(source: str) -> list[Token]:
    return create_excel_lexer(source).tokenize()
