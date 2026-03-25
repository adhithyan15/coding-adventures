# @coding-adventures/verilog-lexer

Tokenizes Verilog (IEEE 1364-2005) source code using the grammar-driven lexer infrastructure, with a built-in preprocessor for `` `define ``/`` `ifdef ``/`` `include `` directives.

## Overview

This package is a thin wrapper around `@coding-adventures/lexer`. It loads the `verilog.tokens` grammar file and optionally runs the Verilog preprocessor as a pre-tokenize step.

## Usage

```typescript
import { tokenizeVerilog } from "@coding-adventures/verilog-lexer";

const tokens = tokenizeVerilog(`
  module and_gate(input a, input b, output y);
    assign y = a & b;
  endmodule
`);

for (const token of tokens) {
  console.log(`${token.type}: ${token.value}`);
}
```

### With Preprocessor

```typescript
// Preprocessor is enabled by default
const tokens = tokenizeVerilog(source, { preprocess: true });
```

## Dependencies

- `@coding-adventures/lexer` — Grammar-driven lexer engine
- `@coding-adventures/grammar-tools` — Token grammar file parser
- `@coding-adventures/state-machine` — DFA for tokenizer (transitive)
- `@coding-adventures/directed-graph` — Graph utilities (transitive)
