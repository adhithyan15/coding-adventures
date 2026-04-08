uv venv --quiet --clear
uv pip install -e ../sql-lexer -e ../grammar-tools -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../sql-parser -e . --quiet
uv pip install pytest pytest-cov ruff --quiet
.venv/bin/python -m pytest tests/ -v
