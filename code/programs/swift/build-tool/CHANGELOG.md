# Changelog

## Unreleased - source-input boundary sync (rust/arithmetic)

- `code/packages/rust/arithmetic` is no longer a consumer of `starlark-rust-library-rule-consumers`: #19311 (Warrant adoption) rewrote its BUILD from a Starlark `rust_library` target into plain shell commands, so nothing loads `rust_library.star` there any more. The checked repository source-input boundary dropped that root, the projection was regenerated with the repository's generator, and the pinned boundary digest moved from `4569b531...` to `f338935e...`. The boundary has 491 scopes and 505 authorizations (were 492 and 506). No behaviour change in the tool.

## Unreleased - BUILD-001 registry update

- Source-input registry and repository boundary updated for BUILD-001 (generated wasm artifacts are built, not committed): `code/packages/rust/engram-wasm/pkg/engram_engine.wasm` is no longer an exact package input of `rust-engram-wasm-build-inputs`, and `visicalc-html/vendor/spreadsheet-engine-wasm.js` is no longer a cross-package input of the Deno VisiCalc boundary. Registry digest is now `8f3d9ee3...`, boundary digest `ec516870... (combined with the #19050 TypeScript boundary sync)`.

## Unreleased

- Cargo link-table security fixes. Inline values continuing onto later lines
  are joined in linear time: a `BracketScanner` carries the bracket depth,
  open-string delimiter and pending escape from line to line instead of
  rescanning the growing value, which was quadratic. Quoted field and
  sub-table keys (`a = { "path" = "../a" }`, `'package' = "..."`) are read as
  the bare key, per the amended contract. A dependency that resolves to the
  package itself is now `DependencySelfEdgeError` (`DEPENDENCY_SELF_EDGE`,
  exit status 2) instead of a silently inserted self-loop. Tests: 11 new
  one-shape cases (57), a 20 000-line continuation, and Cargo self-edges.

- Rust resolution now reads path dependencies from `[build-dependencies]`,
  `[build_dependencies]` and every `[target.<cfg>.…]` form of the three link
  tables, not just the top-level `[dependencies]`, and reads the
  `[dependencies.<name>]` sub-table form. A change to a crate used only by a
  build script or on one target now rebuilds its dependents. This implements
  the Rust line procedure of `code/specs/build-tool-conformance.md` (public
  `Resolver.cargoLinkDependencyNames`, over UTF-8 bytes): headers are split
  into dotted-key segments outside strings (quoted cfgs stay one segment,
  table names compare case-sensitively, `[[...]]` and malformed headers end
  the table); comments are stripped outside strings; inline values whose
  arrays continue onto later lines are joined; `path` and `package` field keys
  must be exact and hold one non-empty string; names are ASCII-lowercased and
  deduplicated. Dev-dependencies (every form), `[patch]`, `[replace]`,
  `[workspace]` and registry entries add no edge.
- The old reader counted any `[dependencies]` line mentioning `path` and
  looked up the entry key, so `x = { version = "1", features = ["path"] }`
  drew an edge and a `package` rename was ignored
  (`os-kernel = { package = "os-kernel-simulator", ... }` pointed at
  `rust/os-kernel`). Both are fixed.
- WASM wrappers are now known only by their declared Cargo name, as in the Go
  reference. Claiming the bare directory name made 25 wrappers (`wasm/graph`,
  `wasm/heap`, ...) depend on themselves instead of on the Rust crate they
  wrap; the forced repository graph now has no self-edges, and its edges into
  Rust and WASM packages equal the Go tool's (bar one TypeScript edge from
  Go's cross-language resolution).
- Tests: the 46 one-shape cases of the Go reference
  `TestCargoLinkDependencyNames`, duplicate collapse, the WASM-wrapper alias
  rule, and the shared `resolution-rust-field-aware.json` fixture, which now
  also seeds this front (reader S) in `code/specs/ci-gate-registry.md` and the
  Go build tool's routing map. The reader returns the same dependency names as
  the Go reader for all 1,650 tracked `Cargo.toml` files.

- Added a typed, process-free graph and diff-selection core with all eight
  graph and twelve diff-selection neutral fixture cases, exact caller-supplied
  source-boundary digest/fanout, canonical closures, structural diagnostics,
  and operation-wide Unicode-scalar match-work preflight. Native tests also
  cover malformed graph structure, portable-path and glob rejection, root
  aliases, and boundary-error precedence. The existing Git-facing CLI remains
  unchanged.

- Added the `forme-plugin-installer-core`, `forme-plugin-runner-conformance`, and `forme-plugin-runner-ts`
  TypeScript roots to the repository source-input boundary projection. Digest updated to
  `3370c811b51962c87757b01e98ad3db5206cad2c75e74c6853bf5209548bd18e`.

- Regenerated both source-input projections for exact lowercase `blib`, the
  current Lua and Forme boundary consumers, and the new canonical registry and
  boundary digests.

- Regenerated the repository source-input boundary projection against the
  neutral fixture: removed the ten Haskell roots that now carry their own
  `cabal.project` (#16153) and added the `lua/der_asn1` and
  `lua/x509_extension` lint roots the fixture gained earlier without this
  projection being regenerated. Digest updated to
  `4b1830332a5e4510d8195ceb8266e0bfec1610ff680d8a319b6c4513a8bd466e`.

- Regenerated the repository source-input boundary projection to pick up the
  `forme-deploy-runner-fs-adapter`, `forme-deploy-runner-github-pages-adapter`,
  and `forme-plugin-host` TypeScript roots plus the shared Lua lint consumers
  added on main without a complete projection refresh. Digest updated to
  `7983f42a84dc9905f50729798a5d7d4000a4356016eb2b9cfd42217b15070b59`.

- Added the two exact registered TypeScript site roots and their 13 reviewed
  authored inputs to the generated source-input projection and neutral tests.
  Repository-aware hashing preserves legacy `unknown/*` graph identities while
  deriving only those exact TypeScript profiles; other sites fail closed.

- Align declared-source glob matching with the language-neutral portable
  grammar and exact 50,000,000-unit match-work budget. Character classes now
  implement Python-compatible negation, ranges, and literal edge characters;
  invalid classes fail before source or diff selection, and dynamic programming
  bounds globstar and segment matching without regex backtracking.
- Adopt the complete checked repository source-input boundary in production
  hashing and diff selection. A generated typed projection covers all 18
  boundaries, 21 registrations, 19 tracked paths, nine neutral cases, and the
  domain-separated digest while pinning the package-local registry it extends.
  Each batch proves applicable paths through bounded NUL-delimited stage-zero
  Git index records with regular modes `100644` or `100755`, rechecks the index
  after hashing, opens exact files through retained no-follow repository
  handles, and unions them with package-local inputs under the existing framed
  hash and byte ceilings. The same projection reverse-indexes boundary-only
  diffs to exact consumers without ambient ancestor, generated-directory,
  credential, secret, symlink, reparse, submodule, conflict-stage, or near-path
  authority.
- Replace the Swift hasher's independent source and metadata maps with a
  generated, source-embedded projection of the complete 23-language neutral
  registry. Production collection now distinguishes all seven selector roles,
  consumes the four package-local neutral fixtures, includes exact hybrid
  companions, resources, scripts, Engram WASM inputs, and root capability
  metadata, rejects unknown languages and portable-path aliases, and bounds
  candidate, result, per-file, and aggregate bytes without accepting a runtime
  registry path. Canonical package paths must contain a package name and match
  the selected language; incremental enumeration applies the candidate ceiling
  before sorting; and tests recompute the domain-separated digest while
  checking every fixture pin. Repository-relative inputs remain a distinct
  exact projection rather than being widened into package-tree authority.
- Bound retained-handle file reads to the exact snapshotted size and probe one
  byte past it without appending, so a concurrent POSIX writer cannot grow a
  source beyond the per-file ceiling before the mutation check.
- Complete portable source hashing with every established lane's primary
  sources and package metadata, the shared 26-component exact pruning registry, OCaml
  coverage, strict declared-source selection, and all 13 neutral collection
  cases. Package digests now use the local pure Swift SHA-256 package and
  hashing-v1 path/content length frames on every platform; no-follow reads
  retain and recheck ancestor/object identities, and failures use one stable,
  root-redacted CLI diagnostic with exit code `2`.
- Add a pure bounded extra-CI toolchain snapshot evaluator that consumes all 11
  language-neutral declaration fixtures. It enforces selected BUILD-front
  precedence, UTF-8 byte and LF-delimited logical-line ceilings, exact CRLF
  grammar, stable deduplication, the complete 16-key registry, and deterministic
  unsupported diagnostics without importing Foundation or gaining host
  authority.
- Make `BUILD_windows` distinguish a missing Swift toolchain from a failing
  native test run. Missing Swift still emits the stable skip diagnostic and
  exits successfully; present-toolchain failures now propagate their exact
  nonzero status.
- Exclude Dune's exact, case-sensitive `_build` generated directory from
  package discovery and source hashing while preserving `_Build` and
  `_build-example`. Shared-fixture and focused Swift tests cover the boundary.
- Add a pure orphan-crate snapshot validator consuming all four shared neutral
  fixtures. It enforces direct and ancestor BUILD coverage, independent empty
  BUILD reporting, exact artifact exclusions, portable and reasoned exemption
  records, NFC full-fold duplicate identities, stale-entry precedence,
  hostile-path redaction, pending counts, and deterministic Unicode-scalar plus
  Python-compatible ASCII JSON diagnostic ordering without filesystem,
  process, environment, credential, link, or network authority.
- Add a pure tracked-artifact snapshot validator consuming all five shared
  neutral fixtures, with hostile-path redaction, exact problem precedence,
  Unicode-scalar length and ordering, inert entry metadata, and exact
  Unicode 17 alias plus Windows-reserved-basename behavior.
- Add generated, source-embedded Unicode 17.0.0 NFC, NFKC, full-fold,
  NFKC-fold, and full-uppercase tables, the Unicode License v3 notice, and a
  required isolated Swift 6.3.3 official-vector CI check that preserves the
  driver-sensitive `swift` and `swiftc` entrypoint names and limits POSIX
  linker discovery to root-owned system tool directories.
- Exclude Cabal's exact, case-sensitive `dist-newstyle` generated directory
  from package discovery and source hashing while preserving near-names such
  as `dist-newstyle-example`. Shared-fixture and focused Swift tests cover the
  boundary.
- Classify package and program buckets with the canonical discovery language
  registry, exclude specification fixture trees, preserve program identities,
  and fail closed on duplicate qualified names with
  `DUPLICATE_PACKAGE_IDENTITY`, sorted repository-relative paths, and CLI exit
  code `2`. Shared registry and duplicate-identity fixtures now cover the API
  and real executable.
- Decode Lua `.rockspec` metadata as strict UTF-8 before dependency parsing.
  Invalid bytes now fail closed with `METADATA_INVALID_UTF8`, stable package
  and repository-relative manifest identity, CLI exit code `2`, and no
  checkout-path disclosure. Resolver and CLI coverage consume the shared
  positive and invalid-byte conformance fixtures and require the exact
  expected edge set.

## 0.1.0

- Added a full Swift port of the monorepo build tool.
- Implemented package discovery, dependency graph resolution, hashing, cache persistence, parallel execution, plan IO, and CI validation.
- Added lightweight Starlark BUILD parsing for the repo's declarative BUILD rules.
- Added focused Swift tests covering discovery, Swift dependency parsing, plan round-tripping, Starlark parsing, and CI validator behavior.
