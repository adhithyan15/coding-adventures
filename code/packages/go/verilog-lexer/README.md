# verilog-lexer

A grammar-driven tokenizer for Verilog HDL (IEEE 1364-2005) source code, with a built-in preprocessor.

## Overview

This package tokenizes Verilog hardware description language source code using the `verilog.tokens` grammar file. It handles all Verilog token types including sized numbers, system identifiers, compiler directives, escaped identifiers, operators, keywords, and comments.

The package also includes a Verilog preprocessor that processes directives like `` `define ``, `` `ifdef ``, `` `include ``, and `` `timescale `` before tokenization.

## How It Fits in the Stack

```
verilog.tokens (grammar file)
        |
        v
grammar-tools (parses .tokens files)
        |
        v
lexer (GrammarLexer engine)
        |
        v
verilog-lexer (this package — loads grammar + preprocessor)
        |
        v
verilog-parser (future — consumes tokens)
```

## Usage

### Basic Tokenization (with preprocessing)

```go
import veriloglexer "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-lexer"

tokens, err := veriloglexer.TokenizeVerilog(`
    module and_gate(input a, input b, output y);
        assign y = a & b;
    endmodule
`)
```

### Raw Tokenization (without preprocessing)

```go
tokens, err := veriloglexer.TokenizeVerilogRaw(source)
```

### Using the Lexer Directly

```go
lex, err := veriloglexer.CreateVerilogLexer(source)
tokens := lex.Tokenize()
```

### Preprocessor Only

```go
preprocessed := veriloglexer.VerilogPreprocess(source)

// With predefined macros
preprocessed := veriloglexer.VerilogPreprocessWithDefines(source, map[string]string{
    "SIMULATION": "",
    "WIDTH":      "32",
})
```

## Token Types

The lexer recognizes all Verilog token types:

- **SIZED_NUMBER** — `4'b1010`, `8'hFF`, `32'd42`
- **REAL_NUMBER** — `3.14`, `1.5e-3`
- **NUMBER** — `42`, `1_000_000`
- **SYSTEM_ID** — `$display`, `$time`, `$finish`
- **DIRECTIVE** — `` `timescale ``, `` `define `` (in raw mode)
- **ESCAPED_IDENT** — `\my.odd.name`
- **STRING** — `"Hello, world!\n"`
- **KEYWORD** — `module`, `wire`, `reg`, `assign`, etc.
- **NAME** — regular identifiers
- **Operators** — `<<<`, `>>>`, `===`, `!==`, `&&`, `||`, `==`, `!=`, `<=`, `>=`, `**`, `->`, `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `~`, `!`, `<`, `>`, `=`, `?`, `:`
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `;`, `,`, `.`, `#`, `@`

## Preprocessor Directives

- `` `define NAME value `` — simple macro
- `` `define NAME(a, b) expr `` — parameterized macro
- `` `undef NAME `` — remove macro
- `` `ifdef NAME `` / `` `ifndef NAME `` — conditional compilation
- `` `else `` / `` `endif `` — conditional branches
- `` `include "file" `` — file inclusion (stubbed)
- `` `timescale unit/prec `` — stripped (no semantic meaning)

## Dependencies

- `lexer` — the GrammarLexer engine
- `grammar-tools` — parses `.tokens` grammar files
