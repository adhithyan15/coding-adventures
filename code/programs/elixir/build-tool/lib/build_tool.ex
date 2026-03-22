defmodule BuildTool do
  @moduledoc """
  Build Tool — Incremental, Parallel Monorepo Build System (Elixir Edition)

  This is a full port of the Go build tool for the coding-adventures monorepo.
  It discovers packages via recursive BUILD file walking, resolves dependencies,
  hashes source files, and only rebuilds packages whose source (or dependency
  source) has changed. Independent packages are built in parallel using Elixir's
  `Task.async_stream`.

  ## The build flow

  The pipeline has 11 steps, identical to the Go implementation:

    1. Find the repo root (walk up looking for `.git`)
    2. Discover packages (walk BUILD files under `code/`)
    3. Filter by language if requested
    4. Resolve dependencies (parse pyproject.toml, .gemspec, go.mod, Cargo.toml, package.json, mix.exs)
    5. Git-diff change detection (default mode: `git diff --name-only <base>...HEAD`)
    6. Hash all packages and their dependencies
    7. Load cache (fallback when git diff is unavailable)
    8. If `--dry-run`, report what would build and exit
    9. Execute builds in parallel by dependency level
   10. Update and save cache
   11. Print report and exit with code 1 if any builds failed

  ## Why Elixir?

  The Elixir implementation demonstrates how OTP patterns map to the build
  tool's concerns:

    - **Agent** for the build cache — a simple key-value store with
      concurrent-safe reads and writes, no manual locking needed.
    - **Task.async_stream** for parallel execution — the BEAM scheduler
      handles lightweight process management, similar to Go goroutines
      but with preemptive scheduling and per-process garbage collection.
    - **GenServer** (via the progress bar) for real-time UI updates —
      the mailbox-based event loop naturally serializes progress events
      from concurrent build processes.

  ## Modules

  | Module                    | Responsibility                                      |
  |---------------------------|-----------------------------------------------------|
  | `BuildTool.CLI`           | Escript entry point, argument parsing, orchestration |
  | `BuildTool.Discovery`     | Recursive directory walk to find BUILD files         |
  | `BuildTool.DirectedGraph` | Inline DAG (Kahn's algorithm, affected nodes)        |
  | `BuildTool.Resolver`      | Parse dependency metadata, build dependency graph    |
  | `BuildTool.GitDiff`       | Git-based change detection                           |
  | `BuildTool.Hasher`        | SHA256 hashing of source files and dependencies      |
  | `BuildTool.Cache`         | Agent-based JSON build cache with atomic writes      |
  | `BuildTool.Executor`      | Parallel build execution with progress tracking      |
  | `BuildTool.Reporter`      | Fixed-width report table formatting                  |
  """
end
