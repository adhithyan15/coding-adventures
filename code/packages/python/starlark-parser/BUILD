uv venv --quiet --clear
uv pip install -e ../grammar-tools -e ../state-machine -e ../lexer -e ../parser -e ../starlark-lexer -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
