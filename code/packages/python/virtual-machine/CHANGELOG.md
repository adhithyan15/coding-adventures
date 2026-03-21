# Changelog

All notable changes to the virtual-machine package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-20

### Added
- `GenericVM` — pluggable virtual machine framework that language-specific VMs (e.g., Starlark VM) can configure by registering opcode handlers and built-in functions.
- `BuiltinFunction` — wrapper type for built-in functions that can be registered with `GenericVM` and called from bytecode.
- `MaxRecursionError` — raised when the call stack exceeds the configured recursion limit.
- `VMTypeError` — raised when an opcode handler encounters a type mismatch (e.g., adding a string to an int without promotion).

## [0.1.0] - Unreleased

### Added
- Initial package scaffolding with pyproject.toml, src layout, and test structure
