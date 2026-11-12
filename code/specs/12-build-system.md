# 12 — Build System

## Overview

The build system discovers, resolves, and builds packages across a
multi-language monorepo. The Go implementation is the primary tool used in CI
and is currently the broadest operational reference.

Executable front doors also exist in C#, Elixir, F#, Haskell, Lua, Perl,
Python, Ruby, Rust, Swift, and TypeScript. Dart, Java, and Kotlin are established
package lanes that still need build-tool implementations. F# currently delegates
to the C# engine. C and C++ are emerging lanes, and OCaml is planned as an
emerging lane.

These implementations are not yet feature-identical. Observable portable
behavior is defined by [the build-tool conformance
contract](build-tool-conformance.md), not by directory presence or this
architecture overview.

The build system is not part of the computing stack — it is the infrastructure that builds and tests the stack.

## Design Goals

1. **Incremental**: Only rebuild packages that changed (via git diff or hash comparison).
2. **Parallel**: Independent packages build concurrently.
3. **Multi-language**: The primary tool discovers every established
   implementation lane and the explicitly supported emerging lanes.
4. **Zero configuration**: Packages are discovered automatically from the directory tree.
5. **Deterministic**: Same inputs always produce the same build plan.

## Package Discovery

### Algorithm

The build system discovers packages by recursively walking the directory tree looking for BUILD files. A directory containing a BUILD file is a package. The walk skips known non-source directories for performance.

```
function discover(directory):
    if directory.name in SKIP_LIST:
        return                          # ignore junk directories

    if BUILD file exists in directory:
        register as package             # leaf node — don't recurse deeper
        return

    for each subdirectory in directory:
        discover(subdirectory)          # recurse
```

This is the same approach used by Bazel, Buck, and Pants. It requires no configuration files to route the walk — the presence of a BUILD file is sufficient.

### Skip List

The following directory names are skipped during discovery. They are known to contain non-source files that should never be treated as packages:

```
.git            # version control
.hg             # Mercurial
.svn            # Subversion
.venv           # Python virtual environments
.tox            # Python tox environments
.mypy_cache     # mypy type checker cache
.pytest_cache   # pytest cache
.ruff_cache     # ruff linter cache
__pycache__     # Python bytecode cache
node_modules    # Node.js dependencies
vendor          # vendored dependencies (Go, Ruby)
dist            # build output
build           # build output
target          # Rust/Java build output
.claude         # Claude Code worktrees and config
Pods            # CocoaPods (iOS)
```

### BUILD File Format

A BUILD file is a plain text file containing shell commands, one per line. Blank lines and lines starting with `#` are ignored. The commands are executed sequentially in the package's directory.

```
# Example BUILD file for a Python package
uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```

```
# Example BUILD file for a Go package
go test ./... -v -cover
```

```
# Example BUILD file for a Rust package
cargo test -p logic-gates -- --nocapture
```

### Platform-Specific BUILD Files

Platform-specific shell files take precedence over shared Unix, Starlark, and
generic fallbacks. The canonical order is specified and tested in
`build-tool-conformance.md`. In particular, a Windows `BUILD_windows` override
must win over a Starlark plan; current implementations that do otherwise are
non-conforming.

An override is a complete standalone recipe, not a patch applied to the
canonical `BUILD`. It must therefore preserve every canonical repository-local
prerequisite and install the complete closure in dependency order before
building the package itself.
Lua packages whose canonical recipe installs sibling rocks require a
`BUILD_windows` recipe with the same sibling closure, Windows path and redirect
syntax, and equivalent dependency-resolution hardening. Each BUILD line runs
in a separate shell process, so no prerequisite may rely on a previous line's
working-directory change.

#### `BUILD_windows` lines are `cmd` syntax

Every non-comment line of a `BUILD_windows` file runs as its own `cmd /C
<line>` process, started in the package directory (the Go reference tool's
`executor.shellCommandForOS`; the only translation it applies first is a
leading POSIX `VAR=value cmd` prefix, rewritten to `set "VAR=value"&& cmd`).
It is not `sh -c`, not a batch file and not PowerShell. A `BUILD_windows` line
must therefore use `cmd` syntax:

| Need                     | POSIX (`BUILD`)                     | `cmd` (`BUILD_windows`)                    |
|--------------------------|-------------------------------------|--------------------------------------------|
| discard output           | `>/dev/null 2>&1`                   | `>nul 2>nul` (or `>nul 2>&1`)              |
| is a tool on PATH?       | `command -v swift >/dev/null`       | `where swift >nul 2>nul`                   |
| does a file exist?       | `[ -f x ]`                          | `if exist x (...)`                         |
| is a variable set?       | `[ -n "$X" ]`, `${X:-}`             | `if defined X (...)`, `%X%`                |
| expand a variable        | `$X`, `${X}`                        | `%X%`                                      |
| conditional / loop       | `if ...; then ...; fi`, `for ...; do ...; done` | `if ... (...) else (...)`, `for %i in (...) do ...` |

POSIX constructs are allowed only inside an explicit POSIX-shell hand-off on
that line, `bash -c "..."` or `sh -c "..."` (GitHub's Windows runners provide
Git Bash), where the double-quoted payload belongs to that shell. Text inside
any double-quoted argument is the receiving program's business, so
`python -c "import os; print(os.devnull)"` or `perl -e "print $x"` is fine.
The parts of a hand-off line outside its quoted payload are still `cmd`'s:
`bash -c "..." >/dev/null` fails, because `cmd` opens the redirect target.
Single quotes are not quotes to `cmd` (it still splits `bash -c 'a && b'` at
the `&&` and redirects at a `>` inside them), so a hand-off payload must be
double-quoted.

The failure mode is quiet, which is why this is validated rather than left to
CI. `where swift >/dev/null 2>/dev/null && swift test || echo skipping` never
fails: `cmd` cannot open `/dev/null`, so the guarded command never runs and the
line always falls through to the `echo`, silently turning a test run into a
skip. Constructs such as `command -v` or `if [ ... ]; then` fail loudly ("-v
was unexpected at this time"), but only when a pull request happens to rebuild
that package on Windows, which for a leaf package can be months after the line
was written.

**Validator rule `windows-posix-syntax`.** `--validate-build-files` reads every
package's `BUILD_windows` directly, on every host, and rejects a line that
uses, outside double-quoted text (which is where a `bash -c "..."` hand-off's
payload lives), any of these POSIX-only constructs:

- `/dev/null`;
- `command -v`;
- `$(...)` command substitution (including `$((...))` arithmetic);
- `${...}` parameter expansion;
- a `[` or `[[` test, as a word of its own;
- the shell keywords `then`, `fi`, `do`, `done` or `esac` directly after a
  `;` (POSIX requires a separator there; `cmd`'s own `for ... do` has none).

The diagnostic names the file, the 1-based line number, the construct and its
`cmd` alternative. Blank lines, `#` comment lines and `rem` lines (a `cmd`
no-op) are not checked. Shared `BUILD` files are not checked by this rule.
The Go reference tool implements it (`internal/validator/windows_posix_syntax.go`);
it has no neutral conformance fixture yet.

### Language Inference

The package's language is inferred from its directory path. The build system scans path components for known language names:

| Classification | Path components |
|---|---|
| Established implementation | `csharp`, `dart`, `elixir`, `fsharp`, `go`, `haskell`, `java`, `kotlin`, `lua`, `perl`, `python`, `ruby`, `rust`, `swift`, `typescript` |
| Emerging implementation | `c`, `cpp`; later `ocaml` |
| Shared execution/build buckets | `dotnet`, `wasm`, `starlark` |

For example, `code/packages/python/logic-gates` yields language `python`. If no known language component is found, the language is `unknown`.

### Package Naming

A package's qualified name is `{language}/{dirname}`. For example:

- `code/packages/python/logic-gates` → `python/logic-gates`
- `code/packages/go/directed-graph` → `go/directed-graph`
- `code/packages/rust/arithmetic` → `rust/arithmetic`

## Dependency Resolution

The build system parses language-specific metadata files to discover inter-package dependencies:

| Language | Metadata file    | Dependency prefix           |
|----------|------------------|-----------------------------|
| Python   | `pyproject.toml` | `coding-adventures-`        |
| Ruby     | `*.gemspec`      | `coding_adventures_`        |
| Go       | `go.mod`         | module path contains repo   |
| TypeScript | `package.json` | `@coding-adventures/`       |
| Rust     | `Cargo.toml`     | workspace member path       |
| Elixir   | `mix.exs`        | `:coding_adventures_`       |
| Mosaic   | `mosaic-package.toml` | `[dependencies]` key = directory basename or `[package].name` |

Dependencies on external packages (not in the monorepo) are silently ignored.

A Rust (or, in the Go tool, WASM) package's edges come from `path` entries in
every Cargo table that links into or builds the crate: `[dependencies]`,
`[build-dependencies]` (and the legacy `build_dependencies`), and the same
tables under `[target.<cfg>.…]` for every cfg, as a union over all targets
because CI builds on several hosts. Entries are read both as inline tables
(`name = { path = "..." }`, whose arrays may continue onto later lines) and as
`[<table>.<name>]` sub-tables with a `path = "..."` key; a quoted `package`
field renames the dependency. A field or sub-table key may be bare or quoted
(`"path" = "..."` is the `path` key, as Cargo reads it). Before
2026-11-01 only the top-level `[dependencies]` table was read, so a change to
a build or target-specific dependency rebuilt and re-tested none of its
dependents: `paint-metal`, `paint-vm-cairo` and `paint-vm-direct2d` under the
barcode crates, the sandbox crates under the chief-of-staff launchers, and
`grammar-tools` under the lexers' and parsers' build scripts were all missed.
`[dev-dependencies]` in every form are not edges; they are test triggers (see
below). The exact line-reading procedure, including the out-of-scope TOML
shapes, is the cross-implementation contract in `build-tool-conformance.md`.
That procedure is lenient, so in the Go tool the strict test-trigger reader
below also screens these link tables of every Rust and WASM package and stops
the plan on any shape the procedure does not model. The edge reader carries
its own limits (a manifest that is not a regular file or exceeds 1 MiB, or one
entry continuing past 256 lines or 64 KiB, stops the plan), because it runs
before that reader. A dependency that resolves to the package itself is a
`DEPENDENCY_SELF_EDGE` error, never a self-loop.

Mosaic is a domain language, not an implementation lane, so only the Go
reference tool resolves it (the conformance contract does not require the
other build-tool implementations to). Only the keys of the root
`[dependencies]` table are
candidates; the requirement string is version data. Lookup is scoped to
Mosaic packages, and
self-references are dropped. A key without the `mosaic-pkg-` prefix also
depends on `mosaic-pkg-<key>`, matching the resolver's directory locator. Because a
missed edge silently skips a dependent's tests, the reader fails closed: a
manifest outside the plain subset it reads (any root-level key; any table
header other than a one-part `[package]`, `[dependencies]`, `[components]`,
`[kernel]`, `[styles]`, `[app]`, `[host_assets]` or `[host_effects]`; a
quoted or dotted key, or a non-string value, in `[package]` or
`[dependencies]`; a multi-line string; any backslash), a manifest that is not a regular file or exceeds 1 MiB, and two Mosaic
packages (libraries or apps) claiming one name are errors that stop the plan.
A leading UTF-8 byte-order mark is stripped, as TOML allows. The resolver builds a directed graph where an edge from A to B means "B depends on A" (A must build before B).

## BUILD Validation

`--validate-build-files` must reject source that the discovery graph would
silently omit. In particular, every supplied Cargo-manifest directory must be
covered by a runnable `BUILD`, `BUILD_windows`, `BUILD_mac`, `BUILD_linux`, or
`BUILD_mac_and_linux` in that directory or an ancestor beneath `code/`. A file
containing only blanks or comments is not runnable and cannot be used as a
one-touch bypass. Generated and vendored directories use the exact artifact
skip registry defined by the language-neutral conformance contract.

Intentionally uncovered manifests are policy data in
`code/BUILD-EXEMPTIONS`. Each `EXCLUDED` or `PENDING` entry requires a portable
repository-relative path and a non-empty reason. Duplicate, unsafe,
out-of-scan, artifact, unknown-kind, and reasonless entries are errors. An
entry is also an error after its directory disappears, its Cargo manifest is
removed, or a runnable BUILD begins covering it. This makes PENDING debt
countable and prevents the ledger from silently outliving the gap.

Portable engines consume the closed `orphan_crate_coverage` snapshot in
[`build-tool-conformance.md`](build-tool-conformance.md#13-closed-pure-domain-fixture-model).
The neutral check receives normalized manifest, directory, BUILD-state, and
ledger records as inert data. It performs no filesystem walk, Git query,
process launch, environment lookup, or network access; native enumeration and
file reading stay outside the process-free oracle.

The generalized `orphan_package_root_coverage` contract is distinct from the
Cargo gate. It enumerates only direct `code/packages/<established-lane>/<name>`
roots, pins one compact source witness selected by the governed language-source
registry, and ignores virtual roots plus exact generated components from that
registry. Its closed snapshot is capped at 8,192 roots, 16,384 BUILD fronts,
8,192 exemptions, and 2,000,000 canonical compact-JSON UTF-8 bytes. It shares
the five exact BUILD names, component-wise ancestor coverage, ledger grammar,
stale-entry behavior, collision rules, and deterministic ordering above while
using package-root-specific unlisted and empty-front diagnostics. Native Go
discovery streams and bounds caller-selected filesystem data without invoking
Git or acquiring process, environment, or network authority. OCaml remains an
emerging lane until its promotion gates pass and is therefore not in this
fifteen-lane denominator.

Haskell packages carry one platform-routing rule. GHC 9.4.8's macOS bindist
hardcodes a Command Line Tools SDK sysroot; when the runner's default developer
directory is an older Xcode beside newer Command Line Tools, that Xcode linker
cannot read the CLT SDK's `arm64e.x1` stubs and every link fails. Cabal builds
a package's whole local project in the one process the package root's BUILD
line starts, so fixing a dependency's BUILD never protects the root that
selected it. Every BUILD line a `haskell` package runs on macOS (the file
Darwin resolves: `BUILD_mac`, then `BUILD_mac_and_linux`, then `BUILD`) must
therefore select `DEVELOPER_DIR=/Library/Developer/CommandLineTools` behind a
`[ "$(uname -s)" = Darwin ]` check, in one of two accepted shapes:

```sh
# Prefix: export inside this line's own `sh -c`, then the unchanged command.
if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal test all
# Inline: the assignment scoped to the Darwin branch's Cabal command.
if [ "$(uname -s)" = Darwin ]; then DEVELOPER_DIR=/Library/Developer/CommandLineTools cabal test all; else cabal test all; fi
```

Both scope the selection to that line's process tree (each line is its own
`sh -c`), leave other platforms' inherited environment untouched, never call
`xcode-select`, and keep the package command's exit status as the line's exit
status. A line consisting only of the `BUILD_TOOL_UNSUPPORTED` protocol record
or of a bare `echo` launches no toolchain and is exempt. Comment lines never
satisfy the rule. `--validate-build-files` rejects a missing selection and
names the file and the governing lesson
(`lessons.d/ghc-must-select-the-linker-matching-its-hardcoded-macos-sdk.md`).
This rule is implemented by the Go reference tool; it has no neutral
conformance fixture yet.

Tracked dependency artifacts are a separate validation boundary. The closed
`tracked_artifact_absence` snapshot supplies bounded path and entry-kind
records as inert data and declares the exact Unicode `17.0.0` data version.
Portable engines use that one version for NFC, NFKC, full default folding, and
root full uppercase rather than inheriting host-runtime tables. They normalize
separators, reject unsafe paths without echoing them, and reject every case or
Unicode compatibility alias of a `node_modules` path component. A separator
may not create an empty component at the end of a path: trailing slash and
backslash inputs fail with
`EMPTY_SEGMENT` after normalization. Regular files, symlinks, and reparse
records are classified identically and never opened or followed. Native Git
index enumeration and host-filesystem metadata collection stay outside the
language-neutral process-free oracle.

## Build Execution

### Change Detection

The primary change detection mode uses git:

```
git diff --name-only <base>...HEAD
```

This produces the list of files that changed relative to the base branch (typically `origin/main`). Changed files are mapped to packages by path prefix matching. The dependency graph is then used to find all affected packages — both directly changed packages and their transitive dependents. A package whose only changes are additions or edits of package-local metadata (its root changelog, capability manifest or assurance evidence; see below) is rebuilt without its dependents.

#### Repository input consumers

A few packages hold *repository-wide guards*: tests that walk the whole
`code/` tree rather than their own directory, so prefix matching never maps
their inputs to them. The Go tool keeps an explicit table of such consumers,
each with a predicate over raw Git paths. When a changed path matches, the
consumer is added to the affected set together with its prerequisites but
**not** its transitive dependents — only the data it reads changed, not its
code. Matching uses raw, uncleaned Git paths and never stats them (a deleted
input still selects), honors a single-language invocation, and fails the
plan if the consumer is missing from discovery.

| Consumer | Selected by |
|---|---|
| `rust/mosaic-package-resolver` | any `mosaic-package.toml`, `.mil`, `.mll`, or `.msl` under `code/` |

#### Cargo dev-dependency test triggers

A Cargo `[dev-dependencies]` entry links a crate into another crate's tests,
never its library, so it is not a build-graph edge (the
`resolution/rust-field-aware` conformance case pins that, while pinning
build and target-specific dependencies as edges). It must still
re-run the dependent's tests: every Mosaic package and app is tested by a
Cargo smoke harness whose only in-repository dependencies are dev-dependencies
on the Mosaic compilers and artifact builder, so without this a change to a
`mosaic-emit-*` backend re-ran none of those suites.

The Go tool therefore keeps a second relation, *test triggers*: for each
package `D`, the packages whose `Cargo.toml` names `D` in a trigger table.
For languages `rust` and `wasm` the trigger tables are `[dev-dependencies]`
and every `[target.<cfg>.dev-dependencies]` (the union over all targets, since
CI runs on several hosts). For a `mosaic` package's harness (the `Cargo.toml`
beside `mosaic-package.toml`) they also include `[dependencies]` and
`[build-dependencies]` and their target forms, because a Mosaic package has no
Cargo graph edges at all. Every `[patch.<source>]` table is read for all three
languages, because Cargo builds the tests against a patched path even for a
registry dependency. The legacy `dev_dependencies`/`build_dependencies`
spellings are the same tables. Self-references are dropped. Change detection
applies the relation **one hop**:

```
closure  = AffectedNodes(changed)
closure ∪= triggers[a]  for every a in closure
affected = closure + prerequisites
```

A dev-dependent's own dependents are not added (its library is unchanged),
triggers are never chased transitively, and prerequisites are added after the
hop so they trigger nothing. Keeping the relation out of the graph is also
what keeps the graph acyclic: Cargo allows A to dev-depend on B while B
depends on A (the repository has such pairs), and as a graph edge that cycle
would fail every package's build.

Entries resolve **by path, not by name**: only entries with a `path` field
count. `filepath.Join(manifest directory, path)` has every symlink resolved
and is matched against the discovered packages' real directories, so neither
a crate declaring the same Cargo `name` nor a symlinked, differently-cased or
Unicode-normalization-aliased spelling of the path can move or drop the
trigger. The target is:

- a discovered package root: that package is a trigger. The chain does not
  end there. The build graph does record a Rust or WASM package's
  `[dependencies]`, `[build-dependencies]` and target forms, so when every
  link of that package resolves to another package, following it only
  re-derives what the one-hop rule already reaches through `AffectedNodes`.
  The graph cannot see the rest, though. It has no node for a crate that is
  not a package root (a nested or orphan crate, which may itself be reached
  only through a dev-dependency), so nothing such a crate links is a graph
  edge. It reads another language's package through that language's manifest,
  so a `Cargo.toml` at its root (a native extension) contributes no edge. And
  it resolves entries by Cargo name, where triggers resolve by path. Following
  every target keeps all three cases covered without depending on which of
  them applies;
- any other crate: if it lies inside a discovered package (another package,
  or the dependent itself), that package is a trigger, since prefix matching
  attributes changes there to it. If no package covers it, it must be listed
  exactly in `code/BUILD-EXEMPTIONS`. `engram-core-wasm` is such an orphan,
  and it is how `engram-app`'s tests reach `engram-core`;
- anything else (nothing there; an absolute or empty path; an uncovered crate
  that the ledger does not list, such as one under `vendor/` or outside
  `code/`) is an error.

In both of the first two cases the target's `[dependencies]` and
`[build-dependencies]` (target forms included, `[dev-dependencies]` not) are
linked into the dependent's tests too, so they are followed transitively with
the same strict reader. Each crate's closure is computed once per resolution
and cached, so the walk is linear in the number of crates (about 0.8 s for the
whole repository). A cycle among those tables is an error, as it is in Cargo,
so a cached closure can never be one a cycle cut short; cycles through
`[dev-dependencies]` (which the repository has) never arise, because
dev-dependencies are read only at the dependent. Chains deeper than 16 crates
are an error, not a truncation. Every repository manifest reached this way
must pass the strict reader. When this was added, `paint-vm-direct2d` (which is
reached) and `paint-vm-gdi` (which is not yet reached) were the only tracked
manifests it refused. Both declared `windows` as a `[dependencies.windows]`
sub-table, and both were rewritten as the equivalent inline table (the parsed
TOML is identical). The reader was not loosened.

Path entries resolve against every discovered package even when `-language`
filters the run, so a path into a filtered-out package is not mistaken for an
unlisted crate. A `.cargo/config` or `.cargo/config.toml` in the package
directory or any ancestor up to the repository root that sets `paths`,
`[patch]` or `[source]` (source replacement, which can serve a registry
dependency from repository files) is an error. Refusing is preferred to treating the file as a
rebuild-everything input, which would silently make every later change a
full rebuild. The repository's four configs only pick a Windows linker.
`[replace]` and `workspace = true` are refused, and so is a `git` source
that is not an `https://` or `ssh://` URL (a `file://` URL or bare path can
name a predictable checkout directory with no trigger).

The cost is deliberate. Measured on the repository when this was added, a
one-line change to `mosaic-emit-qt`, `mosaic-emit-flutter` or
`mosaic-package-artifact-builder` went from 73 selected packages to 158
(`moslayout-compiler`: 74 to 160): 27–28 Mosaic package and app suites,
`rust/venture-browser-qt`, `rust/venture-browser-macos` and
`rust/venture-browser-windows` (which dev-depend on the artifact builder, the
last two through target tables), and their build prerequisites from the normal
prerequisite expansion (about 51 for `venture-browser-qt` alone). Dev-dependents
keep that expansion like every other selected package. The fan-out was
reviewed and accepted at 151, before target tables were read; reading them
added the remaining 7. Following crates transitively (non-package crates,
then package roots too) leaves those numbers unchanged and makes a change to
`engram-core` select 27 packages instead of 14 (`engram-app` and its Mosaic
prerequisites).

The reader fails closed, because a dropped trigger silently skips a
dependent's tests. It reads a plain TOML subset and stops the plan on anything
outside it:

- a multi-line string (`"""` or `'''`) anywhere in the manifest, a one-line
  string left open, mismatched brackets, or a value still open at the end of
  the file;
- a header or key it cannot decode (any backslash, an empty segment, a stray
  bracket). Quoted segments are decoded, so `["dev-dependencies"]` *is* the
  table;
- a trigger table in any shape but `[name]` or `[target.<cfg>.name]`: a
  sub-table (`[dev-dependencies.x]`), an array of tables, or a key elsewhere
  that writes one (`dev-dependencies.x = ...`, `target = { ... }`);
- inside a trigger table, an entry that is not `bare-key = "version"` or a
  one-line inline table (only an array value may continue onto the next
  line). Every inline-table field key must be bare, exactly lowercase and
  unique (Cargo ignores `Package`, so it must not be read as a rename), with a
  one-line string, boolean or array-of-strings value; nested inline tables,
  trailing commas, escapes and `workspace = true` are refused;
- for a Rust or WASM package, a write into a graph-edge table
  (`[dependencies]`, `[build-dependencies]`, `[build_dependencies]` and their
  `[target.<cfg>.…]` forms) in a shape the lenient edge reader does not
  model. Accepted: `[D]` and `[target.<cfg>.D]` holding one-segment keys whose
  value is a one-line string or an inline table (which may continue onto
  later lines) whose fields are all `key = value` with one-segment keys; and
  `[D.<name>]` and `[target.<cfg>.D.<name>]` sub-tables holding one-segment
  keys with one-line values. Refused: dotted keys there or anywhere else that
  reach such a table (`a.path = ...` in `[build-dependencies]`,
  `build-dependencies.a.path = ...` at the root, `dependencies.a = ...` under
  `[target.<cfg>]`), inline tables that spell one
  (`build-dependencies = { ... }` under `[target.<cfg>]`, `target = { ... }`),
  deeper headers and arrays of tables (`[dependencies.a.features]`,
  `[[dependencies]]`), any other value (an array, a number), a value that
  spans lines in a sub-table, any backslash in such a table's values, and
  workspace inheritance (`a = { workspace = true }`, or a `workspace` key in
  a sub-table): Cargo takes that entry's path from the workspace root's
  `[workspace.dependencies]`, which the edge reader never opens;
- a manifest that is not a regular file or exceeds 1 MiB, or one entry
  continuing past 256 lines or 64 KiB. The file is opened once, and the open
  handle must be the very regular file the size check saw (`os.SameFile`),
  so a symlink swapped in between is refused rather than followed.

A leading UTF-8 byte-order mark is stripped, as TOML allows.

#### Package-local metadata

Some files in a package are checked by the package's own BUILD but cannot be
read by any of its dependents' builds or tests. Rebuilding every dependent
when one of them changes costs CI time and proves nothing: before this rule, a
pull request that only added `rust/bounded-json/required_capabilities.json`
and a `CHANGELOG.md` entry selected 556 packages, and one that touched 40
such files across leaf crates selected 42.

The Go tool calls these files *package-local metadata*. A changed path is
package-local metadata when it lies in a package whose source mode is package
prefix (a shell `BUILD`, or a Starlark `BUILD` that declares no `srcs`), Git
reports it as **added or modified** (`A` or `M` in
`git diff --no-renames --name-status`), and its package-relative path is
exactly one of:

| Package-relative path | Read by | Why no dependent can read it |
|---|---|---|
| `CHANGELOG.md` | repository changelog checks in `code/scripts`; packaging file lists (gemspec `files`, `.cabal` `extra-doc-files`, Perl `MANIFEST`, npm `files`) | the checks are scheduled by changed path, not by package; packaging needs the file only to *exist* when a dependent installs the package from its path, and an addition or edit cannot remove it |
| `CHANGELOG.d/<name>.md` (direct children only) | the same changelog checks, and the package's own doc-shard check (for example `typescript/script-ductus`'s BUILD checks only its own shards) | same |
| `required_capabilities.json` | the package's own analyzer, cage-drift and manifest tests; packaging file lists | the generated cage (`gen_capabilities.go`, `src/capabilities.rs`) is source, so regenerating it still propagates; no tool composes a dependency's manifest into a dependent's |
| `effect_certificate.json` | the package's own `ca-capability-analyzer --verify-cert` step | only `rust/ca-assurance-report`, `rust/ct-compare` and `rust/gf256` have one, and each verifies its own |
| `assurance_report.json` | the package's own `ca-assurance-report --check` step, and `code/scripts/assumption_catalogue.py` | the script is scheduled by path |
| `evidence/**` ending in `.json`, `.cnf` or `.drat` | the package's own warrant evidence steps | same; these are the only extensions under any `evidence/` today |

`README.md` is **not** on the list. A Python dependent's BUILD runs
`pip install -e ../<dep>`, and hatchling then reads the dependency's
`readme = "README.md"`: deleting `python/html-renderer/README.md` breaks
`python/programs/pipeline-visualizer`'s install. Packaging reads a README for
the dependent too, not only while building the package itself, so README
changes propagate as before.

**Deletions, renames and type changes always propagate.** Packaging that
lists a file (above) fails when the file is missing, in the dependent's build
as well as the owner's. So only `A` and `M` can be local; `D`, `T` (for
example a file replaced by a symlink) and any status the tool cannot read are
ordinary changes. Renames need no special case: with `--no-renames` a rename
is the deletion of its old path plus the addition of its new one, and the
deletion propagates. A path whose status is unknown propagates.

Matching is exact and case-sensitive, applies only at the package root, and
uses raw Git paths. It is closed by use as well as by name: `CHANGELOG.d/`
admits only direct `*.md` shards and `evidence/` only `.json`, `.cnf` and
`.drat` data, so a script, source file, nested shard or anything else placed
there is ordinary source. `README.md`, `docs/CHANGELOG.md`,
`sub/required_capabilities.json`, `x/evidence/y.json`, `changelog.md`,
`CHANGELOG.md.bak`, a *file* named `CHANGELOG.d` or `evidence`, and
`evidence.bindings` are ordinary source. Nested names matter:
`rust/html-lexer` reads `rust/html-parser/tests/fixtures/README.md`, and
`rust/warrant-grammar` and `rust/warrant-ir` read
`rust/gf256/evidence.bindings`.

The other half of "closed by use" is enforced by `-validate-build-files`
(`internal/validator/metadata_exec.go`), which runs in the detect job of
every pull request: no tracked BUILD front, shell/PowerShell/batch script,
Cargo `path`/`build` value, Rust `#[path]` or Rust `include!` under `code/`
may execute a metadata-class location (any `CHANGELOG.d/` or `evidence/`
descendant, or any `CHANGELOG.md`, `required_capabilities.json`,
`effect_certificate.json` or `assurance_report.json`). Reading those files as
data (`include_str!`, `--check assurance_report.json`) is allowed.

That check is a best-effort tripwire for honest mistakes, not a security
boundary. It matches literal paths in a fixed set of syntactic forms, so
indirection gets past it: a variable (`F=evidence/x; sh $F`), a pipe or
redirect (`sh < evidence/x`), a wrapper (`bash -c`, `env`, `timeout`,
`make -f`, `go run`), `include!(concat!(env!(...), ...))`,
`#[cfg_attr(..., path = ...)]`, a `build.rs` that copies a metadata file, and
anything in `package.json` scripts, Makefiles or non-Rust sources. The type
restriction above is what removes the accidental case. What remains is a
deliberate two-step: a reviewed pull request (which propagates normally,
because the wiring itself is a source change) makes behaviour depend on an
allowed-type data file, and a later `A`/`M` edit of only that file is then
local. Review of the first step is the control; no instance exists in the
repository today.

Other packages do read some of these files, but none of them is a dependent
of the file's owner, so dependency propagation never selected them:
`rust/required-capabilities-compiler` reads
`rust/weather-agent-e2e/required_capabilities.json` (it is declared in the
repository source-input boundary), `rust/ca-assurance-report`'s tests read
`rust/gf256`'s evidence and reports, `rust/ca-capability-analyzer`'s tests
read `rust/ct-compare`, and `typescript/human-language-data`'s tests read
the `CHANGELOG.d/` shards of `typescript/script-ductus` and
`typescript/programs/language-ladder`, both of which depend on it. Before
adding a cross-package read of one of these files from a dependent, take the
file off the list.

A package whose **only** changes are package-local metadata is selected
like a repository input consumer: it and its prerequisites, not its
dependents, and it fires no Cargo dev-dependency test trigger, because its
library did not change. The package itself still rebuilds, since its own
BUILD may check the file it changed. Any other change in the same package
makes it an ordinary changed package again:

```
closure  = AffectedNodes(ordinary changed packages)
closure ∪= triggers[a]  for every a in closure
affected = closure + prerequisites
         ∪ repository input consumers + their prerequisites
         ∪ metadata-only packages     + their prerequisites
```

Both sets are computed per platform from the same platform-filtered path
list. A metadata change next to an edit of `BUILD_windows` therefore leaves
the package local on Linux and macOS (where that front is filtered out) and
makes it an ordinary root on Windows; the package is selected everywhere.

The rule does not apply to:

- strict-glob Starlark packages, which keep their declared-source semantics:
  a declared `CHANGELOG.md` propagates, an undeclared one selects nothing;
- packages seeded for an external fixture, forced packages, or a
  repository-boundary consumer of the file, which are ordinary changed
  packages;
- `-force`, and the hash-based fallback below, which stays conservative.

CI gates (`code/specs/ci-gate-registry.md`) see the narrower affected set. A
gate whose path patterns match the metadata file still fires; one that fired
only because a dependent of the owner was affected no longer does.

The rule is pinned by the `metadata_policy: "package_local"` variant of the
shared `diff_selection` contract (`code/specs/build-tool-conformance.md`,
section 4), which the corpus oracle and the Go tool replay. The contract
models changed paths, not Git statuses or platforms, so the deletion rule and
platform filtering are covered by the Go tool's own tests. Implementations
that do not offer the policy keep the original contract, in which every
package-prefix path propagates; that is conservative, never wrong.

### Hash-Based Fallback

When git diff is unavailable, the build system falls back to SHA256 hashing:

1. For each package, hash all source files (sorted by path for determinism).
2. Compute a "deps hash" by collecting hashes of all transitive dependencies.
3. Compare against a cache file (`.build-cache.json`).
4. If the package hash or deps hash changed, rebuild.

### Parallel Execution

The dependency graph is partitioned into independent groups — sets of packages with no dependencies between them. Groups are executed sequentially (respecting dependency order), but packages within each group run in parallel.

```
Level 0:  logic-gates  (no dependencies)
Level 1:  arithmetic, clock  (depend on logic-gates)
Level 2:  cpu-simulator  (depends on arithmetic + clock)
```

Parallelism is bounded by a configurable job count (default: number of CPU cores).

### Failure Propagation

If a package fails to build, all packages that transitively depend on it are marked "dep-skipped" and not executed. This avoids wasting time on builds that will definitely fail.

## CLI Interface

The implementations intentionally share the same overall CLI shape, but they
are not perfectly feature-identical. The Go tool is the reference behavior used
in CI. The portable parser subset is the bounded long-form grammar in
[`build-tool-conformance.md`](build-tool-conformance.md#12-cli-and-reporting).
That subset is a process-free contract: it normalizes supplied tokens and
deterministic null/default sentinels without consulting the host. Native tools
may retain additional spellings, but portable conformance is measured only
against that closed grammar.

```
build-tool [flags]

Flags:
  -root <path>          Repository root (auto-detect from .git if omitted)
  -diff-base <ref>      Git ref to diff against (default: origin/main)
  -force                Rebuild everything regardless of cache
  -dry-run              Show what would build without executing
  -jobs <N>             Max parallel workers (default: CPU count)
  -language <lang>      Filter: implementation-dependent; Go supports python, ruby, go, rust, typescript, elixir, all
  -cache-file <path>    Path to cache file (default: .build-cache.json)
```

The portable form uses `--root`, `--diff-base`, `--force`, `--dry-run`,
`--jobs`, `--language`, `--cache-file`, `--validate-build-files`,
`--no-validate-build-files`, `--detect-languages`, `--emit-plan`,
`--plan-file`, `--shard-count`, `--shard-index`, `--emit-shard-matrix`, and
`--clippy`. Reserved adapter flags and shell, environment, redirection,
command-substitution, and response-file syntax are data errors; parsers must
not expand or execute them.

## Implementations

| Status | Languages | Notes |
|---|---|---|
| Operational front doors | C#, Elixir, F#, Go, Haskell, Lua, Perl, Python, Ruby, Rust, Swift, TypeScript | Behavior and maturity vary; Go is the CI reference. |
| Missing established front doors | Dart, Java, Kotlin | Required for final parity. |
| Emerging/future | C, C++, OCaml | Graduation requires an explicit applicability decision and conformance. |

The exact implementation inventory and remediation order live in
`package-parity-roadmap.md`.

## Migration Note

The build system previously used DIRS files to route directory traversal. DIRS files are plain text files listing subdirectories to descend into. That mechanism has been replaced by recursive BUILD file discovery because DIRS files create merge conflicts when multiple contributors add packages in parallel. Any remaining DIRS files are legacy and ignored.
