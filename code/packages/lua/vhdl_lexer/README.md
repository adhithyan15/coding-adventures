# coding-adventures-vhdl-lexer

A Lua package that tokenizes VHDL (IEEE 1076-2008) source code using the
grammar-driven lexer infrastructure from the coding-adventures monorepo.

## What is VHDL?

VHDL (VHSIC Hardware Description Language) was designed by the US Department
of Defense for documenting and simulating digital systems. Where Verilog is
terse and C-like, VHDL is verbose and Ada-like: strongly typed, explicitly
declared, and case-insensitive. A VHDL design separates the external interface
(`entity`) from the internal implementation (`architecture`), and uses
concurrent statements to describe hardware that runs in parallel.

**VHDL is case-insensitive**: `ENTITY`, `Entity`, and `entity` all refer to
the same thing. The `vhdl.tokens` grammar sets `case_sensitive: false`, so
the lexer lowercases all input before matching. All token values are lowercase.

## How it fits in the stack

```
vhdl.tokens     (grammar definition)
      ↓
grammar_tools   (parse_token_grammar)
      ↓
lexer           (GrammarLexer)
      ↓
vhdl_lexer      (this package — thin wrapper)
      ↓
Token stream    [{type="ENTITY", value="entity", line=1, col=1}, ...]
```

## Installation

```sh
luarocks make --local coding-adventures-vhdl-lexer-0.1.0-1.rockspec
```

Dependencies (install first):
- `coding-adventures-state-machine`
- `coding-adventures-directed-graph`
- `coding-adventures-grammar-tools`
- `coding-adventures-lexer`

## Usage

```lua
local vh = require("coding_adventures.vhdl_lexer")

local tokens = vh.tokenize("entity adder is port (a : in std_logic);")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
-- ENTITY    entity   1  1
-- NAME      adder    1  8
-- IS        is       1  14
-- PORT      port     1  17
-- ...
-- EOF                1  42
```

## Token types

### Keywords (promoted from NAME, all lowercase)
`ABS`, `ACCESS`, `AFTER`, `ALIAS`, `ALL`, `AND`, `ARCHITECTURE`, `ARRAY`,
`ASSERT`, `ATTRIBUTE`, `BEGIN`, `BLOCK`, `BODY`, `BUFFER`, `BUS`, `CASE`,
`COMPONENT`, `CONFIGURATION`, `CONSTANT`, `DISCONNECT`, `DOWNTO`, `ELSE`,
`ELSIF`, `END`, `ENTITY`, `EXIT`, `FILE`, `FOR`, `FUNCTION`, `GENERATE`,
`GENERIC`, `GROUP`, `GUARDED`, `IF`, `IMPURE`, `IN`, `INOUT`, `IS`,
`LABEL`, `LIBRARY`, `LINKAGE`, `LITERAL`, `LOOP`, `MAP`, `MOD`, `NAND`,
`NEW`, `NEXT`, `NOR`, `NOT`, `NULL`, `OF`, `ON`, `OPEN`, `OR`, `OTHERS`,
`OUT`, `PACKAGE`, `PORT`, `POSTPONED`, `PROCEDURE`, `PROCESS`, `PURE`,
`RANGE`, `RECORD`, `REGISTER`, `REJECT`, `REM`, `REPORT`, `RETURN`, `ROL`,
`ROR`, `SELECT`, `SEVERITY`, `SIGNAL`, `SHARED`, `SLA`, `SLL`, `SRA`,
`SRL`, `SUBTYPE`, `THEN`, `TO`, `TRANSPORT`, `TYPE`, `UNAFFECTED`, `UNITS`,
`UNTIL`, `USE`, `VARIABLE`, `WAIT`, `WHEN`, `WHILE`, `WITH`, `XNOR`, `XOR`

### Literal tokens
`BASED_LITERAL` — e.g. `16#FF#`, `2#1010#`, `8#77#`
`REAL_NUMBER`   — e.g. `3.14`, `1.0E-3`
`NUMBER`        — plain integers like `42`, `1_000`
`STRING`        — double-quoted string (use `""` for embedded quote)
`BIT_STRING`    — prefix + quoted value: `X"FF"`, `B"1010"`, `O"77"`
`CHAR_LITERAL`  — std_logic character: `'0'`, `'1'`, `'X'`, `'Z'`
`EXTENDED_IDENT` — backslash-delimited: `\my odd name\`
`NAME`          — regular identifier

### Two-character operators
`VAR_ASSIGN` (`:=`), `LESS_EQUALS` (`<=`), `GREATER_EQUALS` (`>=`),
`ARROW` (`=>`), `NOT_EQUALS` (`/=`), `POWER` (`**`), `BOX` (`<>`)

### Single-character operators
`PLUS`, `MINUS`, `STAR`, `SLASH`, `AMPERSAND`,
`LESS_THAN`, `GREATER_THAN`, `EQUALS`, `TICK`, `PIPE`

### Delimiters
`LPAREN`, `RPAREN`, `LBRACKET`, `RBRACKET`, `SEMICOLON`, `COMMA`, `DOT`, `COLON`

### Terminal
`EOF` — always the last token; value is `""`

## Running tests

```sh
cd tests && busted . --verbose --pattern=test_
```

## API

### `vh.tokenize(source)`

Tokenize a VHDL source string. Returns an array of token tables, each with:
- `type`  — token type string (e.g. `"ENTITY"`, `"NAME"`, `"BIT_STRING"`)
- `value` — matched text, lowercased (due to `case_sensitive: false`)
- `line`  — 1-based line number
- `col`   — 1-based column number

The last token always has type `"EOF"`. Raises an error on unexpected input.

### `vh.get_grammar()`

Returns the cached `TokenGrammar` object.

### `vh.VERSION`

Version string: `"0.1.0"`.
