"""Lexer — Layer 2 of the computing stack.

Tokenizes source code into a stream of tokens. The lexer reads raw source
code text character by character and groups those characters into meaningful
units called *tokens* — the smallest building blocks a parser can work with.

Usage::

    from lexer import Lexer, LexerConfig, TokenType

    tokens = Lexer("x = 1 + 2").tokenize()

    # With language-specific keywords:
    config = LexerConfig(keywords=["if", "else", "while"])
    tokens = Lexer("if x == 1", config).tokenize()
"""

from lexer.tokenizer import Lexer, LexerConfig, LexerError, Token, TokenType

__all__ = [
    "Lexer",
    "LexerConfig",
    "LexerError",
    "Token",
    "TokenType",
]
