uv venv --quiet --clear
uv pip install -e ../sha256 -e ../graph -e ../directed-graph -e ../state-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../json-lexer -e ../json-parser -e ../json-value -e ../json-serializer -e .[dev] --quiet
.venv/bin/python -m pytest tests/ -v --tb=short --cov=coding_adventures_correlation_vector --cov-report=term-missing
