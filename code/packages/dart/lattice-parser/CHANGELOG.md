# Changelog

All notable changes to `coding_adventures_lattice_parser` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Grammar-driven parser for Lattice stylesheets using pre-compiled grammar.
- Produces `ASTNode` parse trees rooted at `stylesheet`.
- Pre-parse delimiter nesting depth defense (`maxNestingDepth = 256`) and token count ceiling (`maxTokenCount = 65536`).
- Centralized error catalog, sealed exceptions, and payload-blind diagnostics per Standing Rule #20.
