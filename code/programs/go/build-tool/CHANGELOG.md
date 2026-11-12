# Changelog

## Unreleased — Kiln legacy graph export

- **`-emit-kiln-graph <path>`.** Writes the `kiln-legacy-graph/1` document
  (KILN03 section 2): every package with per-OS BUILD file names (or `null`),
  per-OS direct deps, `dev_deps` (inverted test triggers), `repo_inputs` and
  `extra_toolchains`. Implies `-force` and `-language all`, honours
  `-validate-build-files`, and is deterministic. Go-only by design; the
  educational ports are frozen (KILN00 section 16).
- **Discovery across OSes.** New `discovery.DiscoverPackagesAllPlatforms`
  so a package whose only file is `BUILD_windows` (python/starlark-compiler)
  is not invisible on a Linux or macOS host. Normal discovery is unchanged.
- **Repository-input consumers declare paths.** Each table row gains a
  `paths` list mirroring its predicate, pinned by a test; the Mosaic guard is
  predicate-only and has none.

## Unreleased — package-local metadata second review

- **An empty diff is "no answer" again.** `parseNameStatus` returned a
  non-nil empty slice for an empty diff. `run()` tests the length and fell
  back to hashing, but `selectPackageRoots` tests for nil, so `-emit-plan`
  wrote "build nothing" (`[]`) into every platform override while the
  top-level set said "build everything" (`null`); `plan.go` applies the
  override, so build jobs would have built nothing. Empty output now gives
  nil, nil, and the overrides are `null` again, as on main.
- **An undecodable diff falls back as a whole.** A truncated record or an
  empty path used to return the paths read so far and drop the rest. It now
  returns nil, nil, so the caller takes the same fallback as when Git fails.
- **The metadata-execution check is described as what it is.**
  `ValidateNoExecutedMetadata` matches literal paths in fixed forms, so a
  shell variable, pipe, wrapper command, `concat!`/`env!` path,
  `cfg_attr(path)` or `build.rs` copy gets past it. The spec and the
  validator now call it a best-effort tripwire, not a boundary; the type
  restriction is what keeps scripts and sources under `evidence/` and
  `CHANGELOG.d/` propagating.

## Unreleased — package-local metadata review fixes

- **Platform BUILD fronts no longer drop the owner.** Local-only packages
  were classified from the unfiltered path list while ordinary roots used the
  platform-filtered one, so a metadata change beside a `BUILD_windows` edit
  selected nothing on Linux and macOS (287 → 0 for `rust/ct-compare`, 556 → 0
  for `rust/bounded-json`). `selectPackageRoots` now returns both sets from
  the same list; the package is local on Linux/macOS and ordinary on Windows.
- **`README.md` is no longer metadata.** A Python dependent's
  `pip install -e ../<dep>` reads the dependency's `readme`; deleting
  `python/html-renderer/README.md` broke `python/programs/pipeline-visualizer`
  while only the renderer was selected. README changes propagate as before.
- **Only additions and edits can be local.** `gitdiff.GetChangedFilesWithStatus`
  reads `git diff --no-renames --name-status -z`; a deletion, rename (D + A),
  type change or unknown status propagates. `ClassifyChangedFiles` and
  `emitBuildPlanAgainst` take the statuses; a nil map keeps every metadata
  change ordinary.
- **Closed by type.** `CHANGELOG.d/` admits only direct `*.md` shards and
  `evidence/` only `.json`, `.cnf` and `.drat` (the extensions tracked
  today). `-validate-build-files` adds `ValidateNoExecutedMetadata`, which
  flags a tracked BUILD front, script, Cargo `path`/`build`, Rust
  `#[path]` or `include!` that visibly executes a metadata-class location
  (a best-effort tripwire; see above).
- `TestRepositoryInputConsumersPlan` is back to its original README
  expectation (the Mosaic package and the app that depends on it).

## Unreleased — package-local metadata no longer rebuilds dependents

- A package whose only changes are package-local metadata — its root
  `README.md`, `CHANGELOG.md`, `CHANGELOG.d/` entries,
  `required_capabilities.json`, `effect_certificate.json`,
  `assurance_report.json` or `evidence/` files — is now selected with its
  prerequisites but without its dependents or Cargo dev-dependency test
  triggers, the way a repository input consumer is. The package itself still
  rebuilds, because its BUILD may check the file. Any other change in the
  package (or a strict-glob declared source, a fixture seed, a forced or
  boundary-consumer root) propagates as before; `-force` and the hash
  fallback are unchanged. A capability-manifest-only change to
  `rust/bounded-json` now selects 1 package instead of 556.
- New `internal/pkgmeta` holds the closed, root-only, case-sensitive list
  and the repository evidence for each entry. `gitdiff.ClassifyChangedFiles`
  splits changed packages into propagating and local; `MapFilesToPackages`
  is now their union. `package_metadata.go` wires the local set into the
  detect plan and every platform override.
- `internal/graphdiff` replays the new `metadata_policy: "package_local"`
  diff-selection variant (`package-metadata-selection-*.json`), and changes
  to those cases seed the Dart, Go, Java, Kotlin and OCaml build-tool
  readers.
- `TestRepositoryInputConsumersPlan`'s README control now expects only the
  Mosaic package itself, not the app that depends on it.

## Unreleased - BUILD-001 registry update

- Source-input registry and repository boundary updated for BUILD-001 (generated wasm artifacts are built, not committed): `code/packages/rust/engram-wasm/pkg/engram_engine.wasm` is no longer an exact package input of `rust-engram-wasm-build-inputs`, and `visicalc-html/vendor/spreadsheet-engine-wasm.js` is no longer a cross-package input of the Deno VisiCalc boundary. Registry digest is now `8f3d9ee3...`, boundary digest `ec516870... (combined with the #19050 TypeScript boundary sync)`.

## Unreleased — CT01 corpus lanes as repository-input consumers

- `code/specs/fixtures/ct-compare-v1/cases.json` now also selects the Dart,
  Haskell, Lua, OCaml, Perl and Swift `ct-compare` packages, whose tests read
  the corpus outside their own directories (OCaml compares a vendored copy with
  it). Only the exact corpus file selects them: the TFV projection and Python
  scripts remain Rust-adapter inputs. Missing consumers fail closed and language
  filters are retained.

## Unreleased — Native Perl external build inputs (#18844)

- Helper-only and shared `perl-bridge` manifest/build/source edits select
  Conduit, IRC Server Native and Conduit Hello as runtime test consumers.
  Missing consumers fail closed, language filters are retained, and docs or
  lookalike paths do not trigger the mapping. The regression test fails without
  registration because a helper-only edit previously selected no packages.
- Both platform fronts declare Rust for all three native recipes. A test reads
  the real BUILD files and verifies helper-only changes request both Perl and
  Rust for Linux, macOS and Windows, so cold CI does not rely on ambient Cargo.
- The Windows native-suite gate includes these consumers and the helper path;
  the same real-metadata test checks that helper-only plans execute Windows CI.

## 2026-11-02 — `windows-posix-syntax`: BUILD_windows lines must be cmd syntax

- **New `-validate-build-files` rule, `windows-posix-syntax`**
  (`internal/validator/windows_posix_syntax.go`). Every `BUILD_windows` line
  runs as its own `cmd /C`, so the rule rejects POSIX-only constructs outside
  double quotes: `/dev/null`, `command -v`, `$(...)`, `${...}`, `[ ... ]` /
  `[[ ... ]]` tests, and `then`/`fi`/`do`/`done`/`esac` after a `;`. Each
  diagnostic names the file, line, construct and the cmd alternative.
- **Why.** `where swift >/dev/null 2>/dev/null && swift test || echo skipping`
  never fails under cmd: the redirect cannot open `/dev/null`, so the line
  always takes the echo. 49 Swift `BUILD_windows` files skipped their tests on
  Windows that way. Three Perl files and one Python file used `command -v`,
  `$(...)` and `if [ ... ]; then`, which cmd rejects outright, but only when a
  PR happened to rebuild them there. On main the rule's first run found 56
  such lines.
- **False positives.** Double-quoted text belongs to the program it is passed
  to, so `python -c "...; ..."`, `perl -e "... ${x} ..."` and a
  `bash -c "..."` hand-off payload are never read. cmd's own
  `for %i in (...) do ...` has no `;` before `do`, and `echo done` is not a
  keyword after `;`. `#` and `rem` lines are skipped, and so are Starlark
  `BUILD_windows` files (six Go packages, where `[` is a list). Single quotes
  are not quotes to cmd, so single-quoted text is checked.
- Lesson: `lessons.d/build-windows-lines-run-under-cmd-c-so-a-posix-redirect-like.md`.

## 2026-11-02 — Cargo link-table edges: security-review fixes

- **Workspace inheritance is refused in link tables (review round 2).**
  `a = { workspace = true }` in `[dependencies]`, `[build-dependencies]` or a
  target form, or a `workspace` key in a `[D.a]` sub-table (quoted or not),
  passed the link-table screen yet drew no edge: Cargo takes the path from the
  workspace root's `[workspace.dependencies]`, which the edge reader never
  opens. A later change to that dependency would then re-run none of this
  crate's tests. The strict reader now refuses it, as it already did in
  dev-dependency tables. No repository manifest uses it.
- **Manifest-derived keys are quoted** (`%q`) in the new link-screen
  messages, so a quoted key cannot carry control bytes into a CI log.
- **`readCargoManifestBytes` checks the open handle.** It `Stat`s the opened
  file and requires `os.SameFile` with the `Lstat` it size-checked, so a
  symlink swapped in between is refused instead of followed (portable, unlike
  `O_NOFOLLOW`, which Go does not spell for Windows).
- **Linear continuation join.** An inline Cargo entry continuing onto later
  lines was joined by appending each line to the whole value and rescanning
  the value's brackets per line — quadratic; a crafted 400 KB manifest took
  64 s. `cargoLinkDependencyNames` now carries a `tomlBracketScanner`
  (bracket depth, open-string delimiter, pending escape) from line to line,
  collects the pieces and joins them once. The result is identical to the
  rescan, including a string left open at the end of a line, which continues
  across the joining space.
- **Size guards on the edge reader.** `parseRustDeps` runs before the strict
  test-trigger reader, so it now carries its own limits: it reads the
  manifest through `readCargoManifestBytes` (a regular file of at most
  1 MiB) and refuses one entry continuing past 256 non-blank lines or 64 KiB.
  Both refusals are `*CargoManifestError` and fail the whole resolution;
  `parseRustDeps` and `cargoLinkDependencyNames` now return an error.
- **Quoted field keys.** `a = { "path" = "../a" }`, `'package' = "..."` and
  `"path" = "../a"` in a `[dependencies.<name>]` sub-table are the keys TOML
  and Cargo read, and are now edges (they were silently dropped). Keys stay
  case-sensitive (`"Path"` is not `path`) and escapes are not decoded, per the
  amended conformance contract.
- **Fail closed on unmodelled link-table shapes.** For every Rust and WASM
  package, the strict reader (`readCargoTriggerEntries`, `screenLinks`) now
  also screens `[dependencies]`, `[build-dependencies]`,
  `[build_dependencies]` and their `[target.<cfg>.…]` forms, refusing any
  write the lenient edge reader would drop or misread: dotted keys inside
  them (`a.path = ...`), at the root (`build-dependencies.a.path = ...`) or
  under `[target]`/`[target.<cfg>]`; inline tables spelling one
  (`build-dependencies = { ... }` under `[target.<cfg>]`); deeper headers and
  arrays of tables (`[dependencies.a.features]`, `[[dependencies]]`); entry
  values that are neither a string nor an inline table; a multi-line value in
  a sub-table; dotted or non-`key = value` inline fields; and any backslash.
  Every shape the contract models is still accepted. The forced plan of the
  real repository is unchanged (exit 0, identical edge list).
- **`DEPENDENCY_SELF_EDGE` instead of a panic.** A dependency that resolves
  to the package itself (a Cargo path entry naming its own directory alias or
  `[package]` name, for example) made `directed-graph.AddEdge` panic. It is
  now a `*resolver.DependencySelfEdgeError` with a repository-relative
  manifest, reported by the CLI as a stable diagnostic with exit status 2.
- Tests: 11 new rows in the one-shape Cargo table (57 in all), linear-time
  and entry-cap tests, manifest-guard tests, self-edge tests, and
  `test_triggers_link_screen_test.go` (24 refused shapes, the multi-line
  string backstop, 5 accepted groups, and the Rust/WASM-only scope).

## 2026-11-02 — Haskell Darwin BUILDs must select the CLT linker

- New `validateHaskellDarwinLinker` (`internal/validator/haskell_darwin_linker.go`),
  run by `--validate-build-files` alongside the other BUILD rules. Every line of
  the BUILD file a `haskell` package resolves on macOS (`BUILD_mac`, then
  `BUILD_mac_and_linux`, then `BUILD`) must select
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` behind a
  `[ "$(uname -s)" = Darwin ]` check, either as an export prefix
  (`if ...; then export DEVELOPER_DIR=...; fi; <command>`) or as the inline
  assignment on the Darwin branch's Cabal command. GHC 9.4.8's macOS bindist
  hardcodes a CLT SDK sysroot; on runners with CLT 27 beside an older default
  Xcode, an unselected Cabal process links with the Xcode linker, which rejects
  the SDK's `arm64e.x1` stubs. Cabal builds the whole local project in the
  selected root's process, so every root needs the selection, and every line
  needs it because each line is its own `sh -c`. The unsupported-protocol
  record and bare `echo` lines are exempt; comments never satisfy the rule.
  The error names the file and
  `lessons.d/ghc-must-select-the-linker-matching-its-hardcoded-macos-sdk.md`.

## 2026-11-01 — Cargo build and target-specific dependencies are graph edges

- `parseRustDeps` (`internal/resolver/resolver.go`) now reads path
  dependencies from `[dependencies]`, `[build-dependencies]`,
  `[build_dependencies]`, and the same three tables under
  `[target.<cfg>.…]` for any cfg (quoted or a bare triple), as a union
  over all targets. Before, only the top-level `[dependencies]` table was
  read, so a change to a build or target-specific dependency rebuilt and
  re-tested none of its dependents. On the real repository the forced plan
  gains 22 Rust edges and loses none, and the graph stays acyclic: among
  them paint-metal and paint-vm-direct2d into barcode-1d/-2d, the sandbox
  crates into the chief-of-staff launcher, host and e2e crates, sha256 into
  chief-of-staff-spawn-isolation, the board-vm UART/serial crates into
  board-vm-uno-r4-firmware, paint-vm-direct2d and paint-vm-gdi into
  paint-vm-direct2d-c, and objc-bridge into markdown-reader and
  venture-browser-macos.
- The `[dependencies.<name>]` / `[target.<cfg>.dependencies.<name>]`
  sub-table form with `path = "..."` (and optional `package = "..."`) is
  read; it was silently dropped before.
- Headers are split into dotted-key segments outside strings and each
  segment is compared by value, so `[target.'cfg(any(target_os = "linux",
  windows))'.dependencies]` and `["dependencies"]` are read. Every header,
  including malformed ones and `[[bin]]`, ends the current table.
- Inline entries: comments are stripped outside strings first (a trailing
  comment no longer hides the `path` field), fields are split at commas
  outside strings and outside nested `[...]`/`{...}`, the `path` and
  `package` keys must be exact and lowercase, and a value whose `features`
  array continues onto later lines is joined until its brackets close
  (TOML 1.0; `vault-pm-cli-drill` uses it).
- `[dev-dependencies]` in every form, `[patch]`, `[replace]`, `[workspace]`,
  registry-only entries, features, and comments still contribute no edge.
  Dev-dependencies stay in the separate test-trigger relation; the
  `test_triggers.go` comments now say the graph records build and target
  edges and explain why the trigger walk still follows package roots
  (nested and orphan crates, native-extension manifests, path vs. name).
- The Go tool now reads the shared `resolution/rust-field-aware` case
  (`TestRustResolutionConformanceFixture`), and the resolution-fixture CI
  routing in `main.go` and its selection test sends that case to Go as well
  as Haskell. New table-driven `TestCargoLinkDependencyNames` pins each line
  rule of the contract.
- The Lua, Perl, Python, Ruby, Rust, Swift and TypeScript fronts now implement
  the same contract and read the same case, so the routing (and
  `ci-gate-registry.md`) sends `rust-field-aware` to G, H, L, P, Y, B, R, S
  and T.

## 2026-10-26 — Mosaic manifest dependencies and repository-wide Mosaic guards

- The resolver reads `mosaic-package.toml` for language-`mosaic` packages
  (`code/packages/mosaic/*`, `code/programs/mosaic/*`). Keys of the root
  `[dependencies]` table
  resolve within the Mosaic scope by directory basename or `[package].name`
  (each name owned by exactly one package). Before this, Mosaic packages had no edges,
  so bumping or editing `mosaic-pkg-toolkit` never re-ran task-app, engram-app,
  journal-app or the packages built on it. Requirement strings, comments,
  `[components]` exports, `[host_assets].dependencies` third-party
  coordinates, external names and self-references never create edges;
  multi-line values are tracked by bracket depth (`internal/resolver/mosaic.go`).
- The Mosaic reader fails closed (`MosaicManifestError`), because a dropped
  edge silently skips a dependent's tests on every later PR. It reads only a
  plain subset: one-part headers from a fixed list (`[package]`,
  `[dependencies]`, `[components]`, `[kernel]`, `[styles]`, `[app]`,
  `[host_assets]`, `[host_effects]` — every table the repository uses), and
  bare keys with one-line string values in `[package]` and `[dependencies]`.
  Refused with an error: any other header (`[package.meta]`, dotted, quoted,
  `[[...]]`), any root-level key or inline table (`package.name = ...`), a multi-line string, any backslash
  escape (keys, headers and `[package].name` alike), a manifest that is not a
  regular file or exceeds 1 MiB, and two Mosaic packages claiming one name —
  including a library taking an app's name (`checkMosaicNameCollisions`) — all
  stop `ResolveDependencies` with an error. None occurs in the repository. A
  leading UTF-8 byte-order mark is stripped, as `toml_edit` does. A
  dependency key without the `mosaic-pkg-` prefix also depends on
  `mosaic-pkg-<key>`, as the resolver's directory locator tries that first.
- New `repository_inputs.go`: a table of *repository input consumers*,
  packages whose tests walk the whole tree. Its first entry selects
  `rust/mosaic-package-resolver` (`every_repository_requirement_is_satisfied`
  and the layout-binding guard) for any changed `mosaic-package.toml`,
  `.mil`, `.mll` or `.msl` under `code/`. Both the dry-run/build path and
  every per-platform plan add the consumer and its prerequisites but not its
  dependents; seeding it as a changed package instead measured 72 affected
  packages for a one-line `.mll` edit. Raw Git paths, no stat (deletions
  select), `-language` honored, and a missing consumer fails the plan.
- Measured on the repository with `-dry-run -diff-base origin/main`: a
  toolkit version bump went from 1 to 35 affected packages, an app `.mll`
  edit from 1 to 19, both now including the guard and neither including the
  guard's dependents (artifact builder, emitters); a Python/Go-only change is
  identical (168 → 168). Mosaic-only changes now also request the Rust
  toolchain in the plan, because the guard is a Rust crate.

## 2026-10-24 — BUILD_windows PowerShell and batch lines are rejected

- `validateWindowsBuildLines` rejects `BUILD_windows` lines that only work in
  PowerShell (`$env:X = ...`, `$ErrorActionPreference`, `Set-Location`,
  `Push-Location`/`Pop-Location`, `Write-Host`/`Write-Output`,
  `if (-not ...)`/`if ($...)`) or inside a batch file (`%~dp0`). Every
  non-comment line runs as its own `cmd /C` from the package directory, so
  such a line fails, but only when Windows CI happens to rebuild that package.
  The check reads `BUILD_windows` directly, so Linux and macOS runs catch it
  too. It sits beside `validateWindowsEnvAssignments` (#16437), which covers
  POSIX `VAR=value` assignments. Each line is checked command by command
  (split at the `&&`, `||`, `&` and `|` outside double quotes). A command
  that hands off to another shell (`powershell`, `pwsh`, `bash`, `sh`) is
  exempt, so the site BUILD files' `&& powershell -Command "..."` stays
  valid, but every other command is checked, including one after such a
  hand-off. It flags capitalised Verb-Noun cmdlets (`Remove-Item`), PowerShell
  variables braced or not (`$env:`, `${env:}`, `$LASTEXITCODE`, `$null`,
  `$global:`, `$script:`, `$PSScriptRoot`, `$true`/`$false`), a PowerShell
  `if (` or cmdlet after a `;` outside quotes (cmd has no `;` separator; a
  `$` or `exit` there is left alone because POSIX shell lines use them too),
  an unclosed `"`, a leading `[`, a `.ps1` run directly, and the batch
  self-references `%0`/`%~f0`/`%~dpnx0`. A leading UTF-8 byte-order
  mark is stripped first, and a symlinked, non-regular, oversized (over
  1 MiB) or unreadable BUILD_windows is reported instead of passing unseen.
  Harmless leftovers (`rem`, `@echo off`, `if errorlevel`) stay allowed.

## 2026-10-25 — G2D02 .NET fixture selection

- Route the exact Bezier flattening fixture to both C# and F# native consumers
  without forcing unrelated packages or toolchains.
- Preserve language filtering, source unions, deletion/rename-old paths, and
  atomic plan publication when a declared reader is missing.

## 2026-10-24 — TE04 extension fixture selection

- Route the exact portable GFM extension corpus to the TypeScript parser and
  its dependent closure on every platform without forcing a full build.
- Retain the older fenced-only route while rejecting case, separator, backup,
  nested, schema, and documentation lookalikes for both corpus paths.
- Extend missing-reader, language-filter, source-union, toolchain, dependency,
  and atomic-plan regressions to the portable extension corpus.

## 2026-10-22 — TE04 fenced fixture selection

- Route the exact GFM fenced-block fixture to the TypeScript parser and its
  dependent closure without forcing a full build, across all platform plans.
- Preserve source unions, language filters, and deleted/renamed paths; reject
  missing or wrong-language readers before creating or replacing plan output.
- Add native selector, toolchain, dependency-order, and atomic-plan regressions.

## 2026-10-21 — BF03 Go native fixture selection

- Seed go/compiler-ir from the exact compiler-ir-v1/cases.json external path
  before fixture early return for all/Go filters; preserve downstream closure,
  deleted/renamed old paths, source union and three-platform build plans.
- Fail closed on a missing native reader; ignore lookalike/schema/docs paths
  and unrelated languages. Add exact selector and atomic-plan regressions.


All notable changes to the Go build tool will be documented in this file.

## [Unreleased]

### Fixed

- Cargo `[dev-dependencies]` now re-run the dependent's tests
  (`internal/resolver/test_triggers.go`). Every Mosaic package and app is
  tested by a Cargo smoke harness whose only in-repository dependencies are
  dev-dependencies on the compilers and the artifact builder, so a change to
  a `mosaic-emit-*` backend used to re-run none of those suites.
  `mosaic-pkg-notes` and `engram-app` were not selected by a change to
  `mosaic-emit-qt/src/pipeline.rs`; now they are.
  - `ResolveWithTestTriggers` returns the build graph plus `TestTriggers`
    (dependency → packages that dev-depend on it). `ResolveDependencies`
    wraps it. Read for `rust` and `wasm` from `[dev-dependencies]` and every
    `[target.<cfg>.dev-dependencies]` (union over targets); for a `mosaic`
    harness also from `[dependencies]` and `[build-dependencies]` and their
    target forms, since a Mosaic package has no Cargo graph edges.
  - Path entries resolve by real directory (symlinks evaluated), not by
    Cargo name or text: the discovered package at that path is the trigger.
    A crate that is not a package root (an orphan, or one nested in another
    package or in the dependent) has its own `[dependencies]` and
    `[build-dependencies]` followed recursively (depth cap 16), and a
    containing package is a trigger too. Package roots are followed the same
    way, because the build graph records only a Rust or WASM package's
    top-level `[dependencies]`, not its target or build dependencies or a
    native extension's `Cargo.toml`. Each crate's closure is cached, and a
    cycle among these tables is refused, as Cargo refuses it. An uncovered crate must be
    listed exactly in `code/BUILD-EXEMPTIONS`. This is how `engram-app`
    reaches `engram-core` through `engram-core-wasm`. Anything else
    unresolvable (a case or Unicode-normalization alias, a crate under
    `vendor/` or outside `code/`, a missing, absolute or empty path) is an
    error. `-language` runs
    resolve against every discovered package (`ResolveWithTestTriggersAgainst`).
  - `[patch.<source>]` tables are triggers. `[replace]` is refused, as is a
    `git` source that is not `https://` or `ssh://`, and a
    `.cargo/config(.toml)` from the package up to the repository root that
    sets `paths`, `[patch]` or `[source]`.
  - The triggers stay out of the graph and are applied one hop past the build
    closure, before prerequisite expansion (`withDevDependents`). A
    dev-dependent's own dependents are not selected, and prerequisites
    trigger nothing. The repository has real dev-dependency cycles
    (`closure-pass-dce` ⇄ `closure-pass-fold-control-flow`,
    `wolfram-to-semantic-ir` ⇄ `semantic-ir-to-javascript`); as graph edges
    they would have failed every package's build.
  - Fails closed (`CargoManifestError`) after a security review: any
    multi-line string; an open string, mismatched brackets or a value open at
    end of file; an undecodable header or key (quoted segments are decoded, so
    `["dev-dependencies"]` is the table; backslashes are refused); a trigger
    table as a sub-table, an array of tables, or spelled by a key or inline
    table elsewhere; entries other than `key = "version"` or a one-line inline
    table whose fields are bare, exactly-lowercase, unique keys with string,
    boolean or string-array values (only an array may span lines; nested
    tables, trailing commas, escapes and `workspace = true` are refused); a
    non-regular or over-1 MiB manifest, or one entry continuing past 256
    lines or 64 KiB (continuation lines are joined once, so a 1 MiB manifest
    no longer takes half a minute). A leading BOM is stripped.
  - Fan-out, accepted: a one-line change to `mosaic-emit-qt`,
    `mosaic-emit-flutter` or `mosaic-package-artifact-builder` now selects 158
    packages instead of 73 (`moslayout-compiler`: 160 instead of 74). That is
    the 27–28 Mosaic suites, `rust/venture-browser-qt`, `-macos` and
    `-windows`, and their prerequisites (about 51 for `-qt` alone). 151 was
    accepted before target tables were read; they add the last 7. A change
    to `engram-core` now selects 27 packages instead of 14.
- `toolchainForPackageLanguage("mosaic")` now returns `rust`. Every Mosaic
  package and app BUILD runs `cargo test` on its smoke harness, but the
  language mapped to the unknown toolchain `mosaic`, so a plan whose only
  affected packages were Mosaic ones did not ask CI for Rust (`needs_rust`).
  It already worked whenever `rust/mosaic-package-resolver` was selected
  alongside them, which hid the gap for most `.mil`/`.mll`/`.msl` edits.
- The `global:luarocks-tree` lock now covers `luarocks install`, not only
  `make` and `remove`. LuaRocks takes an exclusive lock on the shared rocks
  tree for all three, so an unserialized install fails outright
  ("requires exclusive write access to ~/.luarocks"). `lua/tcp_client`'s
  `luarocks install luasocket --local` did exactly that beside another
  package's `luarocks make` and failed a full-build shard. 17 Lua packages
  run `luarocks install`. The match is now a command-word regex, like the
  opam and gem locks, and applies to any package's BUILD, not only Lua's.

### Added

- `-shard-weights <file>` selects sharding v2: shards are balanced on each
  package's measured build seconds (a `ci-package-durations-v1` file, from
  `code/scripts/ci_package_durations.py`) and whole prerequisite closures are
  kept together. v1 dealt packages by a heuristic and then copied each one's
  prerequisites after it, so on the 2026-10-18 main plan 5,327 packages became
  14,878 package builds over 15 shards; a 21-minute crate was rebuilt on
  twelve of them. Over that same plan v2 makes 5,908 builds with every shard
  at 69 serial minutes (v1: 134-233). An unmeasured package costs its
  toolchain's median. An unreadable or malformed file is an error, never a
  quiet fallback to v1. Without the flag, sharding v1 and its conformance
  fixtures are untouched.
- A `global:opam-switch` resource lock. Every OCaml package, and any BUILD
  that runs `opam`, holds it for its whole build, because they all share one
  opam switch. This is the executor-owned lock OCAML05 deferred, and it
  replaces the `-jobs 1` CI applied to an entire invocation whenever OCaml
  was active, which ran five main shards on one worker each (#17631).
  `TestExecuteBuildsRunsOCamlOneAtATimeAndEverythingElseAlongside` fails
  both without the lock and with whole-run serialization.
- A `global:ruby-gems` resource lock for every BUILD that runs `bundle
  install` or `gem install`. They all write the runner Ruby's one gem home,
  and RubyGems does not lock across processes: on PR #17759's first sharded
  run, two Ruby packages compiled json-3.1.0's native extension in the same
  directory at once and one failed with mkmf's temp log deleted under it.
- `-prune-leaf-targets` also removes a leaf package's `node_modules/` (beside
  a `package.json`) once it has built, by the same rules as Cargo `target/`:
  leaves only, never through a symlink, while the package still holds its
  locks. TypeScript `node_modules` were the largest consumer on that run's
  fullest shard: 21 GB of shard 8 at 90% disk.

- Exact STP01 cases.json changes schedule the committed Go starlark-compiler native reader on each platform, honoring language filters and failing before plan publication when that reader is missing.

- `-prune-leaf-targets`. Once a package has built (pass or fail), the flag
  deletes its own Cargo `target/` if no package depends on it. Main's full
  build turns it on. Each Mosaic package and Rust program is a standalone
  Cargo workspace that compiles its whole dependency tree into its own
  `target/`. Kept until the end, those directories filled the runner disk,
  so "No space left on device" and linker bus errors killed shards 5, 6 and
  12 on every main run (X-7). Only `<package>/target` beside a `Cargo.toml`
  is removed, so a shared workspace's output is never touched. A symlinked
  `target/` is not followed, and a package with dependents keeps its output
  (on a shard the graph is the whole repo's, so this holds across shards).
  `TestPruneLeafCargoTarget` covers each case.

### Fixed

- On Linux every `swift build`, `swift test` and `swift run` a BUILD command
  runs now gets `--build-system native`, SwiftPM's legacy planner. Swift 6.4,
  which the ubuntu-24.04 image ships, plans with its new swift-build backend,
  and that planner segfaults in libdispatch (`_dispatch_event_loop_drain`)
  before compiling anything. It failed `swift/ITF` (#17824 shard 5) and
  `swift/zip` (#17914 shard 3, after its tests had passed) on PRs that touch
  no Swift. The rewrite is applied at run time
  (`swiftNativeBuildSystemCommands`), so BUILD files and their hashes are
  unchanged. A command that already names `--build-system`, and macOS and
  Windows, keep the toolchain default.

- `-validate-build-files` on a CI shard (`-plan-file … -shard-index N`) now
  resolves names and paths against every package on the platform, not just
  the shard's. A package's test-only dependencies (a cpanfile `on 'test'`
  block) are not graph edges, so they are not pulled into its shard. Their
  own prerequisites then looked like undeclared references. Shards 14 and 15
  of every main run failed this way:
  `perl/brainfuck-wasm-compiler … undeclared local package refs:
  perl/wasm-execution` and the same for `perl/ir-to-wasm-compiler` (X-7).
  The new `ValidateBuildFilesAgainst(packages, known, graph)` still checks
  only the shard's packages. `ValidateBuildFiles` is unchanged, and
  discovery mode still validates as before. A validator test pins the shard
  case, and all 15 shards of a local `-force -shard-count 15` plan now pass.

### Added

- Case-exact native CI selection for the five checked `plan-*.json` fixtures.
  `replace-existing` schedules the Python build-tool tests and
  `portable-package-path` schedules TypeScript; the other three cases have
  neutral-only readers. Unknown flat cases fail closed. Raw-path and
  three-platform matching, language filters, missing roots, rename sources,
  emitted plans, and direct-reader drift are covered without forcing a full
  build or changing plan execution behavior. Git diff now emits NUL-delimited
  raw paths, so quoted non-ASCII names cannot bypass unknown-case rejection.

- Case-exact native CI selection for the twenty-two checked
  `validation-*.json` fixtures. Nine orphan-crate/tracked-artifact cases
  select eleven non-Go validator fronts, five package-root/Lua-Windows cases
  select Go, and eight remain neutral-only. Unknown flat cases fail closed;
  exact raw paths, three-platform plans, language filters, missing roots,
  rename sources, and native test-source drift are covered without forcing a
  full build or changing validator behavior.

- Case-exact native CI selection for all eleven checked
  `hashing-cache-*.json` fixtures. C#/F#/Lua read every case; Python reads
  missing/hit/corrupt, Go/Perl/Ruby/Swift read missing, and TypeScript reads
  corrupt. New unclassified flat cases fail before a partial plan. Three-
  platform and language-filtered plans, missing-root atomicity, renamed old
  paths, corpus roster, and native source-reference drift are tested without
  changing production hashing or forcing a full build.

- Case-exact native CI selection for all twenty-six checked
  `resolution-*.json` conformance fixtures. Each case schedules only its
  direct Go, Haskell, Lua, Perl, Python, Ruby, Rust, Swift, or TypeScript
  readers; unknown flat cases fail closed until classified. Three-platform
  plans, language filters, missing-root atomicity, rename-source behavior,
  and native reader/corpus drift are covered without changing the resolver.

- Bounded native CI selection for the eight flat `graph-*.json` and twelve
  `diff-selection-*.json` conformance cases. Exact path matching schedules
  thirteen direct build-tool readers on Linux, macOS, and Windows before
  affected closure, with language filtering, fail-closed missing roots,
  rename-source coverage, emitted-plan assertions, and reader-drift detection.
  This does not force a full build or promote process-free domain cores into
  full build-tool adapters.

- Bounded native CI selection for the twenty flat `source-collection-*.json`
  cases. Seven package-local cases schedule twelve direct build-tool test
  fronts, nine repository cases schedule C#/F#/Swift, and four shared-input
  cases schedule C#/F#. Exact path matching, per-language and three-platform
  plans, fail-closed missing roots, rename-source detection, emitted
  roots/toolchain assertions, and native-reader drift tests keep fixture-only
  changes from passing on neutral validation alone without forcing a full
  build or changing production source collectors.

- Exact flat-family CI selection for the eleven shared
  `toolchain-detection-*.json` cases and their twelve direct native build-tool
  readers. Added, changed, deleted, and renamed case paths seed only the
  applicable roots before affected closure on Linux, macOS, and Windows;
  explicit language filters remain bounded, missing readers fail before a
  plan is written, and focused regressions cover emitted toolchains and
  source-reference drift without forcing unrelated packages.

- Exact flat-family native CI selection for shared
  `ci-gate-selection-*.json` fixture changes. The planner seeds the Go and
  Python build-tool fronts before affected closure on all platforms, preserving
  their native CI-gate suites and toolchain flags without forcing unrelated
  packages. Missing applicable consumers fail planning; detector regressions
  cover path lookalikes, language filters, renamed/deleted sources, emitted
  plans, and direct-reader drift.

- Native Windows CI selection for affected `rust/programs/closurec` and its
  CV02 acceptance specification. The ordinary Windows package-test step no
  longer skips the compiler's publication/ACL regressions on Rust-only changes.
  Real-registry evaluator tests reproduce both missing selection clauses and
  preserve an unrelated Rust negative control.
  Specification-only CV02 diffs now also seed the compiler on every platform
  and require Rust; missing consumers fail planning and non-Rust language filters
  remain bounded. Emitted production-plan regressions reproduce the previously
  empty selection despite an enabled Windows gate and verify the native command.

- Exact-path native CI selection for changes to the shared
  `discovery-language-registry.json` fixture. Detect and all platform build-plan
  overrides now schedule its eleven direct build-tool consumers, including both
  .NET fronts and Haskell; missing registered roots fail closed and a source-reference test
  guards future consumer-map drift without forcing all packages. Single-language
  runs select their own fixture consumer, and rename diffs retain the old path.

- `-validate-build-files` rejects a shared `BUILD` that Windows would run with a
  POSIX environment assignment `cmd /C` cannot execute. Each line is first
  passed through the executor's own Windows rewrite
  (`executor.RewriteInlineEnvPrefixForWindows`, newly exported for this). Any
  `VAR=` left where a command name belongs is reported, at the start of the
  line or after `(`, `;`, `&&` or `||`. That covers values the rewrite refuses,
  such as `RUSTC="$(rustup which rustc)" ...`, and assignments after `&&`. cmd
  would run `VAR` as a program: `RUSTC` even resolves to rustup's `rustc.exe`
  proxy and fails with "unknown proxy name". Packages with a `BUILD_windows` are
  exempt. The first run flagged 18 latent packages, which now have Windows
  files: 8 .NET, 7 JVM, perl/aes-modes, typescript/http1 and
  programs/python/xor-hidden-layer-demo.

### Changed

- Advanced the graphdiff test's repository source-input boundary digest pin
  for the new `forme-plugin-installer-core`, `forme-plugin-runner-conformance`,
  and `forme-plugin-runner-ts` TypeScript roots to
  `3370c811b51962c87757b01e98ad3db5206cad2c75e74c6853bf5209548bd18e`.

- Adopted the generalized established-lane orphan package validator without
  widening the legacy Cargo snapshot contract.
- Regenerated the canonical source-input projection for the exact lowercase
  `blib` exclusion and advanced its language-registry digest and dependent
  repository-boundary digest pins.

### Added

- Added a typed, process-free graph and diff-selection core that dynamically
  consumes all eight graph and eleven diff-selection cases in the neutral v1
  corpus. It now pins canonical graph output, exact closure semantics,
  repository-boundary reverse selection and digest validation, strict portable
  globs, both unknown-path policies, and the fixed 50,000,000 Unicode-scalar
  match-work ceiling without acquiring Git, filesystem, environment, process,
  credential, or network authority.
- Bounded ordinary CI-gate path evaluation to a complete 50,000,000-unit
  Unicode-scalar preflight, returning stable
  `CI_GATE_MATCH_LIMIT_EXCEEDED` failure before any glob call or partial plan
  output. The globstar matcher now memoizes per-call states so accepted
  recursive near misses cannot recompute the same suffix grid exponentially.
  Native registry loading now enforces the neutral gate, entry, identity, and
  portable-glob bounds before either run-all bypasses or match-work accounting,
  and segment matching now follows the neutral Python `fnmatchcase`
  character-class grammar instead of Go's incompatible host grammar.

### Changed

- Split the FIFO-manifest regression from the portable B07 executor probes so
  Windows can compile and run the package tests without the Unix-only
  `syscall.Mkfifo` symbol. Unix hosts still execute the original security
  regression.
- CI-gate match-work preflight now deduplicates identical globs and applies a
  sound literal path-segment prefix/suffix filter before charging the full glob
  grid. Large path-sharded diffs under an unrelated subtree therefore keep
  selective CI instead of crossing the ceiling solely because they contain
  hundreds or thousands of direct-owner files; ambiguous pairs still pay the
  full conservative cost and every hard error remains fail-open at the
  workflow boundary. Measured on the real diffs, #15719 falls from 109,569,282
  declared-product units to 833,868 bounded units with 27 candidate pairs, and
  #15729 falls from 4,480,430,106 to 30,615,897 with 36 candidates. The former
  plans in 3.9 seconds over 5,193 packages, requires TypeScript alone, and
  selects zero of nineteen gated CI jobs instead of the 42-check run-all queue
  observed before this change.

### Fixed

- Strict Starlark diff selection now recognizes only the five canonical BUILD
  front basenames. Near names such as `BUILD_debug`, `BUILD.bak`, and
  `BUILD_windows.old` remain known package paths but no longer select a package
  unless a declared source glob matches.
- A bare `set -e`/`set -eu` line at the top of a `BUILD` file — POSIX
  strict-mode preamble, present in 20+ Rust crates' `BUILD` scripts —
  crashed the whole package's Windows build. `runPackageBuild` runs each
  `BuildCommands` line as its own separate process via `cmd /C` on Windows,
  where bare `set` is cmd's own builtin for reading/defining an environment
  variable: `set -eu` with no `=` means "print the value of a variable
  named `-eu`", which cmd reports as `Environment variable -eu not
  defined` and fails the package. The directive was already functionally
  inert on every platform (the executor already checks each line's exit
  code independently before starting the next one — `set -e` never reached
  across that process boundary), so `readLines` now drops bare
  `set <short-flags>` lines everywhere rather than special-casing Windows.
  Real `set VAR=value` / `set "VAR=value"` assignments (used throughout
  this repo's `*_windows` `BUILD` files) are untouched, since the filter
  only matches a lone flag cluster with no `=`.

- A POSIX inline environment-variable prefix at the start of a BUILD line —
  `RUSTDOCFLAGS="-D warnings" cargo doc -p widget --no-deps`, present in
  17+ Rust crates' BUILD files, plus the same shape for `PYTHONPATH`,
  `NPM_CONFIG_CACHE`, and `PYTHONIOENCODING` in a handful of other
  packages — crashed the whole package's Windows build with `'RUSTDOCFLAGS'
  is not recognized as an internal or external command`. cmd.exe (which
  `runPackageBuild` invokes each BUILD line through on Windows) has no
  notion of "set this variable for just the following command," so it
  tried to run the literal text `RUSTDOCFLAGS="-D` as a program name.
  `shellCommandForOS` now rewrites a simple prefix (bare or double-quoted
  value, no command substitution, no chained assignments) into cmd's own
  `set "VAR=value"&& command` form before invoking cmd.exe on Windows;
  Unix is untouched, since the original line already works there. A
  prefix this narrow translation can't safely express (`$(...)`, a second
  chained assignment, a subshell) is left unchanged, exactly as it was
  before this fix -- those shapes already have a hand-written
  `BUILD_windows` override wherever they're actually exercised on Windows
  CI (verified against every current instance in the repo before writing
  this).

- `packageCost` now weighs a Rust package higher when its build commands
  invoke `cargo tarpaulin`: coverage instrumentation measurably costs several
  times a plain `cargo test`, but the shard cost model treated every Rust
  package alike. The greedy load-balancer in `ComputeShards` only sees that
  single number, so it could unknowingly cluster several tarpaulin packages
  onto one CI shard -- main CI hit a shard that blew its 150-minute build
  ceiling with `cargo-tarpaulin` still running at 936/946 packages while
  sibling shards finished early. The extra weight gives the balancer the
  signal it needs to spread tarpaulin packages across shards instead.

### Added

- Registered only the reviewed blog and landing-page roots as TypeScript
  source-input profiles, including their 13 exact authored resources. Unknown
  sibling, nested, or outside site roots now fail before traversal because the
  bridge is anchored to the nearest real Git repository root, and ordinary
  TypeScript packages cannot inherit same-shaped files.

- Site packages under `code/sites` now use the canonical TypeScript source-input
  profile for hashing while retaining their legacy `unknown/*` build-graph
  identities.

- The pure CI gate evaluator now consumes all seven shared language-neutral
  `ci_gate_selection` fixtures, pinning package/path matches, globstar behavior,
  complete false verdicts, output names, and every fail-open escape for future
  build-tool implementations. Registry loading now rejects gate ids that would
  collide after hyphens are mapped to underscores for Actions outputs.

- **The planner now decides which GitHub Actions jobs a change needs.** A new
  `-ci-gates` flag reads a declarative registry
  (`code/specs/data/ci-gates.json`, default) that names each gated CI job and
  declares the packages it exercises and the non-package paths it reads. During
  plan emission the tool intersects those declarations with the affected closure
  and the raw changed-file list, writes the verdicts into the build plan as an
  optional `ci_jobs` object, and emits one `run_<gate>=true|false` line per gate
  to stdout and `$GITHUB_OUTPUT`. `ci.yml` consumes them as job-level `if:`
  conditions, so a human-languages pull request no longer runs the Ruby
  compiled-grammar regeneration, the four six-language chief-of-staff crypto
  conformance jobs, or the five Unicode 17 runtime checks.

  Measured across 35 successful pull-request runs beforehand: ~91 min mean
  wall-clock against ~48 min of total execution, with `detect` waiting a median
  43 min for a runner. About 70% of a run was queueing, because every run
  claimed 16 concurrent slots from a saturated ceiling and the eleven always-on
  jobs declared no `needs:` — so they were scheduled ahead of the very job that
  decides what needs to run.

  Gate evaluation fails open: force mode, an unavailable git diff, or a change
  to `ci.yml` or the registry itself runs every gate, and main-branch pushes
  already pass `-force`, so a gate that is wrong on a pull request is still
  caught on merge. A malformed or unfireable registry entry is a hard error
  rather than a silent all-false, because a gate that never fires is
  indistinguishable from a gate that always passes. See
  `code/specs/ci-gate-registry.md` and `internal/cigates`.

  Note that a gate needs BOTH clauses to work: `sharedPrefixes` is empty, so
  changes under `code/specs`, `code/fixtures`, `code/grammars`, and
  `code/scripts` map to zero packages and never reach `affected_packages`. A
  package-only gate would skip the D18F job on a pull request that changed only
  the D18F manifest — exactly the drift that job exists to catch.

### Fixed

- **Source selection now consumes the complete generated language registry.**
  The Go hasher replaces its partial engine-local maps with a deterministic
  embedded projection of all 23 languages and seven selector roles. Unknown
  languages fail before filesystem inspection; root, scoped, and exact-package
  rules retain their neutral scope; generated components are pruned before all
  matching; and exact Engram WASM resources cannot widen to sibling packages.
  Strict projection, digest, fixture, and generator-roundtrip tests detect both
  missing and undeclared selectors without any runtime fixture dependency.

- **Source collection and package digests now match portable hashing v1.**
  Extension and declared-source modes share the exact case-sensitive
  26-component generated-directory registry, all five BUILD fronts, OCaml
  source/metadata recognition, root-only unconditional `.opam` manifests, and
  stable symlink/reparse rejection. Sorted normalized repository-relative
  UTF-8 paths and exact raw file bodies are framed with unsigned 64-bit
  big-endian lengths before SHA-256, so same-content renames and binary bytes
  are observable without hashing absolute paths, host metadata, or locale.
  Package-root and nested POSIX links plus Windows junction/reparse points fail
  before traversal; unstable or unreadable sources return a checked error and
  the CLI exits `2` with a stable root-redacted diagnostic whose quoted package
  identity escapes control characters instead of caching a sentinel digest,
  forging a log line, or panicking.

- **Extra CI toolchain declarations strip carriage return only as part of
  CRLF.** A final lone CR and a CR before trailing ASCII whitespace remain
  content and make the lookalike inert. The Go parser now matches the hardened
  language-neutral snapshot and Python reference exactly.

### Added

- **Extra CI toolchain declarations now consume the language-neutral
  process-free corpus.** The Go parser accepts only exact canonical
  `# needs-toolchain: NAME` comment records, supports CRLF and bounded ASCII
  whitespace, stably deduplicates valid names, and ignores empty, unknown, or
  malformed lookalikes. Platform selection shares one precedence table between
  real discovery and inert fixture snapshots; the successful affected-only and
  forced-full decisions are exercised without checkout, process, environment,
  or network access, while the shared validator owns invalid-case and aggregate
  snapshot rejection. Platform plan preparation refreshes declarations from
  the selected BUILD front. Both `java-to-semantic-ir` BUILD fronts now
  truthfully request the Python toolchain used by their execution proofs.

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
