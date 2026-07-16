uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../hash-functions --quiet
uv pip install --python .venv --no-deps -e ../hyperloglog -e ../in-memory-data-store-protocol -e ../resp-protocol --quiet
uv pip install --python .venv --no-deps -e ../in-memory-data-store-engine --quiet
uv pip install --python .venv --no-deps -e ".[dev]" --quiet
uv pip install --python .venv pytest pytest-cov ruff mypy --quiet
uv run --no-project python -m pytest tests/ -v
