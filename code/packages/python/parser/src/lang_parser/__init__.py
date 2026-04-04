"""Parser — Layer 3 of the computing stack.

Builds abstract syntax trees from token streams. This package provides two
parsers:

1. **Hand-written Parser** (``Parser``): Uses recursive descent with specific
   AST node types (``NumberLiteral``, ``BinaryOp``, etc.). Great for learning
   and for cases where you want typed AST nodes.

2. **Grammar-driven Parser** (``GrammarParser``): Reads grammar rules from a
   ``.grammar`` file (via ``grammar_tools``) and produces generic ``ASTNode``
   trees. Language-agnostic — swap the grammar file to parse a different
   language.

Usage (hand-written parser):
    from lexer import Token, TokenType
    from lang_parser import Parser, Program, NumberLiteral, BinaryOp

    tokens = [Token(TokenType.NUMBER, "42", 1, 1), Token(TokenType.EOF, "", 1, 3)]
    parser = Parser(tokens)
    ast = parser.parse()  # Returns a Program node

Usage (grammar-driven parser):
    from grammar_tools import parse_parser_grammar
    from lexer import Lexer
    from lang_parser import GrammarParser, ASTNode

    grammar = parse_parser_grammar(open("python.grammar").read())
    tokens = Lexer("x = 1 + 2").tokenize()
    parser = GrammarParser(tokens, grammar)
    ast = parser.parse()  # Returns a generic ASTNode tree

AST Node Types (hand-written parser):
    NumberLiteral  — A numeric literal (e.g., 42)
    StringLiteral  — A string literal (e.g., "hello")
    Name           — A variable reference (e.g., x)
    BinaryOp       — A binary operation (e.g., 1 + 2)
    Assignment     — A variable assignment (e.g., x = 42)
    Program        — The root node containing all statements

AST Node Types (grammar-driven parser):
    ASTNode        — A generic node with rule_name and children
"""

from lang_parser.grammar_parser import (
    ASTNode,
    GrammarParseError,
    GrammarParser,
    collect_tokens,
    find_nodes,
    is_ast_node,
    walk_ast,
)
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
    # Hand-written parser (specific AST nodes)
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
    # Grammar-driven parser (generic AST nodes)
    "ASTNode",
    "GrammarParseError",
    "GrammarParser",
    "collect_tokens",
    "find_nodes",
    "is_ast_node",
    "walk_ast",
]
