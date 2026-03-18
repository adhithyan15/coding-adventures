"""Parser — Layer 3 of the computing stack.

Builds abstract syntax trees from token streams using recursive descent parsing.

The parser takes a flat list of tokens (from the lexer) and constructs an AST
(Abstract Syntax Tree) that represents the grammatical structure of the source
code. The tree encodes operator precedence, grouping, and statement boundaries.

Usage:
    from lexer import Token, TokenType
    from lang_parser import Parser, Program, NumberLiteral, BinaryOp

    tokens = [Token(TokenType.NUMBER, "42", 1, 1), Token(TokenType.EOF, "", 1, 3)]
    parser = Parser(tokens)
    ast = parser.parse()  # Returns a Program node

AST Node Types:
    NumberLiteral  — A numeric literal (e.g., 42)
    StringLiteral  — A string literal (e.g., "hello")
    Name           — A variable reference (e.g., x)
    BinaryOp       — A binary operation (e.g., 1 + 2)
    Assignment     — A variable assignment (e.g., x = 42)
    Program        — The root node containing all statements
"""

from lang_parser.parser import (
    Assignment,
    BinaryOp,
    Expression,
    Name,
    NumberLiteral,
    ParseError,
    Parser,
    Program,
    Statement,
    StringLiteral,
)

__all__ = [
    "Assignment",
    "BinaryOp",
    "Expression",
    "Name",
    "NumberLiteral",
    "ParseError",
    "Parser",
    "Program",
    "Statement",
    "StringLiteral",
]
