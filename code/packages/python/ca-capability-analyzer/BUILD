uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/ruff check src/ tests/
.venv/bin/ruff format --check src/ tests/
.venv/bin/python -m pytest tests/ -v
