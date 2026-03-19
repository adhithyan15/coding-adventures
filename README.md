# Coding Adventures

A polyglot learning monorepo for building the entire computing stack from scratch — from logic gates to high-level language execution. Every layer is implemented, tested, and documented in multiple languages.

## Vision

Computers are deterministic. There is no magic. This repo exists to prove that by building every layer of the computing stack by hand:

```
Layer 1:  Source Code            You write: x = 1 + 2
Layer 2:  Lexer                  Breaks source text into tokens
Layer 3:  Parser                 Builds abstract syntax trees from tokens
              │
              ├─── Path A (Interpreted) ─────── Path B (Compiled) ───┐
              │                                                      │
Layer 4a: Bytecode Compiler      VM instructions    Layer 4b: Machine Code Compiler
              │                                                      │
Layer 5:  Virtual Machine        Stack-based eval   Layer 6:  Assembler
              │                  (like JVM, CLR)                     │
              ├─→ JIT (future)                      Layer 7:  ISA Simulators
              │                                     RISC-V, ARM, WASM, Intel 4004, JVM, CLR
              └────────────────────────┬─────────────────────────────┘
                                       │
Layer 8:  CPU Simulator          Fetch-decode-execute cycle
Layer 9:  Arithmetic (ALU)       Adders, subtractors — built from gates
Layer 10: Logic Gates            AND, OR, XOR — the irreducible foundation
```

## What's built

**126 packages and 6 programs across 5 languages.**

### Computing stack

The full computing stack is implemented in Python, Ruby, Go, and TypeScript. Rust covers the deep hardware layers.

| Layer | Package | Python | Ruby | Go | TypeScript | Rust |
|-------|---------|--------|------|----|------------|------|
| 10 | Logic Gates | ✅ | ✅ | ✅ | ✅ | ✅ |
| 9 | Arithmetic / ALU | ✅ | ✅ | ✅ | ✅ | ✅ |
| 9 | Floating-Point Arithmetic | ✅ | ✅ | ✅ | ✅ | — |
| 8 | CPU Simulator | ✅ | ✅ | ✅ | ✅ | — |
| 7 | ARM Simulator | ✅ | ✅ | ✅ | ✅ | — |
| 7 | RISC-V Simulator | ✅ | ✅ | ✅ | ✅ | — |
| 7 | WASM Simulator | ✅ | ✅ | ✅ | ✅ | — |
| 7 | Intel 4004 Simulator | ✅ | ✅ | ✅ | ✅ | — |
| 7 | JVM Simulator | ✅ | ✅ | ✅ | ✅ | — |
| 7 | CLR Simulator | ✅ | ✅ | ✅ | ✅ | — |
| 2 | Grammar Tools | ✅ | ✅ | ✅ | ✅ | — |
| 2 | Lexer | ✅ | ✅ | ✅ | ✅ | — |
| 3 | Parser | ✅ | ✅ | ✅ | ✅ | — |
| 4a | Bytecode Compiler | ✅ | ✅ | ✅ | ✅ | — |
| 5 | Virtual Machine | ✅ | ✅ | ✅ | ✅ | — |
| 0 | Pipeline | ✅ | ✅ | — | ✅ | — |
| — | HTML Renderer | ✅ | ✅ | — | ✅ | — |
| — | Assembler (shell) | ✅ | ✅ | ✅ | ✅ | — |
| — | JIT Compiler (shell) | ✅ | ✅ | — | ✅ | — |

### Deep CPU internals (Rust + Python + Ruby + Go + TypeScript)

| Package | Description | Rust | Python | Ruby | Go | TypeScript |
|---------|-------------|------|--------|------|----|------------|
| Cache | L1/L2 cache simulation | ✅ | ✅ | ✅ | ✅ | ✅ |
| Branch Predictor | 1-bit, 2-bit, BTB | ✅ | ✅ | ✅ | ✅ | ✅ |
| Hazard Detection | Data/control/structural hazards | ✅ | ✅ | ✅ | ✅ | ✅ |
| Clock | Clock generator, divider, multi-phase | — | ✅ | ✅ | ✅ | ✅ |

### Cross-language packages

The grammar-driven lexer/parser can tokenize and parse **any language** given grammar files:

| Target Language | Python pkg | Ruby pkg | Go pkg | TypeScript pkg |
|-----------------|-----------|----------|--------|---------------|
| Python | — | `python_lexer`/`python_parser` | `python-lexer`/`python-parser` | `python-lexer`/`python-parser` |
| Ruby | `ruby-lexer`/`ruby-parser` | — | `ruby-lexer`/`ruby-parser` | `ruby-lexer`/`ruby-parser` |
| JavaScript | `javascript-lexer`/`javascript-parser` | `javascript_lexer`/`javascript_parser` | `javascript-lexer`/`javascript-parser` | — |
| TypeScript | `typescript-lexer`/`typescript-parser` | `typescript_lexer`/`typescript_parser` | `typescript-lexer`/`typescript-parser` | — |

### Build system

An incremental, parallel monorepo build tool implemented in four languages:

| Implementation | Role |
|---------------|------|
| **Go** | **Primary** — compiles to native binary, goroutine parallelism |
| **Rust** | Native performance, rayon thread pool |
| Python | Reference implementation |
| Ruby | Educational port |

Features:
- **Recursive BUILD file discovery** — walks the directory tree automatically, no routing files needed
- **Git-diff change detection** — `git diff origin/main...HEAD` determines what changed
- **Dependency-aware** — parses `pyproject.toml`, `.gemspec`, `go.mod`, and `Cargo.toml`
- **Parallel execution** — independent packages run concurrently by topological level
- **Skip list** — automatically ignores `.git`, `.venv`, `node_modules`, `target`, etc.

### Directed graph library

The foundation of the build system — a standalone library in Python, Ruby, and Go. Provides: topological sort, cycle detection, independent groups (parallel levels), affected nodes (incremental builds), transitive closure.

### Grammar files

Declarative grammar definitions shared by all implementations:

- `python.tokens` + `python.grammar` — Python subset
- `ruby.tokens` + `ruby.grammar` — Ruby subset
- `javascript.tokens` + `javascript.grammar` — JavaScript subset
- `typescript.tokens` + `typescript.grammar` — TypeScript subset

Adding a new language requires only writing grammar files — no new lexer or parser code.

## Structure

```
code/
├── specs/          Specifications for each package (the blueprint)
├── grammars/       Language grammar definitions (.tokens, .grammar)
├── learning/       Notes and learning materials per language/topic
├── packages/       Publishable libraries
│   ├── python/     30 Python packages (PyPI-ready)
│   ├── ruby/       30 Ruby gems (RubyGems-ready)
│   ├── go/         29 Go modules
│   ├── rust/       6 Rust crates
│   └── typescript/ 31 TypeScript packages (npm-ready)
└── programs/       Standalone programs
    ├── python/     Hello world, build tool, pipeline visualizer
    ├── ruby/       Build tool
    ├── go/         Build tool (primary)
    └── rust/       Build tool
```

## Languages & Tooling

| Language | Version | Package Manager | Test Framework | Linter |
|----------|---------|----------------|---------------|--------|
| Python | 3.12+ | uv | pytest | ruff |
| Ruby | 3.4 | Bundler | Minitest | Standard Ruby |
| Go | 1.26 | go modules | go test | go vet |
| Rust | stable | Cargo | cargo test | clippy |
| TypeScript | 5.x | npm | vitest | eslint |

All managed via [mise](https://mise.jdx.dev/) — see `mise.toml`.

## CI/CD

### CI — Build & Test

GitHub Actions workflow (`.github/workflows/ci.yml`):
1. Compiles Go build tool from source (~1-2 seconds)
2. Uses `git diff` to detect changed packages
3. Builds dependency graph, finds affected packages
4. Runs them in parallel by topological level
5. Linux on every push, Linux + macOS on PRs to main

### Publish — Release to Registries

GitHub Actions workflow (`.github/workflows/publish.yml`):
- Triggered by GitHub Release with tag format `<language>/<package-name>/v<version>`
- **Python**: Trusted Publishers (OIDC) to PyPI — no API tokens needed
- **Python native extensions**: Builds wheels on Linux, macOS (arm64 + x86_64), and Windows via maturin
- **Ruby**: Pushes gems via `RUBYGEMS_API_KEY` secret

## Philosophy

- **No magic** — build every layer from scratch, understand what computers actually do
- **Go slow and deliberate** — depth over breadth
- **Literate programming** — Knuth-style, every source file teaches
- **Publishable quality** — every package ready for PyPI/RubyGems/npm/crates.io
- **Specs first** — specification → tests → implementation → changelog
- **>80% test coverage** — enforced, typically 95%+
- **Multiple languages** — same concepts, different ecosystems, deeper understanding

## Future roadmap

- [ ] Data structures library (Rust core + language wrappers)
- [ ] Machine learning track (neuron → network → backprop → autograd → attention)
- [ ] GPU computing track (naive matmul → tiled → SIMD → GPU kernel → tensor core)
- [ ] JIT compiler implementation
- [ ] Safe C/C++ data structures (compile-time safety guardrails)
- [ ] HTML pipeline visualizer
- [ ] Full RISC-V RV32I + M-mode extensions

## Copyright

Everything in this repo is copyrighted to Adhithya Rajasekaran. Individual packages may be licensed separately.
