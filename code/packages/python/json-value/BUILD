uv venv --quiet --clear
uv pip install -e ../grammar-tools -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../json-lexer -e ../json-parser -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --tb=short --cov=json_value --cov-report=term-missing
