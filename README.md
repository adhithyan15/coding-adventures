# Coding Adventures

Coding Adventures is a learning-first monorepo for understanding how computers work by building the layers ourselves.

This repo is part computing-stack curriculum, part polyglot package lab, part tooling playground, and part app/program sandbox. The goal is not just to collect implementations, but to make the ideas behind them easier to study, compare, and extend.

## What Lives Here

- `code/specs/` contains design docs and package specs
- `code/learning/` contains plain-language teaching material
- `code/grammars/` contains shared `.grammar` and `.tokens` sources
- `code/src/` contains shared TypeScript support code and token sources
- `code/packages/` contains publishable libraries across multiple ecosystems
- `code/programs/` contains standalone tools, demos, apps, and visualizers
- `code/fixtures/` contains shared assets and sample inputs
- `scripts/` contains repo-level helper scripts
- `.github/workflows/` contains CI, publish, release, and deploy automation

## Current Shape

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

### 1. Computer architecture and the computing stack

This is still one of the core stories in the repo:

| Target Language | Python pkg | Ruby pkg | Go pkg | TypeScript pkg |
|-----------------|-----------|----------|--------|---------------|
| Python | — | `python_lexer`/`python_parser` | `python-lexer`/`python-parser` | `python-lexer`/`python-parser` |
| Ruby | `ruby-lexer`/`ruby-parser` | — | `ruby-lexer`/`ruby-parser` | `ruby-lexer`/`ruby-parser` |
| JavaScript | `javascript-lexer`/`javascript-parser` | `javascript_lexer`/`javascript_parser` | `javascript-lexer`/`javascript-parser` | — |
| TypeScript | `typescript-lexer`/`typescript-parser` | `typescript_lexer`/`typescript_parser` | `typescript-lexer`/`typescript-parser` | — |

### 2. Language tooling and runtimes

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

There is broad coverage of classic and systems-oriented data-structure work:

The foundation of the build system — a standalone library in Python, Ruby, and Go. Provides: topological sort, cycle detection, independent groups (parallel levels), affected nodes (incremental builds), transitive closure.

- hashes, HMAC, HKDF, PBKDF2, scrypt, AES, ChaCha20-Poly1305, Ed25519, X25519
- compression families like LZ77, LZ78, LZSS, LZW, Huffman, Deflate, and Brotli
- barcode and related encoding work such as Code39, Code128, Codabar, EAN-13, ITF, and UPC-A

### 5. Documents, graphics, and rendering

- `python.tokens` + `python.grammar` — Python subset
- `ruby.tokens` + `ruby.grammar` — Ruby subset
- `javascript.tokens` + `javascript.grammar` — JavaScript subset
- `typescript.tokens` + `typescript.grammar` — TypeScript subset

- CommonMark, GFM, AsciiDoc, document ASTs, sanitizers, and HTML rendering
- draw and paint instruction systems
- image codecs, font parsing, display work, and visualizer programs

### 6. Tooling, apps, and learning-oriented programs

`code/programs/` is broader than just demos:

- build tools, scaffold generators, grammar tools, and package materializers
- visualizers such as arithmetic, logic-gates, transistor, ENIAC, and Code39 programs
- apps and product experiments like checklist, journal, Engram, and browser/extension work
- small ML/demo programs such as predictors and classifiers
- multi-language IRC server work via `ircd`

## Repository Map

```text
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

## Learning Material

| Language | Version | Package Manager | Test Framework | Linter |
|----------|---------|----------------|---------------|--------|
| Python | 3.12+ | uv | pytest | ruff |
| Ruby | 3.4 | Bundler | Minitest | Standard Ruby |
| Go | 1.26 | go modules | go test | go vet |
| Rust | stable | Cargo | cargo test | clippy |
| TypeScript | 5.x | npm | vitest | eslint |

Start here:

## CI/CD

### CI — Build & Test

The intended relationship is:

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

The root `mise.toml` currently pins:

- [ ] Data structures library (Rust core + language wrappers)
- [ ] Machine learning track (neuron → network → backprop → autograd → attention)
- [ ] GPU computing track (naive matmul → tiled → SIMD → GPU kernel → tensor core)
- [ ] JIT compiler implementation
- [ ] Safe C/C++ data structures (compile-time safety guardrails)
- [ ] HTML pipeline visualizer
- [ ] Full RISC-V RV32I + M-mode extensions

## Copyright

Everything in this repo is copyrighted to Adhithya Rajasekaran. Individual packages may be licensed separately.
