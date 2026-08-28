# Changelog

All notable changes to the Go build tool will be documented in this file.

## [Unreleased]

### Added

- **OCaml now has a process-free build substrate.** Discovery retains package
  and program identities, field-aware opam and Dune parsing resolves local
  dependency edges without execution, source hashing excludes generated
  `_build` trees, shards account for the pinned toolchain, standalone BUILD
  validation requires explicit prerequisites, and CI emits `needs_ocaml`.
  Discovery also rejects Dune `_build` package decoys. A language-neutral
  fixture pins the resolver boundary. This does not execute builds or claim a
  native OCaml build-tool implementation.

- **Platform exceptions now produce machine-readable `UNSUPPORTED` results
  instead of false `BUILT` successes.** A selected BUILD front may consist of
  exactly `echo BUILD_TOOL_UNSUPPORTED:CODE -- skipped`, where `CODE` is a
  bounded uppercase diagnostic. The executor recognizes the entire record
  without invoking a shell, omits it from the success cache, and reports the
  stable reason. Dependents stop as `DEP-UNSUPPORTED` with
  `DEPENDENCY_UNSUPPORTED`; dry runs preserve the same classification. Mixed,
  malformed, lowercase, or shell-chained lookalikes remain ordinary commands
  and cannot acquire exception authority.
- **Deleting a platform BUILD override now schedules its fallback on that
  platform.** Platform-plan detection recognizes that an absent
  `BUILD_windows`, `BUILD_mac`, `BUILD_linux`, or `BUILD_mac_and_linux` changed
  the selected front, while retaining platform isolation for unrelated lanes.

- **A package can now declare an extra CI toolchain it needs beyond the
  one its own path-bucket language infers.** `inferLanguage` buckets a
  package's `Language` purely by the directory segment right after
  `packages/`/`programs/`, so a Rust crate whose own tests shell out to
  a `javac`/`java` process (e.g. `java-to-semantic-ir`, JV02's planned
  first consumer) had no way to flip CI's `needs_java` flag — it would
  either silently skip its own execution-proof tests or run them against
  whatever JDK version the runner image happens to ship, not the pinned
  version `actions/setup-java` guarantees. A package now opts in with a
  bare BUILD-file comment line, recognized by
  `discovery.parseExtraToolchains` and consulted by
  `computeLanguagesNeeded` alongside (not instead of) the inferred
  language:

  ```
  # needs-toolchain: java
  ```

  Multiple lines are supported for a package needing more than one extra
  toolchain. The directive is inert to the shell that executes
  BUILD/BUILD_windows (`#`-prefixed lines are already dropped before
  execution). New `Package.ExtraToolchains []string` field.

### Fixed

- **`clippyStepFor` scanned only the first BUILD command, silently disabling
  the clippy gate for every Rust package whose BUILD opens with a preamble.**
  `#!/bin/sh` and `#` comments are stripped by the BUILD reader, but `set -e`,
  `export VAR=...`, `cd ...` and `echo "[pkg] Building..."` are not — so a BUILD
  like sql-codegen's (`set -e` / `export CARGO_TARGET_...` / `cargo test ...`)
  presented `set -e` as command zero, matched no `cargo`, and got no clippy
  step on any platform. 22 packages were uncovered this way, including
  `coding-adventures-sql-planner`, which had a live `manual_is_multiple_of`
  error sitting on `main`. The derivation now scans every command, and prefers
  an unconditional cargo invocation over a platform-guarded one.
- **`strings.Contains(cmd, "cargo ")` counted a mention as an invocation.**
  Compile-only crates print the command they cannot run
  (`echo "  To build: cargo build -p font-parser-node --release"`); scanning
  every line would have attached a clippy step to packages whose BUILD
  deliberately builds nothing. Detection now erases single- and double-quoted
  spans before looking for a bare `cargo` word, so `RUSTDOCFLAGS="-D warnings"
  cargo doc` and `cd "$WORKSPACE" && cargo test` still count while an `echo`
  does not.

### Added

- **Tracked `node_modules` paths now fail BUILD validation.** The gate reads
  Git's NUL-delimited index rather than the working tree, so it catches regular
  files and symlinks—including an absolute, machine-local symlink—even though
  the repository's ignore rules correctly exclude newly generated dependency
  directories. This keeps clean worktrees and CI from inheriting private
  filesystem paths.

- **Orphan-crate gate: every directory with a `Cargo.toml` must have a `BUILD`
  file or a reasoned exemption.** The build tool discovers work by scanning for
  BUILD files, which means it is structurally blind to a crate that has none:
  such a crate is never built, its test targets are never compiled, its
  assertions never run, and `cargo clippy --all-targets -- -D warnings` is never
  applied to it, on any platform. In a Cargo workspace the omission is silent —
  every sibling that lists the crate as a path dependency keeps it *compiling*
  while nothing tests or lints it. That is how 84 crates under
  `code/packages/rust` (plus 5 under `code/programs/rust`) accumulated
  unnoticed, two of them carrying live clippy errors.

  Note this could not be folded into `ValidateBuildFiles`: that function
  inspects `[]discovery.Package`, and a package only exists in that slice
  because it *had* a BUILD file. The new `ValidateNoOrphanCrates` scans the
  filesystem instead, so it is the only check able to see the gap. Both now run
  before either can fail the process, so one CI round-trip shows the full punch
  list rather than one problem at a time.

  Exemptions live in `code/BUILD-EXEMPTIONS`, in the form
  `<EXCLUDED|PENDING> <path>  # <reason>`. `EXCLUDED` means the crate genuinely
  should never be built (a compile-only JNI bridge, a wasm-only cdylib);
  `PENDING` is a tracked backlog entry. Keeping them distinct makes the debt
  countable instead of filed away — the tool prints the remaining `PENDING`
  count on every successful validation. A reason is mandatory, since an
  exemption without one is indistinguishable from the oversight this exists to
  prevent.

  **Stale entries fail too.** If a listed crate gains a BUILD file, or its
  directory disappears, validation fails until the line is deleted. So landing a
  BUILD for a `PENDING` crate forces the same change to remove its exemption,
  and the ledger cannot outlive the problem it describes. Verified with a
  control, per the "a check that cannot fail proves nothing" rule: with the
  ledger removed, the real binary reports all 75 remaining orphans; with it in
  place, validation is clean.

- **Windows-only Rust crates are now linted, by a new CI step rather than by
  this tool.** `paint-vm-direct2d` and `paint-vm-gdi` `compile_error!` off
  Windows, so their default `BUILD` is a bare `echo SKIP` and the Linux/macOS
  legs correctly emit no clippy step; their `BUILD_windows` never ran because
  CI skips the build step on the Windows leg. The result was two crates linted
  by nobody, one of which had accumulated 8 `-D warnings` errors that broke
  clippy for every crate depending on it. `.github/workflows/ci.yml` gains a
  `Clippy Windows-only Rust crates` step on the Windows leg, with the crate
  list derived by grepping for the `#[cfg(not(target_os = "windows"))]
  compile_error!` declaration itself, so a new Windows-only crate joins the
  gate automatically.

- **Reject BUILD commands that silently run less than they say.** A BUILD file
  is not a shell script: `discovery.readLines` splits it on newlines and the
  executor runs each line as its own shell invocation, so a line-continuation
  character truncates the command instead of continuing it. The failure was
  silent and partial — `lang-aot`'s first BUILD wrapped one `cargo test` over 40
  lines, so CI ran only the bare `cargo test -p lang-aot --lib` head (15 tests
  instead of the ~1200 in the 39 listed targets) and then failed 40 bogus
  commands with `sh: --: invalid option`. Every listed target looked watched;
  none were. `validateNoSilentlyTruncatedCommands` now rejects two shapes:

  - A trailing continuation character, chosen by the shell that will actually
    run the line rather than by the validating host — `cmd /C` for
    `BUILD_windows` (continuation `^`, where `\` is the PATH SEPARATOR) and
    `sh -c` for everything else. Applying the `sh` rule everywhere would have
    rejected three correct Perl `BUILD_windows` lines ending `prove -l -v t\`
    and taken the Windows CI gate down with them.
  - A trailing bare `&`. `sh -c 'false &'` exits 0 — the command is backgrounded
    and its real status discarded, so a failing build is recorded as passing.
    `&&`, `|` and unbalanced quotes are all loud, so `&` was the only other
    quiet shape.

  Diagnostics quote the offending command rather than an index into
  `BuildCommands`, which is the comment-stripped list and not the file line.
  Verified against all 8280 BUILD and BUILD_windows files: zero trip the rules.

### Fixed

- **Platform BUILD dependencies stay platform-scoped in shared CI plans.** The
  detect job now resolves Linux, macOS, and Windows BUILD metadata into
  separate graph/affected states, unions their toolchain requirements, and
  serializes the optional states in plan v1. Shared shard assignment uses the
  union of those states, then runners select their own state for execution, so
  Windows native prerequisites remain assigned and ordered without being
  scheduled on Unix. Changes to a platform-specific BUILD file also affect
  only runners that select that file. Older plans fall back to the top-level
  state.
- **Build-plan schema accepts case-preserving package identities.** The
  language prefix remains lowercase, while the package portion now admits the
  uppercase characters already used by established Swift roots such as
  `swift/Code128`; generated repository plans validate against their schema.
- **Required build-plan collections serialize as arrays/objects, never JSON
  `null`.** Writers normalize empty package, edge, toolchain, command, and
  platform-edge collections while preserving the contractually distinct
  `affected_packages: null` rebuild-all state.
- **Declared dependency schema matches the emitted source references.** Same-
  language shorthand and repository labels are accepted in `declared_deps`;
  resolved identities remain authoritative in `dependency_edges`.

- **A Starlark BUILD that declares no targets is no longer labelled Starlark.**
  On that path `BuildCommands` is still the raw file lines, which the executor
  runs through the shell exactly as for a shell BUILD, so keeping `IsStarlark`
  set exempted them from the validator's shell-shape checks. It now falls back
  the same way the eval-error path already did.

- **Canonical discovery identity registry.** Go discovery now consumes the
  shared language-registry and duplicate-identity fixtures, recognizes every
  canonical package/program bucket including Mosaic, Twig, OCaml, and retained
  `.NET` hosts, preserves the `programs` identity segment, and excludes
  specification fixture trees.
- **Fail-closed duplicate identities.** Two directories that normalize to one
  graph name now return typed `DUPLICATE_PACKAGE_IDENTITY` details with sorted
  repository-relative paths; the CLI prints the stable root-redacted
  diagnostic and exits `2`.
- **C and C++ as first-class languages.** `inferLanguage` now recognizes `c`
  and `cpp` path components (exact-match, so `c` never fires inside `csharp` or
  `cpp`). Existing C++ packages (`cpp/conduit`, `cpp/conduit-hello`,
  `cpp/mosaic-flux-qt`) are now inferred as language `cpp` instead of `unknown`.
- **`cpp` CI toolchain.** Added to `allToolchains`; both the `c` and `cpp`
  package languages map to the single `cpp` toolchain in
  `toolchainForPackageLanguage` (they share compilers: gcc/g++, clang/clang++,
  cl.exe), mirroring the `csharp`/`fsharp` → `dotnet` collapse. See spec
  `code/specs/CCPP01-c-cpp-iso-multicompiler-lane.md`.
- **`cpp` is now a CI-managed toolchain** (`validateCIFullBuildToolchains`): the
  validator requires `.github/workflows/ci.yml` to bind `needs_cpp` and force it
  on the main full-build path. ci.yml installs Clang alongside GCC on Linux and
  MSVC on Windows so the pure-ISO multi-compiler check sees all three across the
  matrix.

### Fixed

- Discovery now skips Cabal `dist-newstyle` output, preventing generated
  Haskell build trees from becoming packages or disrupting repository scans.
- Lua standalone validation now treats an absent `BUILD_windows` as missing
  the canonical sibling-install closure, matching the shared
  `lua_windows_sibling_parity` fixture plus the Python and Lua validators.
- TypeScript dependency resolution now parses root `package.json` objects,
  registers only the exact top-level `name` alias, and accepts only direct
  keys from `dependencies` and `devDependencies`. Single-line tables resolve
  correctly while peer, optional, nested tool, script, and malformed-JSON
  decoys cannot invent partial graph edges.
- C#, F#, and shared .NET resolution now reads only literal `Include`
  attributes on unqualified `ProjectReference` elements in root project files,
  matches lexically normalized paths to exact project files across the shared
  .NET scope, and ignores XML decoys, dynamic MSBuild paths, nested projects,
  absolute paths, and unknown targets without opening referenced files.
- Java and Kotlin Gradle resolution now reads only comment-aware
  `includeBuild("...")` calls from root `settings.gradle.kts`, supports
  multiline and nested relative paths through exact same-lane package-root
  matching, and ignores strings, build-script coordinates, absolute paths,
  cross-lane targets, and unknown targets without following referenced paths.
  Java/Kotlin source plus Gradle settings/build files now invalidate package
  hashes.
- Dart dependency resolution now accepts only direct package keys under root
  `dependencies:` and `dev_dependencies:` maps, excluding nested source
  options, dependency overrides, comments, and unrelated YAML fields.
- Elixir dependency resolution now reads local `path:` tuples from both direct
  project `deps:` lists and block or shorthand `defp deps` lists, including
  multiline tuples, while excluding comments, application metadata, prose,
  `mix.lock`, and non-path dependencies.
- Perl dependency resolution now reads only top-level runtime `requires`
  declarations from root `cpanfile`s, excludes test and other phase blocks and
  `Makefile.PL` dependency tables, and registers exact declared module names
  plus current and legacy distribution aliases. The aes-modes BUILD recipe now
  declares the newly authoritative local AES prerequisite. Standalone BUILD
  validation separately recognizes test-block source references and their
  runtime closure without promoting test-only dependencies into the graph.
- Ruby dependency resolution now reads only runtime dependency calls on the
  gem specification receiver, treats `add_dependency` and
  `add_runtime_dependency` as synonyms, ignores development dependencies and
  commented-out calls, and registers declared gem names alongside derived
  directory aliases.
- Rust dependency resolution now honors Cargo inline-table `package` renames
  for path dependencies while retaining the top-level `[dependencies]` field
  boundary.
- Lua `.rockspec` metadata is now decoded as strict UTF-8 before dependency
  parsing. Invalid bytes fail closed with `METADATA_INVALID_UTF8`, package and
  repository-relative manifest identity, and CLI exit code 2 without leaking
  checkout paths.
- Haskell dependency resolution now recognizes the plain Cabal names used by
  current packages as well as legacy `coding-adventures-*` names. The resolver
  registers directory and manifest aliases, parses every `build-depends`
  stanza, removes duplicates and self-references, and therefore exposes local
  dependency edges to diff-based affected-package analysis.
- Haskell package discovery now rejects directories with multiple ambiguous
  Cabal manifests instead of selecting one based on enumeration order.

## [0.3.1] - 2026-03-30

### Fixed

- **Windows Lua luarocks serialisation**: `buildResourceKeys` now adds a
  `global:luarocks-windows` lock key for every Lua package command that
  contains `luarocks make` when running on Windows. On Windows, luarocks
  requires exclusive write access to the local rocks tree
  (`~\AppData\Roaming\luarocks`); any two concurrent `luarocks make` calls
  race for that file lock and one fails with "command 'make' requires
  exclusive write access". The global lock key ensures all Lua luarocks
  installs are fully serialised on Windows while leaving Linux/macOS
  unaffected. This mirrors the existing `global:hex-cache` serialisation
  used for Elixir `mix deps.get`.

## [0.3.0] - 2026-03-22

### Added

- **Glob matching library** (`internal/globmatch/`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns. No filesystem access needed — matches patterns against path strings directly.
- **Strict input filtering in git diff**: `MapFilesToPackages()` now respects Starlark `declared_srcs` patterns. For Starlark packages, only files matching declared source patterns (or BUILD files) trigger rebuilds. Editing `README.md` in a Starlark package no longer causes a spurious rebuild.
- **Build plan artifact** (`internal/plan/`): Serializes discovery, resolution, and change detection results as a versioned JSON manifest (`schema_version: 1`). Enables CI detect job to compute the build plan once, upload as artifact, and have build jobs on all 3 platforms skip redundant computation.
- **`--emit-plan` flag**: Writes the build plan JSON to a file and exits. Used by CI detect job.
- **`--plan-file` flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff. Used by CI build jobs.
- **Cross-platform plan loading**: When loading a plan on a different OS than the detect job, re-reads platform-specific BUILD files to get correct commands (e.g., Windows gets `BUILD_windows` commands instead of Linux shell syntax).

### Fixed

- **`**` glob patterns in hasher**: `resolveDeclaredSrcs()` now uses `filepath.WalkDir` + `globmatch.MatchPath` instead of `filepath.Glob`, which silently failed on `**` patterns.

### Changed

- CI workflow now uploads/downloads build plan artifact between detect and build jobs, eliminating duplicate discovery/resolution computation on each platform.

## [0.2.0] - 2026-03-22

### Added

- **`--detect-languages` flag**: Outputs which language toolchains CI needs based on git diff. Enables conditional toolchain installation in CI — only install Python if Python packages changed, etc. Go is always needed (build tool dependency).
- **Starlark BUILD file evaluation**: BUILD files can now be written in Starlark instead of shell. The build tool detects Starlark BUILD files (via `load()` or rule calls) and evaluates them through the Go starlark-interpreter.
- **Starlark evaluator** (`internal/starlark/evaluator.go`): Evaluates Starlark BUILD files, extracts targets with declared srcs/deps, generates shell commands from rule types.
- **Strict input hashing**: When a package has declared srcs (from Starlark BUILD), only those files are hashed for change detection. Falls back to extension-based collection for shell BUILD files.
- **12 rule types supported**: py_library, py_binary, go_library, go_binary, ruby_library, ruby_binary, ts_library, ts_binary, rust_library, rust_binary, elixir_library, elixir_binary.
- **"starlark" language support**: Discovery and hasher recognize "starlark" as a first-class language alongside python/go/ruby/typescript/rust/elixir.
- **TypeScript, Rust, Elixir extension mappings** in hasher (previously only python/ruby/go were mapped).

### Dependencies

- Added starlark-interpreter and its 10 transitive Go package dependencies via replace directives.

## [0.1.0] - 2026-03-18

### Added

- **Package discovery** via DIRS/BUILD file walking. Supports platform-specific BUILD files (BUILD_mac, BUILD_linux) with automatic fallback to generic BUILD.
- **Dependency resolution** for Python (pyproject.toml), Ruby (.gemspec), and Go (go.mod). Internal dependencies are mapped using ecosystem-specific naming conventions (coding-adventures-* for Python, coding_adventures_* for Ruby, module paths for Go).
- **SHA256 content hashing** for incremental builds. Two-level hashing: individual files are hashed, then all hashes are concatenated and hashed again. Language-aware file filtering (only relevant source extensions are included).
- **Dependency hashing** to propagate changes through the dependency tree. If a transitive dependency changes, all dependents are rebuilt.
- **JSON-based build cache** (.build-cache.json) with atomic writes via temporary file + rename. Cache records package hash, dependency hash, timestamp, and build status.
- **Parallel execution** using goroutines with semaphore-based concurrency limiting. Packages are built in topological levels — packages in the same level run in parallel.
- **Failure propagation** — if a package fails, all transitive dependents are marked "dep-skipped".
- **Build report** with aligned columns showing package name, status, and duration. Summary line shows counts by status category.
- **CLI flags**: -root, -force, -dry-run, -jobs, -language, -cache-file.
- **Language filtering** to build only Python, Ruby, Go, or all packages.
- Comprehensive test suite covering all six internal packages.
- Knuth-style literate comments throughout the codebase explaining design decisions.

### Dependencies

- Uses the `directed-graph` package from `code/packages/go/directed-graph` via Go module replace directive.
