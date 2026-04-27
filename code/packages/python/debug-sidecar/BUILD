uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=debug_sidecar --cov-report=term-missing
