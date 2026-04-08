uv venv --quiet --clear
uv pip install -e ../directed-graph --quiet
uv pip install -e ../grammar-tools --quiet
uv pip install -e ../state-machine --quiet
uv pip install -e ../lexer --quiet
uv pip install -e ../lattice-lexer --quiet
uv pip install -e ../parser --quiet
uv pip install -e ../lattice-parser --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
