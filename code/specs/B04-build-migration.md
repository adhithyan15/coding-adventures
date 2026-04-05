# B04 — Migrating BUILD Files to Starlark

## Overview

The coding-adventures monorepo contains over 1,000 shell-script BUILD files and roughly
200 companion BUILD_windows files. These files are the legacy format described in spec 12.
They work, but they carry every limitation that motivated spec 14 (Starlark) and spec 15
(OS-aware build rules):

- **No structure.** A shell BUILD file is a flat list of commands. The build tool cannot
  inspect it to learn what a package depends on, what files it reads, or what it produces.
  That information lives in the *commands themselves*, locked inside opaque strings.

- **Platform duplication.** Every package that behaves differently on Windows needs a
  separate BUILD_windows file. That is 200 files whose only purpose is to quote a path
  differently or swap `sh -c` for `cmd /C`. Spec 15 eliminates this duplication for new
  packages, but the existing 200 files remain.

- **No reuse.** Shell BUILD files cannot call shared functions. If the Python test recipe
  changes (say, adding a coverage threshold), every Python BUILD file must be edited by
  hand. Starlark's `load()` mechanism solves this with importable rule libraries.

This spec describes the migration: how to convert every legacy BUILD (and BUILD_windows)
file into a single Starlark BUILD.lark file, one package at a time, without breaking CI
at any point along the way.

### Why "BUILD.lark"?

During the migration period, both formats coexist. The build tool needs an unambiguous way
to tell them apart. We use the `.lark` extension (short for Starlark) as a temporary
marker. A file named `BUILD.lark` is always Starlark. A file named `BUILD` is always
legacy shell. After every package has migrated, a final rename pass converts all
BUILD.lark files back to BUILD (see Section 9).

This two-name strategy means the migration is fully incremental. Package A can migrate
today while package B stays on shell for another month. The build tool handles both.

---

## Migration Strategy

The migration has three phases, each backed by a dedicated command:

```
Phase 1: Extract          Phase 2: Generate         Phase 3: Migrate
  BUILD ──────────┐
  BUILD_windows ──┤         manifest.json            BUILD.lark
  pyproject.toml ─┤  ──→  (structured deps,   ──→  (Starlark rules,
  package.json ───┤        srcs, commands)           single file,
  mix.exs ────────┘                                  all platforms)
```

**Extract** reads everything the build tool can find about a package — its shell commands,
its language-specific metadata files, its directory layout — and produces a structured JSON
manifest. This is a *read-only* operation. It changes nothing on disk.

**Generate** takes the manifest and writes a BUILD.lark file. This is the creative step:
it maps shell commands to Starlark rule invocations, infers dependencies from metadata
files, and produces a single file that works on every platform.

**Migrate** is the human step: review the generated file, run it, compare results against
the legacy BUILD, commit, and delete the old files. This is deliberately *not* automated
end-to-end because the migration is a one-way door. Human review catches the edge cases
that automation misses.

---

## The `build-tool extract-deps` Command

### Purpose

Analyze a package's existing infrastructure and produce a machine-readable manifest. Think
of it as an X-ray: it looks at everything without touching anything.

### Usage

```bash
build-tool extract-deps --package python/directed-graph    # one package
build-tool extract-deps --all                              # every package
build-tool extract-deps --all --output manifests/          # write to directory
```

### Output Format

The manifest is a JSON object that captures everything the generator needs:

```json
{
  "name": "directed-graph",
  "language": "python",
  "internal_deps": ["python/state-machine"],
  "external_deps": ["pytest>=7.0", "pytest-cov>=4.0"],
  "srcs": ["src/**/*.py"],
  "test_srcs": ["tests/**/*.py"],
  "build_commands": {
    "default": [
      "uv venv --quiet --clear",
      "uv pip install -e .[dev] --quiet",
      ".venv/bin/python -m pytest tests/ -v"
    ],
    "windows": [
      "uv venv --quiet --clear",
      "uv pip install -e .[dev] --quiet",
      "uv run --no-project python -m pytest tests/ -v"
    ]
  },
  "inputs": ["src/**/*.py", "tests/**/*.py", "pyproject.toml"],
  "outputs": [".venv/", "htmlcov/"],
  "side_effects": [".venv/", ".pytest_cache/", ".ruff_cache/"]
}
```

### Data Sources

The extractor consults multiple sources, each contributing different pieces:

| Source | What it provides |
|--------|-----------------|
| `BUILD` | Default build commands (shell, one per line) |
| `BUILD_windows` | Windows-specific build commands |
| `pyproject.toml` | Python deps, dev deps, project name |
| `package.json` | TypeScript/JS deps, scripts, project name |
| `mix.exs` | Elixir deps, project name |
| `go.mod` | Go module path, dependencies |
| `Cargo.toml` | Rust crate name, dependencies |
| `*.gemspec` | Ruby gem name, dependencies |
| `Build.zig` | Zig build configuration |
| Directory tree | Source file globs, test file globs |

The extractor does not execute any commands. It parses files and scans directories. If a
metadata file is missing or unparseable, the corresponding manifest fields are empty — the
generator will fall back to shell-command passthrough for those cases.

### Internal Dependency Detection

Internal dependencies (packages within the monorepo that this package depends on) are
detected from two sources:

1. **Metadata files.** A `pyproject.toml` might list `state-machine = {path = "../state-machine"}`. The extractor resolves relative paths to package identifiers.

2. **BUILD file analysis.** Some BUILD files contain `cd` commands or path references that
   imply ordering dependencies. The extractor uses pattern matching (not full shell
   parsing) to identify these.

---

## The `build-tool tailor` Command

### Purpose

Generate BUILD.lark files from extract-deps manifests. The name "tailor" follows the
convention established by the Pants build system — tailoring means auto-generating build
metadata that fits each package.

### Usage

```bash
build-tool tailor                                          # all packages
build-tool tailor --package python/directed-graph          # one package
build-tool tailor --dry-run                                # preview without writing
build-tool tailor --diff                                   # show diff vs existing BUILD
```

### Generation Logic

The generator maps each language to a set of Starlark rules:

```
Python  → python_library_test()  or  python_program()
Go      → go_test()              or  go_binary()
TypeScript → ts_library_test()   or  ts_program()
Rust    → rust_test()            or  rust_binary()
Elixir  → elixir_test()
Ruby    → ruby_test()
```

For a typical Python package, the generated BUILD.lark looks like:

```starlark
load("rules/python.star", "python_library_test")

python_library_test(
    name = "directed-graph",
    deps = ["//python/state-machine"],
    dev_deps = ["pytest>=7.0", "pytest-cov>=4.0"],
    srcs = glob("src/**/*.py"),
    test_srcs = glob("tests/**/*.py"),
)
```

Compare this to the legacy BUILD file it replaces:

```bash
# Install transitive deps first
cd ../state-machine && uv venv --quiet --clear && uv pip install -e ".[dev]" --quiet && cd -
# Install this package
uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```

The Starlark version is shorter, declarative, and platform-independent. The `python_library_test`
rule knows how to create venvs, install deps, and run pytest on every OS. The developer
declares *what* to build; the rule handles *how*.

### Passthrough Mode

When the generator cannot confidently map shell commands to Starlark rules — because the
BUILD file does something unusual — it falls back to a `shell_commands()` rule that wraps
the original commands verbatim:

```starlark
load("rules/shell.star", "shell_commands")

shell_commands(
    name = "unusual-package",
    default = [
        "custom-tool --flag value",
        "another-command",
    ],
    windows = [
        "custom-tool.exe --flag value",
        "another-command.bat",
    ],
)
```

This ensures every package can migrate, even if the generated file is just a structured
wrapper around the original shell commands. These passthrough packages can be improved
later — the important thing is getting off the dual BUILD/BUILD_windows format.

---

## BUILD.lark File Priority

The build tool's package discovery (spec 12) is updated with a three-tier priority:

```
Priority 1:  BUILD.lark exists     → evaluate with Starlark VM
Priority 2:  BUILD_<platform>      → execute as shell (legacy)
Priority 3:  BUILD exists           → execute as shell (legacy)
```

This ordering means:

- A package with BUILD.lark is always treated as Starlark, even if a legacy BUILD file
  also exists. This is useful during migration: you can add BUILD.lark and test it while
  keeping BUILD as a safety net.

- BUILD_windows still works for un-migrated packages. It is only consulted when no
  BUILD.lark exists.

- A package with only BUILD uses the original shell execution path. Nothing changes for
  packages that have not migrated yet.

The Starlark detection logic already exists in the build tool (it checks for `load(`,
`def `, or known rule name patterns). With file extensions, detection becomes trivial:
`.lark` means Starlark, plain `BUILD` means shell.

---

## Migration Workflow Per Package

This is the step-by-step process a developer follows to migrate one package:

```
Step 1:  build-tool extract-deps --package <pkg>
           → produces manifest.json (inspect it, sanity-check)

Step 2:  build-tool tailor --package <pkg>
           → writes BUILD.lark into the package directory

Step 3:  Human review of BUILD.lark
           → does it look right? Are deps complete? Any weird passthrough?

Step 4:  build-tool tailor --validate --package <pkg>
           → compares BUILD.lark output against legacy BUILD (see Section 7)

Step 5:  Run build with BUILD.lark active
           → build-tool --package <pkg>  (BUILD.lark takes priority)

Step 6:  If all green:
           git add <pkg>/BUILD.lark
           git rm <pkg>/BUILD <pkg>/BUILD_windows   (if it exists)
           git commit -m "build(<pkg>): migrate to Starlark BUILD.lark"

Step 7:  Verify CI passes on macOS, Linux, and Windows
```

If step 5 or 7 fails, the fix is simple: delete BUILD.lark, and the legacy BUILD file
takes over again (see Section 10 on rollback).

---

## Validation

### The `--validate` Flag

```bash
build-tool tailor --validate --package python/directed-graph
```

Validation runs both the legacy path and the Starlark path, then compares:

1. **Command comparison.** For each platform (macOS, Linux, Windows), generate the list of
   shell commands that would be executed. Compare the Starlark-generated commands against
   the legacy BUILD/BUILD_windows commands. Report any differences.

2. **Dependency comparison.** Compare the dependency graph edges produced by BUILD.lark
   against the edges the build tool currently infers from BUILD files. Missing or extra
   edges are flagged.

3. **Input/output comparison.** Verify that BUILD.lark declares the same source files and
   test files that the extractor found.

Validation must pass before a migration commit. It is the safety net that catches
generation bugs before they reach CI.

### Example Output

```
Validating python/directed-graph...
  Commands (darwin):  MATCH (3 commands)
  Commands (linux):   MATCH (3 commands)
  Commands (windows): MATCH (3 commands, adapted)
  Dependencies:       MATCH (1 internal dep)
  Source files:       MATCH (4 .py files)
  Test files:         MATCH (2 test files)
  Result: PASS
```

---

## Batch Migration

Once the per-package workflow is validated on a representative sample, batch migration
accelerates the remaining packages.

### Commands

```bash
build-tool tailor --all                    # generate BUILD.lark for every package
build-tool tailor --all --validate         # generate and validate all
build-tool tailor --all --commit           # generate, validate, and commit in batches
```

### Batching Strategy

The `--commit` flag groups packages by language and commits each group separately:

```
Commit 1:  build(python): migrate 152 packages to Starlark BUILD.lark
Commit 2:  build(typescript): migrate 98 packages to Starlark BUILD.lark
Commit 3:  build(go): migrate 83 packages to Starlark BUILD.lark
Commit 4:  build(elixir): migrate 180 packages to Starlark BUILD.lark
Commit 5:  build(rust): migrate 45 packages to Starlark BUILD.lark
Commit 6:  build(ruby): migrate 40 packages to Starlark BUILD.lark
...
```

Language-based batching keeps each commit reviewable and makes bisection easier if a
problem surfaces later. If the Elixir migration introduces a bug, `git bisect` points
straight at commit 4.

---

## Rename: BUILD.lark to BUILD

After every package has migrated (zero legacy BUILD files remain), the `.lark` extension
has served its purpose. A final cleanup renames all BUILD.lark files back to BUILD:

```bash
# One PR, one commit
find code/packages code/programs -name BUILD.lark -exec sh -c \
  'mv "$1" "$(dirname "$1")/BUILD"' _ {} \;
git add -A
git commit -m "build: rename BUILD.lark → BUILD (migration complete)"
```

The build tool's discovery logic is updated simultaneously:

1. Read the file content (not the extension).
2. If the first non-comment, non-blank line matches a Starlark pattern (`load(`, `def `,
   or a known rule name followed by `(`), treat as Starlark.
3. Otherwise, treat as shell.

This content-based detection is already implemented. The rename simply drops the extension
hint that was only needed during the coexistence period.

---

## Rollback

Mistakes happen. The migration is designed so that rollback is always one `git checkout`
away:

```bash
# Restore legacy files for one package
git checkout HEAD~1 -- code/packages/python/directed-graph/BUILD
git checkout HEAD~1 -- code/packages/python/directed-graph/BUILD_windows
rm code/packages/python/directed-graph/BUILD.lark
```

Because the file priority system (Section 5) prefers BUILD.lark over BUILD, simply
deleting BUILD.lark causes the build tool to fall back to the legacy files. No
configuration change, no flag flip — just file presence.

For batch rollback (reverting an entire language group):

```bash
git revert <commit-hash>    # reverts the batch commit cleanly
```

The revert restores all BUILD/BUILD_windows files and deletes the BUILD.lark files in one
atomic operation. CI runs against the restored state to confirm everything works.

---

## Timeline

The migration is phased to manage risk. Each phase is a PR (or small set of PRs) with its
own CI validation.

| Phase | Scope | Packages | Goal |
|-------|-------|----------|------|
| 1 | Pilot: 10 packages across 5 languages (2 per language) | ~10 | Validate the extract/generate/migrate pipeline end-to-end |
| 2 | All Python packages | ~150 | Largest language group; most BUILD_windows pain |
| 3 | All TypeScript packages | ~100 | Complex `file:` dependency chains stress-test the generator |
| 4 | All Go packages | ~80 | Simplest migration (minimal platform variance) |
| 5 | All remaining: Rust, Ruby, Elixir, Java, Kotlin, Swift, Perl, Lua | ~600 | Long tail; many are structurally similar within each language |
| 6 | Rename BUILD.lark to BUILD | all | Final cleanup; drop the `.lark` extension |

**Phase 1 is the most important.** It surfaces problems in the tooling before batch
migration amplifies them. Choose pilot packages that are representative: one simple, one
with platform-specific commands, one with internal deps, one with external deps, one with
unusual build steps.

**Phase 2 targets Python first** because Python packages account for the most
BUILD_windows files. Every migrated Python package eliminates a BUILD_windows file,
immediately reducing maintenance burden.

**Phase 4 targets Go late** despite being the simplest, because Go packages rarely have
BUILD_windows files. The urgency is lower.

---

## Tracking

### The `build-tool migration-status` Command

```bash
build-tool migration-status
```

Prints a summary like:

```
BUILD File Migration Status
===========================

Total packages:        1,047
Migrated (BUILD.lark):   152  (14.5%)
Legacy (BUILD):          895  (85.5%)

  With BUILD_windows:    198  (HIGH PRIORITY — platform duplication)
  Without BUILD_windows: 697

By language:
  python:       12 / 152  migrated  ( 7.9%)  [78 have BUILD_windows]
  typescript:    0 /  98  migrated  ( 0.0%)  [42 have BUILD_windows]
  go:            0 /  83  migrated  ( 0.0%)  [ 3 have BUILD_windows]
  elixir:        0 / 180  migrated  ( 0.0%)  [51 have BUILD_windows]
  rust:          0 /  45  migrated  ( 0.0%)  [ 8 have BUILD_windows]
  ruby:          0 /  40  migrated  ( 0.0%)  [ 6 have BUILD_windows]
  java:          0 /  32  migrated  ( 0.0%)  [ 4 have BUILD_windows]
  kotlin:        0 /  28  migrated  ( 0.0%)  [ 3 have BUILD_windows]
  swift:         0 /  25  migrated  ( 0.0%)  [ 2 have BUILD_windows]
  perl:          0 /  15  migrated  ( 0.0%)  [ 1 have BUILD_windows]
  lua:           0 /   5  migrated  ( 0.0%)  [ 0 have BUILD_windows]

Progress: ████░░░░░░░░░░░░░░░░░░░░░░░░░░ 14.5%
```

The `BUILD_windows` count is highlighted because those packages benefit most from
migration — each one eliminates a redundant file.

### Machine-Readable Output

```bash
build-tool migration-status --json
```

Produces a JSON object suitable for dashboards or CI checks:

```json
{
  "total": 1047,
  "migrated": 152,
  "legacy": 895,
  "with_windows": 198,
  "by_language": {
    "python": {"total": 152, "migrated": 12, "with_windows": 78},
    "typescript": {"total": 98, "migrated": 0, "with_windows": 42}
  }
}
```

---

## Summary

The migration is an infrastructure investment. It replaces ~1,200 files (BUILD +
BUILD_windows) with ~1,000 BUILD.lark files — a net reduction of 200 files — while making
every remaining file structured, reusable, and platform-independent.

The key design decisions:

1. **Incremental.** One package at a time. No big bang.
2. **Reversible.** Rollback is always `git checkout` plus deleting one file.
3. **Validated.** The `--validate` flag catches generation bugs before they reach CI.
4. **Tracked.** The `migration-status` command shows progress at a glance.
5. **Temporary naming.** BUILD.lark is a migration artifact. It becomes BUILD when done.

The tools do the heavy lifting. The human provides the judgment. That is the right division
of labor for a migration that touches every package in the monorepo.
