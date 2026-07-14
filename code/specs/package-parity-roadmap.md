# Package Parity Roadmap

## Goal

Close package gaps across implementation languages without confusing directory
equality with useful parity. Every deterministic, language-agnostic package
should have a pure implementation in each established language when practical.
Platform, ABI, browser, firmware, and accelerator packages should instead have
honest native implementations or thin tested wrappers.

This roadmap is the durable work queue for the autonomous package-parity PR
loop. It is ordered by leverage: repair the inventory, finish nearly complete
families, then classify and port sparse families in dependency-shaped waves.

## Canonical Inventory

Run:

```sh
python code/scripts/package_parity_report.py --format markdown
python code/scripts/package_parity_report.py --format json
python code/scripts/package_parity_report.py --format csv
```

The reporter reads Git-visible files: tracked files plus untracked files that
are not ignored. It therefore sees newly scaffolded packages without treating
`target`, `node_modules`, `.pytest_cache`, or other ignored build output as
packages.

Package identity is case- and punctuation-insensitive. `directed-graph`,
`directed_graph`, and `Directed.Graph` are one identity. The reporter retains
the original directories and reports collisions when one language contains
multiple directories for the same identity.

### Established implementation lanes

- C#
- Dart
- Elixir
- F#
- Go
- Haskell
- Java
- Kotlin
- Lua
- Perl
- Python
- Ruby
- Rust
- Swift
- TypeScript

### Separately classified lanes

- C++ is an emerging implementation lane. It needs package/scaffold maturity
  before it can join the all-language completion denominator.
- WASM is an execution-target lane, not a requirement for every source package.
- Mosaic and Twig are domain/source-language lanes.
- Starlark is a build/configuration lane.

## July 10, 2026 Baseline

The tracked tree at `8efcb2d5b` contains:

| Metric | Count |
|---|---:|
| Established implementation languages | 15 |
| Tracked package directories in those languages | 4,096 |
| Distinct normalized package identities | 1,102 |
| Package/language slots after identity normalization | 4,094 |
| Missing slots for literal all-language parity | 12,436 |
| Rust identities | 873 |
| Python identities | 494 |
| TypeScript identities | 436 |
| Rust singletons | 465 |
| Python singletons | 88 |
| TypeScript singletons | 81 |
| Packages present in all 15 languages | 34 |

Rust drift is recent as well as cumulative. From June 10 to July 10, Rust added
127 package directories out of 185 total net additions across the 15 lanes.
Rust-only identities grew from 373 to 465.

The previous April baseline had 377 Rust and 377 Python packages. It is no
longer a useful current planning baseline.

## Parity Classes

Every sparse package must receive one of these classifications before a porting
wave treats it as a gap:

| Class | Meaning | Expected action |
|---|---|---|
| `portable` | Pure algorithm, data structure, IR, codec, validator, deterministic transform, simulator, or grammar frontend | Pure implementation in every established language |
| `native-source` | The package directly owns OS, ABI, GPU, firmware, or hardware behavior | Keep the appropriate native implementation |
| `wrapper` | Thin language-facing binding to a native source of truth | Test the wrapper; do not count it as a missing pure port |
| `web-only` | Browser, DOM, Canvas, IndexedDB, Vite, or Web Audio behavior | Keep in web-capable lanes |
| `target-specific` | Compiler backend or artifact writer meaningful only for a particular target | Port only where the target is supported |
| `not-applicable` | The package has no coherent role in the language lane | Document the exception |

Directory presence is not completion. A completed pure port needs matching API
semantics, shared fixtures or reference vectors, package-native tests, README,
CHANGELOG, metadata, BUILD/BUILD_windows where applicable, and CI coverage.

## Work Inventory

The missing matrix is heavily concentrated in singleton packages. Regenerated
on July 14, 2026 after the Swift CLI-builder port:

| Current breadth | Packages | Missing slots to all 15 |
|---|---:|---:|
| Present in 10-15 languages | 171 | 375 |
| Present in 5-9 languages | 122 | 917 |
| Present in 2-4 languages | 157 | 1,972 |
| Present in one language | 694 | 9,716 |

The loop must not start by attempting 9,716 singleton ports. It should finish
the broadly established portable core, then classify the sparse majority.

## Priority 0: Inventory And Identity Integrity

Completed. The reporter now inventories Git-visible files, emits Markdown,
JSON, and CSV, classifies package lanes, detects canonical collisions, and is
covered by CI unit tests. The conflicting `ruby/b_tree` and
`ruby/b_plus_tree` shadow packages were removed in favor of the authoritative
DT11/DT12 `ruby/b-tree` and `ruby/b-plus-tree` implementations. CI now rejects
new canonical identity collisions with `--fail-on-collisions`.

## Priority 1: Complete The 14-Of-15 Set

Priority 1 is complete. Every package that entered this wave at 14-of-15 now
has an implementation in all 15 language lanes.

### Dart: complete

Completed in the Dart lane: `heap`, `bitset`, `pixel-container`,
`image-point-ops`, `logic-gates`, `image-geometric-transforms`, `toml-lexer`.
The grammar-driven `mosaic-lexer`/`mosaic-parser` and
`algol-lexer`/`algol-parser` pairs are also complete.
The dependency-shaped DT11/DT12 `b-tree`/`b-plus-tree` pair is complete.

### Haskell: complete

Completed in the Haskell lane: `activation-functions`, `caesar-cipher`,
`huffman-tree`, `huffman-compression`, `lz77`, `lzss`, `lzw`.

### Swift: complete

Completed in the Swift lane: `wasm-simulator`, `cli-builder`,
`sql-execution-engine`.

Port dependency families together when doing so avoids temporary broken package
graphs. Grammar-generated lexer/parser pairs should be generated from the shared
grammar sources rather than independently handwritten.

## Priority 2: Complete The High-Consensus Core

The 171 packages present in at least ten implementation languages need 374
ports to reach all 15. After Priority 1, select work in this order:

| Language lane | Current high-consensus gaps | Pairing rule |
|---|---:|---|
| Python | 3 | Pair `in-memory-data-store` packages; classify self-hosted `python-parser` carefully |
| Elixir | 1 | Complete alongside the Python parser/frontend decision |
| Lua | 16 | Pair with Perl data-structure/storage wave |
| Perl | 16 | Pair with Lua data-structure/storage wave |
| C# | 17 | Move with F# |
| F# | 17 | Move with C# |
| Haskell | 34 | Dependency-shaped compression, graphics, ML, and protocol waves |
| Swift | 50 | Data structures and generated frontends before native app surfaces |
| Java | 57 | Move with Kotlin |
| Kotlin | 57 | Move with Java |
| Dart | 106 | Algorithms, data structures, codecs, grammar frontends, documents, and paint transforms first |

Go, Ruby, Rust, and TypeScript currently have no gaps within the 10-language
consensus set. They remain reference/template lanes for these waves.

Recommended family order:

1. Leaf algorithms and data structures.
2. Hashing, crypto, compression, and deterministic codecs.
3. Shared JSON/document/IR models and serializers.
4. Grammar-generated lexer/parser pairs.
5. SQL/storage packages in dependency order.
6. Compiler and VM families with their local IR, validator, encoder, and runtime
   dependencies in the same wave.

## Priority 3: Expand The Portable Core

After the high-consensus set is complete:

1. Complete packages already present in 8-9 languages.
2. Complete packages present in 5-7 languages.
3. Recompute the matrix after every merged wave.
4. Prefer families with existing cross-language fixtures and low dependency
   fan-out.
5. Add missing shared conformance fixtures before porting when current tests are
   language-specific and cannot prove equivalent behavior.

This phase covers 122 package identities and 917 current missing slots.

## Priority 4: Classify Sparse And Singleton Families

The singleton inventory is led by 501 Rust, 86 Python, and 82 TypeScript
packages. Classify families before opening implementation PRs.

### Likely portable Rust-led families

- `closure-*` compiler passes
- `dsp-*` algorithms
- `iir-*` IR passes and deterministic target emitters
- portable `image-codec-*` packages
- `state-machine-*` tokenization, serialization, and compilation
- language runtimes and frontends such as `r-*` and `twig-*`, when their
  dependency stacks are ready
- deterministic portions of `vault-*`, `adjudication-*`, and `smart-home-*`

### Likely native, wrapper, or target-specific Rust-led families

- `*-bridge`, `*-capi`, `*-jni`, `*-napi`, and `*-native`
- `silicon-rust-*` bindings
- board firmware and physical transport packages
- OS paint/window backends
- CUDA, Metal, Vulkan, Direct2D, GDI, OpenCL, and similar accelerator/platform
  implementations

### Python-led families requiring classification

- CPU/ISA simulators and gate-level models
- JVM/CLR/BEAM artifact and runtime packages
- Prolog and logic-runtime stacks
- Tetrad, Twig, and Oct compiler backends
- native data-structure wrappers

### TypeScript-led families requiring classification

- the 57-package `forme-*` web/static-site family
- browser, IndexedDB, Vite, Canvas, Web Audio, and UI packages
- layout and document-to-paint packages
- Mosaic web emitters

For each family, add or identify a portable contract spec, dependency order,
reference implementation, shared fixtures, and explicit exception list before
the first port PR.

## Priority 5: Conformance And Regression Prevention

1. Give each portable family a language-neutral fixture corpus or oracle.
2. Add package-level conformance runners where directory presence currently
   masks API or semantic drift.
3. Extend the parity reporter with explicit applicability data rather than
   hard-coding exceptions in reporting logic.
4. Fail CI on unclassified new package buckets.
5. Keep the canonical-collision CI gate enabled.
6. Add a policy check: a new portable singleton must include either another
   language implementation or a declared parity work item/classification.
7. Track package maturity separately from structural presence: manifest,
   source, tests, BUILD, README, CHANGELOG, conformance status, and last verified
   revision.

## Autonomous Loop Protocol

Only one parity PR should be active at a time.

1. Fetch `origin/main` and verify the prior PR state.
2. If CI fails, inspect the actual GitHub Actions logs, make a focused fix, run
   local verification, and push to the same PR.
3. If the branch conflicts with `main`, update it carefully and verify the full
   PR diff contains only intended work.
4. If checks are pending, keep monitoring.
5. If the PR is merged, regenerate the report from the new `origin/main`, update
   priorities with any newly discovered work, and select the highest-impact
   unblocked item.
6. Create a fresh `codex/` branch, implement one coherent dependency-shaped
   work item, validate it, push it, and open the next PR.
7. Continue until the report has no unclassified or eligible portable gaps.

Every PR must state what changed, why the selected slice is next, tests run,
remaining gaps, and any packages deliberately classified as non-portable.

## Completion Definition

The parity program is complete when:

- every package identity is classified;
- every `portable` package has a tested pure implementation in every established
  implementation language, or an explicit reviewed `not-applicable` exception;
- native, wrapper, web-only, and target-specific packages have honest tested
  coverage in their applicable lanes;
- canonical identity collisions are zero;
- the reporter and conformance checks run in CI;
- adding a new package cannot silently create an unplanned singleton;
- the generated matrix contains no eligible unowned gap.
