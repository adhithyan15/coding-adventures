uv venv -q .venv
uv pip install -q -e ../json-rpc -e .[dev]
.venv/bin/python -m pytest tests/ -v --tb=short --cov=ls00 --cov-report=term-missing
