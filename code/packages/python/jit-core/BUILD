uv venv --quiet --clear
uv pip install -e ../interpreter-ir -e ../vm-core -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=jit_core --cov-report=term-missing
