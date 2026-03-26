# Perl Build Tool — Educational Port of the Monorepo Build System

## 1. Overview

The coding-adventures monorepo has three implementations of its build tool:

| Implementation | Language | Role | Status |
|----------------|----------|------|--------|
| Go | Go | Primary — used in CI | Complete |
| Python | Python | Educational port | Complete |
| Ruby | Ruby | Educational port | Complete |
| **Perl** | **Perl** | **Educational port** | **This spec** |

The Perl build tool is the fourth implementation. It follows the same
architecture and produces the same behavior as the other three. Its purpose
is educational: it demonstrates how Perl's strengths — powerful regular
expressions, native text processing, and concise hash/list manipulation —
apply to the problem of build system construction.

### Why port the build tool to Perl?

The monorepo follows a "same tool in every language" philosophy. The build
tool, scaffold generator, and CLI builder each exist in every supported
language. Adding Perl to the monorepo means the build tool itself should
have a Perl implementation. This also serves as a non-trivial integration
test: the Perl build tool uses the Perl `directed-graph` package (from the
starter packages spec), proving that the Perl package ecosystem works.

### Design Constraint: Core Modules Only

The Perl build tool uses **only Perl core modules** plus one internal
dependency (`coding-adventures-directed-graph`). This mirrors the Go build
tool, which has zero external dependencies beyond the monorepo's own
`directed-graph` package. The goal is that anyone with a standard Perl
installation (5.26+) can run the build tool without installing anything
from CPAN — except the monorepo's own packages.

---

## 2. Where It Fits

```
code/programs/perl/build-tool/
    |
    +--> uses: code/packages/perl/directed-graph/
    |         (for dependency graphs, topological sort, independent groups)
    |
    +--> reads: code/packages/*/BUILD files
    |           code/packages/*/cpanfile, pyproject.toml, Gemfile, etc.
    |
    +--> runs: shell commands from BUILD files
```

The build tool sits outside the computing stack — it is infrastructure that
builds and tests the stack. It is a **program** (not a library), located in
`code/programs/perl/build-tool/`.

---

## 3. Package Structure

```
code/programs/perl/build-tool/
  Makefile.PL                                  # ExtUtils::MakeMaker config
  cpanfile                                     # Dependencies
  BUILD                                        # How to test the build tool itself
  README.md                                    # Documentation
  CHANGELOG.md                                 # Version history
  bin/
    build-tool                                 # Executable entry point (#!/usr/bin/env perl)
  lib/
    CodingAdventures/
      BuildTool.pm                             # Main orchestrator
      BuildTool/
        Discovery.pm                           # Package discovery via BUILD file walking
        Resolver.pm                            # Dependency resolution across all languages
        Hasher.pm                              # SHA256-based content hashing
        Executor.pm                            # Parallel build execution via fork()
        Cache.pm                               # JSON cache file for hash-based builds
        GitDiff.pm                             # Git-based change detection (primary mode)
        Reporter.pm                            # Human-readable build output
        Plan.pm                                # Build plan JSON serialization
        GlobMatch.pm                           # Glob pattern matching for Starlark srcs
        StarlarkEval.pm                        # Starlark BUILD file detection and rule mapping
  t/
    00-discovery.t                             # Discovery tests
    01-resolver.t                              # Resolver tests
    02-hasher.t                                # Hasher tests
    03-executor.t                              # Executor tests
    04-cache.t                                 # Cache tests
    05-gitdiff.t                               # Git diff tests
    06-reporter.t                              # Reporter tests
    07-starlark.t                              # Starlark evaluator tests
    08-glob-match.t                            # Glob matching tests
    09-plan.t                                  # Plan serialization tests
    10-integration.t                           # End-to-end tests
    fixtures/                                  # Test fixture directories
      simple/
        pkg-a/BUILD
        pkg-a/src/main.py
        pkg-a/pyproject.toml
      diamond/
        pkgs/python/pkg-a/BUILD
        pkgs/python/pkg-a/pyproject.toml
        ...
```

---

## 4. Module Mapping

Each Go package maps to a Perl module. The table below shows the mapping and
highlights where Perl's idioms differ:

| Go Package | Perl Module | Perl Advantage |
|------------|-------------|----------------|
| `discovery` | `CodingAdventures::BuildTool::Discovery` | `File::Find` for recursive walks; regex for language inference |
| `resolver` | `CodingAdventures::BuildTool::Resolver` | Native regex for parsing all config formats (TOML, JSON, Gemfile, cpanfile) |
| `hasher` | `CodingAdventures::BuildTool::Hasher` | `Digest::SHA` (core) for hashing |
| `executor` | `CodingAdventures::BuildTool::Executor` | `fork()` for parallelism — Unix native, zero-dep |
| `cache` | `CodingAdventures::BuildTool::Cache` | `JSON::PP` (core) for cache serialization |
| `gitdiff` | `CodingAdventures::BuildTool::GitDiff` | Backtick operator for shell commands |
| `reporter` | `CodingAdventures::BuildTool::Reporter` | `printf`/`sprintf` formatting |
| `plan` | `CodingAdventures::BuildTool::Plan` | `JSON::PP` for plan JSON |
| `globmatch` | `CodingAdventures::BuildTool::GlobMatch` | Perl regex engine for glob-to-regex conversion |
| `starlark` | `CodingAdventures::BuildTool::StarlarkEval` | Regex for Starlark detection and target parsing |

---

## 5. Key Design Decisions

### 5.1 Core Modules Only

Every dependency is from Perl's standard distribution (5.26+):

| Module | Purpose | Core since |
|--------|---------|-----------|
| `Digest::SHA` | SHA256 hashing for change detection | 5.9.3 |
| `JSON::PP` | JSON encoding/decoding for cache and plan files | 5.14 |
| `File::Find` | Recursive directory walking for discovery | 5.0 |
| `File::Spec` | Portable path manipulation | 5.0 |
| `File::Basename` | Extract directory/filename from paths | 5.0 |
| `Getopt::Long` | CLI argument parsing | 5.0 |
| `POSIX` | `fork()`, `waitpid()`, `WIFEXITED()` | 5.0 |
| `Cwd` | Get current working directory | 5.0 |
| `File::Path` | `make_path()` for directory creation | 5.0 |

The only external dependency is `coding-adventures-directed-graph`, the
monorepo's own directed graph implementation in Perl.

### 5.2 Parallelism via fork()

Go uses goroutines. Python uses `ThreadPoolExecutor`. Ruby uses
`Concurrent::ThreadPoolExecutor`. Perl uses `fork()`.

```
Parallel execution model:

  Parent process
    |
    +-- fork() --> Child 1 (builds package A)
    +-- fork() --> Child 2 (builds package B)
    +-- fork() --> Child 3 (builds package C)
    |
    waitpid() for all children
    |
    Check exit codes, report results
```

**How it works:**

1. The executor computes independent groups from the dependency graph.
2. For each group (packages with no unbuilt dependencies), it forks up to
   `--jobs` child processes.
3. Each child runs the BUILD commands for one package, then exits with 0
   (success) or 1 (failure).
4. The parent `waitpid()`s for all children in the group.
5. Failed packages propagate: their dependents are marked "dep-skipped."
6. The next group starts.

**Semaphore pattern:** A simple counter limits concurrency. Before forking,
check if `active_children < max_jobs`. If at the limit, `waitpid(-1, 0)` to
reap one child before forking another.

**Why fork() instead of threads:** Perl's threading model (`use threads`) is
heavyweight — each thread gets a complete copy of the interpreter. `fork()`
is lighter on Unix systems and maps more naturally to the "run shell commands
in parallel" use case. The child processes don't share memory, which is fine
because each child runs an independent BUILD file.

**Limitation:** `fork()` is Unix-only. On Windows (without WSL), the build
tool falls back to sequential execution. This matches the Go build tool's
behavior — CI runs on Linux, and developer machines are macOS or Linux.

### 5.3 OO via Blessed Hashrefs

All modules use Perl's native OO mechanism — blessed hash references:

```perl
package CodingAdventures::BuildTool::Discovery;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless {
        root     => $args{root} // '.',
        packages => [],
    }, $class;
}

sub packages { return $_[0]->{packages} }
```

We chose blessed hashrefs over `Moo` or `Moose` because:
- Zero external dependencies (core modules only constraint).
- Educational clarity — readers see exactly how Perl OO works.
- Sufficient for our needs — we don't need type constraints, roles, or
  attribute delegation.

### 5.4 Data Representation

Packages are represented as hash references throughout:

```perl
my $package = {
    name     => "perl/logic-gates",
    path     => "/repo/code/packages/perl/logic-gates",
    language => "perl",
    build    => "/repo/code/packages/perl/logic-gates/BUILD",
};
```

This mirrors Go's `Package` struct. In Perl, we don't define a formal struct
type — the hash keys serve as implicit fields.

---

## 6. CLI Interface

The Perl build tool accepts the same flags as the Go version:

```
Usage: build-tool [options]

Options:
  --root DIR           Repository root (default: current directory)
  --force              Rebuild all packages, ignore cache
  --dry-run            Show build plan without executing
  --jobs N             Max parallel builds (default: number of CPUs)
  --language LANG      Only build packages of this language
  --diff-base REF      Git ref for change detection (default: origin/main)
  --verbose            Extra output for debugging
  --help               Show this help
  --version            Show version
```

**Exit codes** match the Go implementation:
- 0: all builds succeeded
- 1: one or more builds failed
- 2: configuration error (bad flags, root not found)

---

## 7. Perl Idioms Highlighted

The Perl build tool is an educational implementation. These idioms should be
prominently featured with explanatory comments:

### 7.1 Regex for Config Parsing

Perl's first-class regex makes dependency parsing concise:

```perl
# Python: extract dependencies from pyproject.toml
while ($text =~ /"(coding-adventures-[^"]+)"/g) {
    push @deps, $1 if exists $known_names->{$1};
}

# Ruby: extract gem dependencies from Gemfile
while ($text =~ /gem\s+"(coding_adventures_[^"]+)"/g) {
    push @deps, $1 if exists $known_names->{$1};
}

# Perl: extract requires from cpanfile
while ($text =~ /requires\s+['"]coding-adventures-([^'"]+)['"]/g) {
    my $dep_name = "coding-adventures-$1";
    push @deps, $known_names->{$dep_name} if exists $known_names->{$dep_name};
}
```

In Go, these require `regexp.MustCompile()` and `FindAllStringSubmatch()`.
In Perl, they are one-line matches with captures.

### 7.2 Hash Slices for Lookups

```perl
# Check if a file extension is relevant for this language
my %perl_exts = map { $_ => 1 } qw(.pm .pl .t .xs);
return 1 if $perl_exts{$ext};
```

### 7.3 qw() for String Lists

```perl
my @languages = qw(python ruby go typescript rust elixir lua perl starlark);
my @skip_dirs = qw(.git .venv node_modules vendor target __pycache__);
```

### 7.4 Heredocs for Templates

```perl
my $report = <<~"END";
    Build Results:
      Packages: $total
      Passed:   $passed
      Failed:   $failed
      Skipped:  $skipped
    END
```

### 7.5 Backticks for Shell Commands

```perl
# Git diff for change detection
my $output = `git diff --name-only $diff_base...HEAD`;
my @changed_files = split /\n/, $output;
```

---

## 8. Public API

### 8.1 CodingAdventures::BuildTool (Main Entry Point)

```
BuildTool->new(root => $path, options => \%opts)
BuildTool->run()                    # Execute the full build pipeline
BuildTool->plan()                   # Return the build plan without executing
```

### 8.2 Discovery

```
Discovery->new(root => $path)
Discovery->discover()               # Walk tree, find packages
Discovery->packages()               # Return list of package hashrefs
```

### 8.3 Resolver

```
Resolver->resolve(\@packages)       # Returns a Graph object with dependency edges
Resolver->build_known_names(\@packages)  # Returns name mapping hash
```

### 8.4 Hasher

```
Hasher->new()
Hasher->hash_package($package)      # Returns SHA256 hex string for a package
Hasher->collect_source_files($package)  # Returns sorted list of source file paths
```

### 8.5 Executor

```
Executor->new(max_jobs => $n, dry_run => $bool)
Executor->execute(\@packages, $graph)  # Run builds in dependency order
Executor->results()                    # Returns list of result hashrefs
```

### 8.6 GitDiff

```
GitDiff->new(root => $path, diff_base => $ref)
GitDiff->changed_files()            # Returns list of changed file paths
GitDiff->affected_packages(\@packages, $graph)  # Returns list of affected package names
```

### 8.7 Cache

```
Cache->new(path => $cache_file)
Cache->load()                       # Read cache from disk
Cache->save(\%hashes)               # Write hash map to disk
Cache->changed_packages(\@packages, \%current_hashes)  # Compare against cached
```

### 8.8 Reporter

```
Reporter->new()
Reporter->report(\@results)         # Print formatted build report
Reporter->summary(\@results)        # Print one-line summary
```

---

## 9. Test Strategy

Tests use `Test2::V0` and follow the TAP convention. Each module gets its
own test file.

### 9.1 Discovery Tests (`t/00-discovery.t`) — 15 cases

| # | Test | Input | Expected |
|---|------|-------|----------|
| 1 | Discover single package | Fixture dir with one BUILD | 1 package found |
| 2 | Discover nested packages | `code/packages/python/foo/BUILD` | Language: python |
| 3 | Infer Perl language | Path contains `/perl/` | Language: perl |
| 4 | Infer Python language | Path contains `/python/` | Language: python |
| 5 | Skip .git directories | `.git/` present | Not traversed |
| 6 | Skip node_modules | `node_modules/` present | Not traversed |
| 7 | Skip __pycache__ | `__pycache__/` present | Not traversed |
| 8 | Package name format | Path `code/packages/perl/bitset` | Name: `perl/bitset` |
| 9 | Platform BUILD precedence (mac) | Both BUILD and BUILD_mac | Uses BUILD_mac on Darwin |
| 10 | Platform BUILD fallback | Only BUILD exists | Uses BUILD |
| 11 | No BUILD file | Directory without BUILD | Not registered |
| 12 | Empty directory | No files | No packages |
| 13 | Multiple languages | Packages in python/, go/, perl/ | All discovered |
| 14 | Unknown language | Path without known language | Language: unknown |
| 15 | BUILD file content | Valid BUILD | Commands extracted correctly |

### 9.2 Resolver Tests (`t/01-resolver.t`) — 20 cases

| # | Test | Language | Expected |
|---|------|----------|----------|
| 1 | Python dep | `coding-adventures-logic-gates` in pyproject.toml | Resolved |
| 2 | Ruby dep | `coding_adventures_logic_gates` in Gemfile | Resolved |
| 3 | Go dep | Module path in go.mod | Resolved |
| 4 | TypeScript dep | `@coding-adventures/logic-gates` in package.json | Resolved |
| 5 | Rust dep | `logic-gates` with `path = "../"` in Cargo.toml | Resolved |
| 6 | Elixir dep | `coding_adventures_logic_gates` in mix.exs | Resolved |
| 7 | Lua dep | `coding-adventures-logic-gates` in rockspec | Resolved |
| 8 | Perl dep | `coding-adventures-logic-gates` in cpanfile | Resolved |
| 9 | External dep skipped | `requires 'Moo'` in cpanfile | Not in graph |
| 10 | Multiple deps | Package with 3 internal deps | All 3 edges |
| 11 | Diamond dependency | A->B, A->C, B->D, C->D | 4 edges, correct order |
| 12 | No deps | Package with empty config | Node only, no edges |
| 13 | Missing config file | No pyproject.toml | No deps |
| 14 | Known names Python | Dir `logic-gates` | `coding-adventures-logic-gates` |
| 15 | Known names Ruby | Dir `logic_gates` | `coding_adventures_logic_gates` |
| 16 | Known names Perl | Dir `logic-gates` | `coding-adventures-logic-gates` |
| 17 | Known names TypeScript | Dir `logic-gates` | `@coding-adventures/logic-gates` |
| 18 | Known names Rust | Dir `logic-gates` | `logic-gates` |
| 19 | Graph structure | 3 packages in chain | Topological sort correct |
| 20 | Independent groups | A->B, C->D (disconnected) | 2 groups |

### 9.3 Hasher Tests (`t/02-hasher.t`) — 10 cases

| # | Test | Expected |
|---|------|----------|
| 1 | Hash is deterministic | Same files → same hash |
| 2 | Hash changes with content | Modified file → different hash |
| 3 | Hash includes BUILD | BUILD file change → different hash |
| 4 | Python extensions | `.py`, `.toml` included |
| 5 | Perl extensions | `.pm`, `.pl`, `.t` included |
| 6 | Non-source excluded | `.bak`, `.log` excluded |
| 7 | Special files included | `cpanfile`, `Makefile.PL` included |
| 8 | Files sorted | Hash order is deterministic |
| 9 | Empty package | No source files → consistent hash |
| 10 | Subdirectories included | Nested `.pm` files found |

### 9.4 Executor Tests (`t/03-executor.t`) — 10 cases

| # | Test | Expected |
|---|------|----------|
| 1 | Single package success | Exit code 0, status pass |
| 2 | Single package failure | Exit code 1, status fail |
| 3 | Parallel execution | 2 independent packages → both run |
| 4 | Dep-skip on failure | A fails → B (depends on A) skipped |
| 5 | Dry-run mode | No commands executed |
| 6 | Sequential fallback | `--jobs 1` → one at a time |
| 7 | Build order respected | A before B when B depends on A |
| 8 | Multiple groups | Independent groups run in sequence |
| 9 | Command output captured | stdout/stderr recorded |
| 10 | Timeout handling | Long-running command killed after limit |

### 9.5 Cache Tests (`t/04-cache.t`) — 8 cases

| # | Test | Expected |
|---|------|----------|
| 1 | Save and load roundtrip | Hashes preserved |
| 2 | Changed package detected | New hash vs cached hash |
| 3 | New package detected | Not in cache → needs rebuild |
| 4 | Removed package ignored | In cache but not discovered |
| 5 | Missing cache file | Treated as empty cache |
| 6 | Corrupt cache file | Error logged, treated as empty |
| 7 | JSON format | Valid JSON written |
| 8 | Unchanged package | Same hash → no rebuild |

### 9.6 GitDiff Tests (`t/05-gitdiff.t`) — 10 cases

| # | Test | Expected |
|---|------|----------|
| 1 | Changed file maps to package | `code/packages/perl/bitset/lib/Bitset.pm` → `perl/bitset` |
| 2 | File outside packages | `README.md` at root → no package |
| 3 | Multiple files same package | Two changes in `perl/bitset` → one package |
| 4 | Affected nodes propagate | Change in A, B depends on A → both affected |
| 5 | Three-dot diff | Uses merge base correctly |
| 6 | Fallback to two-dot | Three-dot fails → uses two-dot |
| 7 | Empty diff | No changes → no packages to build |
| 8 | Shared prefix change | `code/specs/` change → all packages affected |
| 9 | BUILD change detected | BUILD file change → package affected |
| 10 | New file in package | Added file → package affected |

### 9.7 Reporter Tests (`t/06-reporter.t`) — 5 cases

| # | Test | Expected |
|---|------|----------|
| 1 | All pass | Summary shows 0 failures |
| 2 | Some fail | Failed packages listed |
| 3 | Dep-skipped shown | Skipped packages listed separately |
| 4 | Timing shown | Duration per package |
| 5 | Empty results | "Nothing to build" message |

### 9.8 Starlark Tests (`t/07-starlark.t`) — 8 cases

| # | Test | Expected |
|---|------|----------|
| 1 | Detect perl_library | Starlark: true |
| 2 | Detect load() statement | Starlark: true |
| 3 | Shell BUILD not Starlark | Regular commands | Starlark: false |
| 4 | Generate Perl commands | `perl_library` → cpanm + prove |
| 5 | Generate Python commands | `py_library` → uv + pytest |
| 6 | Generate Go commands | `go_library` → go build + test + vet |
| 7 | Parse targets | Extract name, srcs, deps |
| 8 | Comment handling | Commented rules ignored |

### 9.9 Integration Tests (`t/10-integration.t`) — 2 cases

| # | Test | Expected |
|---|------|----------|
| 1 | Full pipeline (force) | Discover → resolve → execute all |
| 2 | Full pipeline (diff-based) | Only changed packages built |

**Total: ~88 test cases.**

---

## 10. Trade-Offs

### 10.1 Blessed Hashref vs Moo

| | Blessed Hashref | Moo |
|-|-----------------|-----|
| Dependencies | Zero | Moo, Sub::Quote |
| Learning value | Shows raw Perl OO | Hides plumbing |
| Type safety | None (hash keys) | Attribute specs |
| Code verbosity | More boilerplate | Less boilerplate |
| **Decision** | **Blessed hashref** | — |

Educational purity wins. The build tool shows readers exactly how Perl OO
works under the hood.

### 10.2 fork() vs Threads vs Sequential

| | fork() | threads | Sequential |
|-|--------|---------|-----------|
| Unix support | Native | Heavyweight | Always works |
| Windows support | No (without WSL) | Yes | Yes |
| Memory model | Copy-on-write | Shared interpreter | N/A |
| Complexity | Medium | High | Low |
| **Decision** | **fork() with sequential fallback** | — | — |

The primary mode is fork(). If `fork()` is unavailable (rare on our
supported platforms), fall back to sequential execution.

### 10.3 JSON::PP vs Cpanel::JSON::XS

| | JSON::PP | Cpanel::JSON::XS |
|-|----------|-------------------|
| Speed | Slow (~10x) | Fast |
| Dependencies | Core (5.14+) | External |
| Portability | Everywhere | Needs C compiler |
| **Decision** | **JSON::PP** | — |

We use JSON::PP exclusively. The build tool processes small JSON files (cache
and plan), so performance is irrelevant. The core-modules-only constraint
is more important.

---

## 11. Dependencies

| Dependency | Type | Purpose |
|-----------|------|---------|
| `coding-adventures-directed-graph` | Internal (monorepo) | Graph data structure for dependency resolution |
| `Digest::SHA` | Core | Content hashing |
| `JSON::PP` | Core | Cache/plan serialization |
| `File::Find` | Core | Directory walking |
| `Getopt::Long` | Core | CLI argument parsing |
| `POSIX` | Core | fork/waitpid |
| `Test2::V0` | Test-only (CPAN) | Testing framework |

---

## 12. Future Extensions

- **Windows support:** Replace `fork()` with `Win32::Process` or
  `IPC::Open3` for Windows native execution.
- **Starlark evaluation:** Full Starlark interpreter in Perl (beyond
  detection and command generation).
- **Coverage reporting:** Integrate `Devel::Cover` into the build report
  for Perl packages.
- **Performance profiling:** Add `--profile` flag using `Devel::NYTProf`
  to identify bottlenecks in the build tool itself.
- **Watch mode:** Monitor file changes with `Linux::Inotify2` or
  `Mac::FSEvents` and rebuild on change.
