---
category: Cross-platform & Windows BUILD_windows
---

# No-runtime-dep Python packages on Windows

(e.g. grammar-tools): `uv venv --clear` creates the workspace-root venv; `uv run python -m pytest` re-syncs and removes pytest. Use `python -m venv .venv --clear` + `.venv\Scripts\pip install -e .[dev]` + `.venv\Scripts\python -m pytest`.
