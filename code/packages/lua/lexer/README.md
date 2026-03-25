# lexer

Tokenizer breaking source code into tokens: keywords, identifiers, numbers, operators.

## Layer 2

This package is part of Layer 2 of the coding-adventures computing stack.

Ported from the Go implementation at `code/packages/go/lexer/`.

## Overview

The lexer package provides two tokenizers that convert raw source text into typed Token objects:

### Hand-Written Lexer

Character-by-character tokenizer with a DFA-driven dispatch loop. Best for simple languages with a fixed set of token types.

```lua
local lexer = require("coding_adventures.lexer")

local lex = lexer.Lexer.new("x = 1 + 2", { keywords = { "if", "else" } })
local tokens = lex:tokenize()

for _, tok in ipairs(tokens) do
    print(tok)  -- Token(Name, "x", 1:1)
end
```

### Grammar-Driven Lexer

Regex-based tokenizer driven by a grammar table. Supports skip patterns, aliases, indentation mode, pattern groups, and on-token callbacks.

```lua
local lexer = require("coding_adventures.lexer")

local grammar = {
    definitions = {
        { name = "NAME",   pattern = "[a-zA-Z_]+", is_regex = true },
        { name = "NUMBER", pattern = "[0-9]+",      is_regex = true },
        { name = "PLUS",   pattern = "+",            is_regex = false },
    },
    keywords = { "if", "def", "return" },
    skip_definitions = {
        { name = "WS", pattern = "[ \t]+", is_regex = true },
    },
}

local gl = lexer.GrammarLexer.new("def foo 42", grammar)
local tokens = gl:tokenize()
```

## Features

- **Token types**: 23 built-in types (Name, Number, String, Keyword, operators, delimiters, Newline, EOF)
- **Escape sequences**: `\n`, `\t`, `\\`, `\"` in string literals
- **Keyword promotion**: Identifiers matching configured keywords become Keyword tokens
- **Position tracking**: Every token records its line and column
- **Indentation mode**: Emits INDENT/DEDENT tokens for Python-style languages
- **Pattern groups**: Context-sensitive lexing via stackable pattern groups
- **On-token callbacks**: Hook into token emission for group transitions, token rewriting

## Dependencies

- state-machine (for the tokenizer dispatch DFA)
- grammar-tools (for TokenGrammar definitions, when available)

## Development

```bash
# Run tests
cd tests && busted . --verbose --pattern=test_
```
