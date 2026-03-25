# Changelog

All notable changes to the Perl build tool are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-03-23

### Added

**Initial implementation of the Perl build tool** — fourth educational port
of the monorepo build system (after Go, Python, and Ruby).

#### Modules

- `CodingAdventures::BuildTool` — Main orchestrator. Wires Discovery → Resolver →
  GitDiff → Executor → Reporter into a complete build pipeline. Supports
  `--force`, `--dry-run`, `--jobs`, `--language`, `--diff-base`, `--verbose`.

- `CodingAdventures::BuildTool::Discovery` — Recursive BUILD file walk using
  `File::Find` (core module). Infers language from path components. Supports
  platform-specific BUILD files (BUILD_mac, BUILD_linux, BUILD_windows,
  BUILD_mac_and_linux). Skips .git, node_modules, __pycache__, vendor, target,
  and other non-source directories.

- `CodingAdventures::BuildTool::Resolver` — Dependency resolution for all 9
  languages: Perl (cpanfile), Python (pyproject.toml), Ruby (Gemfile), Go
  (go.mod), TypeScript (package.json), Rust (Cargo.toml), Elixir (mix.exs),
  Lua (.rockspec). Uses Perl's `/g` scan loop for concise regex matching.
  Includes an inline `CodingAdventures::BuildTool::Graph` implementation
  (to be replaced by `coding-adventures-directed-graph` once that package
  is published).

- `CodingAdventures::BuildTool::Hasher` — SHA256 content fingerprinting using
  `Digest::SHA` (core). Walks package directories, hashes source files sorted
  by relative path for determinism. Allowlists cover all 9 supported languages.

- `CodingAdventures::BuildTool::Executor` — Parallel build execution via
  `fork()`. Forks up to `--jobs` child processes per independent dependency
  group. Uses pipes for child-to-parent result communication. Falls back to
  sequential execution on Windows (where fork() is unavailable). Propagates
  failure: packages whose dependencies fail are marked "skip".

- `CodingAdventures::BuildTool::Cache` — JSON cache using `JSON::PP` (core).
  Stores SHA256 hashes per package after a successful build. On subsequent
  runs, compares current hashes against cached values to identify changed
  packages. Handles missing and corrupt cache files gracefully.

- `CodingAdventures::BuildTool::GitDiff` — Primary change detection mode.
  Runs `git diff --name-only <base>...HEAD` to find changed files. Maps files
  to packages by path prefix. Uses `graph->affected_nodes()` for transitive
  dependency propagation. Falls back from three-dot to two-dot diff if the
  merge base is unavailable.

- `CodingAdventures::BuildTool::Reporter` — Formatted terminal output.
  Prints `[PASS]`/`[FAIL]`/`[SKIP]` status lines with duration. Prints a
  summary table. Supports ANSI colour output (auto-detected from isatty).

- `CodingAdventures::BuildTool::Plan` — Build plan serialisation to JSON.
  Groups packages by dependency level. Used by `--dry-run` mode.

- `CodingAdventures::BuildTool::GlobMatch` — Converts glob patterns to Perl
  `qr//` regexes. Supports `*`, `**`, `?`, and `[...]` character classes.
  Used for filtering Starlark `srcs = glob([...])` patterns.

- `CodingAdventures::BuildTool::StarlarkEval` — Starlark BUILD file detection
  and rule-to-command mapping. Detects `load()` calls and known rule names.
  Maps `perl_library`, `py_library`, `go_library`, `ruby_library`,
  `ts_library`, `rust_library`, `elixir_library`, `lua_library` to their
  respective shell commands.

#### Entry Point

- `bin/build-tool` — CLI entry point. Uses `Getopt::Long` for argument parsing.
  Validates `--root` and `--jobs`. Delegates to `CodingAdventures::BuildTool->run()`.

#### Test Suite

88 test cases across 11 test files:

| File | Cases | What is tested |
|------|-------|---------------|
| `t/00-discovery.t` | 15 | Package discovery, language inference, platform BUILD |
| `t/01-resolver.t` | 20 | All language parsers, known names, graph structure |
| `t/02-hasher.t` | 10 | SHA256 hashing, extension allowlists |
| `t/03-executor.t` | 10 | Parallel execution, dep-skip, dry-run |
| `t/04-cache.t` | 8 | JSON cache roundtrip, change detection |
| `t/05-gitdiff.t` | 10 | Change detection, transitive propagation |
| `t/06-reporter.t` | 5 | Output formatting |
| `t/07-starlark.t` | 8 | Starlark detection, command generation |
| `t/08-glob-match.t` | 8 | Glob pattern matching |
| `t/09-plan.t` | 3 | Build plan serialisation |
| `t/10-integration.t` | 2 | End-to-end force + dry-run pipelines |

#### Configuration Files

- `Makefile.PL` — ExtUtils::MakeMaker build configuration.
- `cpanfile` — Dependency declarations (runtime: all core; test: Test2::V0).
- `BUILD` — `cpanm --installdeps --quiet . && prove -l -v t/`.

### Design Decisions

- **Blessed hashrefs** (not Moo/Moose): educational clarity, zero CPAN deps.
- **`fork()` parallelism** (not `use threads`): lighter weight on Unix; threads
  would copy the entire Perl interpreter per thread.
- **`JSON::PP`** (not Cpanel::JSON::XS): pure Perl, bundled with Perl 5.14+.
- **Inline DirectedGraph**: avoids a circular dependency on the perl-starter-packages
  spec. Will be replaced once `coding-adventures-directed-graph` is published.
- **Core modules only at runtime**: ensures `perl bin/build-tool` works on any
  standard Perl 5.26+ installation without CPAN installs.
