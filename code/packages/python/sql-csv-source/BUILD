uv venv --quiet --clear
uv pip install -e ../csv-parser -e ../sql-lexer -e ../grammar-tools -e ../directed-graph -e ../lexer -e ../parser -e ../sql-parser -e ../sql-execution-engine -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
