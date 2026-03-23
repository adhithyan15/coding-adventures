# grammar-tools

Grammar definition and manipulation — declarative specifications for lexers and parsers.

## What it does

This package parses and validates two kinds of grammar files:

1. **Token grammars** (`.tokens` files) — define the lexical tokens a lexer recognizes: identifiers, numbers, operators, string literals, etc.

2. **Parser grammars** (`.grammar` files) — define the syntactic structure of a language using EBNF-like rules that reference the tokens from (1).

It also **cross-validates** token and parser grammars to ensure consistency: every token the parser references must be defined, and every token defined should ideally be used somewhere.

## How it fits in the stack

This sits between the grammar specification files and the lexer/parser generators. The flow is:

```
.tokens file  ──→  parse_token_grammar()  ──→  TokenGrammar
                                                    │
.grammar file ──→  parse_parser_grammar() ──→  ParserGrammar
                                                    │
                   cross_validate() ←───────────────┘
                        │
                   issues list (errors + warnings)
```

The lexer package uses `TokenGrammar` to know what tokens to recognize. The parser package uses `ParserGrammar` to know what production rules to follow.

## Usage

```lua
local grammar_tools = require("coding_adventures.grammar_tools")

-- Parse a .tokens file
local token_grammar, err = grammar_tools.parse_token_grammar([[
NAME = /[a-zA-Z_]+/
NUMBER = /[0-9]+/
PLUS = "+"
keywords:
  if
  else
skip:
  WHITESPACE = /[ \t]+/
]])

-- Validate it
local issues = grammar_tools.validate_token_grammar(token_grammar)

-- Parse a .grammar file
local parser_grammar, err = grammar_tools.parse_parser_grammar([[
expression = term { ( PLUS | MINUS ) term } ;
term = NUMBER | NAME ;
]])

-- Validate with token names
local issues = grammar_tools.validate_parser_grammar(
    parser_grammar,
    token_grammar:token_names()
)

-- Cross-validate the pair
local issues = grammar_tools.cross_validate(token_grammar, parser_grammar)
```

## Port lineage

This is a Lua 5.4 port of the Go implementation at `code/packages/go/grammar-tools/`. The Go version is the reference implementation.

## Development

```bash
# Run tests
bash BUILD
```
