uv venv --quiet --clear
uv pip install -e ../csv-parser -e ../sql-lexer -e ../grammar-tools -e ../directed-graph -e ../lexer -e ../parser -e ../sql-parser -e ../sql-execution-engine -e . --quiet
uv pip install pytest pytest-cov ruff --quiet
.venv/bin/python -m pytest tests/ -v
