uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ../logic-gates -e ../arithmetic -e ../cpu-simulator -e ".[dev]" --quiet
uv run --no-project --no-config python -m pytest tests/ -v
