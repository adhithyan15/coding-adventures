# coding-adventures-ecmascript-es3-lexer (Lua)

An ECMAScript 3 (1999) lexer that tokenizes ES3 source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `ecmascript/es3.tokens` grammar file.

## What ES3 adds over ES1

- `===` and `!==` (strict equality -- no type coercion)
- `try`/`catch`/`finally`/`throw` (structured error handling)
- Regular expression literals (`/pattern/flags`)
- `instanceof` operator
- 28 keywords total (5 new: catch, finally, instanceof, throw, try)

## Usage

```lua
local es3_lexer = require("coding_adventures.ecmascript_es3_lexer")

local tokens = es3_lexer.tokenize("var x = 1;")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
ecmascript/es3.tokens  (code/grammars/)
    |  parsed by grammar_tools
TokenGrammar
    |  drives
GrammarLexer  (coding-adventures-lexer)
    |  wrapped by
ecmascript_es3_lexer  <-- you are here
```

## Dependencies

- `coding-adventures-grammar-tools` -- parses `es3.tokens`
- `coding-adventures-lexer` -- provides `GrammarLexer`
- `coding-adventures-state-machine` -- used internally by the lexer
- `coding-adventures-directed-graph` -- used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
