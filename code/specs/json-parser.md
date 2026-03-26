# JSON Parser — Grammar-Driven Infrastructure Validation

## Overview

JSON (JavaScript Object Notation, RFC 8259) is a lightweight data-interchange format.
It is the simplest practical grammar we can target with our grammar-driven lexer/parser
infrastructure — no keywords, no operators, no comments, no indentation, no statement
versus expression distinction. If the infrastructure can parse JSON from just a
`.tokens` file and a `.grammar` file with zero code changes to the engine, we have
proven the architecture works.

This spec covers three things:

1. **The JSON grammar** — `json.tokens` and `json.grammar` files
2. **Python thin wrappers** — `json-lexer` and `json-parser` packages
3. **Elixir full infrastructure** — building the grammar-tools, lexer, and parser
   engines from scratch in Elixir, then using them to parse JSON

## Why JSON?

JSON is the ideal first validation target for the grammar-driven infrastructure:

- **Minimal complexity.** The entire grammar fits in 4 rules and 11 tokens.
- **Well-specified.** RFC 8259 is unambiguous — no dialects, no extensions, no edge
  cases left to interpretation.
- **Universally understood.** Every developer knows JSON. If our parser produces an
  AST for `{"name": "Ada", "age": 36}`, anyone can verify correctness by inspection.
- **No language-specific features needed.** No indentation mode, no keywords section,
  no reserved words. The simplest possible `.tokens` file.
- **Real-world utility.** Configuration files, API responses, and data serialization
  all use JSON. A grammar-driven JSON parser is immediately useful.

## Why Elixir?

Elixir is a functional language built on the Erlang VM (BEAM). It brings several
advantages that make it an excellent host for grammar-driven parsing:

- **Pattern matching.** Elixir's `case`/`with` blocks and function clause matching
  map naturally to recursive descent parsing.
- **Immutable data.** No mutable parser state to reason about — position and memo
  cache are threaded explicitly through function calls.
- **Tagged tuples.** Grammar elements like `{:sequence, elements}` and
  `{:alternation, choices}` are idiomatic Elixir.
- **Educational contrast.** Porting from Python (imperative, class-based) to Elixir
  (functional, module-based) illuminates how the same algorithms adapt to different
  paradigms.

## Layer Position

```
Grammar Files (.tokens, .grammar)
        |
Grammar Tools (parse token/parser grammars, validate, cross-check)
        |
Lexer (grammar-driven: .tokens -> token stream)
        |
Parser (grammar-driven: .grammar -> AST)
        |
[YOU ARE HERE: JSON — simplest real grammar target]
```

**Input:** JSON text (a string conforming to RFC 8259).
**Output:** A generic AST of `ASTNode` objects (same type used for Python, Ruby,
JavaScript, Starlark — truly language-agnostic).

## The JSON Grammar

### Tokens (json.tokens)

JSON has exactly 11 token types. There are no keywords, no identifiers, and no
operators. Every token is either a value literal or a structural delimiter.

| Token     | Pattern                        | Notes                              |
|-----------|--------------------------------|------------------------------------|
| STRING    | `/"([^"\\]\|\\["\\/bfnrt]\|\\u[0-9a-fA-F]{4})*"/` | Double-quoted, with escapes |
| NUMBER    | `/-?(?:0\|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/` | Full RFC 8259 number |
| TRUE      | `"true"`                       | Literal token, not a keyword       |
| FALSE     | `"false"`                      | Literal token, not a keyword       |
| NULL      | `"null"`                       | Literal token, not a keyword       |
| LBRACE    | `"{"`                          | Object open                        |
| RBRACE    | `"}"`                          | Object close                       |
| LBRACKET  | `"["`                          | Array open                         |
| RBRACKET  | `"]"`                          | Array close                        |
| COLON     | `":"`                          | Key-value separator                |
| COMMA     | `","`                          | Element separator                  |

Additionally, whitespace (space, tab, CR, LF) is defined in the `skip:` section
so it is consumed silently without producing tokens. JSON has no comments.

**Design decision — true/false/null as literal tokens:** In programming languages,
`true` is typically a keyword: the lexer matches the NAME pattern first, then
reclassifies the value as KEYWORD. JSON has no NAME token and no identifier concept,
so the keyword mechanism does not apply. Each value literal gets its own token type
(`TRUE`, `FALSE`, `NULL`) defined as literal string patterns.

**Design decision — negative numbers:** In JSON, `-42` is a single number, not a
MINUS operator followed by a NUMBER. The regex includes the optional leading `-` as
part of the NUMBER pattern. This differs from programming languages where `-` is a
separate operator.

### Grammar Rules (json.grammar)

The complete JSON grammar in EBNF:

```
value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
object = LBRACE [ pair { COMMA pair } ] RBRACE ;
pair   = STRING COLON value ;
array  = LBRACKET [ value { COMMA value } ] RBRACKET ;
```

That is the entire grammar — 4 rules. Some observations:

- **`value` is the start symbol.** Any JSON text is a single value.
- **Recursive structure.** `value` references `object` and `array`, which reference
  `value` again. This mutual recursion is what makes JSON able to represent nested
  structures of arbitrary depth.
- **The `[ x { COMMA x } ]` pattern.** This is the standard EBNF idiom for "zero
  or more comma-separated items." The outer `[ ]` makes the whole list optional
  (allowing `{}` and `[]`), while the inner `{ COMMA x }` handles additional items
  after the first.
- **No NEWLINE references.** The parser auto-detects that newlines are insignificant
  in this grammar. Combined with the skip pattern for whitespace, the parser never
  sees newline tokens.

## Infrastructure Enhancement

### Escape Sequence Processing

The current `GrammarLexer._process_escapes()` handles `\n`, `\t`, `\\`, and `\"`.
JSON requires additional escape sequences per RFC 8259 section 7:

| Escape | Character        | Currently handled? |
|--------|------------------|--------------------|
| `\"`   | Quotation mark   | Yes                |
| `\\`   | Reverse solidus  | Yes                |
| `\/`   | Solidus          | Yes (pass-through) |
| `\b`   | Backspace        | No (becomes "b")   |
| `\f`   | Form feed        | No (becomes "f")   |
| `\n`   | Line feed        | Yes                |
| `\r`   | Carriage return  | No (becomes "r")   |
| `\t`   | Tab              | Yes                |
| `\uXXXX` | Unicode char   | No                 |

Enhancement: add `\b`, `\f`, `\r`, `\/` to the escape map, and add `\uXXXX`
handling (consume 4 hex digits after `\u` and convert to the Unicode character).
This change is backward-compatible — previously unhandled escapes passed through
incorrectly, so fixing them improves all grammars.

## Python Packages

### json-lexer

A thin wrapper (~45 lines) that:
1. Reads `json.tokens` from `code/grammars/`
2. Creates a `GrammarLexer` configured for JSON
3. Exports `tokenize_json(source: str) -> list[Token]`

### json-parser

A thin wrapper (~45 lines) that:
1. Tokenizes input using `tokenize_json()`
2. Reads `json.grammar` from `code/grammars/`
3. Creates a `GrammarParser` and parses
4. Exports `parse_json(source: str) -> ASTNode`

## Elixir Packages

### grammar_tools

Full port of the Python `grammar_tools` package. Three modules:

- **TokenGrammar** — Parses `.tokens` files. Returns a `%TokenGrammar{}` struct with
  definitions, keywords, skip definitions, reserved keywords, mode, and aliases.
  Supports the full extended format (skip, aliases, reserved, mode directive).

- **ParserGrammar** — Parses `.grammar` files (EBNF notation). Uses tagged tuples
  for grammar elements: `{:rule_reference, name, is_token}`, `{:literal, value}`,
  `{:sequence, elements}`, `{:alternation, choices}`, `{:repetition, element}`,
  `{:optional, element}`, `{:group, element}`.

- **CrossValidator** — Validates that tokens referenced in the grammar exist in the
  token definitions, and reports unused tokens.

### lexer

Grammar-driven lexer engine. Key module:

- **GrammarLexer** — `tokenize(source, grammar) -> {:ok, tokens} | {:error, msg}`.
  Compiles patterns with Elixir's `Regex` module, uses first-match-wins semantics,
  handles skip patterns, keyword detection, alias resolution, and escape processing.
  Token struct: `%Token{type: String.t(), value: String.t(), line: integer(), column: integer()}`.

Implemented as a recursive function with state threading (position, line, column
passed as arguments) rather than mutable instance variables.

### parser

Grammar-driven parser engine. Key module:

- **GrammarParser** — `parse(tokens, grammar) -> {:ok, ast} | {:error, msg}`.
  Backtracking recursive descent with packrat memoization. State struct with `pos`
  and `memo` map threaded as `{result, state}` tuples. Auto-detects newline
  significance by scanning grammar rules for NEWLINE references.

AST node struct: `%ASTNode{rule_name: String.t(), children: list()}`.

### json_lexer and json_parser

Thin wrappers (same pattern as Python). Read grammar files relative to `__DIR__`,
parse with grammar_tools, feed to lexer/parser engines.

## Test Strategy

### Python json-lexer tests
- Primitive values: strings, numbers, booleans, null
- Number variants: integer, negative, decimal, exponent, negative exponent
- String escapes: `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`
- Structural tokens: `{}`, `[]`, `:`, `,`
- Whitespace handling: spaces, tabs, newlines, carriage returns between tokens
- Position tracking: line and column numbers
- Error cases: unterminated string, unexpected character

### Python json-parser tests
- Each value type produces correct AST node (rule_name = "value")
- Empty object `{}` and empty array `[]`
- Object with pairs: verify pair rule contains STRING COLON value
- Array with elements: verify correct nesting
- Nested structures: objects in arrays, arrays in objects, deep nesting
- Full RFC 8259 examples
- Error cases: missing colon, trailing comma, unclosed brace/bracket

### Elixir tests
Mirror the Python test cases for each package. Additionally:
- grammar_tools: test parsing of `.tokens` and `.grammar` files directly
- lexer: test the engine with inline grammar definitions (not just json.tokens)
- parser: test the engine with inline grammar definitions

### Coverage targets
- Libraries (grammar_tools, lexer, parser): 95%+
- Thin wrappers (json-lexer, json-parser): 80%+

## Future Extensions

- **JSON Schema validation** — validate AST against a JSON Schema
- **JSON Pointer / JSONPath** — query the AST
- **Streaming parser** — parse JSON incrementally for large files
- **Error recovery** — continue parsing after errors to report multiple issues
- **AST-to-value conversion** — walk the generic AST to produce native data structures
