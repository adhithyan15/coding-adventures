uv venv --quiet --clear
uv pip install -e ../compiler-ir -e ../interpreter-ir -e ../ir-optimizer -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=codegen_core --cov-report=term-missing
