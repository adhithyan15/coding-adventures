# coding-adventures-verilog-lexer

A Verilog HDL lexer for the coding-adventures project. This crate tokenizes Verilog (IEEE 1364-2005) source code using the grammar-driven lexer from the `lexer` crate, with an optional preprocessor for handling compiler directives.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `verilog.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of Verilog's tokens — keywords, identifiers, sized numbers, operators, system tasks, and delimiters — in a declarative format.

The crate also includes a preprocessor module that handles Verilog's C-like preprocessor directives (`define, `ifdef, `include, etc.) before tokenization.

## How it fits in the stack

```
verilog.tokens      (grammar file)
       |
       v
grammar-tools       (parses .tokens into TokenGrammar)
       |
       v
lexer               (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
verilog-lexer       (THIS CRATE: wires grammar + lexer + preprocessor together)
```

## Usage

```rust
use coding_adventures_verilog_lexer::{
    create_verilog_lexer, tokenize_verilog, tokenize_verilog_preprocessed
};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_verilog("module top; endmodule");

// With preprocessing — expands macros and conditionals first
let source = r#"`define WIDTH 8
reg [`WIDTH-1:0] data;"#;
let tokens = tokenize_verilog_preprocessed(source);

// Or get the lexer object for more control
let mut lexer = create_verilog_lexer("assign out = a & b;");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Preprocessor

The preprocessor module handles these directives:

| Directive | Description |
|-----------|-------------|
| `` `define NAME value `` | Simple text macro |
| `` `define NAME(a,b) body `` | Parameterized macro |
| `` `undef NAME `` | Remove a macro definition |
| `` `ifdef NAME `` | Conditional if defined |
| `` `ifndef NAME `` | Conditional if not defined |
| `` `else `` | Else branch |
| `` `endif `` | End conditional |
| `` `include "file" `` | Stubbed (comment placeholder) |
| `` `timescale ... `` | Stripped |

```rust
use coding_adventures_verilog_lexer::preprocessor::verilog_preprocess;

let source = r#"`define WIDTH 8
`ifdef USE_CACHE
wire cache_hit;
`endif
reg [`WIDTH-1:0] data;"#;

let processed = verilog_preprocess(source);
```

## Token types

The Verilog lexer produces these token categories:

- **NAME** — identifiers like `clk`, `data_in`, `_valid`
- **KEYWORD** — reserved words: `module`, `wire`, `reg`, `always`, `assign`, etc.
- **NUMBER** — plain integers: `42`, `0`, `1_000`
- **SIZED_NUMBER** — sized literals: `4'b1010`, `8'hFF`, `32'd42`
- **REAL_NUMBER** — floating-point: `3.14`, `1.5e-3`
- **STRING** — string literals: `"hello"`, `"value = %d"`
- **SYSTEM_ID** — system tasks: `$display`, `$finish`, `$time`
- **DIRECTIVE** — compiler directives: `` `define ``, `` `ifdef ``
- **ESCAPED_IDENT** — escaped identifiers: `\my.name`, `\bus[0]`
- **Operators** — `+`, `-`, `*`, `===`, `!==`, `<<<`, `>>>`, `&&`, `||`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `;`, `,`, `.`, `#`, `@`
- **EOF** — end of file
