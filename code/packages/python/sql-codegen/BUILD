uv venv --clear --quiet
uv pip install -e ../sql-backend -e ../sql-planner -e ../sql-optimizer -e ".[dev]" --quiet
.venv/bin/ruff check src/ tests/
.venv/bin/python -m pytest tests/
