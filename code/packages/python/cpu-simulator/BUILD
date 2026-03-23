uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../logic-gates -e ../arithmetic -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
