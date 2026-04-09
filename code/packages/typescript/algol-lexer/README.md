# @coding-adventures/algol-lexer

Tokenizes ALGOL 60 source text using the grammar-driven lexer infrastructure.

## What Is ALGOL 60?

ALGOL 60 (ALGOrithmic Language, 1960) is the common ancestor of virtually every modern programming language. It introduced block structure, lexical scoping, recursion, the call stack, and was the first language whose grammar was formally specified using BNF notation. Its direct descendants include Pascal, C, Ada, and Simula (the first object-oriented language).

## How It Fits in the Stack

```
algol.tokens (grammar definition)
       |
       v
grammar-tools (parseTokenGrammar)
       |
       v
lexer (grammarTokenize)           <-- generic engine, language-agnostic
       |
       v
algol-lexer (tokenizeAlgol)       <-- this package
       |
       v
algol-parser (parseAlgol)
```

This package is a thin wrapper. It reads `algol.tokens`, converts it to a `TokenGrammar`, and passes it to the generic `grammarTokenize` engine. No ALGOL-specific logic lives here.

## Installation

```bash
npm install @coding-adventures/algol-lexer
```

## Usage

```typescript
import { tokenizeAlgol } from "@coding-adventures/algol-lexer";

const tokens = tokenizeAlgol("begin integer x; x := 42 end");
// [
//   { type: "begin",       value: "begin",   line: 1, column: 1  },
//   { type: "integer",     value: "integer", line: 1, column: 7  },
//   { type: "IDENT",       value: "x",       line: 1, column: 15 },
//   { type: "SEMICOLON",   value: ";",        line: 1, column: 16 },
//   { type: "IDENT",       value: "x",       line: 1, column: 18 },
//   { type: "ASSIGN",      value: ":=",      line: 1, column: 20 },
//   { type: "INTEGER_LIT", value: "42",      line: 1, column: 23 },
//   { type: "end",         value: "end",     line: 1, column: 26 },
//   { type: "EOF",         value: "",        line: 1, column: 29 },
// ]
```

## Token Reference

### Value tokens

| Token       | Example      | Description                                        |
|-------------|--------------|-----------------------------------------------------|
| `INTEGER_LIT` | `0`, `42`, `1000` | Integer literal (decimal digits only)         |
| `REAL_LIT`  | `3.14`, `1.5E3`, `1.5E-3` | Real literal (decimal point, exponent, or both) |
| `STRING_LIT` | `'hello'`  | Single-quoted string (no escape sequences)         |
| `IDENT`     | `x`, `sum`, `A1` | Identifier (reclassified to keyword if reserved) |

### Operators and delimiters

| Token      | Text | Description                              |
|------------|------|------------------------------------------|
| `ASSIGN`   | `:=` | Assignment (not equality — that's `EQ`)  |
| `POWER`    | `**` | Exponentiation (Fortran convention)      |
| `CARET`    | `^`  | Exponentiation (alternate form)          |
| `LEQ`      | `<=` | Less-than-or-equal (ASCII for ≤)        |
| `GEQ`      | `>=` | Greater-than-or-equal (ASCII for ≥)     |
| `NEQ`      | `!=` | Not-equal (ASCII for ≠)                 |
| `EQ`       | `=`  | Equality test (NOT assignment)           |
| `PLUS`     | `+`  | Addition                                 |
| `MINUS`    | `-`  | Subtraction                              |
| `STAR`     | `*`  | Multiplication                           |
| `SLASH`    | `/`  | Division                                 |
| `LT`       | `<`  | Less than                                |
| `GT`       | `>`  | Greater than                             |
| `LPAREN`   | `(`  | Open parenthesis                         |
| `RPAREN`   | `)`  | Close parenthesis                        |
| `LBRACKET` | `[`  | Open bracket (array subscript)           |
| `RBRACKET` | `]`  | Close bracket                            |
| `SEMICOLON`| `;`  | Statement separator / comment terminator |
| `COMMA`    | `,`  | List separator                           |
| `COLON`    | `:`  | Bound pair separator                     |

### Keywords

All keywords are case-insensitive. `BEGIN`, `Begin`, and `begin` all produce the same token type.

**Block structure:** `begin`, `end`

**Control flow:** `if`, `then`, `else`, `for`, `do`, `step`, `until`, `while`, `goto`

**Declarations:** `switch`, `procedure`, `own`, `array`, `label`, `value`

**Types:** `integer`, `real`, `boolean`, `string`

**Boolean literals:** `true`, `false`

**Boolean operators:** `not`, `and`, `or`, `impl`, `eqv`

**Arithmetic keywords:** `div` (integer division), `mod` (modulo)

### Comments

```algol
comment this is a comment and will be ignored;
```

The word `comment` followed by any text up to the next `;` is consumed silently. No token is emitted.

## Key Design Notes

**`:=` vs `=`:** Assignment is `:=`; equality test is `=`. This avoids the classic C bug of writing `=` when you mean `==`.

**`**` vs `^`:** Both produce their own distinct token types (`POWER` and `CARET`). The parser treats them identically.

**Keyword boundary:** The lexer uses maximum munch. `beginning` is `IDENT("beginning")`, not `begin + ning`.

**Case insensitivity:** Keywords are normalized to lowercase after matching.
