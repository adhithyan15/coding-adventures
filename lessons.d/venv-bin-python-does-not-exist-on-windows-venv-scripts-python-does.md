---
category: Cross-platform & Windows BUILD_windows
---

# `.venv/bin/python` does not exist on Windows; `.venv/Scripts/python` does

In `BUILD_windows`, always use `.venv\Scripts\python` (BACKSLASHES — `cmd.exe` parses `/` as a switch and `.venv/Scripts/python` becomes command `.venv` with option `/Scripts/python`). Cross-platform alternative: `uv run --no-project python`.
