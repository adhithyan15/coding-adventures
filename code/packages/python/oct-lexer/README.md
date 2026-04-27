# oct-lexer

Tokenizes **Oct** source text using the grammar-driven lexer — a thin wrapper around `GrammarLexer` that loads `oct.tokens`.

**Oct** is a small, statically-typed, 8-bit systems programming language designed to compile to the Intel 8008 microprocessor (1972). The name comes from *octet* — the networking term for exactly 8 bits, the native word size of the 8008 ALU. Oct is the sister language to Nib (which targets the 4-bit Intel 4004).

## Overview

This package is part of the Oct compiler pipeline:

```
Oct source (.oct)
    ↓  [oct-lexer]        tokenise
    ↓  [oct-parser]       parse to AST
    ↓  [oct-type-checker] type check
    ↓  [oct-ir-compiler]  lower to compiler_ir
    ↓  [ir-to-intel-8008-compiler] code generate
    ↓  [intel-8008-assembler] assemble to binary
    ↓  [intel-8008-packager]  produce Intel HEX
```

`oct-lexer` occupies the first stage. It converts raw Oct source text into a flat list of `Token` objects, which `oct-parser` consumes.

## Usage

```python
from oct_lexer import tokenize_oct

tokens = tokenize_oct('let x: u8 = 0xFF;')
for tok in tokens:
    print(tok.type, repr(tok.value))
# let     'let'
# NAME    'x'
# COLON   ':'
# NAME    'u8'
# EQ      '='
# HEX_LIT '0xFF'
# SEMICOLON ';'
# EOF     ''
```

For full control (e.g. to call `.tokenize()` yourself):

```python
from oct_lexer import create_oct_lexer

lexer = create_oct_lexer('fn main() { }')
tokens = lexer.tokenize()
```

## Token Set

**Multi-character operators** (matched before single-character versions):

| Token | Value | Meaning |
|-------|-------|---------|
| `EQ_EQ` | `==` | Equality |
| `NEQ` | `!=` | Not-equal |
| `LEQ` | `<=` | Less-or-equal |
| `GEQ` | `>=` | Greater-or-equal |
| `LAND` | `&&` | Logical AND |
| `LOR` | `\|\|` | Logical OR |
| `ARROW` | `->` | Return type separator |

**Arithmetic & bitwise** (single-character):

| Token | Value | 8008 instruction |
|-------|-------|-----------------|
| `PLUS` | `+` | ADD r |
| `MINUS` | `-` | SUB r |
| `AMP` | `&` | ANA r |
| `PIPE` | `\|` | ORA r |
| `CARET` | `^` | XRA r |
| `TILDE` | `~` | XRI 0xFF |
| `BANG` | `!` | (logical NOT) |

**Literals**:

| Token | Examples | Notes |
|-------|---------|-------|
| `BIN_LIT` | `0b00001111` | Must precede `INT_LIT` |
| `HEX_LIT` | `0xFF` | Must precede `INT_LIT` |
| `INT_LIT` | `42`, `255` | Decimal; valid range 0–255 |
| `NAME` | `x`, `u8`, `bool` | Identifiers and type names |

**Keywords** (promoted from `NAME`):

- Control flow: `fn`, `let`, `static`, `if`, `else`, `while`, `loop`, `break`, `return`
- Boolean literals: `true`, `false`
- Intrinsics: `in`, `out`, `adc`, `sbb`, `rlc`, `rrc`, `ral`, `rar`, `carry`, `parity`

**Skipped silently**: whitespace (spaces, tabs, `\r`, `\n`) and line comments (`// …`).

## Intel 8008 Background

The Intel 8008 (1972) was Intel's first commercial 8-bit microprocessor:

- **8-bit accumulator** — values 0–255, arithmetic wraps modulo 256
- **7 registers** — A (accumulator), B, C, D, E (GP), H:L (memory pointer pair)
- **4 GP registers for locals** — B, C, D, E only; H:L reserved for memory addressing
- **8-level push-down call stack** — 7 usable levels (one always occupied by the PC)
- **16 KB address space** — 14-bit bus (0x0000–0x3FFF)
- **8 input ports + 24 output ports** — port number encoded in instruction opcode
- **4 flags** — CY (carry), Z (zero), S (sign), P (parity)

Oct exposes the carry flag via `carry()`, `adc()`, `sbb()`, and the four rotation intrinsics `rlc()`, `rrc()`, `ral()`, `rar()` directly in the language.

## Dependencies

- `coding-adventures-lexer` — `GrammarLexer` engine
- `coding-adventures-grammar-tools` — `parse_token_grammar`
- `coding-adventures-directed-graph`, `coding-adventures-state-machine` — transitive deps of `lexer`
