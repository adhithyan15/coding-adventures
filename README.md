# Coding Adventures

Coding Adventures is a learning-first monorepo for understanding computers by
building the layers ourselves: logic gates and processors, parsers and virtual
machines, compilers and runtimes, storage engines, UI systems, and complete
applications. Alongside that computing work, a human-language curriculum builds
cumulative books and short practice lessons from shared content.

It is part computing-stack curriculum, part polyglot package laboratory, part
language-platform project, and part application sandbox. The point is not merely
to collect implementations. The point is to make each idea inspectable,
testable, comparable across languages, and teachable.

Start with the [live labs](https://adhithyan15.github.io/coding-adventures/)
to explore in a browser, the [learning index](./code/learning/README.md) to
read, or the [project tracks](#major-project-tracks) below to find the source.

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

### Repository snapshot — October 20, 2026

| Surface | Current scale |
|---|---:|
| Package implementation directories | 5,209 |
| Program and application directories | 217 |
| Grammar families | 57 |
| Top-level Markdown specifications and roadmaps | 1,778 |
| Human-language curriculum tracks | 23 |
| TypeScript Forme packages | 81 |
| CI, release, and deployment workflows | 46 |

Measured from tracked paths at `96d45dfff5` on `main`: packages and programs
count distinct `code/{packages,programs}/<ecosystem>/<name>` directories;
grammars count `code/grammars/<family>` directories; specifications count
`code/specs/*.md`; Forme counts TypeScript `forme-*` directories; workflows
count `.github/workflows/*.yml`. Curriculum tracks come from the active entries
in [`core/languages.json`](./code/learning/human-languages/core/languages.json).
Language mirrors count separately, and directory counts do not imply feature
completion. Learning documents are increasingly sharded, so file count is no
longer a useful measure of lesson coverage.

Current integration threads include Mosaic's native fidelity, Trestle's shared
task engine, Forme's authoring product, language execution and conformance,
circuit simulation, and the human-language books. The generated reports and
project acceptance ledgers remain authoritative as those tracks grow.

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
Coverage varies by frontend, feature, and backend; the
[execution matrix](./code/specs/LANG-PLATFORM-MATRIX.md) and
[non-ALGOL completion backlog](./code/specs/LANG-VM-NON-ALGOL-BACKLOG.md)
separate working routes from the platform vision.
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

The hardware-design track also explores HDL elaboration, simulation, synthesis,
technology mapping, FPGA export, ASIC layout, and verification. SPICE adds
analog DC, AC, transient, noise, and mixed-signal analysis. Follow the
[silicon-stack index](./code/specs/silicon-stack.md) and the implemented
[SPICE engine](./code/packages/rust/spice-engine/); each package's tests and
compatibility notes define its supported subset.

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
boxing/unboxing, heap values, truthiness, and dynamic arithmetic. The native
AOT path now includes precise roots, generational collection, compaction,
incremental collection, variable-length reference arrays, and GC-managed Twig
lists, closures, records, unions, and strings.

The numeric and symbolic side includes arbitrary-precision integers, rationals,
decimals and binary floats; computer algebra packages; rewrite systems;
constraint solving; logic programming; statistics; and the ADJ rule/formula
language. Exactness and explicit lossy conversions are treated as design
properties rather than incidental implementation details.

See the [dynamic-value substrate](./code/specs/DVAL01-generic-dynamic-value-substrate.md)
and [Twig native-AOT GC coverage](./code/specs/AOT00-twig-native-gc-coverage.md).

### Rules, formulas, and auditable reasoning

ADJ is a typed rule, fact, table, and formula system for reasoning that can be
inspected and replayed. Its packages cover exact quantities, bidirectional
formulas, rule adjudication, provenance, uncertainty, explanation rendering,
and state-machine composition. Formula and fact libraries exercise the same
engine across physics, chemistry, biology, medicine, and other domains.

The broader adjudication framework separates source decomposition, structural
coverage, polarity and modality checks, adversarial verification,
clarification, execution, and audit trails. This makes a conclusion more than
an opaque answer: the system can show which facts and rules fired, what evidence
was missing, and how the result was derived.

See the [ADJ overview](./code/specs/ADJ-OVERVIEW.md),
[rule substrate](./code/specs/ADJ-RULE-SUBSTRATE.md), and
[formula libraries](./code/specs/ADJ-FORMULA-LIBRARIES.md).

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
OCaml is a known emerging implementation lane governed by
[OCAML01](./code/specs/OCAML01-emerging-lane-contract.md); its packages remain
outside the established parity denominator until the scaffold, resolver,
capability, build-tool, package, and three-platform promotion gates pass.
The two scaffold-generator front doors implement the byte-stable library and
program contract in
[OCAML02](./code/specs/OCAML02-scaffold-infrastructure.md), including exact
direct opam/Dune constraints, Alcotest, formatting, coverage, and capability
profiles. [OCAML03](./code/specs/OCAML03-ci-toolchain.md) supplies the
CI-toolchain gate with digest-checked per-platform solver locks,
installed-package receipts, and a pinned three-platform workflow that performs
both a fresh-solve comparison and separate locked execution of the two scaffold
kinds. Hosted-runner image values are recorded as diagnostics, not as immutable
host attestations.
The parity report's JSON schema is version 3, and CSV consumers must select
presence columns by header name because recognized emerging lanes extend the
matrix.

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

### Forme: authoring, preview, and publishing

Forme composes typed, capability-aware stages into authoring pipelines. Its 81
TypeScript packages cover content sources, document transforms, collections,
routing, Style IR, static and terminal rendering, interactivity, feeds, search,
asset fingerprints, and deployment. Python and Rust SDKs extend the plugin
boundary beyond TypeScript.

The [completion roadmap](./code/specs/FM00-forme-completion-roadmap.md) records
the Headless v0, Extensible v1, and Authoring v1 milestones as complete, with
their acceptance evidence. The CLI builds, checks, previews, watches, and
deploys; installed plugins run with mediated capabilities and native sandboxes;
the desktop authoring host adds editing, live preview, recovery, and local
publication. Future operational work is tracked against those contracts.

This repository uses Forme for its own
[landing page](./code/sites/landing-page/) and [blog](./code/sites/blog/).
The landing page is a seven-stage pipeline; the blog composes HTML and terminal
outputs, a themed index, feeds, a sitemap, and progressive enhancement.
[`forme-hello-world`](./code/programs/typescript/forme-hello-world/) and
[`forme-doc-demo`](./code/programs/typescript/forme-doc-demo/) provide smaller
examples to study.

Start with the [Forme vision](./code/specs/FM00-forme-vision.md),
[CLI and development server](./code/specs/FM07-forme-cli-dev-server.md),
[deploy runner](./code/specs/FM08-forme-deploy-runner.md), and
[authoring shell](./code/specs/FM09-forme-authoring-shell.md).

### Human languages: books, scripts, and cumulative practice

The learning tree now includes 23 active human-language tracks. Shared content
feeds chapter-sized books, short practice units, narration exports, and the
Language Ladder app. Lessons connect vocabulary to etymology and related
languages, introduce grammar gradually, and teach unfamiliar scripts one
letter at a time. Reading, writing, listening, and speaking develop through
cumulative tasks with explicit chapter payoffs.

Spanish leads the shared course spine and Tamil leads the script addendum;
other tracks adapt that structure to their own sounds, grammar, and writing
systems. Pre-A1 through C2 is the course design target. The strict level gate
certifies a level only when every criterion holds there and at every level
below it. It currently places Spanish, Italian, Portuguese, French, German, and
Latin at B1, twelve tracks at A2, and four at A1; Chinese has not yet closed
pre-A1. Chapter and assessment ledgers track the remaining coverage.

Start with the [human-language index](./code/learning/human-languages/README.md),
[curriculum framework](./code/specs/HL00-human-language-curriculum-framework.md),
[replication method](./code/specs/HL13-spine-layout-and-replication.md), or the
[published book catalog](https://adhithyan15.github.io/coding-adventures/human-languages/books/).

### Machine learning from individual updates to neural graphs

The ML packages cover losses, gradient descent, matrix operations, activations,
normalization, and neural-network execution. The
[neural graph VM](./code/specs/NN00-neural-graph-vm.md) connects models to a
shared execution representation and backend implementations.

The [ML Learning Lab](./code/programs/typescript/ml-learning-visualizer/) has
23 workbenches, including training and optimization microscopes, hidden layers,
convolution, recurrent state, and small decoder training. Intermediate values,
gradients, and parameter updates remain visible. See the
[machine-learning learning material](./code/learning/machine-learning/) and
[live lab](https://adhithyan15.github.io/coding-adventures/ml-learning/).

### Applications and product experiments

Programs are integration surfaces, not just toy examples:

- **Engram** combines a shared Rust core, native/WASM bridges, Mosaic UI work,
  Electron/browser hosts, and Anki compatibility.
- **Trestle / task-app** uses a pure Rust task/project engine for automatic
  scheduling, persistent tasks, and shared projections. Mosaic authors its UI
  once for the web and native hosts. The
  [product acceptance ledger](./code/specs/task-app-platform-completion-v1.md)
  distinguishes interactive release targets from HTML, Web Components, and
  Paint verification outputs; native style gaps remain explicitly tracked.
- **Venture** composes the repository's URL, HTTP, HTML, layout, paint, image,
  windowing, and platform bridges into a native educational web browser. The
  shared Mosaic chrome and host controller handle navigation, bookmarks,
  history, page utilities, and scrolling across native host integrations.
- **smart-home** provides a local-first supervised runtime for normalized device
  state, commands, automations, policy, and audit. Hue, Zigbee, and Z-Wave work
  exercise the adapter boundary without making one vendor protocol the core.
- **language-ladder** turns the human-language curriculum into an interactive
  browser for concepts, scripts, syllabaries, lessons, and review progress.
- **VisiCalc** exercises spreadsheet, UI, and multi-host compilation paths.
- Journal, browser-extension, language-tooling, IRC, ML, document,
  and hardware visualizer programs test other package families end to end.

See [Engram](./code/specs/engram-app.md) and the
[task-app overview](./code/specs/task-app-overview.md),
[Venture](./code/specs/BR01-venture-browser.md), and the
[smart-home runtime](./code/specs/D23-smart-home-runtime.md).

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
|-- CHANGELOG.d/      monorepo-level notable changes, one entry per file
|                     (see its `_meta.md`)
|-- CLAUDE.md         repository policy and working conventions
\-- lessons.d/        accumulated engineering failures and durable fixes,
                     one lesson per file (see its `_meta.md`)
```

Package inventory buckets currently recognize C, C++, C#, Dart, Elixir, F#,
Go, Haskell, Java, Kotlin, Lua, OCaml, Perl, Python, Ruby, Rust, Swift,
TypeScript, and WebAssembly. C, C++, and OCaml are emerging rather than
established parity lanes. Mosaic and Twig are domain-language buckets; Starlark
is used for build configuration rather than as an implementation ecosystem.

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

# Classify Python uv fronts that cannot repeat in the same checkout.
python code/scripts/python_uv_build_front_audit.py --format markdown

# Regenerate the package-to-learning coverage backlog.
python code/scripts/learning_coverage_report.py --output code/learning/COVERAGE.md

# Test the build tool itself.
cd code/programs/go/build-tool && go test ./...
```

The parity report distinguishes established implementation lanes from emerging
ones. Emerging OCaml packages are visible in JSON, Markdown, and CSV output but
do not create missing slots until the explicit promotion contract is completed.

## CI and Quality Gates

The main CI workflow uses the same build planner:

- pull requests build affected packages across Linux, macOS, and Windows when
  the selected languages require those runners;
- pushes to `main` create a forced sharded full-build plan;
- Rust packages run per-package Clippy with warnings denied;
- CodeQL covers JavaScript/TypeScript, Python, Ruby, Go, and Swift;
- Miri blocks on unsafe-bearing runtime crates, with deeper integration checks
  running after merge and nightly;
- dedicated workflows test Rust-to-Node and Rust-to-Python native matrices;
- publish, release, and GitHub Pages workflows ship selected artifacts.

Packages are expected to carry tests, a README, a changelog (often sharded),
and publishable ecosystem metadata. Libraries target at least 80% coverage, with 95% preferred
for most package code.

## Working in the Repository

Read [`CLAUDE.md`](./CLAUDE.md) and [`lessons.d/`](./lessons.d/_meta.md) before
making changes. The recurring workflow is:

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
| WebAssembly internals | [WebAssembly from bytes to execution](./code/learning/language-tooling/webassembly-from-bytes-to-execution.md) |
| Rules, formulas, and auditable reasoning | [ADJ overview](./code/specs/ADJ-OVERVIEW.md) |
| SQL and SQLite | [Mini-SQLite conformance roadmap](./code/specs/mini-sqlite-full-conformance.md) |
| UI compilation | [Mosaic](./code/specs/UI00-mosaic.md) |
| Authoring and static sites | [Forme vision](./code/specs/FM00-forme-vision.md) and [documentation-site vision](./code/specs/DOC00-docs-vision.md) |
| Task/project engine | [Trestle / task-app](./code/programs/mosaic/task-app/) and [platform acceptance](./code/specs/task-app-platform-completion-v1.md) |
| Human-language books and practice | [Curriculum index](./code/learning/human-languages/README.md) |
| Circuit simulation and hardware design | [SPICE engine](./code/packages/rust/spice-engine/) and [silicon stack](./code/specs/silicon-stack.md) |
| Neural networks and training | [ML Learning Lab](./code/programs/typescript/ml-learning-visualizer/) |
| Native browser pipeline | [Venture browser](./code/specs/BR01-venture-browser.md) |
| Smart-home runtime | [Smart-home runtime](./code/specs/D23-smart-home-runtime.md) |
| Learning-content backlog | [Learning coverage roadmap](./code/learning/ROADMAP.md) |
| Vault/security architecture | [Vault master spec](./code/specs/VLT00-vault-master.md) |
| Dependency planning | [Kahn's algorithm](./code/learning/algorithms/kahns-algorithm.md) and [build-tool README](./code/programs/go/build-tool/README.md) |
| Current engineering pitfalls | [Lessons learned](./lessons.d/_meta.md) |

The [learning index](./code/learning/README.md) collects the teaching-oriented
material by subject.

## Live Pages

Selected programs are deployed to
[adhithyan15.github.io/coding-adventures](https://adhithyan15.github.io/coding-adventures/):

- [Arithmetic](https://adhithyan15.github.io/coding-adventures/arithmetic/) —
  adders, ALU, two's-complement, multiplication, and CPU steps;
- [ARM1](https://adhithyan15.github.io/coding-adventures/arm1-web/) — registers,
  pipeline, barrel shifter, and memory views;
- [Busicom](https://adhithyan15.github.io/coding-adventures/busicom/) and
  [ENIAC](https://adhithyan15.github.io/coding-adventures/eniac/) — interactive
  historical machines;
- [Transistors](https://adhithyan15.github.io/coding-adventures/transistors/),
  [Logic Gates](https://adhithyan15.github.io/coding-adventures/logic-gates/),
  and [Electronics Visualizers](https://adhithyan15.github.io/coding-adventures/electronics-visualizers/)
  — inspectable hardware layers;
- [Nib](https://adhithyan15.github.io/coding-adventures/nib-web/),
  [Lattice](https://adhithyan15.github.io/coding-adventures/lattice/), and
  [CommonMark](https://adhithyan15.github.io/coding-adventures/commonmark/) —
  language and compiler playgrounds;
- [Code 39](https://adhithyan15.github.io/coding-adventures/code39/) — barcode
  rendering through the repository's draw-instruction pipeline;
- [Engram](https://adhithyan15.github.io/coding-adventures/engram/) and
  [Engram Docs](https://adhithyan15.github.io/coding-adventures/engram-docs/) —
  a spaced-repetition application and its documentation;
- [Journal](https://adhithyan15.github.io/coding-adventures/journal/) —
  local-first journaling with Markdown editing and browser persistence;
- [ML Learning Lab](https://adhithyan15.github.io/coding-adventures/ml-learning/)
  — training microscopes, neural-network workbenches, and visible gradients;
- [Language Ladder](https://adhithyan15.github.io/coding-adventures/language-ladder/)
  and the [human-language book catalog](https://adhithyan15.github.io/coding-adventures/human-languages/books/)
  — interactive lessons and generated books;
- [Forme notes](https://adhithyan15.github.io/coding-adventures/blog/2026-05-15-hello-forme.html)
  — articles produced by the repository's own authoring pipeline.

The root page is generated from [the Forme landing-page source](./code/sites/landing-page/).
Its Pages workflow builds changes on pull requests and publishes after they
land on `main`.

## Copyright

Everything in this repository is copyrighted to Adhithya Rajasekaran.
Individual packages may be licensed separately.
