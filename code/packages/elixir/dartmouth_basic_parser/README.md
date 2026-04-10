# Dartmouth BASIC Parser (Elixir)

Grammar-driven parser for Dartmouth BASIC 1964, the language that introduced
millions of people to programming.

## Overview

This package is Layer 2 of the Dartmouth BASIC toolchain. It accepts a token
stream produced by `dartmouth_basic_lexer` and produces an Abstract Syntax
Tree (AST) using `dartmouth_basic.grammar` and the grammar-driven parser engine.

```
BASIC source text
      │
      ▼  DartmouthBasicLexer.tokenize/1
┌──────────────────────────────────┐
│   dartmouth_basic_lexer          │
└──────────────────────────────────┘
      │  token stream
      ▼  DartmouthBasicParser.parse/1
┌──────────────────────────────────┐
│   dartmouth_basic_parser         │  ← this package
│   dartmouth_basic.grammar        │
└──────────────────────────────────┘
      │  %ASTNode{rule_name: "program", ...}
      ▼
┌──────────────────────────────────┐
│   dartmouth_basic_compiler       │
└──────────────────────────────────┘
```

## Usage

```elixir
# One-shot: source → AST
{:ok, ast} = CodingAdventures.DartmouthBasicParser.parse_source("""
10 FOR I = 1 TO 5
20 PRINT I
30 NEXT I
40 END
""")
ast.rule_name  # => "program"

# Two-step: tokenize separately, then parse
{:ok, tokens} = CodingAdventures.DartmouthBasicLexer.tokenize("10 LET X = 5\n")
{:ok, ast}    = CodingAdventures.DartmouthBasicParser.parse(tokens)

# Grammar inspection
grammar = CodingAdventures.DartmouthBasicParser.create_parser()
Enum.map(grammar.rules, & &1.name)
# => ["program", "line", "statement", "let_stmt", ...]
```

## Supported Statements

All 17 statement types from the original 1964 Dartmouth BASIC specification:

| Statement | Example |
|-----------|---------|
| LET       | `10 LET X = 5` |
| PRINT     | `10 PRINT X, Y` |
| INPUT     | `10 INPUT A, B` |
| IF/THEN   | `10 IF X > 0 THEN 100` |
| GOTO      | `10 GOTO 50` |
| GOSUB     | `10 GOSUB 200` |
| RETURN    | `200 RETURN` |
| FOR       | `10 FOR I = 1 TO 10` |
| NEXT      | `30 NEXT I` |
| END       | `99 END` |
| STOP      | `99 STOP` |
| REM       | `10 REM A COMMENT` |
| READ      | `10 READ X, Y` |
| DATA      | `20 DATA 1, 2, 3` |
| RESTORE   | `30 RESTORE` |
| DIM       | `10 DIM A(100)` |
| DEF       | `10 DEF FNA(X) = X * X` |

## Expression Precedence

```
expr  → term { (+ | -) term }           lowest precedence
term  → power { (* | /) power }
power → unary [ ^ power ]               right-associative
unary → - primary | primary
primary → NUMBER | BUILTIN_FN(expr) | USER_FN(expr) | variable | (expr)
```

## Dependencies

- `grammar_tools` — parses `.grammar` files into ParserGrammar structs
- `lexer` — grammar-driven tokenization engine
- `parser` — grammar-driven parsing engine with packrat memoization
- `dartmouth_basic_lexer` — Dartmouth BASIC tokenization (LINE_NUM, KEYWORD, etc.)
