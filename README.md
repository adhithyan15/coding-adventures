# Coding Adventures

Coding Adventures is a learning-first monorepo for understanding how computers work by building the layers ourselves.

This repository is not just "a lot of packages." It is really four projects living in one place:

1. A computing-stack curriculum, from logic gates and arithmetic up through lexers, parsers, compilers, virtual machines, assemblers, and ISA simulators.
2. A computer architecture lab, with growing coverage of caches, branch prediction, hazard detection, pipelining, and configurable core design.
3. A polyglot comparison space, where the same ideas are implemented across Python, Ruby, Go, TypeScript, Rust, Elixir, Lua, Perl, and Swift so the concepts are separated from the language.
4. A publishing and tooling playground, where each package is treated like a real artifact with tests, metadata, changelogs, and CI.

## What This Repository Optimizes For

- **Understanding before abstraction**: build the layers directly instead of treating them as magic.
- **Learning through implementation**: every package should teach, not just execute.
- **Architecture as a first-class topic**: this repo is as much about pipelines, caches, and execution models as it is about parsers and bytecode.
- **Cross-language repetition with purpose**: re-implementing the same idea in multiple ecosystems is part of the learning method.
- **Publishable quality**: packages are expected to have tests, READMEs, changelogs, and package metadata.
- **Specs and learning notes alongside code**: the code, the specifications, and the learning material should reinforce each other.

## The Main Tracks

### 1. Digital logic and arithmetic

This track starts at the hardware floor:

- `logic-gates`
- `arithmetic`
- `fp-arithmetic`
- `clock`

The goal is to show how larger behavior emerges from small, deterministic building blocks.

### 2. CPU and ISA simulation

This track models execution closer to real hardware:

- `cpu-simulator`
- `arm-simulator`
- `riscv-simulator`
- `wasm-simulator`
- `intel4004-simulator`
- `jvm-simulator`
- `clr-simulator`

The goal is to understand instruction formats, execution models, fetch-decode-execute loops, and the differences between register machines, stack machines, and older accumulator-style designs.

### 3. Deep computer architecture

This is one of the most important themes in the repo:

- `cpu-cache`
- `branch-predictor`
- `hazard-detection`
- `pipeline`
- `core`

The goal is to move beyond "a CPU executes instructions" and into "how a modern core actually stays fast."

### 4. Language frontends and execution

This track builds the path from source code to execution:

- `grammar-tools`
- `lexer`
- `parser`
- `python-lexer`, `ruby-lexer`, `javascript-lexer`, `typescript-lexer`
- `python-parser`, `ruby-parser`, `javascript-parser`, `typescript-parser`
- `bytecode-compiler`
- `virtual-machine`
- `assembler`
- `jit-compiler`

The goal is to connect programming-language tooling back to the machine beneath it.

### 5. In-memory data stores and protocol stacks

This track is about building transport-agnostic, pluggable storage systems and ordered-set abstractions:

- `resp-protocol`
- `in-memory-data-store-protocol`
- `in-memory-data-store-engine`
- `in-memory-data-store`
- `tree-set`
- `hash-map`, `hash-set`, `heap`, `skip-list`, `hyperloglog`

The goal is to model a single-node data store as a composition of reusable
packages that can grow into new protocols, new transports, and new storage
modules over time.

### 6. Accelerators and parallel execution

This track explores computation outside the classic scalar CPU story:

- `gpu-core`
- `compute-unit`
- `device-simulator`
- `parallel-execution-engine`

The goal is to study throughput-oriented execution, dataflow, and accelerator-style design.

### 7. Machine learning fundamentals

This track covers small, foundational learning components that fit naturally with the accelerator story:

- `loss-functions`
- `gradient-descent`

The goal is to treat optimization primitives as understandable building blocks rather than opaque library calls.

### 8. Tooling, visualization, and infrastructure

This repo also includes the tools needed to sustain the work:

- monorepo build tools
- `directed-graph`
- `html-renderer`
- pipeline visualizers and support programs

These are not side quests. They are part of the project's teaching philosophy: infrastructure is also a thing worth understanding.

## How The Repository Is Organized

```text
code/
├── specs/         Specifications and architecture documents
├── learning/      Explanatory notes and teaching material
├── grammars/      Shared grammar definitions
├── packages/      Publishable libraries in multiple languages
└── programs/      Standalone tools and demos
```

## Languages

The repository currently spans nine core ecosystems:

- Python
- Ruby
- Go
- TypeScript
- Rust
- Elixir
- Lua
- Perl
- Swift

The language split is intentional. A parser should still be recognizable as a parser when moved from Python to Go. A cache should still look like a cache in Ruby or TypeScript. The repetition is part of the point.

## Learning Material

The learning side of the repository is now a first-class part of the structure, not an afterthought.

Start here:

- [Learning Index](./code/learning/README.md)
- [Algorithms](./code/learning/algorithms/README.md)
- [Computer Architecture](./code/learning/computer-architecture/README.md)
- [Language Tooling](./code/learning/language-tooling/README.md)
- [Machine learning notes](./code/learning/loss-functions.md)
- [Optimization notes](./code/learning/gradient-descent.md)
- [Python Ecosystem Notes](./code/learning/python/ecosystem.md)

The intended relationship is:

```text
specs explain what we intend to build
learning explains why the ideas matter
packages show the ideas in code
tests prove the behavior
```

## Current Shape

Today the repo contains:

- 1503 package directories
- 122 program directories
- 9 implementation languages

Those counts matter less than the shape: this is a broad, layered study of computing systems, programming-language tooling, and computer architecture.

## Workflow

The working style for the repository is:

1. Write or refine the spec.
2. Add or update the learning entry for the concept.
3. Add tests.
4. Implement the package or feature.
5. Update changelog and package README.

The long-term goal is that no major concept in the repository exists only as code. It should also exist as a teachable explanation.

## Good Entry Points

If you want to explore the repository by theme:

- Start with [00-architecture.md](./code/specs/00-architecture.md) for the big picture.
- Read [D00-deep-cpu-architecture.md](./code/specs/D00-deep-cpu-architecture.md) for the architecture track.
- Read [DT25 in-memory data store](./code/specs/DT25-mini-redis.md) for the current single-node in-memory data store baseline of the DT data-structure series.
- Read [Kahn's algorithm](./code/learning/algorithms/kahns-algorithm.md) to see how the build system uses graph algorithms.
- Read [computing-stack.md](./code/learning/computer-architecture/computing-stack.md) for the hardware-to-language story.

## Future Direction

The repository is still expanding in a few directions:

- richer learning material tied to every major package family
- deeper computer architecture coverage
- stronger accelerator and GPU material
- more cross-language consistency
- better visualization of the stack and execution flow

## Copyright

Everything in this repo is copyrighted to Adhithya Rajasekaran. Individual packages may be licensed separately.
