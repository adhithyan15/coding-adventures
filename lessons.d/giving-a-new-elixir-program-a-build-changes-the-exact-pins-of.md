---
category: Testing & coverage
---

# Giving a new Elixir program a BUILD changes the exact pins of the Elixir Windows BUILD-front audit

The cowsay path-traversal fix (#12169) gave `code/programs/elixir/cowsay` its
first `BUILD` and `BUILD_windows`. That made it a new Elixir root, and the
"Repo-wide metadata contracts" CI job failed: `code/scripts/tests/
test_elixir_windows_build_fronts.py` pins the audit's exact totals (roots,
program roots, native Windows fronts, Windows overrides) and its rendered
Markdown table. Nothing in the package's own build or tests touches that
file, so the local per-package run was green and only CI caught it.

Fix: bump the pinned totals by the new root (288 -> 289 here) and keep the
Markdown assertions in step.

Next time: when a change adds or removes a `BUILD`/`BUILD_windows` for any
Elixir package or program, run
`python3 -m unittest discover -s code/scripts/tests -p 'test_elixir_windows_build_fronts.py'`
before pushing, and more generally run the commands of the CI job "Repo-wide
metadata contracts" (`.github/workflows/ci.yml`) for any change that adds a
package root.
