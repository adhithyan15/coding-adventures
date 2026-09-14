---
category: Cross-platform & Windows BUILD_windows
---

# `.[dev]` quoting on Windows

`cmd /C` passes `"..."` literally to uv: `uv pip install -e ".[dev]"` fails with "not a valid editable requirement". Use unquoted `-e .[dev]` (with `-e`, no quotes). Dropping `-e` does a non-editable install which breaks `__file__`-based path walks (Windows site-packages depth differs from Linux).
