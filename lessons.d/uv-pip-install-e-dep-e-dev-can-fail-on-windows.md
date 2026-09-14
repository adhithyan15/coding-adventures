---
category: Cross-platform & Windows BUILD_windows
---

# `uv pip install -e ../dep -e .[dev]` can fail on Windows

(universal resolution looks at all extras and may try PyPI). Split into two commands: install local deps first with `--no-deps`, then `uv pip install -e .[dev]`. Also explicitly install pytest/ruff/mypy if needed.
