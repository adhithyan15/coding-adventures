uv venv --quiet --clear
uv pip install -e ../sql-lexer -e ../grammar-tools -e ../directed-graph -e ../lexer -e ../parser -e ../sql-parser -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
