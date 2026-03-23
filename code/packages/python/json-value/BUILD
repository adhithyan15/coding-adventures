uv venv --quiet --clear
uv pip install -e ../directed-graph -e ../state-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../json-lexer -e ../json-parser -e .[dev] --quiet
uv run python -m pytest tests/ -v --tb=short --cov=json_value --cov-report=term-missing
