---
category: Elixir
---

# Format changed Elixir source before running the BUILD check

The microbenchmark `BUILD` stopped at `mix format --check-formatted` because a
changed function used an inline `do`/`else` layout. The formatting gate runs
before the package tests, so none of those later checks ran.

Run `mix format` on the Elixir files changed by the patch, review the formatter
diff, then rerun the complete package `BUILD` with the repository-pinned Elixir
toolchain. The format check is a gate; it does not edit files for you.
