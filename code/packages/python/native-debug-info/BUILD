uv venv --quiet --clear
uv pip install -e ../debug-sidecar -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=native_debug_info --cov-report=term-missing
