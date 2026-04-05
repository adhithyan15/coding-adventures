# TypeScript 5.0 (2023) Parser

Parses TypeScript 5.0 source code into ASTs using the grammar-driven parser.

## Overview

TypeScript 5.0, released March 2023, adds standard TC39 decorators, ``const``
type parameters, the ``accessor`` keyword, and the ``satisfies`` operator on
top of the ES2022 class system.

This package is a thin wrapper around the generic ``GrammarParser``. It loads
the ``ts5.0.grammar`` file, tokenizes the source with the TS 5.0 lexer, and
produces an ``ASTNode`` tree rooted at ``program``.

## Key Grammar Rules

- ``program`` — top-level root node
- ``interface_declaration`` — TypeScript interface definitions
- ``type_alias_declaration`` — ``type Alias = ...`` declarations
- ``enum_declaration`` — TypeScript enums (``const`` and regular)
- ``ts_class_declaration`` — classes with optional decorators and type params
- ``type_parameters`` — generic parameter lists ``<T, U extends V>``
- ``using_declaration`` / ``await_using_declaration`` — TS 5.2 resource management
- ``variable_statement`` — ``var`` declarations

## Usage

```python
from typescript_ts50_parser import parse_ts50

ast = parse_ts50('interface Point { x: number; y: number; }')
print(ast.rule_name)  # "program"
```

## API

- ``parse_ts50(source: str) -> ASTNode`` — Parse source code, returns AST rooted at ``program``.
- ``create_ts50_parser(source: str) -> GrammarParser`` — Create a parser instance for advanced usage.

## Dependencies

- ``coding-adventures-grammar-tools`` — Parses ``.grammar`` files
- ``coding-adventures-parser`` — Provides ``GrammarParser`` engine
- ``coding-adventures-typescript-ts5.0-lexer`` — Tokenizes TS 5.0 source
