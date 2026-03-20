uv venv --quiet --clear
uv pip install -e ../grammar-tools -e ../lexer -e ../parser -e ../virtual-machine -e ../bytecode-compiler -e ../starlark-lexer -e ../starlark-parser -e ../starlark-compiler -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
