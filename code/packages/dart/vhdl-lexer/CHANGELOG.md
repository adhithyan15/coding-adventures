# Changelog

All notable changes to `coding_adventures_vhdl_lexer` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Grammar-driven lexical analyzer for VHDL with embedded 1987, 1993, 2002, 2008 and 2019 token grammars.
- Case-insensitive normalization: `NAME`/`KEYWORD` values are lowercased and retyped by the edition keyword set, matching the TypeScript lane.
- Bounded input validation, surrogate defense, and 100% centralized, payload-blind diagnostics per Standing Rule #20.
