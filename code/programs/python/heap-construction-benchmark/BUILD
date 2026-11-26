uv venv .venv --quiet --no-project --clear --python 3.13
uv pip install --python .venv -e ../../../packages/python/rank-statistics -e ../../../packages/python/microbenchmark -e ../../../packages/python/heap -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m mypy src tests
.venv/bin/python -m pytest tests/ -v
.venv/bin/python tests/verify_cli_smoke.py
