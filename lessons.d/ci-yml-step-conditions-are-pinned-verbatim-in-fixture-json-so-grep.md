---
category: CI & GitHub Actions
---

# ci.yml step conditions are pinned verbatim in fixture JSON, so grep code/specs/fixtures for every expression you rename

PR #17759 renamed the detect output that gates ci.yml's build steps from
`is_main` to `full_build`. I grepped `code/scripts/` for `is_main`, found
`ocaml_toolchain_lock.py`, and updated it along with its pinned digest. The PR
still failed "Repo-wide metadata contracts":
`validate_elixir_windows_build_fronts.AuditError: affected-package build
remains disabled for Elixir on Windows`. That validator lives in
`code/scripts/`, but the condition it compares against is not in the script.
It is a string field (`affected_build_condition`) in
`code/specs/fixtures/elixir-windows-build-front-v1/contract.json`, so a grep
of `code/scripts/` alone missed it.

Fix: update the fixture's condition to the new expression.

Next time:
- Before renaming anything ci.yml exposes (an output, a step name, a step
  `if:`), grep the whole repo outside `.github/`, `code/specs/fixtures`
  included, for the old text, e.g.
  `git grep -n "needs.detect.outputs.is_main" -- ':!.github'`.
- Run the contracts job's whole "Verify repo-wide metadata contracts" script
  locally, not just the one contract you knew about. Extract it from ci.yml
  with `yaml.safe_load`. On macOS, the xvfb and apt tests fail only because
  `timeout` is missing.
