uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../grammar-tools -e ../lexer -e ../state-machine -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
