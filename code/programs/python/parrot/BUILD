uv venv --quiet --clear
uv pip install -e "../../../packages/python/repl" --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
