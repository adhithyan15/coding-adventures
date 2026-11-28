uv venv .venv --quiet --no-project --clear
uv pip install --python .venv -e ../rank-statistics -e ".[dev]"
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m pytest -q
python3 ../../../../code/scripts/microbenchmark-installed-consumer.py python
