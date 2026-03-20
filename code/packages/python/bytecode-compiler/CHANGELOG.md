# Changelog

All notable changes to the bytecode-compiler package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-20

### Added
- `GenericCompiler` — pluggable compiler framework that language-specific compilers (e.g., Starlark compiler) can configure by registering AST rule handlers.
- `CompilerScope` — tracks compilation context (local variables, constants pool, names pool) for nested scopes such as function definitions.
- `CompilerError` — raised when the compiler encounters an AST node it cannot translate (e.g., an unregistered grammar rule or invalid operand).

## [0.1.0] - Unreleased

### Added
- Initial package scaffolding with pyproject.toml, src layout, and test structure
