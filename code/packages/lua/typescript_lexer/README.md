# coding-adventures-typescript-lexer (Lua)

A TypeScript lexer that tokenizes TypeScript source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `typescript.tokens` grammar file.

TypeScript is a strict superset of JavaScript. This lexer recognizes all JavaScript tokens plus TypeScript-specific keywords.

## What it does

Given the input `interface Foo { x: number; }`, the lexer produces:

| # | Type      | Value       |
|---|-----------|-------------|
| 1 | INTERFACE | `interface` |
| 2 | NAME      | `Foo`       |
| 3 | LBRACE    | `{`         |
| 4 | NAME      | `x`         |
| 5 | COLON     | `:`         |
| 6 | NUMBER    | `number`    |
| 7 | SEMICOLON | `;`         |
| 8 | RBRACE    | `}`         |
| 9 | EOF       |             |

Whitespace is silently consumed (declared as a skip pattern in `typescript.tokens`).

## Token types

### Inherited from JavaScript
All JavaScript tokens are recognized: NAME, NUMBER (literal), STRING (literal), LET, CONST, VAR, IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN, CLASS, IMPORT, EXPORT, FROM, AS, NEW, THIS, TYPEOF, INSTANCEOF, TRUE, FALSE, NULL, UNDEFINED, and all operators and delimiters.

### TypeScript-specific keywords
| Token type  | Keyword      |
|-------------|--------------|
| INTERFACE   | `interface`  |
| TYPE        | `type`       |
| ENUM        | `enum`       |
| NAMESPACE   | `namespace`  |
| DECLARE     | `declare`    |
| READONLY    | `readonly`   |
| PUBLIC      | `public`     |
| PRIVATE     | `private`    |
| PROTECTED   | `protected`  |
| ABSTRACT    | `abstract`   |
| IMPLEMENTS  | `implements` |
| EXTENDS     | `extends`    |
| KEYOF       | `keyof`      |
| INFER       | `infer`      |
| NEVER       | `never`      |
| UNKNOWN     | `unknown`    |
| ANY         | `any`        |
| VOID        | `void`       |
| NUMBER      | `number`     |
| STRING      | `string`     |
| BOOLEAN     | `boolean`    |
| OBJECT      | `object`     |
| SYMBOL      | `symbol`     |
| BIGINT      | `bigint`     |

Note: `number`, `string`, `boolean`, etc. appear as keyword token types (e.g., `NUMBER`, `STRING`) because they are reserved TypeScript type names. A number *literal* like `42` also produces a `NUMBER` token — the distinction is in the `.value` field (`"number"` vs `"42"`).

## Usage

```lua
local ts_lexer = require("coding_adventures.typescript_lexer")

local tokens = ts_lexer.tokenize("interface Foo { x: number }")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
typescript.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
typescript_lexer  ← you are here
    ↓  feeds
typescript_parser  (future)
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `typescript.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
