---
category: CI & GitHub Actions
---

# A non-Python package's BUILD must not call ruff, because the Rust and other language runners do not install it

**What went wrong.** #18397 (`latticemico32-simulator`) and #18401
(`zpu-simulator`) are Rust packages whose BUILD files linted their Python
helper scripts with `ruff check` / `ruff format --check`. Every `cargo test`
passed, then the BUILD failed with `sh: 1: ruff: not found`. On main, only
Python packages and programs run ruff, and each installs it into its own venv
first. The Rust build runners have no ruff on `PATH`.

**The fix.** Remove the ruff lines from the non-Python BUILD, keep the script's
own functional check (for example `python3 tools/generate_vectors.py
--check`), and lint the helper locally with ruff before committing.

**What to do differently.** A BUILD file may only call tools its language's CI
runner installs. If a helper must be linted in CI, put it in a Python package
whose BUILD sets up its own venv, rather than calling a bare `ruff` from a
Rust package.
