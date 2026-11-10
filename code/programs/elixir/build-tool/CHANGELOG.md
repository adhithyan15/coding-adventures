# Changelog

All notable changes to this project will be documented in this file.

## Unreleased - BUILD-001 registry update

- Source-input registry and repository boundary updated for BUILD-001 (generated wasm artifacts are built, not committed): `code/packages/rust/engram-wasm/pkg/engram_engine.wasm` is no longer an exact package input of `rust-engram-wasm-build-inputs`, and `visicalc-html/vendor/spreadsheet-engine-wasm.js` is no longer a cross-package input of the Deno VisiCalc boundary. Registry digest is now `8f3d9ee3...`, boundary digest `8bc50843... (combined with the #19050 TypeScript boundary sync)`.

## [Unreleased] - 2026-10-06

### Added

- Add a pure, bounded graph and diff-selection evaluator that dynamically
  consumes the exact eight graph and twelve diff-selection neutral fixtures.
  It provides canonical graph levels, stable all-or-error diagnostics,
  dependent and prerequisite closures, strict memoized Unicode-scalar globs,
  exact BUILD-front handling, domain-separated repository-boundary validation,
  deterministic shared-input fanout, and the operation-wide 50,000,000-unit
  match-work preflight without adding checkout, Git, process, environment,
  network, clock, randomness, credential, or execution authority.
- Bound repository-boundary object graphs before canonical encoding and reject
  shared-container expansion, excessive depth, value count, or aggregate UTF-8
  string data. Package-root overlap detection now uses folded path segments, so
  punctuation ordering and Unicode fold aliases cannot hide ancestry.

### Changed

- Discovery now requires an exact regular canonical `BUILD` before any
  platform-specific variant can select a recipe. The native Windows selector
  maps BEAM's `{:win32, _}` OS result to `:windows`, so `BUILD_windows` wins over
  canonical Starlark content while variant-only directories remain outside the
  package graph on every host. Exact directory-entry checks also reject
  wrong-case `build` and `build_windows` on case-insensitive hosts. Native
  no-follow policy remains separately owned.
- Discovery rejects colliding qualified identities before resolution with a
  typed `DUPLICATE_PACKAGE_IDENTITY` error and sorted repository-relative
  paths. The CLI emits the root-redacted diagnostic and returns exit status 2.
  A package-local test compares the native error projection with the checked
  neutral duplicate fixture; no adapter registration or native no-follow
  authority is claimed.
- Discovery now checks the complete neutral language-registry fixture: all 29
  BUILD records yield the 25 expected package and program identities. The
  walker uses exact boundary buckets, retains `programs/` names, and excludes
  `.dart_tool`, `dist-newstyle`, specification trees, and other generated
  components without excluding `_Build` or `_build-example` source paths.
  Canonical BUILD membership and native Windows overrides are covered by the
  checked fixture projection below.
- Discovery now recognizes the emerging OCaml bucket needed to project the
  shared Dune fixture, and its native test checks exact `_build` exclusion
  alongside positive `_Build` and `_build-example` paths. The Go CI selection
  map now includes Elixir as a direct fixture consumer; broader discovery
  semantics remain separately tracked.
- Source selection now uses an immutable, generated package-local projection of
  the complete language source-input registry rather than eight ad hoc suffix
  maps. It covers all seven selector roles, exact generated-directory pruning,
  root-only capability inputs, and all five BUILD fronts; unknown languages,
  unsafe package roots, and colliding candidate paths fail before selection.
- Package source hashes now use Hashing v1: raw UTF-8 repository-relative path
  and raw content bytes, each framed with an unsigned 64-bit big-endian length.
  Deterministic sorting, binary bytes, and read errors no longer pass through
  the old concatenated per-file-hex-digest or sentinel paths. Native reads are
  streamed with candidate, depth, byte, and opened-file identity bounds.

### Fixed

- Cargo link-table security fixes. Inline values continuing onto later lines
  are joined in linear time: `toml_bracket_scan/2` carries the bracket depth,
  the open string's delimiter and a pending escape (`{depth, open, escape}`)
  from line to line (so a string left open at a line end still continues
  after the joining space), and the pieces are collected and joined once,
  instead of rescanning the growing value with `<>` for every appended line,
  which was quadratic. Quoted field and sub-table keys
  (`a = { "path" = "../a" }`, `'package' = "..."`) are read as the bare key,
  compared verbatim and case-sensitively, per the amended contract. A Cargo
  path entry that resolves to the crate itself now raises
  `BuildTool.Resolver.DependencySelfEdgeError` (`DEPENDENCY_SELF_EDGE`,
  repository-relative manifest; the CLI exits 2) instead of the graph's bare
  `ArgumentError`. The `[package]` `name` reader uses `[ \t]*` instead of a
  multi-line `\s*`, which was quadratic in a run of blank lines. Tests: the
  11 new one-shape cases (57), a 20,000-line continuation, 100,000 blank
  lines before the name, self-edges by alias, by name and from a target
  table, and a CLI exit-2 test.
- Rust resolution now reads path dependencies from `[build-dependencies]`,
  `[build_dependencies]` and every `[target.<cfg>.…]` form of the three link
  tables, not just the top-level `[dependencies]`, and reads the
  `[dependencies.<name>]` sub-table form. A change to a crate used only by a
  build script or on one target now rebuilds its dependents. This implements
  the Rust line procedure of `code/specs/build-tool-conformance.md` (public
  `BuildTool.Resolver.cargo_link_dependency_names/1`, over bytes): headers are
  split into dotted-key segments outside strings (quoted cfgs stay one
  segment, table names compare case-sensitively, `[[...]]` and malformed
  headers end the table); comments are stripped outside strings; inline
  values whose arrays continue onto later lines are joined; `path` and
  `package` field keys must be exact and hold one non-empty string; names are
  ASCII-lowercased and deduplicated. Dev-dependencies (every form),
  `[patch]`, `[replace]`, `[workspace]` and registry entries add no edge.
- The old reader counted any `[dependencies]` line mentioning `path` and
  looked up the entry key, so `x = { version = "1", features = ["path"] }`
  drew an edge and a `package` rename was ignored. Both are fixed.
- WASM wrappers are now known only by their declared Cargo name, as in the Go
  reference. Claiming the bare directory name made 25 wrappers (`wasm/graph`,
  `wasm/heap`, ...) self-edges, which `DirectedGraph.add_edge/3` rejects by
  raising.

### Tests

- Add `test/resolver_test.exs`: the 46 one-shape cases of the Go reference
  `TestCargoLinkDependencyNames`, duplicate collapse, build/target/renamed
  sub-table edges against dev and patch tables, the Rust directory and
  declared-name aliases, the WASM-wrapper alias rule, and the module's
  doctests. Over all 1,650 tracked `Cargo.toml` files the reader returns the
  same dependency names as the Go reader.
- Exercise every neutral graph and diff-selection case through production code,
  pin the exact case-ID roster, assert both shared-input consumers independently,
  and cover structural rejection, no-partial cycle and unknown-path failures,
  glob validation, boundary mismatch, and exact/over-limit work accounting.
- Cover compact exponentially expanding shared boundary input, non-adjacent
  punctuation ancestry, and Unicode full-fold root collisions.
- Project the checked Windows-override and variant-without-canonical cases
  through production discovery on explicit platforms, check real Windows host
  selection and emitted shell-only plan, and retain macOS/Linux precedence and
  canonical fallback regressions.
- Compare the full generated projection and domain-separated digest with the
  checked neutral registry, drive every package-local neutral source-collection
  case through production selection, and cover binary frames, path changes,
  generated/near-name pruning, explicit empty declared sources, and fail-closed
  lookup.

Dependency hashing, repository-boundary inputs, cache persistence, and native
retained-handle snapshot hardening remain separate follow-up contracts.

## [0.5.1] - 2026-09-27

### Changed

- Replaced recursive grapheme glob matching with a linear Unicode-scalar parser
  and rolling-row path and segment dynamic programs.
- Added strict portable character classes and stable typed rejection for
  descending or host-ambiguous class syntax.
- Git-diff selection and strict hashing now validate complete declared-source
  pattern lists before BUILD, earlier-match, empty-change, or filesystem
  short-circuits.

## [0.5.0] - 2026-08-30

### Added

- Added `BuildTool.ToolchainDetection`, a bounded process-free evaluator that
  independently consumes all 11 language-neutral toolchain-detection fixtures,
  emits the complete canonical 16-key registry, and preserves exact platform
  precedence, scheduling, CRLF grammar, stable diagnostics, and resource caps.

### Changed

- Discovery now retains the raw selected BUILD front, and production language
  detection plus emitted plans use the same evaluator so exact extra-toolchain
  declarations and toolchain-scoped CI changes are scheduled consistently.

## [0.4.0] - 2026-08-26

### Added

- The pure Elixir validator now consumes the four shared orphan-crate coverage
  fixtures through a closed snapshot API. It implements rooted runnable-BUILD
  coverage, closest empty-BUILD reporting, exact artifact exclusions, portable
  exemption policy, stale-entry diagnostics, and active pending counts without
  adding filesystem, Git, process, environment, or network authority.
- Focused conformance tests cover hostile-path redaction, Python-exact blank
  reasons, Unicode 17 NFC/full-fold aliases, invalid-field precedence, fixed
  BUILD filename ranking, and Python-compatible ASCII-JSON detail ordering.

### Changed

- Canonical validator detail ordering now uses explicit ASCII-only JSON string
  escaping, including UTF-16 surrogate pairs for supplementary scalars, so
  Elixir diagnostics sort exactly like the language-neutral Python oracle.

## [0.3.0] - 2026-08-25

### Changed

- Tracked-artifact snapshots now use the shared build-tool v1 portable-path
  policy with deterministic, root-redacted diagnostics. The pure in-memory
  validator consumes all five language-neutral fixtures and uses generated,
  source-embedded Unicode 17 tables for NFC, NFKC, full default case folding,
  and uppercase behavior; the Unicode License v3 notice ships with the source.
- CI pins Elixir 1.18.4 and OTP 27.3.4.11 while compiling the generated module
  and exercising every official Unicode 17 normalization, folding, and
  unconditional-uppercase vector, so runtime conformance cannot drift behind
  mocked generator-boundary tests.
- The Perl BUILD-contract helper now uses supported `Enum.filter/2` and
  `Enum.map/2` calls, preserving behavior while keeping warnings-as-errors
  compilation clean on the pinned Elixir toolchain.

## [0.2.0] - 2026-03-22

### Added

- **Glob matching module** (`BuildTool.GlobMatch`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns.
- **Strict input filtering in git diff**: `BuildTool.GitDiff.map_files_to_packages/4` now respects Starlark `declared_srcs` patterns. For Starlark packages, only files matching declared source patterns (or BUILD files) trigger rebuilds.
- **Build plan module** (`BuildTool.Plan`): Serializes/deserializes build plan as versioned JSON (`schema_version: 1`). Supports `write_plan/2` and `read_plan/1` with version checking.
- **`--emit-plan` CLI flag**: Writes build plan JSON to a file and exits.
- **`--plan-file` CLI flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff.
- **`BUILD_windows`**: Windows-compatible BUILD file without shell redirects.

## [0.1.0] - 2026-03-21

### Added

- Initial release: full port of the Go build tool to Elixir.
- `BuildTool.Discovery` — recursive BUILD file discovery with skip list and platform-specific BUILD files.
- `BuildTool.DirectedGraph` — inline directed graph with Kahn's algorithm for topological levels and affected-node computation.
- `BuildTool.Resolver` — dependency resolution for Python, Ruby, Go, TypeScript, Rust, and Elixir packages.
- `BuildTool.GitDiff` — git-based change detection using three-dot diff with two-dot fallback.
- `BuildTool.Hasher` — deterministic SHA256 hashing of source files and transitive dependency hashes.
- `BuildTool.Cache` — Agent-based JSON build cache with atomic writes.
- `BuildTool.Executor` — parallel build execution by dependency level using `Task.async_stream`.
- `BuildTool.Reporter` — fixed-width terminal report table.
- `BuildTool.CLI` — escript entry point with the same flags as the Go build tool.
- Progress bar integration via the `CodingAdventures.ProgressBar` package.
- Comprehensive test suite covering all modules.
