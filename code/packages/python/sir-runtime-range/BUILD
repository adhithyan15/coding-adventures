uv venv --quiet --clear
uv pip install -e .[dev] --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest tests/ -v
