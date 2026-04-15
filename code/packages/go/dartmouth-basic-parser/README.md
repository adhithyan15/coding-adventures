# Dartmouth BASIC Parser (Go)

Grammar-driven parser for Dartmouth BASIC 1964, the language that introduced
millions of people to programming.

## Overview

This package is Layer 2 of the Dartmouth BASIC toolchain. It accepts a token
stream produced by `dartmouth-basic-lexer` and produces an Abstract Syntax
Tree (AST) using `dartmouth_basic.grammar` and the grammar-driven parser engine.

```
BASIC source text
      │
      ▼  dartmouthlexer.TokenizeDartmouthBasic(source)
┌──────────────────────────────────┐
│   dartmouth-basic-lexer          │
└──────────────────────────────────┘
      │  []lexer.Token
      ▼  dartmouthbasicparser.ParseDartmouthBasic(source)
┌──────────────────────────────────┐
│   dartmouth-basic-parser         │  ← this package
│   dartmouth_basic.grammar        │
└──────────────────────────────────┘
      │  *parser.ASTNode{RuleName: "program", ...}
      ▼
┌──────────────────────────────────┐
│   dartmouth-basic-compiler       │
└──────────────────────────────────┘
```

## Usage

```go
// One-shot: source → AST
ast, err := dartmouthbasicparser.ParseDartmouthBasic("10 PRINT \"HELLO\"\n20 END\n")
if err != nil {
    log.Fatal(err)
}
fmt.Println(ast.RuleName) // "program"

// Two-step: create parser, then parse
p, err := dartmouthbasicparser.CreateDartmouthBasicParser("10 LET X = 5\n")
if err != nil {
    log.Fatal(err)
}
ast, err := p.Parse()
```

## Supported Statements

All 17 statement types from the 1964 Dartmouth BASIC specification:

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
expr  → term { (+ | -) term }          lowest
term  → power { (* | /) power }
power → unary [ ^ power ]              right-associative
unary → - primary | primary
primary → NUMBER | BUILTIN_FN(expr) | USER_FN(expr) | variable | (expr)
```

## Dependencies

- `grammar-tools` — parses `.grammar` files into ParserGrammar structs
- `lexer` — grammar-driven tokenization engine
- `parser` — grammar-driven parsing engine with packrat memoization
- `dartmouth-basic-lexer` — Dartmouth BASIC tokenization
