---
category: Python
---

# `uv venv` must use `--no-project`

so it creates a package-local `.venv` instead of finding the workspace root. Pattern: `uv venv .venv --quiet --no-project` then `uv pip install --python .venv ... --quiet` then `uv run --no-project python -m pytest`.
