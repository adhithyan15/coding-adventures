# vhdl-lexer

A grammar-driven tokenizer for VHDL (IEEE 1076-2008) source code with automatic case normalization.

## Overview

This package tokenizes VHDL hardware description language source code using the `vhdl.tokens` grammar file. It handles all VHDL token types including character literals, bit strings, based literals, extended identifiers, keyword operators, and comments.

Unlike the Verilog lexer, this package has no preprocessor — VHDL has no preprocessor directives. Configuration and conditional compilation are handled through language features (generics, generate statements, configurations).

All NAME and KEYWORD token values are automatically normalized to lowercase, reflecting VHDL's case-insensitive semantics.

## How It Fits in the Stack

```
vhdl.tokens (grammar file)
        |
        v
grammar-tools (parses .tokens files)
        |
        v
lexer (GrammarLexer engine)
        |
        v
vhdl-lexer (this package — loads grammar + case normalization)
        |
        v
vhdl-parser (future — consumes tokens)
```

## Usage

### Basic Tokenization

```go
import vhdllexer "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-lexer"

tokens, err := vhdllexer.TokenizeVhdl(`
    entity and_gate is
        port (
            a, b : in  std_logic;
            y    : out std_logic
        );
    end entity and_gate;
`)
// [Token(KEYWORD, "entity"), Token(NAME, "and_gate"), Token(KEYWORD, "is"), ...]
```

### Using the Lexer Directly

```go
lex, err := vhdllexer.CreateVhdlLexer(source)
tokens := lex.Tokenize()
// Note: case normalization is NOT applied when using CreateVhdlLexer directly.
// Use TokenizeVhdl for automatic normalization.
```

## Token Types

The lexer recognizes all VHDL token types:

- **STRING** — `"Hello, World!"`, `"He said ""hello"""`
- **BIT_STRING** — `B"1010"`, `O"77"`, `X"FF"`, `D"42"`
- **CHAR_LITERAL** — `'0'`, `'1'`, `'X'`, `'Z'`
- **BASED_LITERAL** — `16#FF#`, `2#1010#`, `8#77#E2`
- **REAL_NUMBER** — `3.14`, `1.0E-3`
- **NUMBER** — `42`, `1_000_000`
- **EXTENDED_IDENT** — `\my odd name\`
- **NAME** — regular identifiers (lowercased)
- **KEYWORD** — `entity`, `architecture`, `signal`, `process`, etc. (lowercased)
- **Two-char operators** — `:=`, `<=`, `>=`, `=>`, `/=`, `**`, `<>`
- **Single-char operators** — `+`, `-`, `*`, `/`, `&`, `<`, `>`, `=`, `'`, `|`
- **Delimiters** — `(`, `)`, `[`, `]`, `;`, `,`, `.`, `:`

## Case Insensitivity

VHDL is case-insensitive: `ENTITY`, `Entity`, and `entity` are all the same keyword. The lexer normalizes all NAME and KEYWORD values to lowercase automatically:

```go
tokens, _ := vhdllexer.TokenizeVhdl("SIGNAL MyCounter : INTEGER;")
// tokens[0].Value == "signal"     (keyword, lowercased)
// tokens[1].Value == "mycounter"  (name, lowercased)
// tokens[3].Value == "integer"    (keyword, lowercased)
```

## Keyword Operators

VHDL uses keyword operators for logical and arithmetic operations:

| Category | Keywords |
|----------|----------|
| Logical  | `and`, `or`, `nand`, `nor`, `xor`, `xnor`, `not` |
| Shift    | `sll`, `srl`, `sla`, `sra`, `rol`, `ror` |
| Arithmetic | `mod`, `rem`, `abs` |

## Dependencies

- `lexer` — the GrammarLexer engine
- `grammar-tools` — parses `.tokens` grammar files
