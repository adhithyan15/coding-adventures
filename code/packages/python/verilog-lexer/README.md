# Verilog Lexer

Tokenizes Verilog HDL source code using the grammar-driven lexer approach, with a C-like preprocessor hook.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It loads `verilog.tokens` and delegates all tokenization to the generic engine.

What makes it unique is the **preprocessor hook** — a `pre_tokenize` transform that processes Verilog compiler directives (`` `define ``, `` `ifdef ``, `` `include ``) before the lexer sees the source text. This is the same pattern used for C preprocessor handling in the hooks spec.

## How It Fits in the Stack

```
verilog.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |  + pre_tokenize hook:
    |    verilog_preprocess()         -- expands macros, evaluates conditionals
    |
    v
verilog_lexer.tokenize_verilog()     -- thin wrapper (this package)
```

## Usage

```python
from verilog_lexer import tokenize_verilog

tokens = tokenize_verilog('''
    `define WIDTH 8
    module adder(input [`WIDTH-1:0] a, input [`WIDTH-1:0] b, output [`WIDTH-1:0] sum);
        assign sum = a + b;
    endmodule
''')
for token in tokens:
    print(token)
```

## Preprocessor

The preprocessor supports:
- `` `define NAME value `` — simple text macros
- `` `define NAME(a, b) expr `` — parameterized macros
- `` `ifdef / `ifndef / `else / `endif `` — conditional compilation
- `` `undef NAME `` — undefine a macro
- `` `include "file" `` — stubbed (emits a comment)
- `` `timescale `` — stripped

Disable preprocessing with `preprocess=False`:

```python
tokens = tokenize_verilog(source, preprocess=False)
```

## Dependencies

- `coding-adventures-lexer` — provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` — parses `.tokens` files
