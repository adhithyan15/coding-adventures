uv venv --quiet --no-project --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest
.venv/bin/python -m ruff check src/ tests/
