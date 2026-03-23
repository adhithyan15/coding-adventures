uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ../grammar-tools -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../json-lexer -e ../json-parser -e ".[dev]" --quiet
uv run --no-project --no-config python -m pytest tests/ -v --tb=short --cov=json_value --cov-report=term-missing
