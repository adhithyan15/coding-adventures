uv venv --quiet --clear
uv pip install -e "../../../packages/python/progress-bar" --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
