uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../grammar-tools -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../ruby-lexer -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
