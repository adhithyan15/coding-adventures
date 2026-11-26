uv venv .venv --quiet --no-project --clear
uv pip install --python .venv -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m pytest -q
