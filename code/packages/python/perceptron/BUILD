# build-tool: deps=python/activation-functions,python/matrix,python/loss-functions
uv venv .venv --quiet --no-project --clear --python 3.12
uv pip install --python .venv --no-deps -e ../activation-functions -e ../matrix -e ../loss-functions -e . --quiet
uv pip install --python .venv pytest pytest-cov ruff --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m pytest tests/ -v
