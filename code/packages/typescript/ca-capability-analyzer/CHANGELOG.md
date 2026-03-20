# Changelog

All notable changes to `@coding-adventures/ca-capability-analyzer` will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Core AST analyzer using TypeScript Compiler API
  - ESM import detection (`import`/`import *`/`import { name }`)
  - CJS require detection (`const x = require("mod")`, destructured)
  - Function call detection (`fs.readFileSync(...)`, `exec(...)`, etc.)
  - `process.env` access detection (property and bracket notation)
  - `fetch()` global detection
  - Banned construct detection (eval, new Function, dynamic require/import, Reflect)
- Capability manifest system
  - JSON manifest parsing with validation
  - Simple glob matching for targets (no external deps)
  - Comparison engine: matched, undeclared, unused
- CLI with three subcommands: detect, banned, check
- Comprehensive test suite (50+ test cases)
- Zero external runtime dependencies (uses TypeScript Compiler API only)
