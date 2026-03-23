uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../virtual-machine -e ../garbage-collector -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
