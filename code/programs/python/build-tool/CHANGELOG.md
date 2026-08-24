# Changelog

All notable changes to this project will be documented in this file.

## [0.3.8] - 2026-08-24

### Added

- **Tracked-artifact validation conformance**: the Python validator now consumes
  all four language-neutral `tracked_artifact_absence` cases, including exact,
  nested, separator-normalized, case-folded, and Unicode compatibility aliases
  of `node_modules`.
- **Closed redacted failures**: unsafe raw paths produce only the stable
  `TRACKED_ARTIFACT_PATH_INVALID` diagnostic at `repository`; safe forbidden
  paths produce `TRACKED_ARTIFACT_FORBIDDEN` in canonical order. Entry-kind
  metadata remains inert and grants no Git, filesystem, or process authority.

## [0.3.7] - 2026-08-13

### Fixed

- **Atomic repeated plan emission**: `write_plan` now publishes through the
  platform replace-if-present primitive, so a second `--emit-plan` to the same
  path succeeds on Windows instead of failing with `WinError 183`.
- **Failure cleanup contract**: a failed replacement preserves the previously
  published destination and removes the exclusively created writer-owned
  sibling temporary file on a best-effort basis, including after a staging
  write failure.
- **Shared overwrite fixture**: Python consumes the language-neutral
  `plan/replace-existing` case on Windows and POSIX.

## [0.3.6] - 2026-08-12

### Fixed

- **Field-aware Dart resolution**: root `pubspec.yaml` files now contribute
  only direct keys from `dependencies` and `dev_dependencies`, while nested
  `path`, `git`, `url`, `ref`, and `sdk` source metadata remains inert.
- **Closed Dart aliases**: directory snake-case, the legacy
  `coding_adventures_` prefix, and the exact unquoted root `name` value resolve
  only to already discovered Dart packages; ambiguous, unknown, duplicate, and
  self references cannot create edges.
- **Safe Dart filter and toolchain**: `--language dart` now preserves package
  and program identities, skips generated `.dart_tool` trees, and maps to
  Dart-aware CI workflow markers.

## [0.3.5] - 2026-08-12

### Fixed

- **Field-aware .NET resolution**: C#, F#, and shared dotnet programs now
  resolve only literal `ProjectReference Include` paths from root project
  files against already discovered project identities, without reading or
  following referenced targets.
- **Closed XML and MSBuild grammar**: comments, CDATA, processing instructions,
  namespaces, entity-escaped attributes, properties, globs, absolute paths,
  unknown targets, nested test projects, self-references, and duplicates cannot
  create dependency edges.
- **Safe C# and F# filters**: `--language csharp` and `--language fsharp` now
  share the canonical discovery registry, preserve program identities, and map
  to the existing `dotnet` CI toolchain.

## [0.3.4] - 2026-08-11

### Fixed

- **Safe Haskell and JVM filters**: `--language haskell`, `java`, and `kotlin`
  now share the canonical language registry after their Cabal and Gradle
  resolvers became field-aware.
- **Canonical discovery buckets**: language inference now accepts only the
  direct `packages/<language>` and `programs/<language>` buckets instead of
  borrowing a later path component, and skips generated Cabal
  `dist-newstyle` trees.
- **Program identity preservation**: discovered programs retain the
  `<language>/programs/<name>` identity, keeping same-named packages and
  programs distinct and matching the language-neutral discovery corpus.

## [0.3.3] - 2026-08-11

### Fixed

- **Field-aware Cabal resolution**: exactly one root manifest contributes
  dependencies, and only `build-depends` fields are scanned. Directory,
  legacy-prefixed, and declared package names resolve inside the Haskell scope.
- **Lexical Gradle composite resolution**: multiline `includeBuild` calls are
  parsed outside nested comments and unrelated strings, normalized without
  opening targets, and matched only to discovered Java or Kotlin roots.
- **Shared adversarial conformance**: Python now consumes the strengthened
  Haskell, Java, and Kotlin fixtures, including wrong-BUILD affected-closure
  assertions; the Go oracle consumes the same fixture revisions.

## [0.3.2] - 2026-08-10

### Fixed

- **Ecosystem-scoped dependency aliases**: dependency manifests now resolve
  against one alias table per language, preventing a same-spelled package in a
  different ecosystem from redirecting a local dependency edge or selecting
  the wrong BUILD commands.
- **Qualified cross-language BUILD edges**: exact `# build-tool: deps=` package
  identities remain supported without reopening ordinary cross-ecosystem alias
  matching. Duplicate, unknown, and self references are ignored deterministically.
- **Shared adversarial conformance**: Python and Go consume the same 58th
  language-neutral case covering Lua, Perl, Python, Haskell, and a deliberate
  qualified cross-language bridge.

## [0.3.1] - 2026-08-02

### Fixed

- **Deterministic metadata decoding**: Lua rockspec dependency resolution now
  requires UTF-8 and returns `METADATA_INVALID_UTF8` with package and manifest
  identity instead of exposing a host-specific `UnicodeDecodeError` traceback.

## [0.3.0] - 2026-03-29

### Added

- **TypeScript dependency resolution**: `_parse_typescript_deps()` parses `package.json` for `@coding-adventures/` scoped dependencies. `_build_known_names()` now maps TypeScript packages to their npm scoped names.
- **Rust dependency resolution**: `_parse_rust_deps()` parses `Cargo.toml` `[dependencies]` sections with `path =` references. `_build_known_names()` maps Rust packages to their crate names.
- **Swift dependency resolution**: `_parse_swift_deps()` parses `Package.swift` `.package(path: "../dep-name")` references. `_build_known_names()` maps Swift packages to their directory names.
- **Library-over-program priority** in `_build_known_names()`: when a library package and a program share the same ecosystem dependency name, the library entry takes priority. Prevents self-loop dep resolution for programs that depend on their own library.
- **Elixir enhancement**: `_build_known_names()` now reads the actual `app:` atom from `mix.exs` in addition to the convention-based name, ensuring accurate cross-package resolution.
- **`build_content` field** on `Package` dataclass: raw BUILD file text, populated during discovery, for Starlark detection in CLI.
- **Starlark evaluation step** in CLI (`cli.py`): after discovery, Starlark BUILD files are evaluated via `starlark_evaluator.py` to extract declared targets, sources, and build commands.
- **Expanded `--language` choices**: now includes `typescript`, `rust`, `elixir`, `lua`, `perl`, `swift` in addition to `python`, `ruby`, `go`.
- **`--detect-languages` standalone mode**: outputs `needs_<lang>=true|false` for all languages when used without `--emit-plan`. Writes to both stdout and `$GITHUB_OUTPUT`.
- **`ALL_LANGUAGES` constant**: canonical ordered list of all supported languages.
- **`SHARED_PREFIXES` constant**: narrows shared-file detection from any `.github/` path to only `.github/workflows/ci.yml`, avoiding full rebuilds for deployment-only workflow changes.
- **`_expand_affected_set_with_prereqs()`**: ensures transitive prerequisites of affected packages are also scheduled. Prevents failures on fresh CI runners where prerequisite BUILD steps materialize local dependency state.
- **`DirectedGraph.affected_nodes()`**: returns changed packages plus all their transitive dependents.
- **`DirectedGraph.edges()`**: returns all directed edges as (from, to) tuples for plan serialization.

### Fixed

- **Language detection output**: `_output_language_flags()` now uses `needs_<lang>` prefix (matching Go build tool) instead of `need_<lang>`. Also writes to `$GITHUB_OUTPUT` for GitHub Actions integration.
- **Shared-file detection**: narrowed from `startswith(".github/")` to exact match against `SHARED_PREFIXES` to avoid spurious full rebuilds on deployment workflow changes.

## [0.2.0] - 2026-03-22

### Added

- **Glob matching module** (`glob_match.py`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns. Used for strict input filtering and source file resolution.
- **Strict input filtering in git diff**: `map_files_to_packages()` now accepts optional `packages` parameter. For Starlark packages with `declared_srcs`, only files matching declared source patterns (or BUILD files) trigger rebuilds. Non-source file changes (README, CHANGELOG) no longer cause spurious rebuilds.
- **Build plan module** (`plan.py`): Serializes/deserializes build plan as versioned JSON (`schema_version: 1`). Supports `write_plan()` and `read_plan()` with version checking and forward compatibility.
- **`--emit-plan` CLI flag**: Writes build plan JSON to a file and exits.
- **`--plan-file` CLI flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff.
- **`is_starlark`, `declared_srcs`, `declared_deps` fields** on `Package` dataclass for Starlark BUILD file support.

### Fixed

- **Windows path normalization**: `map_files_to_packages()` normalizes backslash paths to forward slashes for consistent prefix matching against git diff output.

## [0.1.0] - 2026-03-18

### Added
- Initial implementation of the monorepo build tool
- Package discovery via recursive DIRS/BUILD file walking
- Platform-specific BUILD file support (BUILD_mac, BUILD_linux)
- Dependency resolution from pyproject.toml (Python), .gemspec (Ruby), go.mod (Go)
- SHA256-based file hashing for change detection
- JSON-based build cache with atomic writes
- Parallel execution via ThreadPoolExecutor, respecting dependency order
- Dependency-skip propagation: if a package fails, dependents are skipped
- Build report with status summary table
- CLI with --root, --force, --dry-run, --jobs, --language, --cache-file options
- Auto-detection of repository root via .git directory
- Test fixtures for simple (single package) and diamond (A->B->D, A->C->D) topologies
