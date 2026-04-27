# Changelog

All notable changes to the coding-adventures monorepo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added — SQL Auto-Index: Composite Multi-Column Index (IX-8)
- **`IndexScan.columns: tuple[str, ...]`** in `sql-planner` — replaces
  `column: str`; single-column scans produce a 1-tuple, composite scans an
  n-tuple matching the leading prefix of the index used.
- **Multi-column bounds** — `IndexScan.lo` / `IndexScan.hi` widened to
  `tuple[object, ...] | None`; `OpenIndexScan` in `sql-codegen` and the VM
  decode them with `list(ins.lo)` for prefix-key comparison in the backend.
- **`_extract_multi_column_bounds`** planner helper — chains `_extract_index_bounds`
  across consecutive index columns; EQ extends the chain, range terminates it.
- **Best-match index selection** — `_try_index_scan` evaluates all indexes and
  picks the one covering the most predicate columns.
- **`IndexAdvisor` pair tracking** — `_pair_hits` accumulates `(table, col_a, col_b)`
  pairs from full-table scans; `_maybe_create_composite_index` creates a
  two-column index when the policy threshold is reached, skipping redundant
  creation when a leading-column single index already exists.
- **`_auto_index_meta`** — maps auto-index names → `(table, columns_tuple)` for
  correct drop-loop bookkeeping without name parsing.
- **21 new tests** in `mini-sqlite/tests/test_tier3_composite.py` covering
  advisor pair logic, planner composite selection, and end-to-end integration.

### Added — ALGOL 60 WASM Pipeline
- Advanced PL04 Phase 5 call-by-name lowering with integer array-element
  eval/store thunk descriptors, including repeated re-location of subscripted
  actuals on formal reads and assignments.
- Enabled read-only ALGOL expression thunks to read arrays, covering
  Jensen's-device terms such as `a[i] * i` through the WASM runtime path.
- Enabled read-only ALGOL expression thunks to call integer procedures, with
  nested procedure failures propagated through thunk helper state.
- Added Phase 5 wrap-up coverage and docs for the completed integer by-name
  subset and its remaining full-ALGOL exclusions.
- Added PL04 Phase 6 direct local labels and `goto` support through the
  ALGOL type-checker, IR compiler, and WASM compiler path, with guards for
  nonlocal and Phase 7 designational forms.
- Added PL04 Phase 7a local switch declarations, switch selections, and
  conditional designational `goto` support through type-checking, IR lowering,
  and WASM execution, while keeping nonlocal frame unwinding guarded.
- Added PL04 Phase 7b direct nonlocal block `goto` support with frame/heap
  unwinding inside one lowered function, while keeping procedure-crossing
  jumps and nonlocal designational forms guarded.

### Added — TypeScript Port + JavaScript/TypeScript Grammars (PR #14)
- **31 TypeScript packages** — complete port of the computing stack to TypeScript
- `javascript.tokens` + `javascript.grammar` — JavaScript grammar definitions
- `typescript.tokens` + `typescript.grammar` — TypeScript grammar definitions
- Cross-language packages: `javascript-lexer`, `javascript-parser`, `typescript-lexer`, `typescript-parser` in Python, Ruby, and Go
- D05 Core package (processor integration) in Python, Ruby, Go, and TypeScript
- Extended RISC-V simulator with full RV32I base integer ISA + M-mode privileged extensions

### Changed — Build System: Recursive Discovery + Rust Build Tool (PRs #16, #17)
- **Recursive BUILD file discovery** replaces DIRS-based routing in all build tools (Go, Python, Ruby)
- Build tools now walk the directory tree automatically — no DIRS files needed
- Added skip list for non-source directories (`.git`, `.venv`, `node_modules`, `target`, `.claude`, etc.)
- **Rust added as recognized language** — 6 Rust packages now properly discovered (were "unknown")
- **New: Rust build tool** — complete port with rayon parallelism, SHA256 hashing, git-diff detection
- **All 18 DIRS files removed** from the repository
- Total discovered packages increased from 77 (DIRS-routed) to 126+ (recursive)

### Added — Publish Workflow + Package Completeness (PR #13)
- `.github/workflows/publish.yml` — release publishing for PyPI and RubyGems
- PyPI publishing via OIDC Trusted Publishers (no API tokens)
- Native extension support via maturin: builds wheels on Linux, macOS (arm64 + x86_64), Windows
- Ruby gem publishing via `RUBYGEMS_API_KEY` secret
- Fixed 8 incomplete packages:
  - Go: README + CHANGELOG for assembler, python-lexer, ruby-lexer
  - Ruby: test suites for assembler, html_renderer, jit_compiler shell gems
  - Python: README/CHANGELOG for hello-world and pipeline-visualizer programs

### Added — Go Port (PR #12)
- **25 Go packages** — complete port of the computing stack to Go
- Go implementations of all hardware layers, simulators, lexer, parser, compiler, and VM
- Grammar-driven lexer/parser with cross-language packages (python-lexer, ruby-lexer)

### Added — Deep CPU Internals (PR #11)
- `cache` — L1/L2 cache simulation with LRU eviction in Python, Ruby, Go, and Rust
- `branch-predictor` — 1-bit, 2-bit saturating counter, branch target buffer in Python, Ruby, Go, and Rust
- `hazard-detection` — data, control, and structural hazard detection in Python, Ruby, Go, and Rust
- `clock` — clock generator, divider, multi-phase clock in Python, Ruby, and Go
- `fp-arithmetic` — IEEE 754 floating-point arithmetic in Python, Ruby, and Go
- Deep CPU architecture specs (D00-D05)
- Floating-point arithmetic spec (FP01)

### Added — Accelerator Computing Stack (PR #10)
- GPU/TPU/NPU computing stack specs and overview
- Accelerator architecture documentation (G00)

### Added — Build System (PR #9)
- **Directed graph library** in Python (73 tests, 98%), Ruby (77 tests, 100%), and Go (39 tests, 94%)
- **Build tool** in Go (primary), Python (reference), and Ruby (educational) — incremental, parallel, git-diff-based change detection
- **BUILD files** for all packages — declarative build commands per package
- **GitHub Actions CI** — compiles Go build tool, runs affected packages in parallel
- Go 1.26 added to mise.toml

### Added — Cross-Language Packages (PR #8)
- `ruby-lexer` (Python) — tokenizes Ruby source code via grammar files (42 tests)
- `ruby-parser` (Python) — parses Ruby source code via grammar files (21 tests)
- `python_lexer` (Ruby) — tokenizes Python source code via grammar files (32 tests)
- `python_parser` (Ruby) — parses Python source code via grammar files (15 tests)

### Added — Ruby Computing Stack (PR #7)
- Complete port of all 18 Python packages to Ruby as publishable gems
- Ruby 3.4.9 via mise, Minitest, SimpleCov, Data.define, Standard Ruby
- `ruby.tokens` and `ruby.grammar` grammar definitions
- 662+ Ruby tests, all packages ≥80% coverage (most 95%+)

### Added — JVM + CLR Simulators and Compiler Backends (PR #6)
- `jvm-simulator` — 26 JVM opcodes with real opcode values (81 tests, 97%)
- `clr-simulator` — 24 CLR IL opcodes with real opcode values (93 tests, 100%)
- `JVMCompiler`, `CLRCompiler`, `WASMCompiler` bytecode compiler backends (133 tests, 100%)

### Added — Software Layers Implementation (PR #5)
- `lexer` — hand-written + grammar-driven tokenizer (76 tests, 98%)
- `parser` — recursive descent + grammar-driven parser (54 tests, 99%)
- `virtual-machine` — general-purpose stack-based VM, 20 opcodes (99 tests, 96%)
- `bytecode-compiler` — AST to bytecode compiler (34 tests, 100%)
- `grammar-tools` — reads .tokens/.grammar files with EBNF (66 tests, 97%)
- `pipeline` — end-to-end orchestrator (40 tests, 100%)
- Grammar-driven lexer and parser that work with any language's grammar files
- `python.tokens` and `python.grammar` grammar definitions
- JIT compiler spec and shell package

### Added — Hardware Layers Implementation (PRs #3, #4)
- `logic-gates` — 7 gates + NAND-derived + multi-input variants (89 tests)
- `arithmetic` — half adder, full adder, ripple carry adder, ALU (34 tests)
- `cpu-simulator` — generic fetch-decode-execute cycle (34 tests)
- `arm-simulator` — ARMv7 subset: MOV, ADD, SUB (16 tests)
- `riscv-simulator` — RISC-V RV32I subset: addi, add, sub (14 tests)
- `wasm-simulator` — WebAssembly stack machine (28 tests)
- `intel4004-simulator` — Intel 4004 accumulator machine (21 tests)
- Layer renumbering from top-down (user perspective → hardware)

### Added — HTML Visualizer Design (PR #2)
- Replaced TUI visualizer with pluggable HTML visualizer architecture
- JSON data contract for cross-language pipeline reports
- HTML renderer package scaffold
- Pipeline visualizer program scaffold

### Added — Initial Repository Structure (PR #1)
- Repository scaffolding: CLAUDE.md, README.md, lessons.md, .gitignore
- 9 Python package scaffolds for the computing stack
- Specification documents for all layers (numbered 01-11)
- Python hello world program
- RISC-V simulator package scaffold
- Pipeline orchestrator and stack visualizer scaffolds

## [0.0.0] - 2026-03-18

### Added
- Initial commit with empty repository
