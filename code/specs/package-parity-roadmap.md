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

- C, C++, and OCaml are emerging implementation lanes. They need their
  respective package, scaffold, build, security, and CI maturity gates before
  they can join the all-language completion denominator.
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

The missing matrix is heavily concentrated in singleton packages. The current
working inventory was regenerated on August 2, 2026 from `80612eb7` after
merged PR #9539 added the mixed Rust HEOS read-only integration and merged PR
#9537 completed the Swift build-tool UTF-8 slice without creating another
implementation package directory. The inventory contains
1,221 normalized implementation identities across 4,369 established-lane
package slots and found zero canonical collisions or unknown language buckets:

| Current breadth | Packages | Missing slots to all 15 |
|---|---:|---:|
| Present in 10-15 languages | 172 | 271 |
| Present in 5-9 languages | 121 | 911 |
| Present in 2-4 languages | 157 | 1,970 |
| Present in one language | 771 | 10,794 |

The loop must not start by attempting 10,780 singleton ports. It should finish
the broadly established portable core, then classify the sparse majority.

The current working inventory on
`80612eb7ba39d864c5ffa27f2cb3ae78e92705c1` is collision-clean at 1,221
normalized implementation identities, 4,369 implementation slots, 172
high-consensus packages, 271 high-consensus missing slots, 771 singletons, 576
Rust singletons, zero canonical collisions, and zero unknown language buckets.
The sixteen newest mixed Rust identities are `smart-home-camera-media`,
`smart-home-onvif-integration`, `smart-home-shelly-integration`,
`smart-home-wled-integration`, `smart-home-govee-lan-integration`,
`smart-home-lifx-lan-integration`, `smart-home-kasa-lan-integration`,
`smart-home-reolink-integration`, `smart-home-roku-ecp-integration`,
`smart-home-wemo-upnp-integration`, `smart-home-sonos-upnp-integration`,
`smart-home-nanoleaf-local-integration`,
`smart-home-tasmota-local-integration`, and
`smart-home-fronius-local-integration`, plus
`smart-home-homewizard-energy-integration`, plus
`smart-home-heos-cli-integration`. All are
mixed splits rather than blind parity ports: camera grant policy,
generation-bound lease state, quotas, and redacted audit are portable, while
authenticated host context and media delivery remain native mediation; ONVIF
discovery/SOAP parsing, deterministic UsernameToken construction, origin policy,
and projection are portable, while sockets, TLS, trusted time/randomness, Vault
access, process I/O, and allowlists remain native; Shelly JSON/RPC normalization,
projection, stable identities, and command planning are portable, while mDNS,
DNS/TCP, plaintext LAN HTTP, trusted time, console I/O, endpoint policy, effect
ordering, future credentials, and capability profiles remain native.
WLED DTO validation, master/segment projection, capability-bit interpretation,
state normalization, and command planning are portable, while mDNS, DNS/TCP,
plaintext LAN HTTP, trusted time, console I/O, pairing/origin policy, runtime
effects, and capability profiles remain native. Govee, LIFX, Kasa, Reolink,
Roku, Wemo, Sonos, Nanoleaf, Tasmota, Fronius, HomeWizard, and HEOS contribute
deterministic codecs, bounded parsers and DTO validation, normalization,
projection, stable identities/errors, command planning, and language-neutral
fixtures to the parity backlog. Wemo specifically contributes SSDP header
parsing, bounded
setup/SOAP XML, service/device
normalization, and switch/light command planning. Sonos additionally contributes
credential-free URL/control-path validation, AVTransport, RenderingControl,
DIDL metadata normalization, deterministic inspection planning, and
protocol-neutral media-player projection. Nanoleaf adds credential syntax and
origin-configuration validation, bounded snapshot/state validation, stable
identity and capability projection, RGB/HSV and mirek conversion, command
planning, and verification. Tasmota adds bounded Status 0 JSON validation,
relay/light/sensor normalization, state and capability projection, command
planning, color conversion, and verification fixtures. Fronius adds bounded
Power Flow and API-status validation, site/inverter measurement normalization,
and deterministic sensor projection. HEOS adds bounded command-result and
response-envelope validation, player/now-playing/volume/mute normalization,
HEOS escaping, stable identities/errors, and deterministic read-only media
projection. UDP multicast, DNS/TCP, LAN HTTP and HEOS TCP execution,
timeouts, endpoint approval, CLI I/O, authorization, and runtime mutation remain
native-host responsibilities.

This loop delivers only deterministic, authority-free package contracts and
implementations. DNS/UDP/TCP/TLS, endpoint review, credentials and Vault,
capability approval, runtime mutation, native executors, and host hardening are
reviewed native-host exceptions and are not selectable parity work. ONVIF is
excluded from this parity tranche. Mixed smart-home packages enter the backlog
only through their portable cores and shared language-neutral fixtures.

The August 2 lane audit is:

| Established lane | Packages present | High-consensus gaps | Rust/Python-core coverage |
|---|---:|---:|---:|
| C# | 196 | 0 | 47.5% |
| Dart | 79 | 101 | 18.6% |
| Elixir | 276 | 0 | 69.3% |
| F# | 195 | 0 | 47.5% |
| Go | 292 | 0 | 72.4% |
| Haskell | 203 | 2 | 48.8% |
| Java | 126 | 58 | 31.0% |
| Kotlin | 125 | 58 | 31.0% |
| Lua | 251 | 0 | 63.3% |
| Perl | 251 | 0 | 63.3% |
| Python | 496 | 1 | 100% |
| Ruby | 294 | 0 | 70.3% |
| Rust | 983 | 0 | 100% |
| Swift | 160 | 51 | 37.8% |
| TypeScript | 439 | 0 | 82.2% |

These are structural counts, not conformance claims. The full review queue must
cover all 15 rows even when a lane has zero gaps in the current
high-consensus subset.

## Priority 0: Inventory And Identity Integrity

Completed. The reporter now inventories Git-visible files, emits Markdown,
JSON, and CSV, classifies package lanes, detects canonical collisions, and is
covered by CI unit tests. The conflicting `ruby/b_tree` and
`ruby/b_plus_tree` shadow packages were removed in favor of the authoritative
DT11/DT12 `ruby/b-tree` and `ruby/b-plus-tree` implementations. CI now rejects
new canonical identity collisions with `--fail-on-collisions`.

Remaining inventory/build-integrity work discovered in the July 29 audit:

- reconcile stale `BUILD_windows` prerequisite declarations reported by the
  build-tool validator across Python, Perl, TypeScript, Swift, Dart, Kotlin, and
  related packages in dependency-shaped waves. The Python validator now also
  materializes a 10-file Lua wave covering the compiler, serializer, language
  server, QR, and compression dependency chains;
- keep the Python build tool's Lua rockspec decoding deterministic. Merged PR
  #9495 normalized the three CP1252 metadata bytes, added positive and invalid-
  UTF-8 fixtures, and returns `METADATA_INVALID_UTF8`; its refreshed full scan
  succeeds across 4,765 packages and 7,100 edges;
- bring the remaining TypeScript, Lua, Perl, Ruby, Elixir, and Haskell
  build-tool resolvers to the shared strict-UTF-8 rockspec contract.
  Their current byte, replacement, silent-drop, and locale-sensitive behavior
  is tracked separately from the Python full-scan blocker. Merged PR #9504
  completed the Go operational-oracle child, and merged PR #9510 completed
  Rust. Merged PR #9537 completed Swift with exact shared success and invalid-
  byte fixtures plus real CLI exit-2 coverage;
- make Swift build-tool file options recognize Windows drive-letter absolute
  paths. The UTF-8 validation run discovered that `--emit-plan` reaches
  resolution but then joins an absolute Windows path under the repository;
  cover `--emit-plan`, `--plan-file`, and `--cache-file` in a separate slice;
- align Swift build-tool discovery with the canonical language and identity
  registry. Its full release plan currently emits 4,768 entries but only 4,594
  unique names, including 143 duplicate identity groups and 397 `unknown`
  entries. This is the selected post-UTF-8 child; consume the shared registry
  and duplicate-identity fixtures without widening into file-option handling;
- merged PR #9521 makes the Rust build tool reject resolver self-edges with a
  stable diagnostic and preserves distinct package/program identities for
  `elixir/grammar_tools`;
- bring Rust build-tool discovery to the complete canonical language and
  identity registry. Merged PR #9527 classifies every repository bucket,
  excludes specification fixture trees, and rejects residual duplicate names;
  its real full plan exits zero with 4,765 entries, 4,765 unique identities,
  and only the intentional language-neutral `code/sites/blog` package;
- expose Haskell through the Python build tool's `--language` filter. Haskell
  is already in its resolver and canonical language registry but is missing
  from the native CLI choices;
- make Python build-plan emission replace an existing destination atomically on
  Windows. Repeated output currently leaves the temporary path and fails with
  `WinError 183`, while a fresh destination succeeds;
- remove environment-specific Starlark grammar lookup so build discovery works
  from arbitrary clean worktrees;
- add explicit applicability and maturity data before either C/C++ or OCaml
  enters the all-language completion denominator.

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

### Current 14-of-15 frontier: reopened

The historical Priority 1 cohort is complete, but later Haskell work promoted
13 packages into a new 14-of-15 frontier. Merged PR #9383 closed `trig` and
`wave`, merged PR #9477 closed `matrix`, `loss-functions`, and
`feature-normalization`, and this branch closes `document-ast`, so the remaining
seven current gaps are Dart-only:

- deterministic data structures: `binary-search-tree`, `fenwick-tree`, and
  `trie`;
- leaf ciphers: `atbash-cipher`, `scytale-cipher`, and `vigenere-cipher`;
- utility leaf: `uuid`.

Merged PR #9375 closed the generator-level prerequisite found by the post-#9363
fixture audit. Dart's native scaffold generator now emits byte-stable,
schema-v1 empty library profiles and truthful generated-program stdout
profiles, while declaring its own reviewed runtime authority. Existing
nonempty Dart profiles remain owned by the legacy migration review. Close the
remaining seven-package frontier as small coherent PRs on top of that scaffold
contract.

Merged PR #9383 completed the first child item: the zero-dependency PHY00
`trig` leaf and its direct PHY01 `wave` consumer. This closed two Dart-only
14-of-15 gaps while exercising the scaffold and capability contract on a real
dependency chain.

Merged PR #9477 completed the ML child item with the independent `matrix`,
`loss-functions`, and `feature-normalization` leaves. The post-merge leverage
pass selected `document-ast` next: its types-only TE00 model has 68 exact
cross-repository consumers and unlocks substantially more follow-on parity work
than the remaining cipher and data-structure leaves. This branch delivers that
sealed, immutable 24-node model with exhaustive discriminator, containment,
value-semantics, and coverage checks. The cipher trio, trie,
binary-search-tree, fenwick-tree, and UUID are explicit remaining child items;
the existing Dart LZ78 private-trie migration is tracked separately.

The post-merge governance audit found no repository-verifiable Layer 5 approval
for #9375's nonempty generator profile: GitHub reports no review decision, and
the merge commit carries the GitHub web-flow signature rather than evidence
bound to `CAPABILITY_SIGNERS`. The externally blocked
`dart-scaffold-capability-layer5-evidence` item owns either recovery of the
actual hardware-key-backed approval or an explicit reviewed policy
reconciliation. The pure `dart-trig-wave` child neither executes nor publishes
the generator and introduces only empty capability profiles.

The child review also exposed three separately owned follow-ups rather than
silently widening that delivery slice: a machine-readable language-neutral
PHY00/PHY01 fixture corpus, a full-range tiny/subnormal PHY00 square-root audit,
and finite-input and overflow-safe PHY01 evaluation reconciliation. Merged PR
#9390 delivered the closed 53-case shared corpus and its first always-on Dart
consumer. The collision-clean post-merge inventory and parallel dependency,
fixture, and security audits then selected
`phy00-small-sqrt-cross-lane-audit` because PHY00 is the
foundational numeric dependency beneath PHY01 and the shared oracle now makes
the known boundary defect testable across all 15 established lanes plus the
emerging C and C++ implementations. PR #9395 merged that repair with all checks
green. The collision-clean `761c60fc3` inventory and dependency audit then
selected `phy01-nonfinite-validation-backfill` as the consumer-side successor;
PR #9400 merged that repair with all 15 checks green. The collision-clean
`20afefa7a` inventory found one newly owned Rust dashboard-core singleton and
selected `phy00-atan-tiny-signed-zero-cross-lane-audit` next because its sole
dependency is merged and every existing trig lane shares the same defect. The
square-root numerical audit had discovered that separate
`phy00-atan-tiny-signed-zero-cross-lane-audit`: current half-angle reduction
underflows at the subnormal floor and loses the sign of negative zero, so that
work remained tracked behind the merged square-root and wave slices rather
than expanding either delivery.
PR #9413 merged that repair at `458405a6e` after all 15 checks passed. The
collision-clean `1e4956369` refresh added the two Rust-only camera identities.
PR #9421 merged the camera-media boundary repair. With no active in-scope parity
PR at `e552707d5`, the leverage pass selected
`dart-current-14-of-15-matrix-family`: the independent `matrix`,
`loss-functions`, and `feature-normalization` leaf packages. PR #9477 merged
that slice at `1233e31db` with successful final-head push CI, PR CI, and
CodeQL. The refreshed leverage pass selected the zero-dependency TE00
`document-ast` model next because its 68 exact consumers make it the strongest
remaining dependency foundation. The branch is
`codex/dart-document-ast-parity`; open ONVIF and other host PRs do not occupy
the scoped parity slot.
The audit also found private matrix/MSE helpers in Dart `single-layer-network`
and `two-layer-network`; their migration to the shared packages is a separate
downstream backlog item rather than hidden scope in this port.
The security pass also found that ML01 does not define a shared NaN/infinity
input policy: Dart and the Python reference can clamp NaN cross-entropy
predictions to a finite boundary. A separate all-lane fixture and conformance
item owns that decision instead of introducing a Dart-only behavioral fork.
The build-tool execution critical path remains blocked on external
immutable-runner and attester provisioning.

Port dependency families together when doing so avoids temporary broken package
graphs. Grammar-generated lexer/parser pairs should be generated from the shared
grammar sources rather than independently handwritten.

## Priority 2: Complete The High-Consensus Core

The 172 packages present in at least ten implementation languages need 275
ports to reach all 15. After Priority 1, select work in this order:

| Language lane | Current high-consensus gaps | Pairing rule |
|---|---:|---|
| C# | 0 | Complete; paired native package wave |
| Dart | 105 | Close the reopened 14-of-15 set, then dependencies before consumers |
| Elixir | 0 | Complete; retain as a reference lane and run conformance fixtures |
| F# | 0 | Complete; paired native package wave |
| Go | 0 | Complete; primary build-tool and portable-core reference lane |
| Haskell | 2 | Finish the generic `event-loop` and pure `brotli` gaps |
| Java | 58 | Move with Kotlin |
| Kotlin | 58 | Move with Java |
| Lua | 0 | Complete; retain as a reference lane and remediate build-tool drift |
| Perl | 0 | Complete; retain as a reference lane and remediate build-tool drift |
| Python | 1 | Classify the remaining self-hosted `python-parser` carefully |
| Ruby | 0 | Complete; retain as a reference lane and run conformance fixtures |
| Rust | 0 | Complete; reference lane for broad and singleton families |
| Swift | 51 | Data structures and generated frontends before native app surfaces |
| TypeScript | 0 | Complete; reference lane for web-capable portable contracts |

Zero-gap lanes remain active reference and conformance lanes; they are not
exempt from semantic review or build-tool parity.

The e34f26fad inventory gives Python's lone high-consensus gap a dedicated
`python-parser-self-hosting-applicability` owner. It must record whether a native
Python parser port is portable, self-hosting-applicable, or a reviewed exception
through explicit reporter applicability data instead of leaving the gap
unclassified.

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

The thirteenth paired C#/F# slice is complete: `ed25519` now provides native
RFC 8032 key generation, deterministic signing, and verification in both
lanes, composing their existing SHA-512 packages with extended Edwards
coordinates over `2^255 - 19`. Package-native tests cover the first three RFC
vectors, deterministic key and signature derivation, wrong messages and keys,
tampered signature halves, non-canonical scalars, malformed point encodings,
and strict seed and secret-key formats. The package now spans 12 implementation
lanes, reduces the high-consensus backlog to 317 slots, and leaves 4 paired
gaps in each lane.

The fourteenth paired C#/F# slice is complete: `font-parser` now provides
dependency-free metrics-only OpenType and TrueType readers in both lanes.
Native big-endian table parsing covers global metrics and names, BMP `cmap`
format 4 glyph lookup, complete and shared `hmtx` records, optional `OS/2`
heights, and legacy `kern` format 0 pairs. Package-native tests exercise the
shared Inter fixture, in-memory synthetic fonts, malformed directories and
sentinels, immutable input ownership, unsupported mappings, shared advances,
and sorted kerning lookup. The package now spans 12 implementation lanes,
reduces the high-consensus backlog to 315 slots, and leaves 3 paired gaps in
each lane.

The fifteenth paired C#/F# slice is complete: `asciidoc-parser` now provides
native block and inline parsers over each lane's shared `document-ast` model.
Both implementations cover headings, paragraphs and breaks, source, literal,
passthrough, and recursive quote blocks, ordered and unordered nested lists,
comments, thematic breaks, emphasis, strong text, code spans, link and image
macros, cross-references, and HTTP autolinks. Mirrored package-native suites
exercise 33 test cases in each lane, including lenient unterminated blocks and
malformed inline delimiters, with more than 97% line coverage. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 313 slots, and leaves 2 paired gaps in each lane.

The sixteenth paired C#/F# slice is complete: `fpga` now provides native
SRAM-backed lookup tables, dual-LUT slices with optional registers and carry
chains, configurable logic blocks, programmable switch matrices, I/O pads, and
immutable JSON bitstream configuration in both lanes. The ports compose the
existing native `logic-gates` and `block-ram` packages, and their package-local
suites exercise 40 C# and 38 F# cases with more than 97% line coverage. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 311 slots, and leaves only `zstd` as a paired gap in C# and F#.

The seventeenth paired C#/F# slice is complete: `zstd` now provides native
CMP07 educational Zstandard codecs in both lanes. The ports compose their
existing native `lzss` packages and implement frame headers, 128 KiB raw, RLE,
and compressed blocks, raw literal sections, and the predefined
literal-length, match-length, and offset FSE tables. Package-native suites
exercise 29 C# and 20 F# cases with more than 90% line coverage, including
multi-block frames, malformed input, compression ratios, and cross-language
compatibility with the established Ruby implementation. The package now spans
12 implementation lanes, reduces the high-consensus backlog to 309 slots, and
closes the remaining high-consensus gaps in both C# and F#.

The first Haskell high-consensus slice is complete: `atbash-cipher` now
provides a dependency-free CR01 implementation that mirrors ASCII letters
while preserving case and passing all other characters through unchanged.
Its package-native suite exercises 11 examples with 100% expression coverage,
including complete alphabets, non-ASCII pass-through, and the cipher's
self-inverse property. The package now spans 13 implementation lanes, reduces
the high-consensus backlog to 308 slots, and leaves 33 gaps in the Haskell
lane.

The second Haskell high-consensus slice is complete: `scytale-cipher` now
provides dependency-free CR02 encryption, decryption, explicit key validation,
and brute-force candidate generation. Its package-native suite exercises 17
examples with 98% expression coverage, including reference vectors, padded and
uneven grids, mixed-character round trips, and the complete shared key range.
The package now spans 14 implementation lanes, reduces the high-consensus
backlog to 307 slots, and leaves 32 gaps in the Haskell lane.

The third Haskell high-consensus slice is complete: `feature-normalization`
now provides dependency-free ML05 standard and min-max scaler fitting and
transformation with explicit rectangular-matrix and scaler-width validation.
Its package-native suite exercises 14 examples with 99% expression coverage,
including the shared matrix, population deviation, constant columns, negative
ranges, new observations, and every validation branch. The package now spans
14 implementation lanes, reduces the high-consensus backlog to 306 slots, and
leaves 31 gaps in the Haskell lane.

The fourth Haskell high-consensus slice is complete: `loss-functions` now
provides dependency-free ML04 mean squared, mean absolute, binary
cross-entropy, and categorical cross-entropy losses together with their
prediction gradients, explicit vector validation, and finite probability
clamping. Its package-native suite exercises 15 examples covering reference
values, every derivative branch, boundary probabilities, and all validation
paths. The package now spans 14 implementation lanes, reduces the
high-consensus backlog to 305 slots, and leaves 30 gaps in the Haskell lane.

The fifth Haskell high-consensus slice is complete: `trig` now provides the
dependency-free PHY00 angle constants, range-reduced sine and cosine series,
degree/radian conversions, Newton-method square root, pole-guarded tangent,
and inverse tangent functions. Its package-native suite exercises 17 examples
with 99% expression coverage, including reference angles, identities,
large-input reduction, conversions, domain validation, tangent poles, inverse
ranges, axes, and all quadrants. The package now spans 14 implementation
lanes, reduces the high-consensus backlog to 304 slots, leaves 29 gaps in the
Haskell lane, and unlocks the dependent `wave` port.

The sixth Haskell high-consensus slice is complete: `wave` now builds on the
local `trig` layer to provide validated PHY01 sinusoidal waves, periods,
angular frequencies, phase offsets, and time-domain evaluation. Its
package-native suite exercises 17 examples with 95% expression coverage,
including construction, validation, derived quantities, the full cycle,
periodicity, phase offsets, and the zero-amplitude case. The package now spans
14 implementation lanes, reduces the high-consensus backlog to 303 slots, and
leaves 28 gaps in the Haskell lane.

The seventh Haskell high-consensus slice is complete: `matrix` now provides
immutable rectangular matrices with factories, arithmetic, multiplication,
indexed updates, reductions, element-wise math, shape operations, and exact or
tolerant comparison. Its package-native suite exercises 34 examples with 96%
expression coverage, including rectangular validation, every operation family,
empty and zero-width shapes, mismatched dimensions, invalid indices, and
half-open slices. The package now spans 14 implementation lanes, reduces the
high-consensus backlog to 302 slots, and leaves 27 gaps in the Haskell lane.

The eighth Haskell high-consensus slice is complete: `vigenere-cipher` now
provides case-preserving encryption and decryption, strict ASCII-key
validation, index-of-coincidence key-length estimation, chi-squared key
recovery, and automatic cipher breaking. Its package-native suite exercises 26
examples with 97% expression coverage, including parity vectors, punctuation,
Unicode pass-through, invalid keys, round trips, three recovery key lengths,
and short-input behavior. The package now spans 14 implementation lanes,
reduces the high-consensus backlog to 301 slots, and leaves 26 gaps in the
Haskell lane.

The ninth Haskell high-consensus slice is complete: `uuid` now provides strict
128-bit construction, parsing and rendering, standard namespaces, metadata,
and native v1, v3, v4, v5, and v7 generation. It builds name-based UUIDs on the
existing Haskell MD5 and SHA-1 ports and uses native time and randomness for
the generated versions. Its package-native suite exercises 17 examples with
87% expression coverage, including accepted and rejected text forms, integer
and byte round trips, all variants, RFC name-based vectors, Unicode names,
random uniqueness, multicast nodes, and v7 timestamps. The package now spans
14 implementation lanes, reduces the high-consensus backlog to 300 slots, and
leaves 25 gaps in the Haskell lane.

The tenth Haskell high-consensus slice is complete: `document-ast` now provides
immutable algebraic data types for the TE00 block and inline model together
with the shared GFM task-list, strikethrough, and table extensions. Exhaustive
unions and stable discriminator helpers support typed parser and renderer
traversal without external dependencies. Its package-native suite exercises
11 examples with 100% expression and alternative coverage across every node
family, payload accessor, nesting shape, and discriminator. The package now
spans 14 implementation lanes, reduces the high-consensus backlog to 299 slots,
and leaves 24 gaps in the Haskell lane.

The eleventh Haskell high-consensus slice is complete: `lz78` now provides the
CMP01 token model, an immutable byte-trie cursor, dictionary-capped encoding,
checked decoding, strict big-endian wire serialization, and deterministic
one-shot compression. Its package-native suite exercises 13 examples with 98%
expression and 96% alternative coverage, including both canonical token
vectors, end-of-stream flushing, dictionary caps, text and binary round trips,
exact wire bytes, and every malformed-input error family. The package now spans
13 implementation lanes, reduces the high-consensus backlog to 298 slots, and
leaves 23 gaps in the Haskell lane.

The twelfth Haskell high-consensus slice is complete: `deflate` now provides a
pure CMP05 encoder and strict decoder that compose the existing Haskell LZSS
and canonical Huffman packages. Its package-native suite exercises 26 examples
with 91% expression coverage, including exact Python-compatible wire vectors,
literal-only and match-heavy streams, binary round trips, parameter validation,
and malformed headers, tables, prefixes, backreferences, and output lengths.
The package now spans 13 implementation lanes, reduces the high-consensus
backlog to 297 slots, and leaves 22 gaps in the Haskell lane.

The thirteenth Haskell high-consensus slice is complete: `point2d` now provides
immutable G2D00 point/vector arithmetic and half-open axis-aligned rectangle
geometry on top of the existing pure Haskell `trig` package. Its package-native
suite exercises construction, products, norms, normalization, distance,
interpolation, axis angles, empty and negative extents, boundary containment,
union, strict positive-area intersection, and symmetric expansion. The package
now spans 12 implementation lanes, reduces the high-consensus backlog to 296
slots, leaves 21 gaps in the Haskell lane, and unlocks the dependent `affine2d`,
`bezier2d`, and `arc2d` graphics wave.

The fourteenth Haskell high-consensus slice is complete: `affine2d` now
provides the immutable G2D01 six-scalar matrix, all standard factories,
ordered composition, separate point and vector application, determinant,
checked inversion, tolerance predicates, and SVG/Canvas component ordering.
It composes the existing pure Haskell `point2d` and `trig` packages and covers
centered rotation, skew, non-commutativity, singularity thresholds, and inverse
round trips in its package-native suite. The package now spans 12 implementation
lanes, reduces the high-consensus backlog to 295 slots, and leaves 20 gaps in
the Haskell lane.

The fifteenth Haskell high-consensus slice is complete: `bezier2d` now
provides immutable G2D02 quadratic and cubic curves, numerically stable de
Casteljau evaluation and exact splitting, unnormalized derivatives, adaptive
polyline flattening, tight derivative-root bounds, and exact quadratic degree
elevation. It composes only the existing pure Haskell `point2d` package. Its
package-native suite exercises 21 examples with 100% expression and
alternative coverage, including exact reparameterized splits, tolerance-driven
subdivision, both quadratic extrema paths, and full, linear, constant, and
negative-discriminant cubic derivative cases. The package now spans 12
implementation lanes, reduces the high-consensus backlog to 294 slots, leaves
19 gaps in the Haskell lane, and unlocks the dependent `arc2d` port.

The sixteenth Haskell high-consensus slice is complete: `arc2d` now provides
G2D03 SVG endpoint and center arc forms, W3C endpoint-to-center conversion,
parametric evaluation, unnormalized tangents, analytic bounds for rotated
ellipse arcs, and cubic Bezier approximation. It composes the existing pure
Haskell `point2d`, `bezier2d`, and `trig` packages. Its package-native suite
exercises 25 examples with 99% expression and 100% alternative coverage,
including degeneracy thresholds, both sweep corrections, radius scaling,
nonzero rotation, positive and negative tight-bound extrema, zero sweep, the
quarter-circle magic controls, segmentation, and continuity. The package now
spans 12 implementation lanes, reduces the high-consensus backlog to 293 slots,
and leaves 18 gaps in the Haskell lane.

The seventeenth Haskell high-consensus slice is complete: `gradient-descent`
now provides the dependency-free ML02 stochastic-gradient-descent update with
explicit rejection of empty and length-mismatched vectors. Its package-native
suite exercises 11 examples with 100% expression and alternative coverage,
including the shared parity vector, singleton and multi-element inputs, zero
and negative learning rates, mixed gradient signs, input preservation, and all
validation paths. The package now spans 11 implementation lanes, reduces the
high-consensus backlog to 292 slots, and leaves 17 gaps in the Haskell lane.

The eighteenth Haskell high-consensus slice is complete: `perceptron` now
provides a pure sigmoid/BCE single-neuron classifier that composes the existing
Haskell `matrix`, `loss-functions`, and `activation-functions` packages. Its
package-native suite exercises 14 examples covering shared AND-gate
convergence, scalar and column labels, epoch-zero updates, deterministic
refitting, prediction guards, hyperparameter validation, feature shapes, label
counts, and finite-value checks. The package now spans 11 implementation
lanes, reduces the high-consensus backlog to 291 slots, and leaves 16 gaps in
the Haskell lane.

The nineteenth Haskell high-consensus slice is complete:
`type-checker-protocol` now provides immutable diagnostics and typed results,
a functional checker contract, and pure phase/kind hook dispatch with explicit
fall-through, exact-before-wildcard precedence, source-location helpers, and
clean diagnostic lifecycles. Its base-only package-native suite exercises 19
examples covering checker outcomes, partial typed ASTs, normalization, hook
ordering, wildcard and argument dispatch, error collection, and reusable
state, with 100% alternative and 99% expression coverage. The package now
spans 11 implementation lanes, reduces the high-consensus backlog to 290
slots, leaves 15 gaps in the Haskell lane, and establishes the shared contract
needed by later typed-frontend ports.

The twentieth Haskell high-consensus slice is complete: `paint-vm-ascii` now
provides a pure terminal renderer for the shared `paint-instructions` IR. It
maps scene coordinates through configurable character-cell scales, clips
visible filled rectangles into the scene buffer, trims terminal whitespace,
and rejects paths explicitly rather than returning incomplete output. Its
package-native suite exercises 13 examples covering shared defaults, filled
rectangles, clipping, transparent paints, default scaling, half-cell rounding,
zero-sized scenes, unsupported paths, invalid scales, scene dimensions, and
rectangle geometry. The package now spans 11 implementation lanes, reduces the
high-consensus backlog to 289 slots, and leaves 14 gaps in the Haskell lane.

The twenty-first Haskell high-consensus slice is complete:
`barcode-layout-1d` now provides the shared pure geometry layer for linear
barcodes. It validates alternating bar/space runs, expands binary and
narrow/wide patterns, computes inferred or explicit symbol spans and quiet
zones, and emits metadata-rich rectangle-only scenes through the existing
Haskell `paint-instructions` package. Its package-native suite exercises 18
examples with 97% expression and 89% alternative coverage across shared
defaults, both pattern families, custom ratios and markers, attribution,
symbol inference and descriptors, empty content, rendering geometry and
metadata, every validation family, and the deliberate text-shaping guard. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 288 slots, leaves 13 gaps in the Haskell lane, and unlocks the dependent
Code 39, Codabar, ITF, UPC-A, EAN-13, Code 128, and `barcode-1d` ports.

The twenty-second Haskell high-consensus slice is complete: `itf` now provides
the pure Interleaved 2 of 5 encoder unlocked by `barcode-layout-1d`. It
validates non-empty even-length ASCII digit payloads, exposes typed digit-pair
patterns, interleaves first-digit bar widths with second-digit space widths,
and emits explicit start, data, and stop symbol geometry plus authoritative
symbology metadata. Its package-native suite exercises shared patterns, the
complete digit table, source attribution, exact module geometry, customized
paint output, metadata precedence, aliases, and both local and shared
validation paths. The package now spans 12 implementation lanes, reduces the
high-consensus backlog to 287 slots, and leaves 12 gaps in the Haskell lane.

The twenty-third Haskell high-consensus slice is complete: `code39` now
provides the pure linear-barcode encoder unlocked by `barcode-layout-1d`. It
normalizes lowercase input, validates the complete standard alphabet, protects
the reserved delimiter, exposes all 44 typed narrow/wide symbol patterns, and
emits attributed start, data, stop, and inter-character-gap runs through the
shared paint geometry. Its package-native suite exercises normalization,
educational errors, the complete symbol table, exact patterns and module
counts, semantic attribution, empty payloads, customized paint output,
metadata precedence, aliases, and both local and shared validation paths. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 286 slots, and leaves 11 gaps in the Haskell lane.

The twenty-fourth Haskell high-consensus slice is complete: `codabar` now
provides the pure configurable-guard encoder unlocked by `barcode-layout-1d`.
It accepts body-only or explicitly guarded input, validates configurable `A`-`D`
start and stop choices, exposes all 20 typed binary symbol patterns, and emits
attributed start, data, stop, and inter-character-gap runs through the shared
paint geometry. Its package-native suite exercises guard insertion and
preservation, educational errors, the complete symbol table, exact module
counts, semantic attribution, empty payloads, customized paint output,
metadata precedence, aliases, and both local and shared validation paths. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 285 slots, and leaves 10 gaps in the Haskell lane.

The twenty-fifth Haskell high-consensus slice is complete: `code128` now
provides the pure Code Set B encoder unlocked by `barcode-layout-1d`. It
validates printable ASCII, exposes the complete 107-pattern symbol table,
computes the required weighted modulo-103 checksum, and emits attributed start,
data, check, and stop runs through the shared paint geometry. Its package-native
suite exercises ASCII boundaries, educational errors, the complete pattern
table, reference values and checksums, empty payloads, exact module geometry,
customized paint output, metadata precedence, aliases, and both local and shared
validation paths. The package now spans 12 implementation lanes, reduces the
high-consensus backlog to 284 slots, and leaves 9 gaps in the Haskell lane.

The twenty-sixth Haskell high-consensus slice is complete: `upc-a` now provides
the pure retail-barcode encoder unlocked by `barcode-layout-1d`. It accepts
11-digit payloads or validated 12-digit codes, computes the required modulo-10
check digit, exposes all twenty left/right digit patterns, and emits the fixed
95-module start, digit, center, and end structure with typed source attribution
and explicit symbol spans. Its package-native suite exercises 23 examples with
91% expression and 81% alternative coverage, including reference checksums,
all standard patterns, computed and supplied checks, ASCII and length guards,
exact module geometry, metadata precedence, aliases, and shared validation
paths. The package now spans 12 implementation lanes, reduces the
high-consensus backlog to 283 slots, leaves 8 gaps in the Haskell lane, and
unlocks the dependent `ean-13` port.

The twenty-seventh Haskell high-consensus slice is complete: `ean-13` now
provides the pure retail-barcode encoder unlocked by `barcode-layout-1d`. It
accepts 12-digit payloads or validated 13-digit codes, computes the required
weighted modulo-10 check digit, exposes all thirty L/G/R digit patterns and all
ten leading-digit parity sequences, and emits the fixed 95-module guard and
visible-digit structure with typed source attribution and explicit symbol
spans. Its package-native suite exercises 26 examples with 92% expression and
78% alternative coverage, including reference checksums, every standard digit
and parity pattern, computed and supplied checks, ASCII and length guards,
exact module geometry, metadata precedence, aliases, and shared validation
paths. The package now spans 12 implementation lanes, reduces the
high-consensus backlog to 282 slots, and leaves 7 gaps in the Haskell lane.

The twenty-eighth Haskell high-consensus slice is complete: `sql-csv-source`
now provides the filesystem adapter for the existing pure
`sql-execution-engine`. It loads every CSV table through an explicit IO
boundary, preserves header order in an immutable data-source snapshot, handles
quoted commas, escaped quotes, embedded newlines, and CRLF, validates malformed
records, and coerces null, boolean, integer, finite real, and text values into
the shared SQL types. Its package-native suite exercises 19 examples with 89%
expression and 86% alternative coverage, including parsing failures, missing
directories and tables, typed scans, filters, ordering, null predicates, joins,
grouping, aggregates, limits, and total result wrappers. The package now spans
13 implementation lanes, reduces the high-consensus backlog to 281 slots, and
leaves 6 gaps in the Haskell lane.

The twenty-ninth Haskell high-consensus slice is complete: `zstd` now provides
the pure CMP07 educational Zstandard frame codec on top of the existing native
`lzss` package. It emits standard single-segment frames and 128 KiB raw, RLE,
and compressed blocks, encodes raw literal sections and the predefined
literal-length, match-length, and offset FSE tables, and strictly validates
headers, modes, truncation, backreferences, trailing data, and output limits.
Its package-native suite exercises 16 examples with 86% expression and 77%
alternative coverage, including exact cross-language compressed bytes, all
block families, multi-block RLE, dictionary-id and content-size header forms,
checksums, deterministic binary data, compression ratios, and malformed
frames. The package now spans 13 implementation lanes, reduces the
high-consensus backlog to 280 slots, and leaves 5 gaps in the Haskell lane.

The thirtieth Haskell high-consensus slice is complete: `barcode-1d` now
coordinates the six existing native Haskell symbology packages for Code 39,
Codabar, Code 128, EAN-13, ITF, and UPC-A. It normalizes user-facing
symbology names, preserves typed encoder and layout failures, forwards shared
paint options and Codabar guards, emits backend-neutral `PaintScene` values,
and renders them through the pure ASCII Paint VM without claiming an absent
native raster backend. Its package-native suite exercises 13 examples with
93% expression and 100% alternative coverage, including every route, default
selection, normalized spellings, unsupported names, custom geometry, custom
guards, typed failures, and ASCII backend errors. The package now spans 12
implementation lanes, reduces the high-consensus backlog to 279 slots, and
leaves 4 gaps in the Haskell lane.

The thirty-first Haskell high-consensus slice is complete: `http-core` now
provides the dependency-free NET03 semantic model with ordered duplicate
headers, bounded versions and status codes, body-framing hints, request and
response heads, content helpers, raw request-target/query handling, and
path-only route patterns. Its package-native suite exercises 19 examples with
96% expression and 95% alternative coverage across valid and malformed
versions, ASCII-only header matching, length overflow, content parameters,
message delegates, raw queries, repeated slashes, captures, and route
mismatches. The port also repairs Haskell scaffold naming/test conventions and
build-graph dependency discovery so the follow-on `http1` edge is visible to
incremental CI. The package now spans 12 established lanes, reduces the
high-consensus backlog to 278 slots, and leaves 3 gaps in the Haskell lane.

The merged thirty-second Haskell slice is `http1`, the direct NET04 consumer
of that merged `http-core` foundation. It closes one ready protocol edge,
validates the repaired Cabal dependency graph, and unlocks the remaining
Haskell high-consensus tail. Its security review also exposed a shared NET04
contract gap: established implementations need fail-closed framing for
transfer-encoding/content-length ambiguity, strict wire grammar, bounded head
resources, response-method context, and redacted typed errors. The Haskell
slice establishes that contract; `http1-safe-framing-backfill` tracks the
remaining established lanes as a separate dependency-shaped tranche.

The same slice exposed legacy Haskell scaffold capability metadata:
`required_capabilities.json` is still emitted as an incomplete one-field
object, and the merged `http-core` package retains that shape. The current
`http1` package carries a schema-valid v1 pure-computation manifest;
`haskell-scaffold-capability-schema` tracks generator golden/schema coverage
and the existing-package backfill.

The selected thirty-third Haskell slice is that scaffold capability-schema
repair. It is a generator-level prerequisite for the remaining `event-loop`
and `brotli` ports: the Go, TypeScript, and native Haskell scaffold paths must
emit schema-valid v1 manifests instead of propagating the legacy one-field
shape or omitting explicit scaffold metadata. The backfill covers all ten
existing invalid Haskell manifests. Eight are explicit pure-computation
profiles; `conduit` and `conduit-hello` carry proposed FFI, network, time,
environment, and standard-output declarations under the canonical taxonomy,
pending the Layer 5 review required for nonempty profiles.

That audit also discovered four independently owned follow-ups:
`haskell-capability-policy-audit` classifies the effective profiles of every
Haskell package and program that currently relies on an absent legacy
manifest; `haskell-scaffold-convention-reconciliation` brings the older
TypeScript Haskell Cabal/Hspec templates to full canonical parity; and
`scaffold-description-injection-hardening` closes structural delimiter
injection across all generated metadata and source-comment contexts.
`capability-schema-category-action-constraints` encodes the taxonomy's valid
category-action pairs in both schemas and every enforcement backend. Merged PR
#9363 completed that restriction-only contract and directly unblocked the
future OCaml analyzer, `adj-lang-cli` profile reconciliation, native Matter
controller review, and scaffold manifest repairs.

The implementation audit also separated a legacy migration instead of
silently forcing unlike files through the new schema. The tracked tree
currently has 2,885 `required_capabilities.json` paths: 150 top-level
dependency arrays, 82 metadata objects without a `capabilities` key, 164 object
manifests with string-list capabilities, and 2,489 objects whose capability
list is empty or contains structured entries. Of the last group, 2,316
currently validate as schema-v1 manifests. The structured entries total 373:
359 use valid current-vocabulary pairs and 14 use separately owned legacy
vocabulary such as `hardware`, `audio`, `ffi:export`, `process:spawn`,
`network:listen`, and `fs:read_write`; no current-vocabulary entry forms an
invalid cross-pair. The pending
`capability-manifest-legacy-shape-and-vocabulary-migration` item must classify
those semantic owners before migration, must not reinterpret build-dependency
arrays as security manifests, and must obtain Layer-5 review before changing
any nonempty authority profile.

The same post-merge audit found six additional repository-wide generator
blockers not owned by that migration: three valid underscore-bearing Go package
names rejected by the generator's narrow package-name rule and three reviewed
`ffi:call` manifests rejected by its duplicated vocabulary. The new
`capability-generator-all-mode-reconciliation` item follows legacy
classification, reconciles those rules without reopening the closed taxonomy,
and makes `--all --dry-run` a tested green repository contract.

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

This phase covers 121 package identities and 911 current missing slots.

The DT02 graph-substrate slice adds pure Go and Rust
`multi-directed-graph` implementations alongside the existing Python and
TypeScript references. Both ports provide stable edge identity, parallel
directed edges, copied property bags, multiplicity-aware DAG algorithms,
package-native tests, capability declarations, and downstream integration with
their `neural-network` and `neural-graph-vm` packages. This moves the portable
package from two to four established implementation lanes; the remaining
language ports stay eligible for later dependency-shaped waves.

## Priority 4: Classify Sparse And Singleton Families

The singleton inventory is led by 572 Rust, 86 Python, and 84 TypeScript
packages. Classify families before opening implementation PRs.

The July 30-August 2 inventories added thirty-one Rust singleton identities that now
have explicit classification work in the loop state: `axiom-to-semantic-ir` is a
likely portable deterministic lowering; `http1-client` needs its portable
protocol core separated from native transport behavior; and
`venture-browser-core` needs a portable-core versus native-boundary review.
`smart-home-discovery-service`, `hue-integration`, and
`smart-home-runtime-store`, `smart-home-automation-runtime`,
`smart-home-zwave-integration`, the newly merged `smart-home-zwave-host` serial
host, `smart-home-mqtt-integration`, and `smart-home-zigbee-integration` are
native/service/storage-boundary applicability cases. The Zigbee adapter is pure
logic above coordinator transport and carries only an opaque `VaultRef`, so its
likely portable adapter core should be separated conceptually from a future
native coordinator host. The MQTT host also needs explicit capability metadata
and Vault-mediated credentials before that classification is complete; its deterministic
discovery/topic/entity/value/payload transforms are candidates for a separately
fixture-driven portable core. `venture-browser-macos` is an Apple-native
AppKit/CoreText/Metal host whose expected classification is `native-source`,
not a blind 14-lane port.

The twentieth identity, `venture-browser-windows`, is a mixed split rather than
a complete `native-source` exception. It duplicates deterministic session,
chrome, status, navigation, link, JSON projection, and Mosaic event-bridge logic
from the macOS host; an explicit follow-up moves that behavior into
`venture-browser-core` or a shared fixture-driven bridge. The residual Windows
DLL, WinUI adapter, Direct2D BGRA rendering, native text shaping, and C ABI remain
native-source. The same owner adds the missing capability profile and Windows
ABI, generated-project, pointer-lifetime, panic-containment, and pixel-buffer
validation; other lanes must not duplicate the native shell.

The twenty-first identity, `smart-home-camera-media`, currently mixes a portable
authorization and lease-policy core with host authority. Its first implementation
returns the secret snapshot/stream URI to the lease holder, accepts caller-asserted
identity and time, does not bind a lease to an endpoint generation, owns OS entropy,
and leaves endpoint/lease maps unbounded. The hardening owner replaces redemption
with a host-owned service that installs identity/time/nonce/executor authority once,
revalidates current grants, bounds snapshot bytes, owns broker-minted stream sessions,
retains failed teardown for reported retry, rejects URL userinfo and default plaintext,
binds leases to generations, and imposes global and per-principal quotas. An explicit
policy opt-in exists only for loopback fixtures, where query strings remain forbidden;
secure query tokens remain confined to the executor. A later fixture owner expands
only the resulting authority-free policy core.

The twenty-second identity, `smart-home-onvif-integration`, is also a mixed split.
Correlated discovery and SOAP parsing, deterministic UsernameToken construction,
origin policy, profile projection, and hostile input handling form a portable core.
UDP/DNS/TCP/TLS, trusted time and randomness, Vault credential leases, process I/O,
and reviewed endpoint allowlists remain native. Before extraction, the host must
stop following discovery- or device-controlled XAddr values across origins with a
fresh credential digest, fail closed on insecure non-loopback transport, and replace
ambient username/password environment variables with Vault-mediated credentials.
The nonempty native profiles then require separately tracked Layer 5 evidence.
Its current installation path also mutates bridge and camera endpoint state before
every profile, device, and entity has passed validation. A separate dependency-
ordered item adds validate-and-plan preflight plus atomic commit or rollback so a
late URI, quota, or registry failure cannot leave partial runtime state or rotate
previously valid endpoint generations.

The twenty-third identity, `smart-home-shelly-integration`, is another mixed
split. Gen2/Gen3 device-info and status parsing, authentication-required
classification, component projection, stable identifiers, capability/state
normalization, RPC envelope validation, and command planning are deterministic
portable-core candidates. mDNS, DNS/TCP, plaintext LAN HTTP, trusted time,
console I/O, origin allowlists, runtime effect application, and future Vault
credential delivery remain in the native Rust host. Before cross-lane expansion,
the host must install authenticated session/clock authority once, narrow public
arbitrary RPC access, bind discovery to reviewed private origins, defend DNS
rebinding, redact and bound device-controlled data, make installation and
command effects transactional or compensating, and declare truthful nonempty
runtime/test capability profiles. Authentication-enabled devices remain
fail-closed until a reviewed Vault-mediated flow exists; Layer 5 approval stays
a separate external gate. The same dependency audit found capability drift in
the shared Rust network substrates: `tcp-client` and `http1-client` claim empty
profiles despite concrete network calls, while `udp-client` and
`smart-home-discovery` lack manifests. A separate high-leverage owner corrects
those native boundaries before downstream approval.

The twenty-fourth identity, `smart-home-wled-integration`, follows the same
mixed pattern. `/json/si` DTO validation, master and segment projection, stable
identifiers, capability-bit interpretation, state normalization, brightness,
RGB and mirek conversion, and JSON command planning form the portable candidate.
mDNS, DNS/TCP, plaintext LAN HTTP, trusted time, console I/O, pairing/origin
policy, and runtime effects remain native. Before extraction, the host must stop
accepting caller-asserted identity/time and arbitrary public state updates, bind
discovery to reviewed private origins, defend DNS rebinding, bound/redact device
data, reject identity/segment collisions, reconcile returned device state, and
make runtime/device effects transactional or compensating. Truthful nonempty
host profiles and external Layer 5 evidence remain separate owners. A shared
follow-up consolidates the duplicated Shelly/WLED DNS, TCP, request encoding,
bounded response, chunked decoding, and error projection behind a native LAN-
HTTP executor while keeping `smart-home-local-http` a pure request planner.

The newest identity, `smart-home-nanoleaf-local-integration`, is another mixed
split rather than a blind port. Credential syntax and credential-free origin
configuration, bounded snapshot and state validation, stable identifiers,
capability and state normalization, RGB/HSV and mirek conversion, command
planning, verification, and hostile inputs are deterministic portable-core
candidates. mDNS, DNS/TCP, LAN HTTP execution, physical-presence pairing, token
and Vault handling, trusted time, endpoint approval, CLI I/O, authorization,
and SmartHomeRuntime mutation remain native-host responsibilities. Its portable
owner depends on the shared confirmed command-effect lifecycle so that fixtures
describe confirmed device state rather than optimistic runtime acceptance.

The fourteenth identity, `smart-home-home-assistant-migration`, is a mixed
boundary rather than an automatic fifteen-lane port. Its deterministic export
parsing, normalization, planning, diagnostics, IDs, fingerprints, and receipts
are portable-core candidates; runtime application, atomic filesystem writes,
CLI behavior, three Rust-only runtime dependencies, and the missing capability
manifest require explicit host-boundary and authority classification.

The fifteenth identity, `smart-home-home-assistant-export`, is another mixed
boundary. Its deterministic normalization and export-core logic are portable
candidates, while TLS/WebSocket transport, environment-token intake,
filesystem output, wall-clock metadata, console reporting, and its missing
capability manifest remain host-owned authority that must be classified before
any cross-language extraction.

The sixteenth identity, `smart-home-home-assistant-history`, is also a mixed
split candidate. Deterministic history DTO validation, ordering, fingerprints,
state projection, event planning, diagnostics, and receipts belong in a
fixture-driven portable core. WebSocket/TLS collection, runtime application,
artifact I/O, clock and console effects, and CLI orchestration stay in a Rust
native host. That host must declare reviewed capabilities, use Vault-mediated
token delivery and secure transport, enforce closed resource ceilings, redact
all error paths, and replace artifacts through no-follow durable atomic writes.
The urgent capability, Vault, transport, limit, redaction, and write hardening
depends only on the merged capability-taxonomy contract; it must not wait for
portable-core extraction. A later host refactor can depend on both the hardened
boundary and the extracted core.

The seventeenth identity, `smart-home-home-assistant-definitions`, is the same
kind of deliberate split rather than a blind fourteen-lane port. Its safe-
subset validation and normalization, state and time-pattern triggers,
conditions, scene/action mapping, ordering, uniqueness, fingerprints, reports,
and diagnostics are fixture-driven portable-core candidates. WebSocket and
HTTP/TLS collection, administrator credentials, artifact I/O, wall clock,
console output, and CLI orchestration stay in the Rust native host. The current
host lacks `required_capabilities.json`, accepts plaintext transports, reads an
ambient administrator token, persists server-provided error text, and creates
a predictable link-following temporary output. Repository-local transport,
Vault, limit, redaction, and durable no-follow write hardening must land without
claiming capability approval; a separately blocked Layer 5 item owns the
hardware-key-backed approval evidence, and the later host refactor depends on
both the portable core and hardened boundary.

The eighteenth identity, `smart-home-home-assistant-dashboard-migration`, is
another mixed split. Lovelace DTO validation and normalization,
reviewed-topology projection, standard-card compilation, ordered layout
flattening, entity mapping, diagnostics, summaries, fingerprints, blocking
decisions, receipts, and artifacts are fixture-driven portable-core candidates.
WebSocket/TLS collection, administrator credentials, filesystem and wall-clock
effects, console output, and CLI orchestration stay in the Rust host. That host
currently lacks capability metadata, accepts arbitrary plaintext endpoints,
reads an ambient token, persists server-controlled error text and resource URLs,
has no closed input or nesting limits, and uses predictable link-following
temporary output. Repository-local hardening is therefore independent of broad
classification; a separate blocked item owns Layer 5 approval, and a later host
refactor depends on both hardening and core extraction.

The nineteenth identity, `smart-home-dashboard-core`, is not a host-boundary
exception. It contains deterministic versioned dashboard, view, card, and
resource DTO parsing; identifier and duplicate validation; dry-run rejection;
applied-migration projection; and summaries without runtime capabilities. A
dedicated item owns its explicit empty capability profile, closed language-
neutral fixtures, and dependency-shaped cross-lane expansion. The Home
Assistant dashboard portable-core extraction depends on this native dashboard
contract so it does not duplicate the target representation. Controller
transport, dashboard serving, filesystem I/O, and other host effects remain
outside that portable contract.

The new `smart-home-matter-integration` is also a mixed split candidate rather
than a native-source exception: endpoint projection, report normalization, and
command planning are deterministic zero-capability logic, while D23 keeps the
residual `SmartHomeRuntime` integration Rust-canonical. The backlog now tracks
language-neutral fixtures and hardening for the portable core separately from a
future controller host that owns mDNS, commissioning, certificates, PASE/CASE
sessions, subscriptions, retries, Vault leases, and reviewed host capabilities.

### Likely portable Rust-led families

- `closure-*` compiler passes
- `dsp-*` algorithms
- `iir-*` IR passes and deterministic target emitters
- portable `image-codec-*` packages
- `state-machine-*` tokenization, serialization, and compilation
- language runtimes and frontends such as `r-*` and `twig-*`, when their
  dependency stacks are ready
- deterministic portions of `vault-*`, `adjudication-*`, and `smart-home-*`
- the new Axiom, IDL, and Q frontend/runtime stacks in dependency order
- deterministic SIR lowerings such as `idl-to-semantic-ir`,
  `q-to-semantic-ir`, and `scilab-to-semantic-ir`
- `html-to-layout`, after its document and layout dependencies are classified

`sir-bench` remains a likely tool/harness exception, while
`chief-of-staff-vault-runtime` needs an explicit domain/native applicability
review before any port is selected.

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
   Start with NET03 `http-core`: align raw request-target, query, and route
   behavior across the established ports that currently expose only the basic
   header/head surface.
3. Extend the parity reporter with explicit applicability data rather than
   hard-coding exceptions in reporting logic.
4. Fail CI on unclassified new package buckets.
5. Keep the canonical-collision CI gate enabled.
6. Add a policy check: a new portable singleton must include either another
   language implementation or a declared parity work item/classification.
7. Track package maturity separately from structural presence: manifest,
   source, tests, BUILD, README, CHANGELOG, conformance status, and last verified
   revision.

## Cross-Cutting Stream A: Build-Tool Parity

The build tool is itself a cross-language program and must follow the same
rules as portable packages: directory presence is not behavioral parity.

Executable front doors currently exist in 12 of the 15 established lanes:

- C# and F# under `code/programs/dotnet/`
- Elixir, Go, Haskell, Lua, Perl, Python, Ruby, Rust, Swift, and TypeScript
  under their respective `code/programs/<language>/build-tool` directories

Dart, Java, and Kotlin have no implementation. The F# entry point is currently
a thin facade over the C# engine rather than an independent native engine.
C and C++ are emerging lanes without build tools; OCaml must receive one before
it can graduate from emerging status. WASM is an execution target, Mosaic and
Twig are domain languages, and Starlark is a build language, so none enters this
program-level requirement without a separate applicability decision.

The existing implementations have materially drifted. The Go program is the
broadest reference, but even it has unfinished OS-aware structured command and
Windows execution work. Other ports differ on package discovery, ecosystem
resolvers, diff selection, hashing, Starlark, plan read/write, sharding,
toolchain detection, parallelism, and failure propagation. Specifications 12,
`build-plan-v1`, `build-plan-sharding`, 15, and B05 must be reconciled with
current code before more directory-only ports are added.

Delivery order:

1. Define `build-tool-conformance.md`, a versioned fixture schema, and stable
   machine-readable results for discovery, resolution, graph order, diff
   selection, hashing/cache, Starlark, plan interchange, sharding, execution,
   validation, platform BUILD selection, and toolchain detection. Completed by
   PR #9151.
2. Add a cross-implementation runner. The runner may orchestrate processes in
   Python, but the fixtures and canonical JSON are the behavior oracle. Start
   with non-execution domains and fail closed on every execution case.
   Completed by PR #9157 with the process-free bootstrap runner, implementation
   inventory, and closed discovery, resolution, graph, and plan corpus.
3. Expand the bootstrap's closed input/result schemas and positive plus
   adversarial corpus from discovery, resolution, graph, and plan into the
   remaining non-execution domains: diff selection, hashing/cache, Starlark,
   sharding, validation, toolchain detection, and CLI. The current selected
   tranche now provides a 30-case, 11-domain process-free corpus, including
   conservative unknown-path handling, typed cache states, inline-only
   Starlark loads, prerequisite-closed shard verification, the OCaml-aware
   toolchain registry, and fail-closed BUILD-file validation.
4. Define the process-free trusted-execution policy core: closed execution
   input/result records, reviewed corpus and adapter digests, explicit operator
   authorization, stable backend-unavailable results, and a backend interface
   with no host-execution fallback. This tranche validates authority but never
   executes fixture code. Completed by PR #9178.
5. Implement the Linux OCI backend first. The completed first tranche defines a
   closed identity for exact rootless Podman, `crun`, Conmon, OCI manifest/config,
   seccomp, shim, and invariant-probe artifacts; proves local non-remote
   rootless operation, cgroup v2 delegation, seccomp, exact binaries, and the
   already-present image; and constructs the exact no-pull, private-namespace,
   read-only, capability-free probe-container argv. It invokes neither that
   argv nor a fixture, and Linux remains unavailable. Completed by PR #9189.
6. Before trusted Linux execution, bind operator approval to the complete
   reviewed authority bundle, bind case selection to the exact hashed
   root-handle/no-follow snapshot, and normalize dependency-skip/result-state
   semantics. Then execute the runner-owned Linux invariant probe with
   aggregate cgroup CPU metering, combined streaming output/result accounting,
   hard writable-workspace semantics, cancellation, and verified
   whole-container kill/reap/removal. The completed first prerequisite
   replaced corpus-only approval with a
   process-free, domain-separated external bundle over the reviewed source
   revision, policy, schemas, authority verifier, Linux preflight backend, and
   external backend identity. This first closed profile binds no corpus or
   adapter and approves bytes for capability inspection only; it exposes no
   process handoff. PR #9208 added a separately domain-bound atomic exact-byte
   backend/import-closure loader and a one-shot isolated loadability worker; it
   runs no capability command. PR #9231 completed the separately authorized
   protected command broker: it retains verified statically linked Podman,
   `crun`, Conmon, and state descriptors; permits only the two closed preflight
   operations; confines pathname-backed execution to retained Podman with
   Landlock; streams one combined output ceiling; and owns delegated-cgroup
   descendant cleanup. PR #9270 closed anonymous executable memfds, `dlopen`,
   executable mappings, and descriptor-replenishment paths with separately
   reviewed kernel enforcement and Linux integration probes. A verifiable
   immutable runner-image TCB attestation is also required before the
   subsequent invariant-probe authority profile; this covers the protected
   interpreter, standard library/native extensions, libc, loader, container
   configuration defaults, and other immutable host dependencies outside the
   three explicitly bound runtime executables. Run
   protected probe enforcement to produce containment evidence, then add a
   distinct trusted-execution profile binding that evidence, the exact case
   snapshot, and one adapter before trusted-case enforcement. Only protected
   evidence can mark Linux
   ready.

   The attestation item is not implementable on the repository's mutable
   GitHub-hosted `ubuntu-latest` runner without circular self-attestation.
   `build-tool-immutable-runner-attester-provisioning` is therefore an explicit
   externally blocked prerequisite: infrastructure owners must select and
   provision the immutable or measured runner/image subject, measurement root,
   out-of-band attester trust root, protected no-secret workflow, receipt
   storage, and rotation/revocation procedure. The repository attestation item
   remains pending until that reviewed subject exists; neither item runs a
   capability probe, fixture, adapter, or invariant probe. Unrelated
   deliverable parity edges continue meanwhile.
7. Implement the native Windows boundary with AppContainer or LPAC plus Job
   Objects and root-handle reparse-safe filesystem operations. Keep macOS
   non-passing until a signed helper or isolated VM can prove the same
   filesystem, network, resource, and tree-termination guarantees;
   `sandbox-exec` alone is not sufficient evidence.
8. Add closed execution-semantics cases for command ordering, failure
   propagation, dependency skips, dry-run, jobs, resource locks, legacy shell
   behavior, and direct argv. Keep escape, network, environment, link-race,
   cancellation, and resource-exhaustion checks as runner-owned non-oracular
   probes.
9. Make the Go reference pass the contract, including structured Starlark
   context/commands and the B05 Windows executor contract.
10. Remediate existing ports in fixture-failure order: C#/F#, Python, Ruby,
   Swift, TypeScript, Elixir, Rust, Perl, Haskell, and Lua. Each independent
   engine is its own PR; a reviewed shared-engine exception must be explicit.
11. Add language-native Java and Kotlin implementations using shared JVM fixture
   infrastructure but separate engines, then add Dart.
12. Add the OCaml implementation after the lane foundation is stable.
13. Decide and document whether C and C++ require native build tools before
   either emerging lane graduates.
14. Gate completion in CI: every supported implementation runs the same
   conformance corpus on its applicable operating systems.

Merged PR #9368 completed the repository-owned
`build-tool-execution-status-normalization` slice. It replaces the duplicated
`dependency-skipped` schema term with normative `dep-skipped` and closes
contradictory command exit codes, package return codes, dry-run states, overall
outcomes, duplicate result identities, fail-stop ordering, and dependency
propagation. This bounded schema-and-validator repair lands before execution
cases, trusted execution, the Go oracle, OCaml's build substrate, or adapter
work consumes those records.

Merged PR #9371 completed `build-tool-execution-case-snapshot`. It defines the
typed selector and held-snapshot boundary, opens one bounded direct-member
corpus snapshot through a retained no-follow root, and preserves the exact
hashed bytes for later execution while rejecting path, rename, symlink,
hardlink, case/Unicode-alias, and post-digest substitution. This process-free
slice grants no execution authority and marks no backend or adapter ready.

Merged PR #9375 completed `dart-scaffold-capability-schema` with green Linux,
macOS, Windows, and CodeQL checks. The collision-checked post-merge inventory
found thirteen Dart-only 14-of-15 gaps. The serial loop selected
`dart-trig-wave` because PHY00 `trig` is a dependency-free numeric leaf and
PHY01 `wave` is its direct consumer. PR #9383 merged that pair with green Linux,
macOS, Windows, and CodeQL checks, and PR #9390 then merged the closed shared
PHY00/PHY01 oracle plus its always-on Dart consumer. The refreshed
collision-clean inventory has 11 remaining Dart-only frontier gaps and one
newly owned Rust Home Assistant definitions singleton. The next build-tool
execution successor remains `build-tool-bootstrap-execution-fixture`, which is
still blocked on the Linux OCI enforcement chain.

Final Dart review hardened the selected implementation for subnormal square
roots, infinity and signed zero, non-finite wave parameters and time, angular-
frequency overflow, zero-amplitude evaluation, and extreme finite products.
The shared PHY00/PHY01 corpus is now merged. The serial loop selected the
existing-lane PHY00 square-root audit next because it is the foundational
dependency and its tiny-normal, minimum-subnormal, maximum-finite, infinity,
NaN, and signed-zero cases are now normative. PR #9395 merged that foundational
repair with all 15 checks green. The collision-clean post-merge inventory then
selected `phy01-nonfinite-validation-backfill`; PR #9400 merged that explicit
consumer repair with all 15 checks green. The refreshed `20afefa7a` inventory
then selected `phy00-atan-tiny-signed-zero-cross-lane-audit` because its only
dependency is merged and the shared corpus can now express the cross-lane
negative-zero and subnormal-floor failures exactly.

The post-#9368 dependency audit also found that trusted authority requires one
held case and one selected in-image adapter even though both checked-in
inventories are empty and normal adapter work sits downstream of the execution
semantics corpus. The new `build-tool-bootstrap-execution-fixture` item follows
the snapshot, status normalization, and Linux OCI enforcing boundary, then
lands one inert schema-valid case plus one untrusted digest-bindable bootstrap
adapter without claiming conformance or execution permission. Trusted authority
and Linux trusted execution explicitly depend on that bootstrap; the full
cross-platform corpus remains downstream of every platform sandbox boundary.

Pull-request CI may validate the policy, schemas, digests, and fake-backend
tests, but it must not authorize execution from branch-modifiable code or
fixtures. Real sandbox probes run only from reviewed immutable revisions in a
protected `push`/manual workflow with read-only repository permissions, no
repository secrets, and an out-of-band approved authority-bundle digest. The
corpus digest remains an internal consistency identity and cannot authorize
execution. The trusted runner never pulls a container image and never falls
back to unsandboxed host execution.

The pure-domain security review also discovered three follow-on gates that must
land before final adapter parity:

- define a common inert CLI argv grammar and typed parse-result corpus; the
  current process-free CLI cases intentionally classify exit decisions only;
- add adversarial Starlark metering fixtures for fuel, recursion, allocation,
  load depth/count/cycles, and diagnostic output before any native evaluator
  can claim final Starlark conformance.
- model deterministic inline inputs and semantic oracles for dependency,
  standalone-prerequisite, Starlark-declaration, identity, toolchain, and path
  validation checks before admitting them to the closed v1 schema.

The 41622fa7 dependency audit made those three gates direct prerequisites of
`build-tool-go-oracle`; the merged pure-domain umbrella alone cannot make the
oracle ready while its CLI, Starlark-metering, and validation-oracle children
remain pending. The same audit split `ocaml-build-substrate-process-free-core`
from the execution-coupled OCaml substrate. Repository-owned OCaml discovery,
opam/Dune resolution, hashing, validation, shard cost, affected-node behavior,
and workflow markers can now land before external execution infrastructure,
while opam-switch serialization, canonical execution conformance, and OCaml
promotion remain gated on the Go oracle and trusted-execution contract.

The Linux OCI review discovered eight additional dependency gates:

- replace corpus-only approval with a domain-separated external authority
  profile. The first profile binds the exact source, policy, schemas, verifier,
  Linux preflight backend, and external identity for capability inspection
  only; later profiles add launcher, seccomp, image, shim, corpus, and adapter
  bytes only after those components are enforced;
- load the process-free backend only from its exact retained approved import
  closure, and execute process-owning code only through the separately bound
  broker, using atomic beneath-root handles rather than name-based imports or
  check-then-open traversal;
- require the retained Podman identity to declare static linkage, reject any
  malformed/non-amd64 ELF or `PT_INTERP`, and confine each brokered capability
  command with a mandatory Landlock execute ruleset whose only allow-rule is
  the exact retained Podman inode, so a dynamic loader cannot become a
  trampoline and Podman
  constructor hooks, pause helpers, and every other pathname-backed helper in
  the reviewed flow cannot escape the closed command authority;
- close anonymous executable memfds, executable mappings, and other
  in-memory-code paths with separately reviewed kernel enforcement before any
  invariant-probe evidence becomes authoritative;
- bind protected Linux evidence to a verifiable immutable runner-image TCB
  receipt before an invariant-probe authority can rely on ambient interpreter,
  libc/loader, configuration, or helper dependencies;
- execute only the exact direct corpus member bytes from the approved
  root-handle/no-follow snapshot, never a reopened caller path;
- normalize `dep-skipped` terminology and enforce result-state/return-code
  invariants before execution fixtures land;
- separate identity/preflight and exact command construction from actual
  containment enforcement, aggregate resource accounting, lifecycle cleanup,
  and protected runner-owned probe evidence.

The post-#9323 security audit also found that `adj-lang-cli` has a legacy
effective zero-capability profile despite filesystem, environment, process, and
stdout use in its runtime/tests, with PR #9324 adding another E2E instance.
Keep that unrelated repair in the dedicated `adj-lang-cli-capability-profile`
item: separate harness authority from the publishable runtime profile, declare
truthful capabilities, and obtain Layer-5 approval for every nonempty profile.

## Cross-Cutting Stream B: Introduce OCaml Safely

OCaml begins as an `emerging_implementation` lane. It must not silently change
the current 15-language denominator until its package, build, security, and CI
substrate is real. Use OCaml 5.2.1, opam 2.5.2, Dune 3.17.2 with Dune language
3.16, Alcotest 1.9.0, `bisect_ppx` 2.8.3, and `ocamlformat` 0.27.0.

Merged PR #9323 delivered `ocaml-lane-contract`. OCAML01 classifies
OCaml as a known emerging bucket, keeps it outside established coverage and
missing-slot calculations, and derives the reporter's upper completion-band
bound from the established-language count. This is the denominator-safe
prerequisite for every remaining bootstrap step.

Bootstrap order:

1. Add an OCaml lane contract. Make the reporter's hard-coded `10-15`
   completion band denominator-safe, classify OCaml as emerging, and test that
   OCaml packages create neither unknown buckets nor 15-lane missing slots.
   Complete in PR #9323.
2. Add complete Go and TypeScript scaffold templates plus golden tests,
   repository ignores, capability metadata/schema support, and the
   `code/packages/ocaml/` lane README. Complete in PR #9336.
3. Provision the exact direct OCaml/opam/Dune/Alcotest/`bisect_ppx`/
   `ocamlformat` toolchain on Ubuntu, macOS, and Windows; lock the
   opam-repository/switch transitive solver state separately per runner family;
   verify lock and package-receipt digests against a fresh solve; and run both
   generated scaffold kinds without skips. Hosted-runner image metadata is
   diagnostic evidence only and is not an immutable host-image attestation.
   Complete in merged PR #9354. Final runs 30605415709, 30605415708,
   30605413650, and 30605415829 prove the contract, three fresh solves, three
   locked fixtures, repository builds, and CodeQL detection are green.
4. Add the process-free OCaml build substrate now that the cross-platform
   toolchain evidence and pure-domain corpus are complete: discovery,
   opam/Dune dependency resolution, source hashing, language detection,
   validator support, shard cost, affected-node behavior, and CI workflow
   markers. Keep opam-switch serialization and canonical execution
   conformance in the separately tracked execution-coupled tranche, which
   remains gated on `build-tool-go-oracle`.
5. Exercise the full path with a real dependency chain:
   `logic-gates`, then `graph -> directed-graph -> state-machine`. Every package
   needs native tests, formatting, measured coverage, README, changelog,
   capability metadata, opam/Dune manifests, and BUILD/BUILD_windows.
6. Add an OCaml capability analyzer over compiler-libs ASTs, covering process
   execution, dynamic loading, unsafe marshaling, and `Obj.magic` under the
   repository's explicit capability/exception policy after the shared
   category/action constraint schema is complete.
7. Implement the OCaml build tool on `directed-graph` and require two-way
   build-plan interchange plus the shared conformance corpus.
8. Promote OCaml into the implementation denominator only when Ubuntu, macOS,
   and Windows run real tests without a skip path, the representative chain and
   build tool are green, capability enforcement is active, and the generated
   16-lane backlog has been explicitly reviewed.

After promotion, start the OCaml portable-core queue with `http-core -> http1`,
then recompute dependency-shaped high-consensus work alongside the existing
Dart, Java/Kotlin, Swift, and Haskell lanes.

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
