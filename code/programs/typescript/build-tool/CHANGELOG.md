# Changelog

All notable changes to the TypeScript build tool will be documented in this file.

## [Unreleased]

### Changed

- Rust dependency names are deduplicated with a set rather than a list scan,
  so a manifest naming many crates costs time linear in its size (security
  review, round 2).
- Replace the nine-language hard-coded source suffix/metadata allowlists with
  the complete checked 23-language, seven-role package-data registry. Production
  selection now honors root-only manifests and capability metadata, scoped
  resources/native companions, exact TypeScript site inputs, and fail-closed
  unknown languages in extension and declared-source modes.
- Source hashing now follows the 27-component neutral generated-directory
  registry, pruning exact lowercase `blib` while retaining `Blib` and
  `blib-example` in both extension and declared-source modes.

### Added

- Add pure bounded graph and diff-selection operations that dynamically consume
  the exact eight graph and twelve diff-selection neutral fixtures. The new
  boundary preserves canonical levels and closures, strict portable globs,
  exact shared-input fanout, a 50,000,000-unit preflight ceiling, ancestor-safe
  Unicode root identity, stable no-partial failures, and bounded alias-resistant
  repository-boundary digest validation without adding host authority.
- Make the generic BUILD front install the exact lockfile with `npm ci`, avoiding
  host npm metadata rewrites before typechecking and coverage.
- Add an atomic snapshot sync/check command and native tests comparing every
  registry field and domain-separated digest to the checked neutral fixture.
  All seven package-local source-collection cases run through the production
  selector; repository-boundary adoption remains separate.
- Add a pure bounded `evaluateToolchainSnapshot()` API and a package-local
  suite that dynamically discovers all 11 language-neutral declaration
  fixtures and evaluates them through it. The adapter preserves exact CRLF and
  platform-front semantics, returns a fresh complete 16-key result, and rejects
  hostile direct snapshots without adding host or execution authority.
- Add exact pinned TypeScript and Node declaration development dependencies,
  an explicit strict no-emit `typecheck` command, and a generic BUILD-front
  gate that runs static validation before coverage on every platform.
- Add a pure `validateOrphanCrateSnapshot()` API that independently consumes
  all four language-neutral orphan fixtures, derives exact direct and ancestor
  BUILD coverage, validates and redacts exemption policy, reports stale debt
  and active `PENDING` counts, and uses pinned Unicode 17 identities without
  adding filesystem, Git, process, environment, or network authority.
- Add a pure `validateTrackedArtifactSnapshot()` API that independently
  consumes all five language-neutral tracked-artifact fixtures, redacts unsafe
  input, normalizes separators, detects exact/case/compatibility aliases of
  `node_modules`, and sorts diagnostics by Unicode scalar value without adding
  Git, filesystem, process, environment, or network authority.
- Add generated source-embedded Unicode 17.0.0 NFC, NFKC, full default case
  folding, and full uppercase tables. The generator verifies exact upstream
  sizes and SHA-256 identities plus official normalization/casing vectors; the
  package now ships `UNICODE-LICENSE.txt` and declares
  `MIT AND Unicode-3.0`.
- Add an explicit package-file allowlist so local coverage and dependency trees
  cannot make the npm distribution state-dependent or leak generated reports.

### Fixed

- Cargo link-table security fixes. Inline values continuing onto later lines
  are joined in linear time: a `TomlBracketScanner` carries the bracket
  depth, the open string's delimiter and a pending escape from line to line
  (so a string left open at a line end still continues after the joining
  space), and the pieces are joined once, instead of rescanning the growing
  value for every appended line, which was quadratic. Quoted field and
  sub-table keys (`a = { "path" = "../a" }`, `'package' = "..."`) are read as
  the bare key, compared verbatim and case-sensitively, per the amended
  contract. A Cargo path entry that resolves to the crate itself is now
  `DependencySelfEdgeError` (`DEPENDENCY_SELF_EDGE`, repository-relative
  manifest; the CLI exits 2) instead of a silently drawn self-loop.
  `trimSpacesAndTabs` is an index scan (its `/[ \t]+$/` took 50 s on a 100 KB
  interior run of spaces) and the `[package]` `name` reader uses `[ \t]*`
  instead of a multi-line `\s*` (32 s on 100,000 blank lines). Tests: the 11
  new one-shape cases (57, now with a row-count assert), a 20,000-line
  continuation, a 100 KB run of spaces, 100,000 blank lines before the name,
  self-edges by alias, by name and from a target table, and a real CLI exit-2
  test.
- Rust resolution now reads path dependencies from `[build-dependencies]`,
  `[build_dependencies]` and every `[target.<cfg>.…]` form of the three link
  tables, not just the top-level `[dependencies]`, and reads the
  `[dependencies.<name>]` sub-table form, so a change to a crate used only by
  a build script or on one target now rebuilds its dependents. This implements
  the Rust line procedure of `code/specs/build-tool-conformance.md`, ported
  from the Go reference: headers are split into dotted-key segments outside
  strings (quoted cfgs stay one segment, table names compare case-sensitively,
  `[[...]]` and malformed headers end the table); comments are stripped
  outside strings; inline values whose arrays continue onto later lines are
  joined; `path` and `package` field keys must be exact and hold one non-empty
  string; names are ASCII-lowercased and deduplicated. Dev-dependencies (every
  form), `[patch]`, `[replace]`, `[workspace]` and registry entries add no
  edge.
- The old reader counted any `[dependencies]` line mentioning `path`, so a
  registry entry with `features = ["path"]` drew an edge, and it looked up
  the entry key instead of a `package` rename. Both are fixed.
- Tests: the 46 one-shape cases of the Go reference
  `TestCargoLinkDependencyNames` (`cargoLinkDependencyNames` is exported
  for them), duplicate collapse, and the shared
  `resolution-rust-field-aware.json` fixture. The reader returns the same
  dependency names as the Go reader for all 1,650 tracked `Cargo.toml`
  files.

- Replace recursive globstar suffix exploration and UTF-16 code-unit matching
  with compiled Unicode-scalar patterns and bounded rolling-row dynamic
  programs. Portable character classes now follow the strict shared
  Python-fnmatchcase contract, invalid classes raise one stable typed error,
  and hashing plus Git-diff selection validate complete declared-source lists
  before filesystem enumeration or candidate short-circuits.
- Complete portable TypeScript source hashing with OCaml `.ml`, `.mli`, and
  `.opam` recognition plus exact `.ocamlformat`, `dune`, and `dune-project`
  metadata in extension and declared-source collection.
- Frame sorted normalized repository-relative UTF-8 paths and exact raw file
  bytes with unsigned 64-bit lengths, matching hashing v1 and making
  same-content renames invalidate the cache without leaking checkout paths,
  host locale, decoded text, or metadata into the digest.
- Exclude the complete exact, case-sensitive 26-component generated,
  dependency, VCS, cache, and temporary-directory registry before both
  extension and declared-source hashing. Case variants and near names remain
  eligible, and directory links remain outside the recursive walker.
- Exclude Dune's exact case-sensitive `_build` output component from package
  discovery while preserving `_Build` and `_build-example` source directories.
  The shared language-registry fixture and a direct Windows-safe regression
  cover the boundary.
- Exclude Cabal `dist-newstyle` output from package discovery while preserving
  similarly named source directories, matching the shared language-registry
  fixture.
- Normalize Git-diff package roots to repository-relative forward-slash paths
  and make the Git integration fixtures shell-independent on Windows.
- Emit build-plan package `rel_path` values with portable forward slashes on
  Windows, backed by the shared plan fixture and a real CLI subprocess test.
- Classify languages only from the canonical package/program bucket, preserve
  `language/programs/name` identities, and exclude specification fixtures.
- Reject residual duplicate qualified names with the stable
  `DUPLICATE_PACKAGE_IDENTITY` diagnostic, sorted repository-relative paths,
  and CLI exit code 2.
- Decode Lua rockspec metadata as strict UTF-8 and fail closed with the portable
  `METADATA_INVALID_UTF8` diagnostic and CLI exit code 2 for malformed input.
- Consume the shared positive and invalid UTF-8 resolver fixtures, including a
  real CLI subprocess test and a regression guard for valid literal U+FFFD text.

## [1.1.0] - 2026-03-22

### Added

- **Glob matching module** (`glob-match.ts`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns.
- **Strict input filtering in git diff**: `mapFilesToPackages()` now respects Starlark `declaredSrcs` patterns. For Starlark packages, only files matching declared source patterns (or BUILD files) trigger rebuilds.
- **Build plan module** (`plan.ts`): Serializes/deserializes build plan as versioned JSON (`schema_version: 1`). Supports `writePlan()` and `readPlan()` with version checking and forward compatibility.
- **`--emit-plan` CLI flag**: Writes build plan JSON to a file and exits.
- **`--plan-file` CLI flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff.

### Fixed

- **Windows path splitting**: `inferLanguage()` now splits on both `/` and `\` for correct language detection on Windows where paths may use backslashes.

## [1.0.0] - 2026-03-21

### Added

- Initial release: complete port of the Python build tool to TypeScript.
- Package discovery via recursive BUILD file walk (`discovery.ts`).
- Platform-specific BUILD file support: `BUILD_mac`, `BUILD_linux`, `BUILD_windows`, `BUILD_mac_and_linux`.
- Dependency resolution for all 6 languages: Python, Ruby, Go, TypeScript, Rust, Elixir (`resolver.ts`).
- Inline DirectedGraph implementation with Kahn's algorithm for topological sorting.
- Git-based change detection using `git diff --name-only` (`gitdiff.ts`).
- SHA256 file hashing for cache-based change detection (`hasher.ts`).
- JSON build cache with atomic writes (`cache.ts`).
- Parallel build execution respecting dependency order (`executor.ts`).
- Human-readable build report formatting (`reporter.ts`).
- CLI entry point with `--root`, `--force`, `--dry-run`, `--jobs`, `--language`, `--diff-base`, and `--cache-file` options (`index.ts`).
- Comprehensive test suite with >80% coverage.
- Zero runtime dependencies -- only Node.js built-in modules.
- Knuth-style literate programming throughout all source files.
