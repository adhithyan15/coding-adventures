# coding-adventures-ecmascript-es5-lexer (Lua)

An ECMAScript 5 (2009) lexer that tokenizes ES5 source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `ecmascript/es5.tokens` grammar file.

## What ES5 adds over ES3

- `debugger` keyword (moved from future-reserved to keyword)
- Getter/setter syntax in object literals
- String line continuation
- Trailing commas in object literals

ES5 retains all ES3 features: strict equality, try/catch/finally/throw, instanceof, and regex literals.

## Usage

```lua
local es5_lexer = require("coding_adventures.ecmascript_es5_lexer")

local tokens = es5_lexer.tokenize("var x = 1;")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
ecmascript/es5.tokens  (code/grammars/)
    |  parsed by grammar_tools
TokenGrammar
    |  drives
GrammarLexer  (coding-adventures-lexer)
    |  wrapped by
ecmascript_es5_lexer  <-- you are here
```

## Dependencies

- `coding-adventures-grammar-tools` -- parses `es5.tokens`
- `coding-adventures-lexer` -- provides `GrammarLexer`
- `coding-adventures-state-machine` -- used internally by the lexer
- `coding-adventures-directed-graph` -- used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
