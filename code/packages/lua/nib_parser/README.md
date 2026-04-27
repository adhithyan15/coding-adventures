# coding-adventures-starlark-parser

Parses Starlark source code into an Abstract Syntax Tree (AST) using the
grammar-driven parser engine.

## What is Starlark?

Starlark is a deterministic subset of Python used for configuration files,
most famously in [Bazel](https://bazel.build/) BUILD files. It looks like
Python but with important constraints that guarantee termination and
deterministic evaluation:

- No `while` loops (all iteration is over finite collections)
- No classes or class definitions
- No `try`/`except`/`raise`
- No `global`/`nonlocal`
- Recursion is disabled (functions cannot call themselves)

These constraints make Starlark safe for build systems: every file terminates,
and repeated evaluation always produces the same result.

## Architecture

```
starlark source
    ↓
starlark_lexer.tokenize()   →  flat token stream (with INDENT/DEDENT/NEWLINE)
    ↓
grammar_tools.parse_parser_grammar("starlark.grammar")  →  ParserGrammar
    ↓
parser.GrammarParser.new(tokens, grammar):parse()  →  AST root
```

The root node has `rule_name == "file"` (matching the first rule in
`code/grammars/starlark.grammar`).

## Usage

```lua
local starlark_parser = require("coding_adventures.starlark_parser")

-- Parse a simple assignment
local ast = starlark_parser.parse("x = 1\n")
print(ast.rule_name)  -- "file"

-- Parse a BUILD file rule
local build_ast = starlark_parser.parse('cc_library(name="foo", srcs=["foo.cc"])\n')

-- Parse a function definition
local func_ast = starlark_parser.parse([[
def greet(name, greeting="Hello"):
    return greeting + ", " + name
]])

-- Inspect grammar rules
local g = starlark_parser.get_grammar()
print(g.rules[1].name)  -- "file"

-- Create a parser for manual control
local p = starlark_parser.create_parser("x = 1\n")
local ast, err = p:parse()
```

## API

### `starlark_parser.parse(source)`

Tokenize `source` and return the root ASTNode. The root has
`rule_name == "file"`. Raises an error on parse failure.

### `starlark_parser.create_parser(source)`

Tokenize `source` and return a `GrammarParser` instance without parsing.
Call `:parse()` on the result to get `(ast, err)`.

### `starlark_parser.get_grammar()`

Return the cached (or freshly loaded) `ParserGrammar` for Starlark.

## Grammar

The grammar is at `code/grammars/starlark.grammar`. Key rules:

| Rule | Description |
|------|-------------|
| `file` | Top-level: zero or more statements |
| `statement` | compound or simple statement |
| `simple_stmt` | small statements separated by `;` |
| `assign_stmt` | assignment or expression statement |
| `def_stmt` | function definition |
| `if_stmt` | if/elif/else |
| `for_stmt` | for loop (no while!) |
| `load_stmt` | import symbols from another `.star` file |
| `expression` | full expression (lambda, ternary, or `or_expr`) |
| `lambda_expr` | `lambda params: expr` |

## Dependencies

- `coding-adventures-starlark-lexer` — tokenizes Starlark source
- `coding-adventures-parser` — grammar-driven `GrammarParser` engine
- `coding-adventures-grammar-tools` — loads and compiles `.grammar` files

## Stack position

```
starlark_parser      ← this package
├── starlark_lexer
├── parser
│   └── grammar_tools
└── grammar_tools
    ├── lexer
    │   └── state_machine
    └── directed_graph
```

## License

MIT
