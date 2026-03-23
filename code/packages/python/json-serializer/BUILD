uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ../directed-graph -e ../state-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../json-lexer -e ../json-parser -e ../json-value -e ".[dev]" --quiet
uv run --no-project --no-config python -m pytest tests/ -v --tb=short --cov=json_serializer --cov-report=term-missing
