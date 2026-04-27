uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=interpreter_ir --cov-report=term-missing
