uv venv .venv --quiet --no-project --clear --python 3.13
uv pip install --python .venv -e ../hash-functions --quiet
uv pip install --python .venv --no-deps -e ../hyperloglog -e ../in-memory-data-store-protocol -e ../resp-protocol --quiet
uv pip install --python .venv --no-deps -e ../in-memory-data-store-engine --quiet
uv pip install --python .venv -e .[dev] --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m mypy --strict --follow-untyped-imports src tests
.venv/bin/python -m pytest tests/ -v
