uv venv --quiet --clear
uv pip install -e ../interpreter-ir -e ../vm-core -e ../jit-core -e ".[dev]" --quiet
python3.12 -m pytest tests/ -v --cov=aot_core --cov-report=term-missing
