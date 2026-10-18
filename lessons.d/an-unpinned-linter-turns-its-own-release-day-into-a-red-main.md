---
category: Python
---

# An unpinned linter turns its own release day into a red main

`python/window-native` failed main's shard 3 two runs in a row with

    src/window_native/__init__.py:71  PYI034 / UP037  def __enter__(self) -> "Window"
    src/window_native/__init__.py:80  S110 / BLE001   except Exception: pass

Nothing in the package had changed. Its `BUILD` runs
`uv pip install ... ruff` with no version, and `pyproject.toml` has no
`[tool.ruff]` section, so the rule set is "whatever ruff's defaults are
today". ruff 0.17.0 shipped on 2026-10-09 and widened them.

How to confirm a suspicion like this: run both versions on the same tree.

    uvx ruff@0.15.8 check src tests   # All checks passed!
    uvx ruff@0.17.0 check src tests   # the 4 errors above

**Fix used:** make the code satisfy the new rules (`-> Self`, and
`contextlib.suppress(Exception)` in `__del__`). That is cheaper than a pin
and does not freeze the package on an old ruff.

**What to do differently:** when a lint step goes red on main with no
change to the package, check the linter's release date before reading the
code. Only one package in the repo regressed this time because most either
already wrote modern code or declare `[tool.ruff.lint] select`; a package
that wants stability should declare its rule set explicitly.
