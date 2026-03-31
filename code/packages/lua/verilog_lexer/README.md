# coding-adventures-verilog-lexer

A Lua package that tokenizes Verilog (IEEE 1364-2005) source code using the
grammar-driven lexer infrastructure from the coding-adventures monorepo.

## What is Verilog?

Verilog is a Hardware Description Language (HDL). Where software languages
describe sequential computations on a processor, Verilog describes physical
hardware structures — gates, wires, flip-flops — that exist simultaneously
and operate in parallel. A Verilog module is a blueprint for a hardware
component: you define its inputs, outputs, and internal logic, and a synthesis
tool converts that description into actual transistors on a chip.

## How it fits in the stack

```
verilog.tokens  (grammar definition)
      ↓
grammar_tools   (parse_token_grammar)
      ↓
lexer           (GrammarLexer)
      ↓
verilog_lexer   (this package — thin wrapper)
      ↓
Token stream    [{type="MODULE", value="module", line=1, col=1}, ...]
```

The package reads `code/grammars/verilog.tokens` once (cached) and delegates
all tokenization work to the grammar-driven `GrammarLexer`.

## Installation

```sh
luarocks make --local coding-adventures-verilog-lexer-0.1.0-1.rockspec
```

Dependencies (install first):
- `coding-adventures-state-machine`
- `coding-adventures-directed-graph`
- `coding-adventures-grammar-tools`
- `coding-adventures-lexer`

## Usage

```lua
local vl = require("coding_adventures.verilog_lexer")

local tokens = vl.tokenize("module adder(input a, input b, output y);")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
-- MODULE   module  1  1
-- NAME     adder   1  8
-- LPAREN   (       1  13
-- INPUT    input   1  14
-- NAME     a       1  20
-- ...
-- EOF              1  42
```

## Token types

### Keywords (promoted from NAME)
`MODULE`, `ENDMODULE`, `INPUT`, `OUTPUT`, `INOUT`, `WIRE`, `REG`,
`INTEGER`, `REAL`, `SIGNED`, `UNSIGNED`, `TRI`, `SUPPLY0`, `SUPPLY1`,
`ALWAYS`, `INITIAL`, `BEGIN`, `END`, `IF`, `ELSE`, `CASE`, `CASEX`,
`CASEZ`, `ENDCASE`, `DEFAULT`, `FOR`, `ASSIGN`, `DEFPARAM`, `PARAMETER`,
`LOCALPARAM`, `GENERATE`, `ENDGENERATE`, `GENVAR`, `POSEDGE`, `NEGEDGE`,
`OR`, `FUNCTION`, `ENDFUNCTION`, `TASK`, `ENDTASK`,
`AND`, `NAND`, `NOR`, `NOT`, `BUF`, `XOR`, `XNOR`

### Literal tokens
`SIZED_NUMBER` — sized literals like `4'b1010`, `8'hFF`, `32'd42`
`REAL_NUMBER`  — floating point like `3.14`, `1.5e-3`
`NUMBER`       — plain integers like `42`, `1_000`
`STRING`       — double-quoted string like `"hello\n"`
`SYSTEM_ID`    — system task/function like `$display`, `$time`
`DIRECTIVE`    — compiler directive like `` `define ``, `` `ifdef ``
`ESCAPED_IDENT` — escaped identifier like `\my.odd.name`
`NAME`         — regular identifier

### Operators (three-char, matched first)
`ARITH_LEFT_SHIFT` (`<<<`), `ARITH_RIGHT_SHIFT` (`>>>`),
`CASE_EQ` (`===`), `CASE_NEQ` (`!==`)

### Operators (two-char)
`LOGIC_AND` (`&&`), `LOGIC_OR` (`||`), `LEFT_SHIFT` (`<<`),
`RIGHT_SHIFT` (`>>`), `EQUALS_EQUALS` (`==`), `NOT_EQUALS` (`!=`),
`LESS_EQUALS` (`<=`), `GREATER_EQUALS` (`>=`), `POWER` (`**`),
`TRIGGER` (`->`)

### Operators (single-char)
`PLUS`, `MINUS`, `STAR`, `SLASH`, `PERCENT`, `AMP`, `PIPE`, `CARET`,
`TILDE`, `BANG`, `LESS_THAN`, `GREATER_THAN`, `EQUALS`, `QUESTION`, `COLON`

### Delimiters
`LPAREN`, `RPAREN`, `LBRACKET`, `RBRACKET`, `LBRACE`, `RBRACE`,
`SEMICOLON`, `COMMA`, `DOT`, `HASH`, `AT`

### Terminal
`EOF` — always the last token; value is `""`

## Running tests

```sh
cd tests && busted . --verbose --pattern=test_
```

## API

### `vl.tokenize(source)`

Tokenize a Verilog source string. Returns an array of token tables, each with:
- `type`  — token type string (e.g. `"MODULE"`, `"NAME"`, `"SIZED_NUMBER"`)
- `value` — matched text as it appeared in the source
- `line`  — 1-based line number
- `col`   — 1-based column number

The last token always has type `"EOF"`. Raises an error on unexpected input.

### `vl.get_grammar()`

Returns the cached `TokenGrammar` object. Useful for building custom
`GrammarLexer` instances or inspecting the grammar definitions.

### `vl.VERSION`

Version string: `"0.1.0"`.
