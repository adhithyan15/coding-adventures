# Learning Index

The `code/learning` directory is the teaching companion to the rest of the repository.

The basic rule for this folder is simple: if a concept is important enough to have a package, it should also have a learning entry that explains the idea in plain language.

## How To Read This Folder

There are four kinds of material in this directory:

1. **Track overviews**: explain a whole area, like computer architecture or language tooling.
2. **Deep dives**: explain one important concept in detail, like Kahn's algorithm.
3. **Ecosystem notes**: explain tooling and language-specific conventions.
4. **Personal curricula**: self-contained, standalone skill-building tracks that aren't tied to any package in this repo — e.g. the human-language curriculum. These follow their own spec lineage (see each track's README) rather than the package-companion coverage map below.

## Current Tracks

- [Algorithms](./algorithms/README.md)
- [Compression and Encoding](./compression/README.md)
- [Computer Architecture](./computer-architecture/README.md)
- [Cryptography](./cryptography/README.md)
- [Language Tooling](./language-tooling/README.md)
- [Documents and Media](./documents-media/README.md)
- [Machine Learning](./machine-learning/README.md)
- [SQL and Storage](./sql-storage/README.md)
- [Python Ecosystem](./programming-languages/python-ecosystem.md)
- [Human Languages](./human-languages/README.md) — personal curriculum, not a package companion

The hand-authored [Learning Coverage Roadmap](./ROADMAP.md) explains how the
generated inventory is prioritized and records the baseline survey.

The generated [Learning Coverage Inventory](./COVERAGE.md) is the complete
package-to-learning backlog. Regenerate it after adding or renaming packages or
learning material:

```powershell
python code/scripts/learning_coverage_report.py --output code/learning/COVERAGE.md
```

## Coverage Map

This table is the current answer to "what learning entry explains this package family?"

| Repository area | Packages / concepts | Learning entry |
|-----------------|---------------------|----------------|
| Graph algorithms and build planning | `directed-graph`, build-tool dependency planning | [Kahn's algorithm](./algorithms/kahns-algorithm.md) |
| Graph modeling | nodes, edges, topological ordering, affected dependents | [Algorithms index](./algorithms/README.md) |
| Trees and approximate indexes | balanced trees, tries, range structures, Bloom filters, HyperLogLog | [Trees, indexes, and probabilistic structures](./algorithms/trees-indexes-and-probabilistic-structures.md) |
| Compression and error correction | LZ families, Huffman coding, Deflate, Brotli, Zstandard, Reed-Solomon | [Dictionary and entropy coding](./compression/dictionary-and-entropy-coding.md) |
| Cryptographic building blocks | hashes, authentication, encryption, password hashing, signatures, key agreement | [Cryptographic primitives and composition](./cryptography/primitives-and-composition.md) |
| Digital logic to execution | `logic-gates`, `arithmetic`, `fp-arithmetic`, `cpu-simulator`, ISA simulators | [Computing stack](./computer-architecture/computing-stack.md) |
| Execution models | register machines, stack machines, accumulator machines, bytecode vs machine code | [Instruction-set models](./computer-architecture/instruction-set-models.md) |
| Deep micro-architecture | `cache`, `branch-predictor`, `hazard-detection`, `pipeline`, `core` | [Pipelines, caches, and speculation](./computer-architecture/pipelines-caches-and-speculation.md) |
| Frontend and compiler pipeline | `grammar-tools`, `lexer`, `parser`, `bytecode-compiler`, `virtual-machine`, `assembler`, `jit-compiler` | [Language tooling index](./language-tooling/README.md) |
| Intermediate representations | semantic IR, compiler IR, interpreter IR, lowering, source identity | [Intermediate representations](./language-tooling/intermediate-representations.md) |
| WebAssembly | binary modules, LEB128, validation, instantiation, stack execution | [WebAssembly from bytes to execution](./language-tooling/webassembly-from-bytes-to-execution.md) |
| Structured text | JSON, CSV, TOML, XML, lexing, parsing, values, serialization | [Structured text from bytes to values](./language-tooling/structured-text-from-bytes-to-values.md) |
| Machine learning | `activation-functions`, `loss-functions`, `gradient-descent`, dense networks, autograd, neural graph execution | [Machine learning track](./machine-learning/README.md) |
| SQL execution | lexing, parsing, logical query order, backends, SQLite | [From SQL text to stored rows](./sql-storage/from-sql-text-to-stored-rows.md) |
| Barcodes | 1D/2D symbols, finite fields, Reed-Solomon correction, layout | [Barcodes and error correction](./documents-media/barcodes-and-error-correction.md) |
| Python project conventions | packaging, linting, types, `uv`, `ruff`, `pyproject.toml` | [Modern Python ecosystem](./programming-languages/python-ecosystem.md) |

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
