uv venv --quiet --clear
uv pip install -e ../grammar-tools -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../virtual-machine -e ../bytecode-compiler -e ../starlark-lexer -e ../starlark-parser -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
