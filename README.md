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

The repo changes quickly, so this README describes the shape of the project
rather than trying to keep a live inventory in sync.

- `code/packages/` contains publishable libraries across ecosystems such as
  C#, Dart, Elixir, F#, Go, Haskell, Java, Kotlin, Lua, Perl, Python, Ruby,
  Rust, Swift, TypeScript, and WebAssembly.
- `code/programs/` contains standalone tools, demos, visualizers, apps, and
  experiments across the same general language families.
- `code/specs/` contains the package and system design documents.
- `code/learning/` contains teaching material that explains the concepts behind
  the packages.
- `code/grammars/` contains shared grammar and token sources.
- Starlark is used as a build-time rule/configuration language, not as a
  package implementation ecosystem.

## Main Themes

### 1. Computer architecture and the computing stack

This is still one of the core stories in the repo:

- logic gates, transistors, clocks, arithmetic, and floating-point arithmetic
- CPU simulators and ISA simulators
- ARM, ARM1, RISC-V, WASM, Intel 4004, Intel 8008, JVM, and CLR execution models
- cache, branch prediction, hazard detection, pipeline, and core design
- accelerator and GPU-oriented work such as `gpu-core`, `compute-unit`, and `parallel-execution-engine`

### 2. Language tooling and runtimes

The repo has deep coverage of the path from text to execution:

- shared grammars and generated frontends
- lexers and parsers for multiple languages
- bytecode compilers, virtual machines, assemblers, and IR tooling
- grammar tools and build-time code generation

### 3. Data structures, storage, and execution infrastructure

There is broad coverage of classic and systems-oriented data-structure work:

- trees, tries, heaps, skip lists, bloom filters, hyperloglog, and graph packages
- RESP and in-memory data-store protocol work
- file-system, process-manager, event-loop, IPC, and network-stack packages

### 4. Cryptography, compression, and encoding

This area is much more prominent than the older README suggested:

- hashes, HMAC, HKDF, PBKDF2, scrypt, AES, ChaCha20-Poly1305, Ed25519, X25519
- compression families like LZ77, LZ78, LZSS, LZW, Huffman, Deflate, and Brotli
- barcode and related encoding work such as Code39, Code128, Codabar, EAN-13, ITF, and UPC-A

### 5. Documents, graphics, and rendering

The repo also includes a substantial content/rendering track:

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
|-- fixtures/
|-- grammars/
|-- learning/
|-- packages/
|-- programs/
|-- specs/
`-- src/
```

## Learning Material

The learning side of the repo is a first-class part of the structure, not an afterthought.

Start here:

- [Learning Index](./code/learning/README.md)
- [Algorithms](./code/learning/algorithms/README.md)
- [Computer Architecture](./code/learning/computer-architecture/README.md)
- [Language Tooling](./code/learning/language-tooling/README.md)
- [Python Ecosystem](./code/learning/python/ecosystem.md)

The intended relationship is:

```text
specs explain what we intend to build
learning explains why the ideas matter
packages show the ideas in code
tests prove the behavior
```

## Tooling

The root `mise.toml` currently pins:

- Dart `latest`
- Go `latest`
- Python `3.12`
- Ruby `3.4`
- Rust `stable`

The main repo-level grammar helper is:

- [scripts/generate-compiled-grammars.sh](./scripts/generate-compiled-grammars.sh)

## Workflow

The working style for the repo is:

1. Write or refine the spec.
2. Add or update the matching learning entry.
3. Add tests.
4. Implement the package or feature.
5. Update the package README and changelog.

The long-term goal is that no major concept in the repo exists only as code. It should also exist as a teachable explanation.

## Good Entry Points

If you want to explore the repo by theme, start here:

- [00-architecture.md](./code/specs/00-architecture.md) for the big picture
- [D00-deep-cpu-architecture.md](./code/specs/D00-deep-cpu-architecture.md) for the architecture track
- [DT25-mini-redis.md](./code/specs/DT25-mini-redis.md) for the single-node data-store baseline
- [Kahn's algorithm](./code/learning/algorithms/kahns-algorithm.md) for the build-planning story
- [computing-stack.md](./code/learning/computer-architecture/computing-stack.md) for the hardware-to-language story
- [code/programs/typescript](./code/programs/typescript/) for the current app and visualizer-heavy program surface

## Copyright

Everything in this repo is copyrighted to Adhithya Rajasekaran. Individual packages may be licensed separately.
