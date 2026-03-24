# Perl Build Tool

An incremental, parallel monorepo build system — the Perl port of the Go build tool.

## Overview

The Perl build tool is the fourth implementation of the coding-adventures build
system, following Go (primary), Python, and Ruby. It is an educational port that
demonstrates how Perl's strengths — powerful regular expressions, native text
processing, and concise hash/list manipulation — apply to the problem of build
system construction.

```
code/programs/perl/build-tool/
    |
    +--> uses: code/packages/perl/directed-graph/  (inline until package is ready)
    |
    +--> reads: code/packages/*/BUILD files
    |           code/packages/*/cpanfile, pyproject.toml, Gemfile, etc.
    |
    +--> runs: shell commands from BUILD files
```

## Features

- **Incremental builds**: Uses `git diff` to find changed packages; only rebuilds what changed plus their transitive dependents.
- **Parallel execution**: Forks up to `--jobs` child processes per independent dependency group.
- **All 9 languages**: Discovers and resolves dependencies for Python, Ruby, Go, TypeScript, Rust, Elixir, Lua, Perl, and Starlark packages.
- **Hash-based cache**: Optional content-hash cache as a fallback when git diff is unavailable.
- **Zero CPAN deps at runtime**: Uses only Perl core modules (5.26+). Only `Test2::V0` is required for tests.

## Installation

```bash
# Install test dependencies
cpanm --installdeps --quiet .

# Build and test
prove -l -v t/
```

## Usage

```bash
# Build changed packages since origin/main
perl bin/build-tool --root /path/to/repo

# Build all packages
perl bin/build-tool --root /path/to/repo --force

# Dry run — show plan without executing
perl bin/build-tool --root /path/to/repo --force --dry-run

# Build only Perl packages
perl bin/build-tool --root /path/to/repo --language perl --force

# 8 parallel jobs
perl bin/build-tool --root /path/to/repo --force --jobs 8
```

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--root DIR` | Repository root | Current directory |
| `--force` | Rebuild all, ignore cache | false |
| `--dry-run` | Show plan, no execution | false |
| `--jobs N` | Max parallel builds | CPU count |
| `--language LANG` | Only build this language | all |
| `--diff-base REF` | Git ref for change detection | `origin/main` |
| `--verbose` | Extra debug output | false |
| `--help` | Show help | |
| `--version` | Show version | |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All builds succeeded (or nothing to build) |
| `1` | One or more builds failed |
| `2` | Configuration error |

## Architecture

| Module | Responsibility |
|--------|---------------|
| `BuildTool.pm` | Orchestrator: wires all sub-modules together |
| `Discovery.pm` | Recursive BUILD file walk; language inference |
| `Resolver.pm` | Metadata parsing; dependency graph construction |
| `Hasher.pm` | SHA256 content fingerprinting |
| `Executor.pm` | `fork()`-based parallel build execution |
| `Cache.pm` | JSON hash cache for incremental builds |
| `GitDiff.pm` | `git diff` change detection |
| `Reporter.pm` | Formatted terminal output |
| `Plan.pm` | JSON build plan serialisation |
| `GlobMatch.pm` | Glob-to-regex conversion for Starlark `srcs` |
| `StarlarkEval.pm` | Starlark BUILD detection and rule mapping |

## Perl Idioms

This implementation highlights Perl's strengths:

**Regex for config parsing** — one-line dependency extraction:
```perl
while ($text =~ /requires\s+['"]coding-adventures-([^'"]+)['"]/g) {
    my $dep = "coding-adventures-$1";
    push @deps, $known{$dep} if exists $known{$dep};
}
```

**`qw()` for string lists** — no quotes or commas needed:
```perl
my @languages = qw(python ruby go typescript rust elixir lua perl starlark);
```

**`fork()` for parallelism** — Unix-native, zero-overhead:
```perl
my $pid = fork();
if ($pid == 0) {
    # child: run build commands, exit with status
    POSIX::_exit($ok ? 0 : 1);
}
# parent: waitpid for all children
```

**Slurp mode** — read entire file at once:
```perl
local $/;   # undef $/ = slurp mode
my $content = <$fh>;
```

## Dependencies

| Dependency | Source | Purpose |
|-----------|--------|---------|
| `Digest::SHA` | Core (5.9.3+) | SHA256 hashing |
| `JSON::PP` | Core (5.14+) | Cache/plan serialisation |
| `File::Find` | Core | Directory walking |
| `Getopt::Long` | Core | CLI argument parsing |
| `POSIX` | Core | `fork()`, `waitpid()` |
| `Test2::V0` | CPAN (test only) | Modern testing framework |

## Tests

```
t/00-discovery.t    15 cases — package discovery, language inference
t/01-resolver.t     20 cases — dependency resolution, graph building
t/02-hasher.t       10 cases — SHA256 hashing, extension allowlists
t/03-executor.t     10 cases — parallel execution, dep-skip, dry-run
t/04-cache.t         8 cases — JSON cache roundtrip, change detection
t/05-gitdiff.t      10 cases — git diff integration, transitive deps
t/06-reporter.t      5 cases — output formatting
t/07-starlark.t      8 cases — Starlark detection, command generation
t/08-glob-match.t    8 cases — glob pattern matching
t/09-plan.t          3 cases — build plan serialisation
t/10-integration.t   2 cases — end-to-end pipeline
```

Run all tests:
```bash
prove -l -v t/
```

## Comparison with Other Implementations

| Feature | Go | Python | Ruby | Perl |
|---------|-----|--------|------|------|
| Parallelism | goroutines | ThreadPoolExecutor | Concurrent::ThreadPoolExecutor | fork() |
| Config parsing | string scanning | re module | regex | native regex + /g |
| JSON | encoding/json | json stdlib | JSON gem | JSON::PP (core) |
| Directory walk | os.WalkDir | os.walk | Pathname.find | File::Find (core) |
| CLI parsing | flag stdlib | argparse | OptionParser | Getopt::Long (core) |
| Hashing | crypto/sha256 | hashlib.sha256 | Digest::SHA gem | Digest::SHA (core) |
