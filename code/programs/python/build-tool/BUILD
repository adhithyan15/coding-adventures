uv venv .venv --quiet --no-project
uv pip install --python .venv -e "../../../packages/python/progress-bar" --quiet
uv pip install --python .venv -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
