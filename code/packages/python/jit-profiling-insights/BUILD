uv venv --quiet --clear
uv pip install -e ../interpreter-ir -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=jit_profiling_insights --cov-report=term-missing
