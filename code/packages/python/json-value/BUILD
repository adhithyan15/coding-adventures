uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../grammar-tools -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../json-lexer -e ../json-parser -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v --tb=short --cov=json_value --cov-report=term-missing
