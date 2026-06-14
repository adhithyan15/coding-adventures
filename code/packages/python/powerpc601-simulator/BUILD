uv venv --quiet --clear
uv pip install -e ../simulator-protocol -e ".[dev]" --quiet
.venv/bin/ruff check src/ tests/
.venv/bin/python -m pytest tests/ -v
