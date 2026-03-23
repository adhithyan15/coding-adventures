uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ../grammar-tools -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../virtual-machine -e ../bytecode-compiler -e ../starlark-lexer -e ../starlark-parser -e ../starlark-ast-to-bytecode-compiler -e ../starlark-vm -e ".[dev]" --quiet
uv run --no-project --no-config python -m pytest tests/ -v
