# Build Tool (Go)

The **primary build tool** for the coding-adventures monorepo. Compiled to a native binary for fast, dependency-free CI execution.

Repository-input fixture relations use the exact package identities emitted by
discovery. Names preserve the package directory basename: for example,
`code/packages/ruby/unix_history_simulator` is registered as
`ruby/unix_history_simulator`. The same identity is used for fixture selection
and for the package's fallback cache inputs.

Changes to the shared native Perl Windows helper or the Rust `perl-bridge`
manifest, build script and source directory also select Conduit, IRC Server
Native and Conduit Hello for all/Perl builds. These external inputs must still
run the real XS consumers when no Perl package file changed. Missing consumers
fail closed; other language filters and lookalike helper paths stay separate.
The native recipes explicitly request Rust, and their shared helper also enables
the Windows native-suite gate, including for helper-only edits.

Changes to the exact shared
`code/specs/fixtures/build-tool-v1/cases/discovery-duplicate-identity.json`
fixture schedule its direct Elixir, Go, Ruby, Rust, Swift, and TypeScript
build-tool readers on every platform. The relation is case- and path-exact,
honors explicit language selection, unions ordinary package changes, and keeps
normal prerequisite closure. Missing applicable readers fail before a plan is
written; nested, renamed-target, backslash, backup, and other lookalike paths
do not acquire the relation. A rename or deletion still schedules readers from
the changed old path.

Changes to the exact `code/specs/fixtures/starlark-opcode-provider-v1/cases.json`
path also schedule the committed `go/starlark-compiler` native conformance reader
on every platform for all/Go invocations. Nearby schema/docs paths and other
language filters do not acquire that relation. Missing required readers fail
before a plan is written; ordinary affected-package propagation is preserved.

The exact `code/specs/fixtures/compiler-ir-v1/cases.json` path similarly seeds
the real `go/compiler-ir` BF03 native reader for all/Go selections, including
deleted/renamed old paths. Normal dependency propagation selects its consumers.
Other language filters and nearby schema/docs/lookalike paths do not gain that
relation, and a missing Go reader fails before plan output is replaced.

Changes to the exact `code/specs/gfm-fenced-block-v1/cases.json` or
`code/specs/fixtures/gfm-extension-v1/cases.json` path seed the TypeScript
`gfm-parser` native readers for all/TypeScript selections on all three
platforms, including deleted or renamed old paths. Normal dependency
propagation schedules their consumers. Nearby files and unrelated language
filters do not gain this relation; an absent or wrong-language reader fails
before plan output is created or replaced.

## What it does

This tool discovers packages in the monorepo by recursively walking for `BUILD` files, resolves inter-package dependencies, hashes source files for change detection, and only rebuilds packages whose source or dependency inputs changed. Independent packages are built in parallel using Go goroutines.

Changes to the exact shared language-registry discovery fixture schedule all
eleven native build-tool consumers, including Haskell, on every platform; a
source-reference check guards the selector map against new direct adopters.

Changes to the flat shared `graph-*.json` and `diff-selection-*.json`
conformance cases also schedule their thirteen direct native readers on each
platform. This fixture-only CI selection is bounded by the active language
filter, fails if an applicable reader root is missing, and does not force a
full build. Java, Kotlin, Dart, and OCaml readers are process-free domain
cores; the selection does not imply full build-tool adapter parity.

Changes to a flat shared `resolution-*.json` case schedule only its checked
direct native readers. The twenty-six cases have different reader subsets
across Go, Haskell, Lua, Perl, Python, Ruby, Rust, Swift, and TypeScript;
new cases must be classified before CI can plan them. This selection remains
unforced, honors the language filter, and fails if an applicable native root
is missing. The [CI-gate registry](../../../specs/ci-gate-registry.md)
contains the exact case-to-reader relation.

Changes to one of the eleven flat shared `hashing-cache-*.json` cases
schedule its direct native test readers without forcing all build tools.
C#, F#, and Lua read every case; Python reads missing, hit, and corrupt;
Go, Perl, and Swift read missing; Ruby reads missing plus the three dependency
adversarial cases; TypeScript reads corrupt. Newly added flat cases require
explicit classification before planning. The selector
keeps ordinary changed packages, works on all three platform plans and
single-language runs, and fails if an applicable reader root is missing.
Some native tests assert only digest slices; this scheduling rule does not
claim they replay the full neutral cache-decision oracle.

Changes to a flat shared `validation-*.json` case schedule only its direct
native validator readers alongside the neutral fixture gate. Nine checked
orphan-crate/tracked-artifact cases have eleven non-Go readers, five
package-root/Lua-Windows-sibling cases have a Go reader, and eight cases are
neutral-only. All twenty-two case names are classified explicitly: a newly
added flat case fails before planning until its reader relation is reviewed.
Raw-path matching excludes nested, backslash, case-varied, and backup
lookalikes; renamed/deleted source paths, ordinary package edits, three
platforms, and single-language plans retain exact selection. The
[CI-gate registry](../../../specs/ci-gate-registry.md) pins the relation.

Changes to a flat shared `plan-*.json` case similarly schedule only its
direct native reader. Python reads `replace-existing`, TypeScript reads
`portable-package-path`, and the other three checked cases are neutral-only.
An unclassified flat case fails before planning; exact raw-path matching,
language filters, and missing-root checks keep the affected plan bounded.
The change detector uses NUL-delimited Git paths, preserving non-ASCII names
without quote-path escaping before fixture classification.
The [CI-gate registry](../../../specs/ci-gate-registry.md) pins this relation.

## Rust fixed-point cache inputs

The `rust/fixed-point` package adds an exact repository-input relation for its Q16.16 Warrant model and named cases. The same six paths are used for change selection and package hashing: the root and Rust-workspace Cargo target configs, the workspace `Cargo.toml` and `Cargo.lock`, `code/specs/fixed-point.warrant`, and `code/specs/fixtures/fixed-point-q16-v1/cases.json`. Shared Cargo metadata also selects the existing Rust game-clock consumer and the two test-only fixed-point verifier consumers, `rust/ca-assurance-report` and `rust/warrant-ir`.

The report verifier reads the fixed-point `src/` and `evidence/` trees, package `Cargo.toml`, `effect_certificate.json`, `evidence.bindings`, `assurance-profile.json`, the Warrant spec, and the existing Warrant lane inputs. The WIR proof test reads the `src/` tree, `evidence.bindings`, Warrant spec, and package `Cargo.toml` to validate the default library target and `package.autolib`. Both hash the exact Rust workspace target configs, manifest and lockfile because their BUILD recipes invoke Cargo. These readers are registered as raw data inputs; they do not create Cargo graph edges back to fixed-point.

An exact registered directory is recursively hashed with a tree-specific domain, sorted repository paths, file-kind markers, lengths and raw bytes. File and tree kinds come from the trusted registration; changing a registered root from one kind to the other fails closed. Additions, removals and same-length edits change persisted cache identity, including new untracked files inside a consumed tree; untracked files outside registered roots remain excluded. The traversal rejects linked ancestors, links and special files, reads directory entries in bounded pages, and enforces depth, entry, per-file and cumulative actual-byte limits. It checks opened-file and final-path identities so a replacement or mutation during hashing cannot be cached. The build-tool tests these consumers with isolated repository copies, cache hit/miss/restore probes, and Rust-filtered plan emit/replay.

## Portable source hashing

Extension and declared-source collection share the language-neutral v1 rules.
The Go implementation embeds a generated, strictly decoded projection of the
complete checked 23-language source-input registry. Production code never
reads the neutral fixture at runtime. Its seven role families distinguish
recursive sources and metadata, root-only exact and variable manifests, fixed
relative inputs, path-scoped companions, and exact package resources. Both
modes also include the five exact BUILD fronts and the universal root-only
`required_capabilities.json` input.

Unknown language keys fail before the package root is inspected. Exact package
inputs bind to their complete canonical `code/packages/<language>/<package>` or
`code/programs/<language>/<package>` root, so the Engram WASM resources cannot
widen to a sibling Rust package. The only nonstandard roots are the exact
registered `code/sites/blog` and `code/sites/landing-page` TypeScript profiles;
they are derived only from exact paths relative to the nearest real Git
repository root, so arbitrary, nested, and outside site roots fail before
traversal. Their eight and five authored
inputs are exact paths, so Markdown, identity JSON, image, and root CSS suffixes
do not widen to other packages. The exact case-sensitive 27-component
generated-directory registry is applied before every selector while preserving
near names such as `_Build`, `Blib`, `blib-example`, and `_build-example`; symlink and Windows
reparse-point components are never traversed.

Regenerate the embedded projection deterministically from the repository root:

```powershell
pwsh code/programs/go/build-tool/tools/generate-language-source-input-registry.ps1
```

The generator requires PowerShell Core (`pwsh`); Windows PowerShell 5.1 is not
supported because it does not provide the strict JSON APIs used to reject
duplicate keys before generation.

Package digests use hashing v1. Included files sort by normalized
repository-relative forward-slash UTF-8 path. Each path and its exact raw file
bytes are framed with unsigned 64-bit big-endian byte lengths before one
SHA-256 digest is computed. Absolute checkout paths, host locale, timestamps,
ownership, and directory enumeration order do not enter the digest, while a
same-content rename does. Stable lexical link/reparse checks run before and
after streaming and source identity/length is rechecked through the open file;
this is deliberately not described as an atomic descriptor-relative TOCTOU
boundary on every supported host.

Unreadable, unstable, non-portable, linked, or reparse-backed source inputs do
not produce a cache digest. `HashPackage` returns a checked error, and the CLI
emits the root-redacted `Error: HASH_PACKAGE_FAILED "<package-identity>"`
record with Go-escaped control characters and exits `2` instead of caching a
sentinel digest or printing an uncontrolled stack trace. Executor regressions
exercise hostile FIFO manifests on POSIX hosts; Windows builds use a
build-tagged helper and explicitly skip only that unavailable POSIX primitive.

## Process-free graph and diff selection

`internal/graphdiff` is the typed, host-authority-free core for graph planning
and diff selection. It consumes only caller-supplied immutable package, edge,
path, glob, and repository-boundary values; it does not read a checkout, invoke
Git, inspect the environment, launch a process, or use the network. The core
returns canonical prerequisite-first levels, exact dependent and prerequisite
closures, and stable empty-output diagnostics for graph cycles, unknown paths,
and match-work exhaustion.

The package-local suite dynamically enumerates the exact eight graph and twelve
diff-selection cases in the language-neutral v1 corpus. It covers the five
recursive BUILD fronts, strict character classes, repository-boundary reverse
selection and digest verification, both unknown-path policies, and the fixed
50,000,000-unit Unicode-scalar preflight. Structural and boundary validation
complete before the preflight; the preflight completes before any glob call or
unknown-path decision. This module is a native domain core and does not claim a
neutral adapter or any additional execution authority.

## Bounded CI gate selection

CI-gate evaluation remains process-free after the registry and change
snapshots are loaded. For ordinary pull-request snapshots, the evaluator first
extracts the literal leading and trailing path segments around each distinct
glob's wildcard region. Every distinct pattern/file pair pays for that bounded
segment filter; only pairs whose literal bounds can match pay the full
`(pattern scalars + 1) * (path scalars + 1)` matcher-grid cost. The combined
operation has a fixed 50,000,000 Unicode-scalar work-unit ceiling and completes
before package intersection or the first glob call. Exactly the ceiling is
accepted. A larger or overflowing operation returns
`CI_GATE_MATCH_LIMIT_EXCEEDED`; the front door exits before writing a build
plan or any partial gate outputs. Force, missing snapshots, and recognized
gating-machinery changes retain their all-gates-true bypass.

The native loader applies the neutral registry bounds first: at most 128 gates,
4,096 unique package names and path globs per gate, 512 Unicode scalars per
portable NFC glob, and collision-free gate outputs. Invalid registries and
globs therefore fail before a run-all bypass or ceiling result can hide them.

The shared globstar matcher memoizes each `(pattern index, path index)` state
for one call, including failed states. Its `**` recurrence consumes either zero
segments or one segment, which bounds recursive near misses by the dynamic-
programming state grid without retaining attacker-shaped data across calls.
Within a segment it implements the neutral Python `fnmatchcase` class grammar,
including leading-`!` negation, literal `^`, leading `]`, leading/trailing
hyphens, ascending ranges, and unmatched `[` literals.

## Building

```bash
cd code/programs/go/build-tool
go build -o build-tool .
```

On Windows, build the executable with the `.exe` suffix so PowerShell runs it
directly instead of asking which application should open an extensionless file:

```powershell
cd code\programs\go\build-tool
go build -o ..\..\..\..\build-tool.exe .
```

This produces a single static binary with no runtime dependencies.

## Usage

```bash
# Auto-detect repo root, build all changed packages
./build-tool

# Specify root explicitly
./build-tool -root /path/to/repo

# Rebuild everything regardless of cache
./build-tool -force

# Show what would build without actually building
./build-tool -dry-run

# Limit parallel workers
./build-tool -jobs 4

# Only build Python packages
./build-tool -language python

# Custom git diff base
./build-tool -diff-base origin/develop

# Custom cache file location
./build-tool -cache-file /tmp/my-cache.json
```

On Windows, use the compiled `.exe`:

```powershell
.\build-tool.exe -root . -diff-base origin/main
.\build-tool.exe -root . -validate-build-files -detect-languages -emit-plan build-plan.json
```

## CLI flags

| Flag | Default | Description |
|------|---------|-------------|
| `-root` | auto-detect | Repo root directory (walks up looking for .git) |
| `-force` | false | Rebuild everything regardless of cache |
| `-dry-run` | false | Show what would build without executing |
| `-jobs` | NumCPU | Maximum parallel build jobs |
| `-language` | all | Filter to any canonical discovery bucket, or `all` |
| `-plan-closed-language` | (none) | Only with `-emit-plan`: emit just the packages of this language whose transitive prerequisites are all of this language (a package with a foreign prerequisite is left out, not pulled in; `all` is refused). `-language` keeps its meaning; see `plan_language.go` |
| `-diff-base` | origin/main | Git ref to diff against for change detection |
| `-cache-file` | .build-cache.json | Path to the build cache file |
| `-validate-build-files` | true | Validate BUILD dependency metadata, established-lane package-root coverage, crate coverage, tracked artifacts, BUILD-front invocations, and negative-control checker users declaring Python |
| `-ci-gates` | code/specs/data/ci-gates.json | CI gate registry used to decide which Actions jobs this change needs; empty disables gating |
| `-shard-count` | 0 | Split an emitted plan (`-emit-plan`) into this many prerequisite-closed CI shards |
| `-shard-weights` | (none) | Measured per-package seconds (`ci-package-durations-v1`); selects locality-aware sharding v2 (see below) |
| `-shard-index` | -1 | With `-plan-file`, run only that shard |
| `-prune-leaf-targets` | false | Delete a leaf package's own Cargo `target/` once it has built (CI disk budget) |
| `-emit-kiln-graph` | (none) | Write the `kiln-legacy-graph/1` export (KILN03 section 2) to this path and exit: every package, all three OSes, implies `-force` and `-language all`. Go-only; the other ports do not implement it |
| `-emit-kiln-steps` | (none) | Only with `-emit-kiln-graph`: also write the `kiln-legacy-steps/1` export (per package, the lines `executor.EffectiveStepLines` returns for this OS) from the same discovery pass; KILN04 section 4.8 |
| `-exec-wrapper` | (none) | JSON array of strings: a prefix for every BUILD step, run as `wrapper... sh -c <line>`. The wrapper (not the BUILD) gets `KILN_ACTION`, `KILN_STEP`, `KILN_STEP_COUNT`. Incompatible with `-clippy` and `-prune-leaf-targets` |
| `-command-timeout` | `90m` | Kill any single BUILD command that runs longer than this (Go duration such as `45m`; `0` disables; negative is an error). See [Command timeout](#command-timeout) |
| `-emit-results` | (none) | After the run, write the `kiln-legacy-results/1` record (status and step counts per package) to this path |

### Command timeout

Every BUILD command runs under a timer (`-command-timeout`, default 90 minutes,
`0` to disable). A command that outlives it has its whole process tree killed
(a process group on Unix, `taskkill /T /F` on Windows), the wait for its output
pipes is bounded so a leftover grandchild cannot hold the build, and the package
FAILS with return code 124. The package's output ends with a line like:

```
rust/lzw: command 2 of 3 `cargo test` timed out after 1h30m0s (limit 1h30m0s); killed its process tree
```

All output buffered before the hang is printed as for any failure. The clock is
per command (it restarts for each BUILD line), and there are no per-package
overrides. The default exists because a single hung Windows command once idled a
CI job for 4h47m, with no log of which command it was. Because each command runs in its own process group, Ctrl-C (SIGINT) and SIGTERM
are caught by the tool and kill every running command's tree; the run then
exits 130 and reports it was interrupted. A command that exits cleanly but
leaves a background process holding its output fails, and that leftover group
is killed. On Windows the tree kill needs `taskkill`; if it is missing from
PATH the tool falls back to killing only the shell, so descendants may linger
(the wait for their pipes is still bounded). See
`code/specs/12-build-system.md`, "Per-Command Timeout".

### Kiln legacy graph export

`-emit-kiln-graph out.json` hands the repository's language knowledge to Kiln
as plain data, so Kiln's core never has to parse a manifest. For every
package (including ones that only have a BUILD file for another OS) it lists
the selected BUILD file per OS (or `null`), direct dependencies per OS,
`dev_deps` (the Cargo test-trigger relation, inverted), `repo_inputs` (every
external input the change selector uses: the repository-input consumer table,
the native fixture readers, the external corpora and the CV02 spec; built in
`external_inputs.go` from the selector's own tables so they cannot drift) and `extra_toolchains`.
Lists are sorted and de-duplicated, so output is byte-identical for a tree.
See `kiln_graph.go` for the field-by-field contract and
`testdata/kiln-legacy-graph.golden.json` for the pinned format. `repo_input_globs`
holds pattern-valued inputs (`**` any depth, `*` within a segment), e.g. the
Mosaic guard's `code/**/?*.mll`; `repo_inputs_unknown` is set only for a
consumer that declares neither (none today).

The CLI fails closed: it exits non-zero if no packages are found, if a
selector relation names a package discovery did not find, if a package path
cannot be made repo-relative, or if the output path is a symlink. The file is
written atomically (temp file, fsync, rename).

### Kiln tracing hooks

Kiln traces legacy BUILD steps (KILN04 section 4.3) by running this tool with
two flags.

`-exec-wrapper '["kiln","sandbox-exec","--"]'` puts the wrapper argv in front
of every step: `kiln sandbox-exec -- sh -c <line>` (`cmd /C <line>` on
Windows). The value is a JSON array of non-empty strings; anything else is a
usage error, as is combining it with `-clippy` or `-prune-leaf-targets`
(recording needs both off). The BUILD line is passed through unchanged. The
wrapper process receives `KILN_ACTION` (package path relative to the repo
root), `KILN_STEP` (0-based) and `KILN_STEP_COUNT`; these are for the
wrapper, which must scrub them before the step runs, and a BUILD must never
read them. Without the flag nothing changes.

`-emit-results out.json` writes, atomically (temp file, fsync, rename; a
symlink path is refused), after the run and even if a package failed:

```json
{"schema":"kiln-legacy-results/1","packages":[{"package":"code/packages/go/demo","status":"built","steps_run":3,"steps_total":3}]}
```

`status` is the executor's own status (`built`, `failed`, `skipped`,
`dep-skipped`, `unsupported`, `dep-unsupported`, `would-build`). `steps_run`
counts steps started, so a failure in step 2 of 3 reports 2 of 3; a package
that never ran reports 0 of its BUILD's step count. Packages are sorted by
path. It works with or without `-exec-wrapper`.

### Sharding v1 and v2

Main CI splits its forced full build across 15 runners. Each shard must be
*prerequisite-closed* -- it builds everything its packages depend on -- so
any prerequisite two shards share is built twice. Sharding v1 deals packages
round the shards by a heuristic cost ("a Rust crate costs 6") and only then
copies each one's prerequisites in, which on the real repo multiplied the
work by 2.8x. Sharding v2 (`-shard-weights`) hands out whole dependency
closures, biggest first, to the shard that gains the least *new* work while
staying within a fair share, and costs packages in measured seconds:

| 2026-10-18 main plan, 15 shards | package builds | heaviest shard | lightest shard |
|---|---:|---:|---:|
| v1 | 14,878 | 233 min | 134 min |
| v2 | 5,908 | 69 min | 69 min |

Refresh the durations from finished main runs with
`python3 code/scripts/ci_package_durations.py --runs <run ids>`. A package
the file has not seen yet costs its toolchain's median, so a new package
never needs a refresh to be scheduled sensibly. See
`internal/plan/shard_weights.go` for the algorithm and
`code/specs/build-plan-sharding.md` for the contract.

For strict Starlark packages with declared sources, diff selection recognizes
only the exact recursive build fronts `BUILD`, `BUILD_windows`, `BUILD_mac`,
`BUILD_linux`, and `BUILD_mac_and_linux`. Near names such as `BUILD_debug` and
`BUILD.bak` are known in-package paths but do not select the package unless a
declared source glob matches.

`-validate-build-files` also fails closed when Git tracks any `node_modules`
path. Dependency directories are machine-local build products; committing one
can hide an absolute symlink that works only in its author's checkout.

The same gate streams direct roots under each of the fifteen established
`code/packages/<language>/` lanes. A root is eligible only when the governed
language-source registry supplies a compact source witness; exact generated
components and reviewed virtual roots stay excluded. A runnable one of the
five canonical BUILD fronts in the root or a component-wise ancestor covers
it. Remaining reviewed gaps must carry a reasoned `EXCLUDED` or `PENDING`
entry in `code/BUILD-EXEMPTIONS`, and stale entries fail the gate. Discovery is
bounded before retention and never shells out to Git.

## Deciding which CI jobs to run

Emitting a plan also decides which GitHub Actions jobs the change needs. The
tool reads the registry named by `-ci-gates`, intersects each gate's declared
packages with the affected closure and its declared path globs with the raw
changed-file list, then writes the verdicts into the plan as `ci_jobs` and emits
`run_<gate>=true|false` to stdout and `$GITHUB_OUTPUT`. `ci.yml` uses those as
job-level `if:` conditions.

Evaluation fails open. Force mode, an unavailable git diff, or a change to
`ci.yml` or the registry itself runs every gate; main-branch pushes already pass
`-force`. A gate declaring neither packages nor paths is rejected at load time,
because a gate that can never fire looks exactly like one that always passes.
The package-local suite also consumes every process-free
`ci-gate-selection-*.json` fixture from the shared build-tool v1 corpus, which
is the portable oracle for other implementation lanes.

Every gate needs both clauses. Files under `code/specs`, `code/fixtures`,
`code/grammars`, and `code/scripts` map to no package at all, so the path clause
is the only thing that can see a fixture or grammar edit. See
`code/specs/ci-gate-registry.md`.

The `build-windows-os-suites` step gate includes `rust/programs/closurec`
and its CV02 acceptance specification. Rust toolchain selection alone does not
run ordinary Windows package tests; this gate makes the compiler's native
publication/ACL regressions execute for compiler or affected dependency changes.
The exact CV02 specification path also selects `rust/programs/closurec` on all
three platforms and requires Rust, including specification-only diffs. Planning
fails if that consumer is missing; explicit non-Rust language filters skip it.
Emitted-plan regressions check the actual test command, affected compiler,
toolchain and Windows gate together without forcing unrelated packages.
Acceptance still requires a non-skipped test step and successful package result.

The shared `discovery/language-registry` case needs an additional, bounded
package-selection rule: changing its exact JSON file schedules each native
build-tool test consumer (the two .NET fronts plus Elixir, Go, Haskell, Lua,
Perl, Python, Ruby, Rust, Swift, and TypeScript) in the ordinary affected build plan. The rule
unites those roots with other changed packages on every platform; it does not
force a repository-wide build or apply to neighboring fixtures. A missing
registered consumer is an error rather than a silently skipped test. Native
fixture adopters must extend the consumer map and its drift test together.
An explicit `-language` filter selects only the matching consumer; default
all-language CI validates all twelve. Rename detection retains both the old and
new paths so moving the fixture cannot silently bypass this rule.

The flat `ci-gate-selection-*.json` case family separately seeds the C#,
F#, Go, and Python build-tool fronts. Their BUILD commands run the native
CI-gate fixture suites independently. This exact relation
applies to fixture additions, edits, deletions, and renamed old paths on all
platforms without a forced full build. Other fixture domains, nested paths,
and filename lookalikes do not select these roots. Missing applicable roots
fail planning; a single-language run selects only its own consumer. Detector
tests check both the emitted affected roots/toolchains and reader-map drift.

The flat `test-trigger-selection-*.json` case family independently seeds the
Go and Rust build-tool fronts, which are its only current native readers. The
relation accepts exact flat JSON case names, including deleted and renamed old
paths, while rejecting empty stems, nested paths, backslash spellings, case
variants, extensionless names, and backup suffixes. It unions those roots with
ordinary changed packages on Linux, macOS, and Windows before prerequisite
closure. Explicit language plans select only their applicable reader, and a
missing applicable reader aborts planning before any partial plan is written.
A source-reference drift test keeps the two-reader roster synchronized with
the native conformance suites.

The flat `toolchain-detection-*.json` case family likewise seeds its twelve
direct native test fronts: C#, F#, Elixir, Go, Haskell, Lua, Perl, Python,
Ruby, Rust, Swift, and TypeScript. Its exact path check includes deleted or
renamed old sources, but excludes nested, backup-suffixed, case-varied, and
sibling fixture paths. Each platform plan retains these package roots before
dependency closure, while explicit single-language plans seed only their
applicable reader. A missing reader fails before a partial plan is written;
unforced plans expose all selected roots and their eleven canonical toolchain
flags (C# and F# share `dotnet`). A source-reference test guards map drift.

The flat `source-collection-*.json` cases have three separate native reader
sets. The seven package-local cases seed C#, F#, Elixir, Go, Haskell, Lua,
Perl, Python, Ruby, Rust, Swift, and TypeScript. The thirteen `repository-` cases
seed only C#, F#, and Swift; the four `shared-input-` cases seed only C# and
F#. C# and F# enumerate every checked case, while other fronts execute only
their declared subfamilies or subsets. This exact relation schedules native
BUILD tests for fixture-only additions, edits, deletions, and renamed old
paths on detect and all three platform plans. It rejects nested and filename
lookalikes, keeps ordinary changed roots, honors explicit language filters,
and fails if an applicable native reader is absent before writing a plan.
Focused tests verify unforced affected roots and toolchain flags as well as
the direct test-source references; the neutral corpus gate remains separate.

## Metadata safety

A package's CI toolchain is normally inferred purely from its path bucket
(`packages/<language>/...`) — but a package whose own tests need a
*different* toolchain can declare it with a bare BUILD-file comment such as
`# needs-toolchain: python`. This is consulted alongside the inferred
language, not instead of it, and multiple lines are supported for a package
needing more than one extra toolchain. Only lowercase names from the canonical
CI registry are accepted. Empty, unknown, wrong-case, fused, or suffixed
lookalikes are inert; valid names retain first-occurrence order and are
deduplicated. BUILD content splits on LF, and only a CR immediately before
that LF is stripped. A final lone CR or CR before trailing ASCII whitespace
remains content, so those lookalikes are inert. Parsing is bounded to 65,536
bytes and 4,096 logical lines.

Only the platform-selected BUILD front contributes declarations:
`BUILD_windows`; `BUILD_mac` or `BUILD_linux`; then the shared
`BUILD_mac_and_linux` on Unix; then generic `BUILD`. A language-neutral
in-memory fixture suite exercises that same precedence and the affected-only
and forced-full decisions without reading a checkout or probing host tools.

Build plans carry optional platform-specific dependency graphs and affected
closures. The detect job resolves Linux, Darwin, and Windows BUILD metadata,
unions their required toolchains, and assigns shared shards from the union so
platform-only work cannot disappear from the matrix. Each build runner then
selects only its own graph and affected set. A Windows-only native prerequisite
is therefore installed and ordered on Windows without causing unrelated Unix
packages to build. Older v1 plans without platform overrides continue to use
the top-level graph and affected set. Deleting a selected platform override is
also a platform-scoped change: that runner schedules the package so the newly
selected canonical fallback cannot escape validation.

### Machine-readable platform exceptions

A platform-specific BUILD front that cannot exist on that target must not use
a free-form `echo` and then appear as `BUILT`. Reviewed exceptions use one
closed command:

```text
echo BUILD_TOOL_UNSUPPORTED:STABLE_DIAGNOSTIC_CODE -- skipped
```

The executor recognizes the complete command as data and does not run a shell.
The package is reported as `UNSUPPORTED` with the exact reason code and is not
written to the success cache. A dependent that does not declare its own
reviewed exception is `DEP-UNSUPPORTED` with `DEPENDENCY_UNSUPPORTED`. Extra
commands, shell operators, lowercase codes, and free-form skip prose do not
match the protocol. Repository-specific exception manifests additionally bind
each authorized package path to its code; the command alone is not permission
to add a new exception.

Text package metadata is decoded according to the language-neutral build-tool
contract. In particular, Lua `.rockspec` files must be strict UTF-8. Invalid
bytes stop resolution with `METADATA_INVALID_UTF8`, identify the package and
repository-relative manifest, and return CLI exit code 2 without exposing the
checkout path or silently replacing input.

Lua standalone-build validation follows the shared language-neutral fixture
contract. When a canonical `BUILD` installs repository-local sibling rocks,
an absent `BUILD_windows` is an empty Windows closure rather than a reason to
skip comparison. The validator reports every missing canonical sibling in
stable sorted order; an explicit Windows recipe may add transitive local rocks.

Cargo dependency resolution reads path dependencies from every table Cargo
compiles into the crate: `[dependencies]`, `[build-dependencies]` (and the
legacy `build_dependencies`), and the same tables under `[target.<cfg>.…]` for
any cfg, quoted (`[target.'cfg(any(unix, windows))'.dependencies]`) or a bare
triple — the union over all targets, because CI builds on several hosts. Both
entry forms count: an inline table (`name = { path = "..." }`, whose
`features` array may continue onto later lines) and a `[dependencies.name]`
sub-table with a `path = "..."` key. When a dependency uses a local source
alias together with `package = "published-name"`, the published package name
drives internal lookup. `[dev-dependencies]` in every form (they are test
triggers instead; see `internal/resolver/test_triggers.go` and "Cargo
dev-dependency test triggers" in `code/specs/12-build-system.md`), `[patch]`,
`[replace]`, `[workspace]`, package metadata, features, and registry-only
dependencies remain outside this graph contract. The exact line procedure is
the Rust paragraph of `code/specs/build-tool-conformance.md`, pinned by the
shared `resolution/rust-field-aware` case that this tool and the Haskell
front both read. Field and sub-table keys may be bare or quoted
(`"path" = "..."`). That procedure is lenient, so for every Rust and WASM
package the strict test-trigger reader also screens these tables and stops the
plan on any shape the procedure does not model (a dotted key such as
`a.path = "..."` or `build-dependencies.a.path = "..."`, an inline table
spelling a link table, `[dependencies.a.features]`, a multi-line string, a
backslash, ...). The edge reader carries its own limits (a regular file of at
most 1 MiB, one entry of at most 256 lines or 64 KiB) and joins continuation
lines in linear time. A dependency that resolves to the package itself is a
`DEPENDENCY_SELF_EDGE` diagnostic with exit status 2.

Ruby dependency resolution reads only `add_dependency` and
`add_runtime_dependency` calls on the gem specification receiver. Development
dependencies, metadata, and comments remain outside the graph. Declared gem
names are registered alongside directory-derived aliases, so valid
hyphen/underscore naming differences resolve deterministically. The shared
field-boundary fixture and complete 301-package lane resolve exactly 454 edges.

Perl dependency resolution reads only top-level runtime `requires`
declarations from each root `cpanfile`. Requirements inside test or other
phase blocks, `Makefile.PL` dependency tables, metadata, and comments remain
outside the graph. Exact `Makefile.PL` `NAME` values and current and legacy
distribution spellings are aliases only. The shared field-boundary fixture and
complete 256-package lane resolve exactly 217 total edges: 216 from
authoritative manifests and one qualified BUILD dependency.

Elixir dependency resolution reads local dependency tuples only from direct
project `deps:` lists and lists returned by block or shorthand `defp deps`
functions. Multiline tuples are accepted when they contain a quoted `path:`
option. Project and application metadata, source prose, comments, `mix.lock`,
and non-path Hex or Git dependencies cannot invent graph edges. The shared
field-boundary fixture covers direct, block, shorthand, multiline, comment,
metadata, and external-dependency cases; the complete 282-package lane resolves
exactly 472 edges.

Dart dependency resolution reads only direct package keys under the root
`dependencies:` and `dev_dependencies:` maps. Scalar constraints and nested
source maps are both valid direct entries, while nested `path:`, `git:`,
`url:`, `ref:`, and `sdk:` options, dependency overrides, comments, and
unrelated YAML fields cannot invent graph edges. Root `name:` values and
directory-derived snake-case aliases identify packages. The shared
field-boundary fixture covers these boundaries, and the complete 82-package
lane resolves exactly 67 edges.

TypeScript dependency resolution parses the root `package.json` and reads only
direct property names from `dependencies` and `devDependencies`. Exact
top-level `name` strings and directory-derived scoped or unscoped names supply
aliases. Dependency values are not followed, and peer or optional dependency
tables, scripts, nested tool configuration, nested names, and malformed JSON
cannot invent partial graph edges. The shared field-boundary fixture covers
single-line objects and adversarial decoys; the complete 470-package lane
resolves exactly 1,076 edges.

Java and Kotlin dependency resolution reads only `includeBuild("...")` calls
from root `settings.gradle.kts` files. Calls may span lines; their quoted
relative paths are normalized lexically and must exactly match a discovered
package root in the same language lane. Comments, unrelated strings,
`include(...)`, build-script coordinates, absolute paths, cross-lane paths,
and unknown targets cannot invent graph edges. Referenced targets are never
opened. Java/Kotlin source and Gradle settings/build files participate in the
package hash. The two shared field-boundary fixtures cover direct and nested
paths plus adversarial decoys; the complete Java and Kotlin lanes resolve
exactly 186 and 166 edges respectively.

OCaml dependency resolution is process-free. It reads the `depends` field from
one unambiguous root `.opam` file and top-level `libraries` fields from the
fixed `dune`, `src/dune`, `bin/dune`, and `test/dune` files. Comments, other
opam fields, Dune strings outside `libraries`, multiple root opam manifests,
unknown names, and self references cannot invent graph edges. Directory,
conventional repository, and declared opam names are aliases, with library
packages preferred over same-named programs. OCaml source, interface,
manifest, Dune, formatting, and BUILD files participate in hashing while
generated `_build` trees do not. Build plans expose `needs_ocaml`, validate
standalone prerequisite declarations, and charge the pinned OCaml toolchain
cost without executing opam or Dune.

C#, F#, and shared .NET dependency resolution reads only literal `Include`
attributes on unqualified `ProjectReference` start elements in root `.csproj`
and `.fsproj` files. Relative paths use portable separators, normalize
lexically, and must exactly match another discovered root project file in the
shared .NET scope. Comments, CDATA, processing instructions, escaped examples,
namespaced elements, package references, MSBuild properties, globs, absolute
paths, nested test projects, and unknown targets cannot invent graph edges;
referenced files are never opened. Three shared fixtures cover both language
lanes and cross-language references. The complete 198-package C# and
197-package F# lanes resolve exactly 238 and 239 edges respectively.

## Canonical discovery identities

Discovery uses only the exact bucket immediately below a `packages` or
`programs` path component. It recognizes every established implementation
lane, the emerging C, C++, and OCaml lanes, the WASM target, the Mosaic and
Twig domain languages, the Starlark build language, and the retained shared
`.NET` host bucket. Generated Dune `_build` trees are excluded before BUILD
membership, alongside the existing ecosystem output directories. Programs
keep a `programs` segment, such as
`go/programs/build-tool`, so they cannot collide with a library of the same
name. Specification fixture trees are not buildable packages.

Discovery fails closed when two directories normalize to the same identity:

```text
DUPLICATE_PACKAGE_IDENTITY: package=unknown/demo paths=code/packages/alpha/demo,code/packages/beta/demo
```

The diagnostic contains sorted repository-relative paths, never the checkout
root, and the CLI returns exit code `2`.

### Mosaic packages and the repository-wide Mosaic guards

Mosaic packages (`code/packages/mosaic/*`, `code/programs/mosaic/*`) are
ordinary graph nodes: the resolver reads the keys of the root
`[dependencies]` table of each `mosaic-package.toml` and resolves them,
within the Mosaic scope, by directory basename or `[package].name`. Editing
or bumping `mosaic-pkg-toolkit` therefore re-runs every Mosaic package and app
that requires it. Requirement strings, `[components]` exports and unrelated
tables such as `[host_assets].dependencies` never create edges. The reader
fails closed: it reads only a plain subset of TOML (one-part headers from a
fixed table list, bare keys and one-line strings in `[package]` and
`[dependencies]`), and anything else, such as `[package.meta]`,
`package.name = ...` at the root, a multi-line string, any backslash
escape), a symlinked or oversized manifest, and two Mosaic packages claiming
one name stop the plan with an error rather than dropping edges.

Some tests read the whole tree rather than their own directory. These
*repository input consumers* are listed in `repository_inputs.go` together
with the changed paths that select them. Today there is one:
`rust/mosaic-package-resolver`, whose `every_repository_requirement_is_satisfied`
and layout-binding guards walk every `mosaic-package.toml` and `.mll` under
`code/`. Any changed `mosaic-package.toml`, `.mil`, `.mll` or `.msl` under
`code/` selects it. A consumer is added to the affected set with its
prerequisites but **without its dependents**: its code did not change, only
the data it reads, and seeding it as a changed package would rebuild 72
packages (every emitter, the artifact builder, mosaic-compile, ...) for a
one-line layout edit. Selection uses raw Git paths (deletions count), honors
`-language`, and fails the plan if the consumer is missing from discovery.
It applies to Git-diff mode only; `-force` and the hash fallback already
build everything they consider changed.

`rust/warrant-grammar` is a second guard of this kind. Its `v0_compat` test
parses and checks every `.warrant` document in the repository, wherever it
lives, so any changed path ending in `.warrant` (the glob `**/?*.warrant`, a
bare `.warrant` excluded) selects it. A PR that adds an invalid document is
therefore checked by the PR that adds it, instead of failing a later unrelated
PR that rebuilds `warrant-grammar`. `*.warrant.draft` files are not documents
and do not select it. That guard selects only `rust/warrant-grammar`, with
its prerequisites; the Warrant adopters are selected by the per-crate rows
below.

A third guard is the crate's **own spec**. Every Rust crate whose BUILD runs
`gen_evidence -- --check <crate>` checks its committed evidence against
`code/specs/<crate>.warrant`, a file outside the package. An edit to exactly
that path (no lookalike, no `.warrant.draft`) selects `rust/<crate>`, in
addition to whatever else the spec already selected (the grammar guard above,
the corpus-v2 generator and oracle for the four corpus-v2 adopters, the lane
registry rows). Before this rule (CV2-F13) a spec-only edit selected at most
those consumers but not the crate itself, so the check first ran in some later
unrelated PR. The rows are nine inline `warrantEvidenceRow` entries in the
repository-input table (arithmetic, ct-compare, gf256, hash-functions,
logic-gates, lzw, wasm-leb128, x25519, zeroize), next to the four VOS crates,
which carry the same rule through `vosImplementationRow`.

`TestWarrantEvidenceRowsCoverEveryEvidenceCrate` keeps the rows complete. It
reads `code/packages/rust/*/evidence.bindings` and `BUILD*`, and requires a row
named `rust/<dir>` that matches `code/specs/<dir>.warrant` for every directory
with a bindings file or a `gen_evidence ... --check` line (argument order does
not matter); it also fails on a `warrantEvidenceRow` whose crate directory
exists but is no longer an adopter. Because that test reads those files, a PR
that adds an adopter must run it: the row `go/programs/build-tool` is selected
by exactly `code/packages/rust/<dir>/evidence.bindings` (one path segment). BUILD
edits are not a trigger, since that would select the build tool on every Rust
BUILD change; every adopter adds a bindings file anyway.

### Package-local files do not rebuild dependents

Adding or editing a package's root `CHANGELOG.md`, a direct
`CHANGELOG.d/*.md` shard, `required_capabilities.json`,
`effect_certificate.json`, `assurance_report.json` or `evidence/` data
(`.json`, `.cnf`, `.drat`) rebuilds that package (its BUILD may check the file)
and its prerequisites, but not its dependents, and it fires no Cargo
dev-dependency test trigger. Nothing a dependent builds or tests can read
those files; `internal/pkgmeta` lists them with the repository evidence for
each, and `package_metadata.go` wires them in exactly the way repository
input consumers are. Adding only a capability manifest and a changelog entry
to `rust/bounded-json` used to select 556 packages; it now selects 1.

The rule is deliberately narrow:

- `README.md` is not metadata: a Python dependent's `pip install -e ../<dep>`
  reads the dependency's `readme`.
- Only additions and in-place edits can be local. The tool reads
  `git diff --no-renames --name-status`; a deletion, a rename (a deletion
  plus an addition), a type change or an unknown status propagates, because
  packaging that lists a file needs it to exist in a dependent's build.
- Only exact, case-sensitive names at the package root count, closed by
  type: a script under `evidence/` or a nested `CHANGELOG.d/` shard is
  source. `-validate-build-files` also refuses any BUILD, script, Cargo
  `path`/`build`, Rust `#[path]` or `include!` that executes a metadata-class
  location (`internal/validator/metadata_exec.go`).
- Local and ordinary roots come from the same platform-filtered path list, so
  the package is selected everywhere. A metadata change beside a
  `BUILD_windows` edit is local on every platform when that edit is an
  in-place edit of a Rust package's front (below); otherwise the front is an
  ordinary change on Windows only.
- Strict-glob Starlark packages keep their declared-source semantics; any
  other change in the same package, fixture seeds, forced roots and
  repository-boundary consumers propagate; `-force` and the hash fallback are
  unchanged.

**A package's own BUILD front.** An in-place edit (Git status `M`) of the
`BUILD`, `BUILD_linux`, `BUILD_mac`, `BUILD_mac_and_linux` or `BUILD_windows`
at the root of a Rust package is package-local too: the package and its
prerequisites are rebuilt, its dependents are not. A dependent compiles a
Rust prerequisite from source with its own `cargo` invocation and never runs
or reads the prerequisite's BUILD, so nothing it builds can change. Moving
the negative-control lines of eight Rust hub crates to a shared checker used
to select 803 packages and time out the Windows job at 300 minutes; it now
selects 35.

The rule is narrow on purpose:

- **Rust only.** In other ecosystems dependents consume what a
  prerequisite's BUILD run leaves behind (`lua/lexer` needs the rocks its
  prerequisites' BUILDs install; `typescript/bitset` reads
  `../test-bootstrap/dist`, which that package's BUILD compiles). The gate is
  a closed list in `internal/pkgmeta` with the evidence for each entry.
- **Only `M`.** An added or deleted front changes which file a platform
  selects, or whether the directory is a package there, and propagates.
- **Only the front the platform selects**, at the package root, by exact
  name. A nested `BUILD` or `BUILD.bak` is source.
- **Shell BUILDs only.** A Starlark BUILD declares sources and dependencies
  and always propagates.
- **Not a BUILD another package runs.** If any BUILD names a BUILD front and
  this package's directory on one line (`(cd ../x && sh BUILD)`), edits to
  this package's fronts propagate. `-validate-build-files` refuses a BUILD
  invocation that names no package directory, and any script that runs a
  BUILD front.
- **Not across the platform-unsupported protocol.** A package with
  `BUILD_TOOL_UNSUPPORTED:` in any front, or a diff that adds or removes such
  a line (one extra `git diff -G` call), propagates: marking and un-marking
  change whether dependents run.
- Any other change in the package, fixture seeds, forced roots and
  repository input consumers propagate; `-force` and the hash fallback are
  unchanged. The edited BUILD is still checked by `-validate-build-files`,
  which runs over every package on every pull request.

`-validate-build-files` also fails when a `BUILD*` file directly in a package
directory discovered for any platform (so a `BUILD_windows`-only package is
covered on Linux; nested non-package BUILD files are not checked, as the build
tool never runs them) contains `warrant_negctl.py` (the shared negative-control checker,
plain substring match, comments included) but does not declare
`# needs-toolchain: python` for the platform it serves (`_windows` as
Windows, `_mac` as macOS, anything else as Linux). Unreadable files are reported, not skipped. CI installs Python only for
packages that declare it. The check lives in
`internal/validator/negctl_python.go` because a pull request that touches only a
package's BUILD never selects this program's tests.

What this does not cover: a dependent that relies on a toolchain only its
prerequisite's BUILD declares with `# needs-toolchain:` is not re-run when
that declaration is removed. The scheduled full builds (every six hours on
Linux, daily on macOS and Windows) are the backstop.

The neutral `metadata_policy: "package_local"` diff-selection variant
(`package-metadata-selection-*.json`) pins the path rules, including the root
BUILD fronts, and `internal/graphdiff` replays it.

## Architecture

The tool is organized into seven internal packages, each responsible for one phase of the build pipeline:

1. **discovery** -- Recursively walks for `BUILD` files to find packages
2. **resolver** -- Parses ecosystem metadata including `pyproject.toml`,
   `.gemspec`, `go.mod`, `Cargo.toml`, `package.json`, `mix.exs`, `.rockspec`,
   `pubspec.yaml`, Cabal files, Gradle files, .NET project references, and
   `mosaic-package.toml`
3. **hasher** -- SHA256 hashing for change detection
4. **cache** -- JSON-based build cache (read/write with atomic saves)
5. **executor** -- Parallel execution with goroutines + semaphore
6. **gitdiff** -- Git-based change detection for incremental builds
   (with **pkgmeta**, which names the package-local files (metadata, BUILD fronts) whose
   changes do not reach dependents)
7. **reporter** -- Terminal-friendly build report formatting

## Go concurrency advantage

The key advantage of the Go implementation over Python/Ruby is concurrency. Go uses goroutines -- lightweight user-space threads (~2KB each vs ~8MB for OS threads). The executor spawns one goroutine per package at each dependency level, with a semaphore (buffered channel) limiting actual concurrency to `-jobs`.

```go
semaphore := make(chan struct{}, maxJobs)
var wg sync.WaitGroup

for _, pkg := range level {
    wg.Add(1)
    go func(p Package) {
        defer wg.Done()
        semaphore <- struct{}{}        // acquire
        defer func() { <-semaphore }() // release
        result := runPackageBuild(p)
        results <- result
    }(pkg)
}
wg.Wait()
```

## Comparison with Python/Ruby implementations

| Feature | Go (this) | Python | Ruby |
|---------|-----------|--------|------|
| Startup time | ~5ms | ~200ms | ~300ms |
| Concurrency | goroutines | ThreadPoolExecutor | (planned) |
| Dependencies | none (static binary) | Python 3.12+ | Ruby 3.4+ |
| CI-ready | yes (commit binary) | requires interpreter | requires interpreter |

## Running tests

```bash
go test ./... -v
```

## How it fits in the stack

This is a **program** (not a library). It uses the `directed-graph` package from `code/packages/go/directed-graph` for dependency graph operations (topological sort, independent groups, transitive dependents).
