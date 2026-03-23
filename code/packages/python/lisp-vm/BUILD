uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ../virtual-machine -e ../garbage-collector -e ".[dev]" --quiet
uv run --no-project --no-config python -m pytest tests/ -v
