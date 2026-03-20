"""Lexer — Layer 2 of the computing stack.

Tokenizes source code into a stream of tokens. The lexer reads raw source
code text character by character and groups those characters into meaningful
units called *tokens* — the smallest building blocks a parser can work with.

Two lexer implementations are provided:

- ``Lexer`` — the hand-written reference implementation with hardcoded
  character-dispatching logic.
- ``GrammarLexer`` — a grammar-driven alternative that reads token
  definitions from a ``.tokens`` file (via ``grammar_tools``).

Both produce identical ``Token`` objects and are fully interchangeable.

Usage::

    from lexer import Lexer, LexerConfig, TokenType

    tokens = Lexer("x = 1 + 2").tokenize()

    # With language-specific keywords:
    config = LexerConfig(keywords=["if", "else", "while"])
    tokens = Lexer("if x == 1", config).tokenize()

    # Grammar-driven alternative:
    from grammar_tools import parse_token_grammar
    from lexer import GrammarLexer

    grammar = parse_token_grammar(open("python.tokens").read())
    tokens = GrammarLexer("x = 1 + 2", grammar).tokenize()
"""

from lexer.grammar_lexer import GrammarLexer
from lexer.tokenizer import (
    TOKENIZER_DFA,
    Lexer,
    LexerConfig,
    LexerError,
    Token,
    TokenType,
    classify_char,
)

__all__ = [
    "GrammarLexer",
    "Lexer",
    "LexerConfig",
    "LexerError",
    "TOKENIZER_DFA",
    "Token",
    "TokenType",
    "classify_char",
]
