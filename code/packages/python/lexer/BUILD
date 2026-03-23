uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../directed-graph -e ../state-machine -e ../grammar-tools -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
