uv venv .venv --python 3.12 --quiet --no-project --clear
uv pip install --python .venv -e .[dev] --quiet
.venv/bin/ruff check src tests
.venv/bin/python -m pytest tests/ -v
