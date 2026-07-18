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
on July 18, 2026 after the paired Lua/Perl `fenwick-tree`, `binary-tree`,
`binary-search-tree`, `in-memory-data-store-protocol`, `avl-tree`, `tree-set`,
`skip-list`, `hyperloglog`, `trie`, `radix-tree`, and `resp-protocol` ports,
the paired `hash-functions` prerequisite, the paired `bloom-filter`, `hash-map`,
and `hash-set` ports, the paired `in-memory-data-store-engine` and
`in-memory-data-store` ports, and the paired C#/F# `wasm-module-encoder`,
`x25519`, `brainfuck-wasm-compiler`, `argon2i`, `argon2d`, and `argon2id`
ports, and the paired C#/F# `chacha20-poly1305`, `xml-lexer`, `block-ram`,
`nib-wasm-compiler`, `dartmouth-basic-lexer`, and `dartmouth-basic-parser`
ports:

| Current breadth | Packages | Missing slots to all 15 |
|---|---:|---:|
| Present in 10-15 languages | 172 | 319 |
| Present in 5-9 languages | 121 | 911 |
| Present in 2-4 languages | 157 | 1,972 |
| Present in one language | 701 | 9,814 |

The loop must not start by attempting 9,814 singleton ports. It should finish
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

The 172 packages present in at least ten implementation languages need 319
ports to reach all 15. After Priority 1, select work in this order:

| Language lane | Current high-consensus gaps | Pairing rule |
|---|---:|---|
| Python | 1 | Classify the remaining self-hosted `python-parser` carefully |
| Elixir | 0 | Complete; `python-parser` uses the shared grammar-driven frontend |
| Lua | 0 | Complete; paired data-structure/storage wave |
| Perl | 0 | Complete; paired data-structure/storage wave |
| C# | 5 | Move with F# |
| F# | 5 | Move with C# |
| Haskell | 34 | Dependency-shaped compression, graphics, ML, and protocol waves |
| Swift | 51 | Data structures and generated frontends before native app surfaces |
| Java | 58 | Move with Kotlin |
| Kotlin | 58 | Move with Java |
| Dart | 107 | Algorithms, data structures, codecs, grammar frontends, documents, and paint transforms first |

Go, Ruby, Rust, and TypeScript currently have no gaps within the 10-language
consensus set. They remain reference/template lanes for these waves.

The first seventeen paired Lua/Perl slices are complete: `fenwick-tree`,
`binary-tree`, `binary-search-tree`, `in-memory-data-store-protocol`,
`avl-tree`, `tree-set`, `skip-list`, `hyperloglog`, `trie`, `radix-tree`,
`resp-protocol`, `hash-functions`, `bloom-filter`, `hash-map`, `hash-set`,
`in-memory-data-store-engine`, and `in-memory-data-store` now have pure
implementations, package-native tests, metadata, and capability declarations
in both lanes.
The protocol slice establishes the dependency-free IR needed before the higher
in-memory data store layers move; the AVL slice supplies the ordered backend
for `tree-set`; and the dependency-free skip-list slice adds a span-augmented
ordered map with logarithmic rank and selection. The HyperLogLog slice adds a
fixed-memory approximate distinct counter with deterministic internal hashing.
The trie slice adds Unicode-aware prefix storage with sorted scans, pruning
deletion, and longest-prefix matching without introducing new dependencies.
The radix-tree slice compresses those paths into whole-substring edges while
retaining Unicode-safe splits, post-deletion merges, and mid-edge prefix scans.
The RESP2 slice adds a typed, binary-safe wire codec with distinct null bulk and
null array values plus an incremental decoder tested across arbitrary stream
fragmentation, establishing the wire layer needed by the higher storage stack.
The paired `hash-functions` prerequisite adds binary-safe FNV-1a, DJB2,
polynomial rolling, and MurmurHash3 implementations with deterministic analysis
helpers. It moves the package from 9 to 11 implementation lanes and unblocks
the remaining Bloom-filter and hash-map slices without external dependencies.
The Bloom-filter slice uses that prerequisite for correlation-mixed double
hashing, deterministic composite-value encoding, exact sizing helpers, and live
fill and false-positive statistics, reducing the paired high-consensus gap to
four packages per lane.
The hash-map slice implements both DT18 collision strategies from first
principles, including chaining, linear probing, tombstone deletion, automatic
resizing, deterministic DT17 bucket hashes, bulk access, merge, and clone-based
functional operations. It reduces the paired high-consensus gap to three
packages per lane and supplies the direct dependency for the `hash-set` slice.
The hash-set slice composes that map into a persistent DT19 collection with
copy-on-write add and remove, complete set algebra and relation predicates,
option preservation, identity-safe reference elements, and resize coverage for
both collision strategies. It reduces the paired high-consensus gap to two
packages per lane, leaving the higher in-memory data store layers as the final
paired wave.
The engine slice consumes the existing protocol IR and HyperLogLog packages to
provide binary-safe strings, typed collections, sorted sets, expiry, 16 logical
databases, deterministic response ordering, and the complete 57-command
execution surface. It reduces the paired high-consensus gap to one package per
lane, leaving only the top-level `in-memory-data-store` facade.
The facade slice composes the RESP2 streaming decoder, command protocol IR, and
execution engine into incremental and pipelined byte-stream entry points with
binary-safe response conversion. It moves the package from 11 to 13
implementation lanes and closes the remaining high-consensus gaps in both Lua
and Perl.

The first paired C#/F# slice is complete: `wasm-module-encoder` now has native
implementations in both lanes, built on their existing `wasm-leb128` and
`wasm-types` packages. Parser round-trip tests cover all WebAssembly 1.0 module
sections and import descriptor validation. The package now spans 12 lanes,
reduces each paired high-consensus gap to 16, and unlocks the later
`brainfuck-wasm-compiler` and `nib-wasm-compiler` ports.

The second paired C#/F# slice is complete: `x25519` now has native,
dependency-free implementations in both lanes using the RFC 7748 Montgomery
ladder over `2^255 - 19`. RFC scalar-multiplication, Diffie-Hellman,
high-bit-masking, low-order rejection, and 1,000-round iterated vectors provide
conformance coverage. The package now spans 12 implementation lanes and
reduces each paired high-consensus gap to 15.

The third paired C#/F# slice is complete: `brainfuck-wasm-compiler` now builds
typed WebAssembly modules through the new native encoders in both lanes.
Package-native tests cover source filtering, balanced and depth-limited loops,
8-bit cell and pointer emission, optional WASI I/O imports, file output,
parser/validator round trips, and locally supported runtime execution. The
package now spans 13 implementation lanes and reduces each paired
high-consensus gap to 14.

The fourth paired C#/F# slice is complete: `argon2i` now implements the RFC
9106 data-independent memory-hard password hash in both lanes on top of their
existing BLAKE2b packages. RFC vectors cover secret keys, associated data,
multiple lanes and passes, variable tag lengths, and address-block rollover.
The package now spans 12 target implementation lanes and reduces each paired
high-consensus gap to 13.

The fifth paired C#/F# slice is complete: `argon2d` now implements the RFC
9106 data-dependent memory-hard password hash in both lanes on top of their
existing BLAKE2b packages. RFC vectors cover secret keys, associated data,
multiple lanes and passes, variable tag lengths, and deterministic memory-cost
rounding. The package now spans 12 target implementation lanes and reduces
each paired high-consensus gap to 12.

The sixth paired C#/F# slice is complete: `argon2id` now implements RFC 9106's
recommended hybrid password hash in both lanes, using data-independent
addresses for the first half of pass zero and data-dependent addresses
thereafter. Canonical vectors cover the address-mode transition, secret keys,
associated data, multiple lanes and passes, and variable tag lengths. The
package now spans 12 target implementation lanes and reduces each paired
high-consensus gap to 11.

The seventh paired C#/F# slice is complete: `chacha20-poly1305` now implements
the self-contained RFC 8439 ChaCha20 stream cipher, Poly1305 one-time MAC, and
combined AEAD construction in both lanes. Canonical block, stream, MAC, and
AEAD vectors cover the full construction, while multiblock round trips and
tamper tests verify counter progression and authenticate-before-decrypt
behavior. The package now spans 12 implementation lanes and reduces each
paired high-consensus gap to 10.

The eighth paired C#/F# slice is complete: `xml-lexer` now provides native,
context-sensitive scanners in both lanes while reusing their existing lexer
token models. Package-local state transitions match the shared XML grammar's
content, tag, comment, CDATA, and processing-instruction groups; tests cover
namespaces, quoted attributes, entity and character references, significant
whitespace, token positions, malformed input, and EOF behavior. The package
now spans 12 implementation lanes, reduces the high-consensus backlog to 327
slots, and leaves 9 paired gaps in each lane.

The ninth paired C#/F# slice is complete: `block-ram` now models SRAM cells,
row-addressed arrays, rising-edge single-port and true dual-port RAM, all three
read-during-write modes, same-address collision detection, and configurable
FPGA-style width/depth aspect ratios in both lanes. Package-native tests cover
cross-port visibility, edge behavior, reconfiguration clearing, defensive
copies, and invalid signals and dimensions. The package now spans 12 target
implementation lanes, reduces the high-consensus backlog to 325 slots, and
leaves 8 paired gaps in each lane.

The tenth paired C#/F# slice is complete: `nib-wasm-compiler` now compiles the
portable typed Nib `u4` function subset in both lanes. Native parsers cover
literals, parameters, nested calls, and wrapping `+%` addition, while the
existing `wasm-module-encoder`, `wasm-types`, and `wasm-leb128` packages produce
validated modules that export every declared function. Package-native tests
cover executable literals and calls, wrapping-opcode validation, malformed
source, depth and size limits, defensive results, and optional file output. The
package now spans 13 implementation lanes, reduces the high-consensus backlog
to 323 slots, and leaves 7 paired gaps in each lane.

The eleventh paired C#/F# slice is complete: `dartmouth-basic-lexer` now loads
the shared token grammar from an embedded resource in both lanes. The native
wrappers normalize case-insensitive token values, relabel only physical-line
labels as `LINE_NUM`, preserve string case without quotes, and suppress `REM`
bodies while retaining their terminating newline. Package-native tests cover
operators, numeric formats, functions, unknown input, CRLF positions, blank
lines, and multi-line remark recovery. The package now spans 12 implementation
lanes, reduces the high-consensus backlog to 321 slots, leaves 6 paired gaps in
each lane, and unlocks the dependency-safe `dartmouth-basic-parser` slice.

The twelfth paired C#/F# slice is complete: `dartmouth-basic-parser` now
combines those native lexers with each lane's grammar-driven parser and the
shared BASIC grammar embedded as a package resource. The adapters enforce
complete non-EOF token consumption so malformed statements cannot collapse to
the grammar's valid empty program, while package-native tests exercise all 17
statement forms, expression precedence, configured and one-shot APIs, empty
and bare-line programs, and syntax failures. The package now spans 12
implementation lanes, reduces the high-consensus backlog to 319 slots, and
leaves 5 paired gaps in each lane.

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

The singleton inventory is led by 501 Rust, 86 Python, and 83 TypeScript
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
