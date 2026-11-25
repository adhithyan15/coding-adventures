---
category: CI & GitHub Actions
---

# Python package BUILD files should use their uv venv interpreter for every tool

A Python package's BUILD front invoked `pyenv` on POSIX and expected Ruff to be
installed in the global Python on Windows. CI runners did not provide either
assumption: Linux stopped with `pyenv: not found`, and Windows stopped with
`No module named ruff` before the tests ran.

Create the package-local environment with `uv venv .venv --quiet --no-project
--clear`, install the editable package and dev extras with `uv pip install
--python .venv -e .[dev]`, and invoke lint and tests through that environment's
Python (`.venv/bin/python` on POSIX, `.venv\Scripts\python` on Windows). That
keeps pytest, Ruff, and the code under test on the same declared dependency set.
