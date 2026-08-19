uv venv --quiet --clear
uv pip install -e ../symbolic-ir -e ../cas-pattern-matching -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest tests/ -v
