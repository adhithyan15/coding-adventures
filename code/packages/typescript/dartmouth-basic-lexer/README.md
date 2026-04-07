# @coding-adventures/dartmouth-basic-lexer

Tokenizer for the 1964 Dartmouth BASIC programming language.

## What Is Dartmouth BASIC?

Dartmouth BASIC (Beginner's All-purpose Symbolic Instruction Code) was created
by John G. Kemeny and Thomas E. Kurtz at Dartmouth College in 1964. It ran on a
GE-225 mainframe, accessed through uppercase-only teletype terminals. It was the
first programming language designed specifically for students with no science or
mathematics background, and it ran in a time-shared environment where multiple
students could use the computer simultaneously.

Its descendants — Microsoft BASIC (1975), Applesoft BASIC, GW-BASIC, and
hundreds of home computer BASICs — are how millions of people first learned
to program.

A typical 1964 BASIC program looks like this:

```
10 LET X = 1
20 FOR I = 1 TO 5
30 LET X = X * I
40 NEXT I
50 PRINT X
60 END
```

Every statement lives on a numbered line. The line number is both the line's
address (for sorting) and its jump target (for `GOTO`/`GOSUB`/`IF...THEN`).

## Where This Package Fits

```
BASIC source text
      │
      ▼
┌─────────────────────────────────┐
│  dartmouth-basic-lexer          │  ← this package
│  dartmouth_basic.tokens grammar │
└─────────────────────────────────┘
      │  [{type, value, line, column}, ...]
      ▼
┌─────────────────────────────────┐
│  dartmouth-basic-parser         │  (future package)
└─────────────────────────────────┘
```

The lexer is a **thin wrapper** around the generic
`@coding-adventures/lexer` package. It loads the `dartmouth_basic.tokens`
grammar file and applies two post-tokenize hooks to handle:

1. **LINE_NUM disambiguation** — integers at the start of a line are line
   labels, not numeric values.
2. **REM suppression** — everything after `REM` on a line is a comment
   and must be invisible to the parser.

## Installation

This package is part of the `coding-adventures` monorepo. Add it to your
TypeScript package's `package.json`:

```json
{
  "dependencies": {
    "@coding-adventures/dartmouth-basic-lexer": "file:../dartmouth-basic-lexer"
  }
}
```

## Usage

```typescript
import { tokenizeDartmouthBasic } from "@coding-adventures/dartmouth-basic-lexer";

const tokens = tokenizeDartmouthBasic("10 LET X = 5\n20 PRINT X\n30 END\n");

// tokens:
// [
//   { type: "LINE_NUM", value: "10",    line: 1, column: 1 },
//   { type: "KEYWORD",  value: "LET",   line: 1, column: 4 },
//   { type: "NAME",     value: "X",     line: 1, column: 8 },
//   { type: "EQ",       value: "=",     line: 1, column: 10 },
//   { type: "NUMBER",   value: "5",     line: 1, column: 12 },
//   { type: "NEWLINE",  value: "\n",    line: 1, column: 13 },
//   { type: "LINE_NUM", value: "20",    line: 2, column: 1 },
//   { type: "KEYWORD",  value: "PRINT", line: 2, column: 4 },
//   { type: "NAME",     value: "X",     line: 2, column: 10 },
//   { type: "NEWLINE",  value: "\n",    line: 2, column: 11 },
//   { type: "LINE_NUM", value: "30",    line: 3, column: 1 },
//   { type: "KEYWORD",  value: "END",   line: 3, column: 4 },
//   { type: "NEWLINE",  value: "\n",    line: 3, column: 7 },
//   { type: "EOF",      value: "",      line: 3, column: 8 },
// ]
```

### Advanced Usage — Custom Hooks

If you need to attach additional post-tokenize hooks:

```typescript
import { createDartmouthBasicLexer } from "@coding-adventures/dartmouth-basic-lexer";
import type { Token } from "@coding-adventures/lexer";

function myHook(tokens: Token[]): Token[] {
  // ... transform tokens ...
  return tokens;
}

const lex = createDartmouthBasicLexer("10 LET X = 1\n");
lex.addPostTokenize(myHook);
const tokens = lex.tokenize();
```

## Token Types

| Type        | Example            | Description                                    |
|-------------|--------------------|-------------------------------------------------|
| `LINE_NUM`  | `"10"`, `"999"`    | Integer at the start of a line (relabeled)     |
| `NUMBER`    | `"3.14"`, `"1.5E3"`| Numeric literal in an expression               |
| `STRING`    | `"\"HELLO\""`      | Double-quoted string literal (includes quotes) |
| `KEYWORD`   | `"PRINT"`, `"LET"` | Reserved word (always uppercase)               |
| `BUILTIN_FN`| `"SIN"`, `"LOG"`   | One of the 11 built-in math functions          |
| `USER_FN`   | `"FNA"`, `"FNZ"`   | User-defined function (FN + one letter)        |
| `NAME`      | `"X"`, `"A1"`      | Variable name (one letter + optional digit)    |
| `PLUS`      | `"+"`              | Addition                                       |
| `MINUS`     | `"-"`              | Subtraction / unary negation                   |
| `STAR`      | `"*"`              | Multiplication                                 |
| `SLASH`     | `"/"`              | Division                                       |
| `CARET`     | `"^"`              | Exponentiation (right-associative)             |
| `EQ`        | `"="`              | Assignment (LET) or equality (IF)              |
| `LT`        | `"<"`              | Less than                                      |
| `GT`        | `">"`              | Greater than                                   |
| `LE`        | `"<="`             | Less-than-or-equal                             |
| `GE`        | `">="`             | Greater-than-or-equal                          |
| `NE`        | `"<>"`             | Not-equal                                      |
| `LPAREN`    | `"("`              | Open parenthesis                               |
| `RPAREN`    | `")"`              | Close parenthesis                              |
| `COMMA`     | `","`              | Print zone separator                           |
| `SEMICOLON` | `";"`              | Print tight separator                          |
| `NEWLINE`   | `"\n"` or `"\r\n"` | Statement terminator (significant!)            |
| `EOF`       | `""`               | End of input (always last)                     |
| `UNKNOWN`   | `"@"`              | Unrecognised character (error recovery)        |

## Design Notes

### Case Insensitivity

The grammar uses `@case_insensitive true`, which uppercases the entire source
before matching. This reflects the historical reality: 1964 Dartmouth BASIC
ran on teletypes with no lowercase keys. `print`, `Print`, and `PRINT` all
produce `KEYWORD("PRINT")`.

### LINE_NUM vs NUMBER

A bare integer serves two roles in BASIC:
- **Line label**: `10 LET X = 5` — the `10` names the line.
- **Expression value**: `LET X = 42` — the `42` is a number.
- **Jump target**: `GOTO 30` — the `30` is a branch destination.

The grammar file cannot distinguish these by pattern. The `relabelLineNumbers`
post-tokenize hook walks the completed token list and relabels: the first
integer on each line becomes `LINE_NUM`; all others stay `NUMBER`.

### REM Comments

`REM` (remark) introduces a comment that runs to the end of the line:

```
10 REM EVERYTHING AFTER REM IS IGNORED
20 LET X = 1  REM THIS PART IS ALSO IGNORED
```

Wait — in BASIC, REM is only valid as the sole statement on its line. You
cannot put code before or after REM on the same line. The lexer suppresses
everything after REM up to (but not including) the next NEWLINE.

### NEWLINE Is Significant

Unlike most languages where newlines are whitespace, BASIC's NEWLINE
terminates each statement. NEWLINE tokens are included in the output.

## Grammar File

The grammar is defined in `code/grammars/dartmouth_basic.tokens` at the
repository root. All language implementations of the BASIC lexer share
this single grammar file.

## Dependencies

| Package                               | Role                                         |
|---------------------------------------|----------------------------------------------|
| `@coding-adventures/grammar-tools`    | Parses `dartmouth_basic.tokens` grammar file |
| `@coding-adventures/lexer`            | Generic `GrammarLexer` engine                |
| `@coding-adventures/directed-graph`   | Transitive dependency of `lexer`             |
