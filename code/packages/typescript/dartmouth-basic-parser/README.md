# @coding-adventures/dartmouth-basic-parser

Parser for the 1964 Dartmouth BASIC language. Takes a stream of BASIC tokens
and produces an Abstract Syntax Tree (AST) by applying the
`dartmouth_basic.grammar` rules through the grammar-driven `GrammarParser` engine.

## What This Is

This package is the second stage of the Dartmouth BASIC front-end pipeline:

```
Source text
    │
    ▼
@coding-adventures/dartmouth-basic-lexer    → Token[]
    │
    ▼
dartmouth_basic.grammar                     → ParserGrammar (rules)
    │
    ▼
@coding-adventures/parser (GrammarParser)   → ASTNode (AST)
    │
    ▼
compiler or interpreter
```

The parser is **grammar-driven**: no hand-written recursive descent code.
The `dartmouth_basic.grammar` file defines all 17 statement types and the
expression precedence hierarchy. The `GrammarParser` engine interprets these
rules at runtime using recursive descent with packrat memoization.

## Historical Context

Dartmouth BASIC was created by John G. Kemeny and Thomas E. Kurtz at Dartmouth
College in 1964. Running on a GE-225 mainframe accessed via uppercase-only
teletypes, it was the first programming language designed for non-science
students — the goal was to make computing accessible to everyone.

The 17 statement types in the 1964 specification:

| Statement | Purpose |
|-----------|---------|
| LET       | Variable assignment: `10 LET X = 5` |
| PRINT     | Output to terminal: `20 PRINT X, Y` |
| INPUT     | Read from user: `30 INPUT A, B` |
| IF-THEN   | Conditional branch: `40 IF X > 0 THEN 100` |
| GOTO      | Unconditional jump: `50 GOTO 200` |
| GOSUB     | Subroutine call: `60 GOSUB 300` |
| RETURN    | Return from subroutine: `300 RETURN` |
| FOR       | Start counted loop: `70 FOR I = 1 TO 10` |
| NEXT      | End counted loop: `80 NEXT I` |
| END       | Normal program termination |
| STOP      | Halt with message (resumable in DTSS) |
| REM       | Comment / remark |
| READ      | Read from DATA pool: `90 READ X, Y` |
| DATA      | Define data pool: `100 DATA 1, 2, 3` |
| RESTORE   | Reset DATA pool pointer |
| DIM       | Declare array size: `110 DIM A(100)` |
| DEF       | Define user function: `120 DEF FNA(X) = X*X` |

## Usage

```typescript
import { parseDartmouthBasic } from "@coding-adventures/dartmouth-basic-parser";

// Parse a complete BASIC program
const ast = parseDartmouthBasic("10 LET X = 5\n20 PRINT X\n30 END\n");
console.log(ast.ruleName); // "program"
```

## AST Structure

The root node has `ruleName = "program"`. Its children are `line` nodes,
each containing a LINE_NUM token, an optional statement, and a NEWLINE:

```
ASTNode("program", [
  ASTNode("line", [
    Token(LINE_NUM, "10"),
    ASTNode("statement", [
      ASTNode("let_stmt", [
        Token(KEYWORD, "LET"),
        ASTNode("variable", [Token(NAME, "X")]),
        Token(EQ, "="),
        ASTNode("expr", [...])
      ])
    ]),
    Token(NEWLINE, "\n")
  ])
])
```

## How It Fits in the Stack

- **Depends on**: `@coding-adventures/dartmouth-basic-lexer`,
  `@coding-adventures/grammar-tools`, `@coding-adventures/parser`,
  `@coding-adventures/lexer`, `@coding-adventures/directed-graph`
- **Used by**: future `dartmouth-basic-compiler` and `dartmouth-basic-vm` packages
- **Grammar file**: `code/grammars/dartmouth_basic.grammar` (shared across
  all language implementations — Rust, TypeScript, etc.)

## Running Tests

```
npm ci
npx vitest run --coverage
```
