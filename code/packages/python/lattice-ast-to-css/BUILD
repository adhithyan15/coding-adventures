uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ../directed-graph --quiet
uv pip install --no-config --python .venv -e ../grammar-tools --quiet
uv pip install --no-config --python .venv -e ../lexer --quiet
uv pip install --no-config --python .venv -e ../lattice-lexer --quiet
uv pip install --no-config --python .venv -e ../parser --quiet
uv pip install --no-config --python .venv -e ../lattice-parser --quiet
uv pip install --no-config --python .venv -e ".[dev]" --quiet
uv run --no-project --no-config python -m pytest tests/ -v
