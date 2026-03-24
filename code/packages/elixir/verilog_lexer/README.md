# Verilog Lexer (Elixir)

Tokenizes Verilog HDL source code using the grammar-driven lexer engine, with an optional preprocessor for compiler directives.

## Usage

```elixir
# Basic tokenization
{:ok, tokens} = CodingAdventures.VerilogLexer.tokenize("module counter; endmodule")
# => [%Token{type: "KEYWORD", value: "module"}, %Token{type: "NAME", value: "counter"}, ...]

# With preprocessing (expands macros, evaluates conditionals)
source = """
`define WIDTH 8
wire [`WIDTH-1:0] bus;
"""
{:ok, tokens} = CodingAdventures.VerilogLexer.tokenize(source, preprocess: true)
```

## Preprocessor

The `VerilogLexer.Preprocessor` module handles Verilog compiler directives before tokenization:

- **`define / `undef** — simple and parameterized macro definition/removal
- **`ifdef / `ifndef / `else / `endif** — conditional compilation with nesting
- **`include** — file inclusion (stubbed: emits a comment placeholder)
- **`timescale** — time unit specification (stripped entirely)

```elixir
# Standalone preprocessor usage
processed = CodingAdventures.VerilogLexer.Preprocessor.process(source)
```

## How It Works

1. Reads `verilog.tokens` from the shared grammars directory
2. Optionally preprocesses the source (macro expansion, conditional compilation)
3. Delegates to `GrammarLexer.tokenize/2` for grammar-driven tokenization
4. Grammar is cached via `persistent_term` for fast repeated access

## Dependencies

- `grammar_tools` — parses `.tokens` files
- `lexer` — grammar-driven tokenization engine
