# coding-adventures-vhdl-lexer

A VHDL (IEEE 1076-2008) lexer for the coding-adventures project. This crate tokenizes VHDL source code using the grammar-driven lexer from the `lexer` crate, with automatic case normalization for identifiers and keywords.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `vhdl.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of VHDL's tokens — keywords, identifiers, bit strings, based literals, character literals, operators, and delimiters — in a declarative format.

After tokenization, the crate applies a post-processing step: all NAME and KEYWORD token values are lowercased. This implements VHDL's case-insensitive semantics — `ENTITY`, `Entity`, and `entity` all become `entity`.

Unlike the Verilog lexer, VHDL has no preprocessor. What you write is what gets compiled.

## How it fits in the stack

```
vhdl.tokens         (grammar file)
       |
       v
grammar-tools       (parses .tokens into TokenGrammar)
       |
       v
lexer               (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
vhdl-lexer          (THIS CRATE: wires grammar + lexer + case normalization)
```

## Usage

```rust
use coding_adventures_vhdl_lexer::{create_vhdl_lexer, tokenize_vhdl};

// Quick tokenization — returns a Vec<Token> with lowercased identifiers
let tokens = tokenize_vhdl("ENTITY top IS END ENTITY top;");

// Or get the lexer object for more control (no case normalization applied)
let mut lexer = create_vhdl_lexer("signal clk : std_logic;");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Case insensitivity

VHDL is case-insensitive. The lexer normalizes all NAME and KEYWORD token values to lowercase after tokenization:

```
Input:    SIGNAL Clk : STD_LOGIC;
Tokens:   KEYWORD("signal") NAME("clk") ... NAME("std_logic") ...
```

Extended identifiers (`\Like This\`) are NOT normalized — they preserve their original case, as required by the VHDL standard.

## Token types

The VHDL lexer produces these token categories:

- **NAME** — identifiers like `clk`, `data_in`, `std_logic` (always lowercased)
- **KEYWORD** — reserved words: `entity`, `architecture`, `signal`, `process`, etc. (always lowercased)
- **NUMBER** — plain integers: `42`, `0`, `1_000_000`
- **REAL_NUMBER** — floating-point: `3.14`, `1.0E-3`
- **BASED_LITERAL** — based numbers: `16#FF#`, `2#1010#`, `8#77#`
- **BIT_STRING** — bit string literals: `B"1010"`, `X"FF"`, `O"77"`
- **CHAR_LITERAL** — character literals: `'0'`, `'1'`, `'X'`, `'Z'`
- **STRING** — string literals: `"hello"`, `"He said ""hi"""`
- **EXTENDED_IDENT** — extended identifiers: `\my name\`, `\VHDL-2008\`
- **Operators** — `:=`, `<=`, `=>`, `/=`, `**`, `<>`, `+`, `-`, `*`, `/`, `&`, `=`, `<`, `>`, `'`, `|`
- **Delimiters** — `(`, `)`, `[`, `]`, `;`, `,`, `.`, `:`
- **EOF** — end of file
