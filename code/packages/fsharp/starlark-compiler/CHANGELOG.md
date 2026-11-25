# Changelog

All notable changes to `CodingAdventures.StarlarkCompiler.FSharp` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Pure F# Starlark bytecode opcode metadata provider conforming to STP01 specification.
- Full 61 opcode definitions and 12 opcode categories.
- 34 operator mappings (12 binary, 8 comparison, 12 augmented assignment, 2 unary).
- Standing Rule #20 sealed exception hierarchy and centralized error catalog.
- Bounded corpus parser and validator for `cases.json` fixture.
