# Coding Adventures

Coding Adventures is a learning-first monorepo for understanding computers by
building the layers ourselves: logic gates and processors, parsers and virtual
machines, compilers and runtimes, storage engines, UI systems, and complete
applications.

It is part computing-stack curriculum, part polyglot package laboratory, part
language-platform project, and part application sandbox. The point is not merely
to collect implementations. The point is to make each idea inspectable,
testable, comparable across languages, and teachable.

## The Big Picture

The repository has four reinforcing layers:

```text
specs       define the intended design and contracts
learning    explains why the ideas work
packages    implements the reusable pieces
programs    composes those pieces into tools, demos, and applications
```

Tests, conformance suites, and the dependency-aware build system connect all
four. A shared package change is tested not only in isolation but also through
the downstream packages affected by it.

The repository changes quickly. This README describes the durable architecture
and current direction; it deliberately avoids a hand-maintained package
inventory. Run the [package parity report](./code/scripts/package_parity_report.py)
for a live cross-language view.

## What Makes This Repo Different

### A language platform with two shared hubs

The north-star is a language platform where a language author supplies grammar
files and one frontend, then reuses shared tooling, optimizers, runtimes, and
backends.

```text
source
  |
  +--> .tokens + .grammar --> generated lexer/parser --> AST/CST
                                                        |
                                      +-----------------+-----------------+
                                      |                                   |
                                      v                                   v
                               IIR: execution hub                  SIR: semantic hub
                                      |                                   |
                   +------------------+------------------+       +--------+---------+
                   |        |        |        |         |       |        |         |
                 native    LLVM     WASM     JVM/CLR    VM/JIT  Python   Ruby   JS/TS/Go/
                                                                            Rust/C
```

- **IIR** is the lower, typed execution representation. It feeds native,
  LLVM, WASM, JVM, CLR, BEAM, VM, JIT, and retro-CPU targets.
- **SIR** is the higher semantic representation. It preserves source-language
  concepts and emits runnable Python, Ruby, JavaScript, TypeScript, Go, Rust,
  and C.
- The planned IIR/SIR bridges turn compilation and transpilation into routes
  through one stage graph instead of bespoke language-to-language projects.

The execution matrix runs real programs across backends and compares results,
so a backend is not considered supported merely because it accepts the IR.
See the [LANG-VM platform vision](./code/specs/LANG-PLATFORM-VISION.md),
[generic language pipeline](./code/specs/LANG00-generic-language-pipeline.md),
[IIR](./code/specs/LANG01-interpreter-ir.md), and
[Semantic IR](./code/specs/SIR00-semantic-ir.md).

### The computing stack, bottom to top

Another central thread builds computation from physical and logical primitives:

```text
transistors --> gates --> arithmetic --> CPU/microarchitecture --> ISA
     --> assembler --> compiler/runtime --> language --> application
```

The repo includes transistor and gate models, ALUs, caches, branch prediction,
pipelines, processor cores, and simulators for architectures including ARM,
ARM1, RISC-V, WASM, Intel 4004/8008, JVM, and CLR. Several browser visualizers
make those layers interactive.

Start with the [architecture overview](./code/specs/00-architecture.md),
[deep CPU architecture](./code/specs/D00-deep-cpu-architecture.md), and
[computing-stack learning guide](./code/learning/computer-architecture/computing-stack.md).

### Reference implementations and language mirrors

The language directories are not identical snapshots. Rust is the broadest
implementation surface and often carries systems-facing reference work, native
backends, and shared engines first. Python, TypeScript, Go, Ruby, and the other
ecosystems contain a mix of:

- independent implementations of the same concept;
- conformance mirrors ported from a reference;
- native bindings around shared Rust cores;
- platform-specific implementations and applications;
- educational ports used to compare language design and tooling.

The parity reporter groups those implementations by concept, highlights
high-consensus packages, and tracks single-language work without pretending
every package must exist everywhere.

## Major Project Tracks

### Languages, compilers, and runtimes

The repository covers the whole path from source text to execution:

- shared token and grammar formats;
- generated lexers and parsers;
- ASTs, type checkers, formatters, LSP and DAP infrastructure;
- bytecode compilers, VMs, JIT/AOT infrastructure, and native backends;
- backend-independent IR validation, lowering, optimization, and conformance;
- historical and experimental languages including BASIC, ALGOL, Nib, Twig,
  APL, FLOW-MATIC, COBOL-60, MATLAB/Octave, Wolfram, Macsyma/Maxima, and more;
- JavaScript parsing and Closure Compiler compatibility work.

The FLOW-MATIC and COBOL code-generation track is building toward the intended
reuse model: frontends target IIR to run across execution backends and SIR to
transpile across source backends. Math-language work adds array/matrix and
symbolic domains to the same semantic hub. See
[PL09 code generation](./code/specs/PL09-codegen.md) and
[HML01 math languages to Semantic IR](./code/specs/HML01-math-to-semantic-ir.md).

### Dynamic values, exact numerics, and symbolic systems

The runtime stack includes a language-neutral tagged dynamic-value substrate,
boxing/unboxing, heap values, truthiness, dynamic arithmetic, and a path toward
garbage-collected dynamic-language execution.

The numeric and symbolic side includes arbitrary-precision integers, rationals,
decimals and binary floats; computer algebra packages; rewrite systems;
constraint solving; logic programming; statistics; and the ADJ rule/formula
language. Exactness and explicit lossy conversions are treated as design
properties rather than incidental implementation details.

See the [dynamic-value substrate](./code/specs/DVAL01-generic-dynamic-value-substrate.md)
and [symbolic computation overview](./code/specs/symbolic-computation.md).

### SQL, databases, and storage

The SQL stack is deliberately decomposed:

```text
SQL text --> lexer --> parser --> planner --> optimizer --> codegen --> SQL VM
                                                                   |
                                                                   v
                                                            Backend trait
                                                                   |
                                                   in-memory or SQLite files
```

This supports both a composable query engine and a path toward a byte-compatible,
from-scratch SQLite replacement. Work covers SQL semantics, query planning,
indexes, transactions, SQLite file pages/B-trees, and differential checks
against real SQLite.

See the [full mini-SQLite conformance roadmap](./code/specs/mini-sqlite-full-conformance.md)
and [storage-sqlite](./code/specs/storage-sqlite.md).

### Data structures, systems, networking, and security

Package families include trees, tries, graphs, heaps, probabilistic structures,
filesystems, virtual memory, processes, event loops, reactors, IPC, TCP/HTTP,
RPC, device protocols, and operating-system abstractions.

Security and encoding work includes hashes, HMAC, HKDF, PBKDF2, scrypt, AES,
ChaCha20-Poly1305, Ed25519, X25519, compression formats, image/document
decoders, barcodes, and strict bounds-checked binary parsing.

The C and C++ lanes compile pure-ISO ports under GCC, Clang, Apple Clang, and
MSVC with strict conformance flags. See the
[C/C++ multi-compiler lane](./code/specs/CCPP01-c-cpp-iso-multicompiler-lane.md).

### Documents, graphics, and UI compilation

The content/rendering stack includes CommonMark, GFM, AsciiDoc, document ASTs,
HTML sanitization, Office file formats, font parsing, image codecs, layout,
draw/paint instruction systems, and native/GPU rendering backends.

Mosaic is the compile-time UI language. A typed UI description can be emitted
to web components, React, SwiftUI, Jetpack Compose, Flutter, Qt, XAML, HTML,
and paint-oriented backends. The same pattern lets a headless Rust engine power
multiple native and web hosts without duplicating product behavior.

See the [Mosaic overview](./code/specs/UI00-mosaic.md) and
[Mosaic compiler pipeline](./code/specs/UI16-mosaic-compiler-pipeline.md).

### Applications and product experiments

Programs are integration surfaces, not just toy examples:

- **Engram** combines a shared Rust core, native/WASM bridges, Mosaic UI work,
  Electron/browser hosts, and Anki compatibility.
- **task-app** is applying the same architecture to a general task/project
  engine. Its pure `task-core` model and operations/formula API have begun
  landing; the roadmap projects one model as checklist, todo, kanban, Gantt,
  flowchart, and table views with scheduling.
- **VisiCalc** exercises spreadsheet, UI, and multi-host compilation paths.
- Journal, checklist, browser-extension, language-tooling, IRC, ML, document,
  and hardware visualizer programs test other package families end to end.

See [Engram](./code/specs/engram-app.md) and the
[task-app overview](./code/specs/task-app-overview.md).

## Repository Layout

```text
.
|-- code/
|   |-- benchmarks/   reproducible performance experiments
|   |-- datasets/     shared data used by packages and programs
|   |-- fixtures/     shared binary/text fixtures and sample inputs
|   |-- grammars/     canonical .tokens and .grammar sources
|   |-- learning/     plain-language teaching material
|   |-- packages/     reusable libraries grouped by ecosystem
|   |-- programs/     executables, demos, apps, and visualizers
|   |-- scripts/      repository-wide generation, reporting, and safety tools
|   |-- sites/        website source/content
|   \-- specs/        architecture, package, and roadmap specifications
|-- .github/workflows CI, CodeQL, safety, publishing, releases, and deployment
|-- CHANGELOG.md      monorepo-level notable changes
|-- CLAUDE.md         repository policy and working conventions
\-- lessons.md        accumulated engineering failures and durable fixes
```

Package implementations currently span C, C++, C#, Dart, Elixir, F#, Go,
Haskell, Java, Kotlin, Lua, Perl, Python, Ruby, Rust, Swift, TypeScript, and
WebAssembly. Mosaic and Twig are domain-language buckets; Starlark is used for
build configuration rather than as an implementation ecosystem.

## Build System

The primary build tool is the Go program in
[`code/programs/go/build-tool`](./code/programs/go/build-tool/). It:

1. discovers packages through `BUILD` files;
2. evaluates Starlark build definitions where used;
3. reads ecosystem metadata such as Cargo, Go, Python, npm, Gradle, Swift,
   Ruby, Elixir, Dart, Haskell, and .NET manifests;
4. constructs the cross-language dependency graph;
5. maps a Git diff to changed packages and all transitive dependents;
6. validates declared dependencies and standalone build prerequisites;
7. schedules independent packages concurrently;
8. emits reusable and sharded build plans for CI.

`BUILD_windows` files provide legacy/platform-specific Windows commands while
the repository migrates more rules to OS-aware Starlark definitions.
`required_capabilities.json` files declare runtime/toolchain requirements for
packages that need capability-aware execution.

### Toolchains

The root [`mise.toml`](./mise.toml) pins the local toolchain baseline:

- Cabal `3.16.1.0` and GHC `9.14.1`;
- Dart `latest`;
- Go `latest`;
- Gradle `8.14` and Kotlin `2.1.20`;
- Lua `5.4`;
- Python `3.12`;
- Ruby `3.4`;
- Rust `stable`.

CI installs its own explicit versions and does not depend on mise.

### Build the build tool

On macOS/Linux:

```bash
cd code/programs/go/build-tool
go build -o ../../../../build-tool .
cd ../../../..
./build-tool -root . -diff-base origin/main -dry-run
```

On Windows:

```powershell
cd code\programs\go\build-tool
go build -o ..\..\..\..\build-tool.exe .
cd ..\..\..\..
.\build-tool.exe -root . -diff-base origin/main -dry-run
```

The diff-based plan compares committed branch history with `origin/main`.
Commit or otherwise verify the intended diff before treating its affected set
as authoritative. Use the package's own `BUILD`/test command for fast,
uncommitted iteration.

Useful repository checks:

```bash
# Validate and show the affected plan without running it.
./build-tool -root . -diff-base origin/main -dry-run -validate-build-files

# Show which CI toolchains the branch needs.
./build-tool -root . -diff-base origin/main -detect-languages

# Inspect cross-language coverage and reject naming collisions.
python code/scripts/package_parity_report.py --fail-on-collisions

# Regenerate the package-to-learning coverage backlog.
python code/scripts/learning_coverage_report.py --output code/learning/COVERAGE.md

# Test the build tool itself.
cd code/programs/go/build-tool && go test ./...
```

## CI and Quality Gates

The main CI workflow uses the same build planner:

- branch pushes build affected packages;
- pull requests validate affected work across Linux, macOS, and Windows when
  the selected languages require those runners;
- pushes to `main` create a forced five-shard full-build plan;
- Rust packages run per-package Clippy with warnings denied;
- CodeQL covers JavaScript/TypeScript, Python, Ruby, Go, and Swift;
- Miri blocks on unsafe-bearing runtime crates, with deeper integration checks
  running after merge and nightly;
- dedicated workflows test Rust-to-Node and Rust-to-Python native matrices;
- publish, release, and GitHub Pages workflows ship selected artifacts.

Packages are expected to carry tests, a README, a changelog, and publishable
ecosystem metadata. Libraries target at least 80% coverage, with 95% preferred
for most package code.

## Working in the Repository

Read [`CLAUDE.md`](./CLAUDE.md) and [`lessons.md`](./lessons.md) before making
changes. The recurring workflow is:

1. Fetch the latest `origin/main`.
2. Create a feature branch, preferably in a fresh worktree.
3. Write or refine the specification.
4. Add or update the relevant learning material.
5. Add tests before or alongside implementation.
6. Implement the smallest coherent change.
7. Update package README and CHANGELOG files.
8. Run the package tests and the affected-package plan.
9. Review the complete branch diff, including generated files and downstream
   consumers.
10. Run the required security review before pushing.

Important conventions:

- Do not commit directly to `main`.
- Shared grammar sources are authoritative; regenerate compiled grammars rather
  than hand-editing generated files.
- `BUILD` commands must be standalone and include all transitive local
  prerequisites in dependency order.
- New package scaffolding should go through the scaffold generator.
- Stage explicit files and keep build artifacts out of commits.
- If implementation and specification diverge, update the spec and document
  the decision.

## Where to Start

Choose a path based on what you want to understand:

| Interest | Start here |
|---|---|
| Whole computing stack | [Architecture overview](./code/specs/00-architecture.md) and [computing-stack guide](./code/learning/computer-architecture/computing-stack.md) |
| Language platform | [LANG-VM platform vision](./code/specs/LANG-PLATFORM-VISION.md) |
| Shared execution IR | [Interpreter IR](./code/specs/LANG01-interpreter-ir.md) |
| Source-level semantic IR | [Semantic IR](./code/specs/SIR00-semantic-ir.md) |
| Building a new language | [Generic language pipeline](./code/specs/LANG00-generic-language-pipeline.md) |
| SQL and SQLite | [Mini-SQLite conformance roadmap](./code/specs/mini-sqlite-full-conformance.md) |
| UI compilation | [Mosaic](./code/specs/UI00-mosaic.md) |
| Task/project engine | [task-app overview](./code/specs/task-app-overview.md) |
| Vault/security architecture | [Vault master spec](./code/specs/VLT00-vault-master.md) |
| Dependency planning | [Kahn's algorithm](./code/learning/algorithms/kahns-algorithm.md) and [build-tool README](./code/programs/go/build-tool/README.md) |
| Current engineering pitfalls | [Lessons learned](./lessons.md) |

The [learning index](./code/learning/README.md) collects the teaching-oriented
material by subject.

## Live Pages

Selected programs are deployed to
[adhithyan15.github.io/coding-adventures](https://adhithyan15.github.io/coding-adventures/):

- `arithmetic/` — adders, ALU, two's-complement, multiplication, and CPU steps;
- `arm1-web/` — ARM1 registers, pipeline, barrel shifter, and memory views;
- `busicom/` — an Intel 4004-powered Busicom calculator;
- `code39/` — text-to-barcode rendering through draw instructions;
- `commonmark/` — a from-scratch Markdown renderer;
- `electronics-visualizers/` — circuit and architecture visualizers;
- `engram/` and `engram-docs/` — the Engram app and documentation;
- `eniac/` — decimal vacuum-tube computation beside binary equivalents;
- `journal/` — the journaling application;
- `lattice/` — language documentation and a live transpiler;
- `logic-gates/` — gates, truth tables, and CMOS layouts;
- `ml-learning/` — interactive machine-learning demonstrations;
- `nib-web/` — Nib to Intel 4004 assembly/binary/HEX and simulation;
- `transistors/` — vacuum tube, BJT, MOSFET, and CMOS models.

The repository also deploys its blog and root landing page through dedicated
workflows.

## Copyright

Everything in this repository is copyrighted to Adhithya Rajasekaran.
Individual packages may be licensed separately.
