uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../directed-graph --quiet
uv pip install --python .venv -e ../grammar-tools --quiet
uv pip install --python .venv -e ../lexer --quiet
uv pip install --python .venv -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
