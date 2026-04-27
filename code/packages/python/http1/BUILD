set -eu
if command -v uv >/dev/null 2>&1; then uv venv --quiet --clear; else python3 -m venv .venv; fi
if command -v uv >/dev/null 2>&1; then uv pip install -e ../http-core --quiet; else .venv/bin/python -m pip install --quiet ../http-core; fi
if command -v uv >/dev/null 2>&1; then uv pip install -e .\[dev\] --quiet; else .venv/bin/python -m pip install --quiet .\[dev\]; fi
if command -v uv >/dev/null 2>&1; then uv run --no-project python -m pytest tests/ -v; else .venv/bin/python -m pytest tests/ -v; fi
