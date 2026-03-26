# Lisp Parser

Parses Lisp source code into ASTs using the grammar-driven parser infrastructure.

## What is this?

A thin wrapper around the generic `GrammarParser` that loads the `lisp.grammar` grammar file. It tokenizes input using the Lisp lexer, then parses the token stream into an abstract syntax tree (AST).

## How it fits in the stack

```
Logic Gates → Arithmetic → CPU → Assembler → Lexer → [Parser] → Compiler → GC → VM
                                                          ↑
                                                   lisp.grammar
```

## Usage

```python
from lisp_parser import parse_lisp

ast = parse_lisp("(+ 1 2)")
# ASTNode(rule_name="program", children=[
#     ASTNode(rule_name="sexpr", children=[
#         ASTNode(rule_name="list", children=[
#             Token(LPAREN, '('),
#             ASTNode(rule_name="list_body", children=[
#                 ASTNode(rule_name="sexpr", children=[
#                     ASTNode(rule_name="atom", children=[Token(SYMBOL, '+')])
#                 ]),
#                 ASTNode(rule_name="sexpr", children=[
#                     ASTNode(rule_name="atom", children=[Token(NUMBER, '1')])
#                 ]),
#                 ASTNode(rule_name="sexpr", children=[
#                     ASTNode(rule_name="atom", children=[Token(NUMBER, '2')])
#                 ]),
#             ]),
#             Token(RPAREN, ')')
#         ])
#     ])
# ])
```

## Grammar rules

```
program   = { sexpr } ;
sexpr     = atom | list | quoted ;
atom      = NUMBER | SYMBOL | STRING ;
list      = LPAREN list_body RPAREN ;
list_body = [ sexpr { sexpr } [ DOT sexpr ] ] ;
quoted    = QUOTE sexpr ;
```
