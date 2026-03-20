# Learning Index

The `code/learning` directory is the teaching companion to the rest of the repository.

The basic rule for this folder is simple: if a concept is important enough to have a package, it should also have a learning entry that explains the idea in plain language.

## How To Read This Folder

There are three kinds of material in this directory:

1. **Track overviews**: explain a whole area, like computer architecture or language tooling.
2. **Deep dives**: explain one important concept in detail, like Kahn's algorithm.
3. **Ecosystem notes**: explain tooling and language-specific conventions.

## Current Tracks

- [Algorithms](./algorithms/README.md)
- [Computer Architecture](./computer-architecture/README.md)
- [Language Tooling](./language-tooling/README.md)
- [Python Ecosystem](./python/ecosystem.md)

## Coverage Map

This table is the current answer to "what learning entry explains this package family?"

| Repository area | Packages / concepts | Learning entry |
|-----------------|---------------------|----------------|
| Graph algorithms and build planning | `directed-graph`, build-tool dependency planning | [Kahn's algorithm](./algorithms/kahns-algorithm.md) |
| Graph modeling | nodes, edges, topological ordering, affected dependents | [Algorithms index](./algorithms/README.md) |
| Digital logic to execution | `logic-gates`, `arithmetic`, `fp-arithmetic`, `cpu-simulator`, ISA simulators | [Computing stack](./computer-architecture/computing-stack.md) |
| Execution models | register machines, stack machines, accumulator machines, bytecode vs machine code | [Instruction-set models](./computer-architecture/instruction-set-models.md) |
| Deep micro-architecture | `cache`, `branch-predictor`, `hazard-detection`, `pipeline`, `core` | [Pipelines, caches, and speculation](./computer-architecture/pipelines-caches-and-speculation.md) |
| Frontend and compiler pipeline | `grammar-tools`, `lexer`, `parser`, `bytecode-compiler`, `virtual-machine`, `assembler`, `jit-compiler` | [Language tooling index](./language-tooling/README.md) |
| Python project conventions | packaging, linting, types, `uv`, `ruff`, `pyproject.toml` | [Modern Python ecosystem](./python/ecosystem.md) |

## Relationship To Specs

Specs and learning notes serve different jobs:

- `code/specs/` explains the intended design and public API.
- `code/learning/` explains the ideas behind the design.

For example:

- a spec might say "the graph package exposes `TopologicalSort()`"
- a learning note should explain what topological sorting is, why cycles break it, and why Kahn's algorithm works

## What We Want Over Time

The end state for this directory is:

- every major package family has at least one learning entry
- major algorithms have dedicated deep dives
- architecture topics have diagrams and worked examples
- package READMEs stay focused on the package
- learning notes carry the broader teaching burden

This folder is meant to grow along with the repository.
