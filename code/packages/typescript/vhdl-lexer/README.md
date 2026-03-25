# @coding-adventures/vhdl-lexer

Tokenizes VHDL (IEEE 1076-2008) source code using the grammar-driven lexer infrastructure, with case normalization for identifiers and keywords.

## Overview

This package is a thin wrapper around `@coding-adventures/lexer`. It loads the `vhdl.tokens` grammar file and applies VHDL-specific post-processing: lowercasing all NAME and KEYWORD token values to reflect VHDL's case-insensitive nature.

Unlike the Verilog lexer, this package has **no preprocessor** — VHDL handles code reuse and conditional compilation through its language-level library/package/generic system.

## Usage

```typescript
import { tokenizeVhdl } from "@coding-adventures/vhdl-lexer";

const tokens = tokenizeVhdl(`
  library ieee;
  use ieee.std_logic_1164.all;

  entity and_gate is
    port(
      a, b : in  std_logic;
      y    : out std_logic
    );
  end entity and_gate;

  architecture rtl of and_gate is
  begin
    y <= a and b;
  end architecture rtl;
`);

for (const token of tokens) {
  console.log(`${token.type}: ${token.value}`);
}
```

### Case Normalization

VHDL is case-insensitive, so `ENTITY`, `Entity`, and `entity` are all the same keyword. The `tokenizeVhdl` function normalizes NAME and KEYWORD values to lowercase:

```typescript
const tokens = tokenizeVhdl("ENTITY My_Chip IS END ENTITY;");
// KEYWORD("entity"), NAME("my_chip"), KEYWORD("is"),
// KEYWORD("end"), KEYWORD("entity"), SEMICOLON(";"), EOF("")
```

String literals, bit strings, and character literals are NOT normalized — they preserve their original casing.

## Dependencies

- `@coding-adventures/lexer` — Grammar-driven lexer engine
- `@coding-adventures/grammar-tools` — Token grammar file parser
- `@coding-adventures/state-machine` — DFA for tokenizer (transitive)
- `@coding-adventures/directed-graph` — Graph utilities (transitive)
