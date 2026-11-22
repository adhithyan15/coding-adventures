# Changelog

All notable changes to `coding_adventures_vhdl_parser` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Grammar-driven parser for VHDL with embedded 1987, 1993, 2002, 2008 and 2019 parser grammars.
- Produces `ASTNode` parse trees rooted at `design_file`.
- Combined nesting depth defense (`maxNestingDepth = 64`) over parentheses, block-opening keywords and prefix-operator chains, plus a token ceiling (`maxTokenCount = 65536`) and a typed stack-overflow backstop.
- Centralized error catalog, sealed exceptions, and payload-blind diagnostics per Standing Rule #20.
