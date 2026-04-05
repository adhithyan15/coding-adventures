# TypeScript 5.8 (2025) Parser

Parses TypeScript 5.8 source code into ASTs using the grammar-driven parser.

## Overview

TypeScript 5.8, released February 2025, targets the ES2025 baseline. ES2025
standardizes TC39 decorators, import attributes (``with { type: "json" }``),
and explicit resource management (``using`` / ``await using``). TS 5.8 adds
``export type *`` re-exports and ``--erasableSyntaxOnly`` mode.

This package is a thin wrapper around the generic ``GrammarParser``. It loads
the ``ts5.8.grammar`` file, tokenizes the source with the TS 5.8 lexer, and
produces an ``ASTNode`` tree rooted at ``program``.

## Key Grammar Rules (TS 5.8 additions)

- ``using_declaration`` — ``using x = resource();`` (ES2025)
- ``await_using_declaration`` — ``await using db = await connect();``
- ``import_attributes`` — ``with { type: "json" }`` on imports
- ``ambient_module_declaration`` — ``module "specifier" { ... }``
- ``export type *`` — re-export all types from a module (TS 5.8)
- ``program`` — optionally begins with HASHBANG token (ES2025)

## Usage

```python
from typescript_ts58_parser import parse_ts58

ast = parse_ts58('using conn = openConnection();')
print(ast.rule_name)  # "program"
```

## API

- ``parse_ts58(source: str) -> ASTNode`` — Parse source code, returns AST rooted at ``program``.
- ``create_ts58_parser(source: str) -> GrammarParser`` — Create a parser instance for advanced usage.

## Dependencies

- ``coding-adventures-grammar-tools`` — Parses ``.grammar`` files
- ``coding-adventures-parser`` — Provides ``GrammarParser`` engine
- ``coding-adventures-typescript-ts5.8-lexer`` — Tokenizes TS 5.8 source
