# VHDL Lexer

Tokenizes VHDL source code using the grammar-driven lexer approach, with case-insensitive normalization.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It loads `vhdl.tokens` and delegates all tokenization to the generic engine.

What makes it unique is the **case normalization** post-processing step. VHDL is case-insensitive: `ENTITY`, `Entity`, and `entity` are all identical. After tokenization, any token with type `NAME` or `KEYWORD` has its value lowercased.

Unlike the Verilog lexer, VHDL has **no preprocessor**. All configuration in VHDL is done through first-class language constructs: `generic` parameters, `generate` statements, and `configuration` declarations.

## How It Fits in the Stack

```
vhdl.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
vhdl_lexer._normalize_case()        -- lowercases NAME/KEYWORD values
    |
    v
vhdl_lexer.tokenize_vhdl()          -- thin wrapper (this package)
```

## Usage

```python
from vhdl_lexer import tokenize_vhdl

tokens = tokenize_vhdl('''
    library ieee;
    use ieee.std_logic_1164.all;

    entity and_gate is
        port(a, b : in std_logic; y : out std_logic);
    end entity and_gate;

    architecture rtl of and_gate is
    begin
        y <= a and b;
    end architecture rtl;
''')
for token in tokens:
    print(token)
```

### Raw Lexer (No Normalization)

If you need the original case preserved, use `create_vhdl_lexer()`:

```python
from vhdl_lexer import create_vhdl_lexer

lexer = create_vhdl_lexer('ENTITY MyEntity IS END ENTITY MyEntity;')
tokens = lexer.tokenize()  # values preserve original case
```

## Key VHDL Lexical Features

- **Case insensitivity** — `ENTITY`, `Entity`, `entity` all normalized to `entity`
- **No preprocessor** — no `define`, `ifdef`, etc.
- **Character literals** — `'0'`, `'1'`, `'X'`, `'Z'` (std_logic values)
- **Bit string literals** — `B"1010"`, `X"FF"`, `O"77"`
- **Based literals** — `16#FF#`, `2#1010#`
- **Keyword operators** — `and`, `or`, `xor`, `not`, `mod`, `rem`, `sll`, `srl`
- **Extended identifiers** — `\my name\` (case-sensitive, unlike basic identifiers)
- **Doubled-quote escaping** — `"He said ""hello"""` (no backslash escaping)

## Dependencies

- `coding-adventures-lexer` — provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` — parses `.tokens` files
