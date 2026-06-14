uv venv --quiet --clear
uv pip install -e ../compiler-ir -e ../interpreter-ir -e ../ir-optimizer -e ../codegen-core -e ../vm-core -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=jit_core --cov-report=term-missing
