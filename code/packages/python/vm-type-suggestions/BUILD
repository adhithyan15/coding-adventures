uv venv --quiet --clear
uv pip install -e ../interpreter-ir -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=vm_type_suggestions --cov-report=term-missing
