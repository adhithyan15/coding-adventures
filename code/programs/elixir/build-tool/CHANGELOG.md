# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2026-10-06

### Changed

- Discovery now checks the complete neutral language-registry fixture: all 29
  BUILD records yield the 25 expected package and program identities. The
  walker uses exact boundary buckets, retains `programs/` names, and excludes
  `.dart_tool`, `dist-newstyle`, specification trees, and other generated
  components without excluding `_Build` or `_build-example` source paths.
  Duplicate identities, canonical BUILD membership, and native Windows
  overrides remain separate follow-up contracts.
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

### Tests

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
