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

### Computing stack — Python (21 packages) and Ruby (20 gems)

Every package is implemented in both Python and Ruby with identical APIs:

| Layer | Package | Python Tests | Ruby Tests |
|-------|---------|-------------|------------|
| 10 | Logic Gates | ✅ 89 | ✅ 79 |
| 9 | Arithmetic / ALU | ✅ 34 | ✅ 63 |
| 8 | CPU Simulator | ✅ 34 | ✅ 28 |
| 7 | ARM Simulator | ✅ 16 | ✅ 17 |
| 7 | RISC-V Simulator | ✅ 14 | ✅ 17 |
| 7 | WASM Simulator | ✅ 28 | ✅ 22 |
| 7 | Intel 4004 Simulator | ✅ 21 | ✅ 12 |
| 7 | JVM Simulator | ✅ 81 | ✅ 42 |
| 7 | CLR Simulator | ✅ 93 | ✅ 39 |
| 2 | Grammar Tools | ✅ 66 | ✅ 55 |
| 2 | Lexer | ✅ 146 | ✅ 44 |
| 3 | Parser | ✅ 88 | ✅ 37 |
| 4a | Bytecode Compiler | ✅ 133 | ✅ 106 |
| 5 | Virtual Machine | ✅ 99 | ✅ 61 |
| 0 | Pipeline | ✅ 40 | ✅ 40 |

### Cross-language packages

The grammar-driven lexer/parser can tokenize and parse **any language** given grammar files:

| Package | Written in | Tokenizes/Parses | Tests |
|---------|-----------|-----------------|-------|
| `ruby-lexer` | Python | Ruby source code | 42 |
| `ruby-parser` | Python | Ruby source code | 21 |
| `python_lexer` | Ruby | Python source code | 32 |
| `python_parser` | Ruby | Python source code | 15 |

### Build system

An incremental, parallel monorepo build tool implemented in three languages:

| Implementation | Tests | Coverage | Role |
|---------------|-------|---------|------|
| **Go** | 62+ | 88-96% | **Primary** — compiles to native binary, goroutine parallelism |
| Python | 118 | 95% | Reference implementation |
| Ruby | 90 | 96% | Educational port |

Features:
- **Git-diff change detection** — `git diff origin/main...HEAD` determines what changed, no cache file needed
- **Dependency-aware** — automatically parses `pyproject.toml` and `.gemspec` to build a dependency graph
- **Parallel execution** — independent packages run concurrently (8 packages at level 0!)
- **BUILD/DIRS files** — simple text files declaring build commands and directory structure

### Directed graph library

The foundation of the build system — a standalone library in three languages:

| Language | Tests | Coverage |
|----------|-------|---------|
| Python | 73 | 98% |
| Ruby | 77 | 100% |
| Go | 39 | 94% |

Provides: topological sort, cycle detection, independent groups (parallel levels), affected nodes (incremental builds), transitive closure.

### Grammar files

Declarative grammar definitions shared by all implementations:

- `code/grammars/python.tokens` + `python.grammar` — Python subset
- `code/grammars/ruby.tokens` + `ruby.grammar` — Ruby subset

Adding a new language requires only writing grammar files — no new lexer or parser code.

## Structure

```
code/
├── specs/          Specifications for each package (the blueprint)
├── grammars/       Language grammar definitions (.tokens, .grammar)
├── learning/       Notes and learning materials per language/topic
├── packages/       Publishable libraries
│   ├── python/     21 Python packages (PyPI-ready)
│   ├── ruby/       20 Ruby gems (RubyGems-ready)
│   └── go/         Go modules
└── programs/       Standalone programs
    ├── python/     Hello world, build tool, pipeline visualizer
    ├── ruby/       Build tool
    └── go/         Build tool (primary)
```

## Languages & Tooling

| Language | Version | Package Manager | Test Framework | Linter | Type Checker |
|----------|---------|----------------|---------------|--------|-------------|
| Python | 3.12+ | uv | pytest | ruff | mypy |
| Ruby | 3.4 | Bundler | Minitest | Standard Ruby | RBS + Steep |
| Go | 1.26 | go modules | go test | go vet | built-in |

All managed via [mise](https://mise.jdx.dev/) — see `mise.toml`.

## CI

GitHub Actions workflow (`.github/workflows/ci.yml`):
1. Compiles Go build tool from source (~1-2 seconds)
2. Uses `git diff` to detect changed packages
3. Builds dependency graph, finds affected packages
4. Runs them in parallel by topological level
5. Linux on every push, Linux + macOS on PRs to main

## Philosophy

- **No magic** — build every layer from scratch, understand what computers actually do
- **Go slow and deliberate** — depth over breadth
- **Literate programming** — Knuth-style, every source file teaches
- **Publishable quality** — every package ready for PyPI/RubyGems
- **Specs first** — specification → tests → implementation → changelog
- **>80% test coverage** — enforced, typically 95%+
- **Multiple languages** — same concepts, different ecosystems, deeper understanding

## Future roadmap

- [ ] Machine learning track (neuron → network → backprop → autograd → attention)
- [ ] GPU computing track (naive matmul → tiled → SIMD → GPU kernel → tensor core)
- [ ] JIT compiler implementation
- [ ] Sandbox-based build isolation (like Bazel)
- [ ] TypeScript implementation of the computing stack
- [ ] VS Code extension for `.tokens`/`.grammar` files
- [ ] HTML pipeline visualizer

## Copyright

Everything in this repo is copyrighted to Adhithya Rajasekaran. Individual packages may be licensed separately.
