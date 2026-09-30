uv venv .venv --quiet --no-project --clear --python 3.12
uv pip install --python .venv --no-deps -e . --quiet
uv pip install --python .venv pytest pytest-cov ruff --quiet
.venv/bin/python -m ruff check loss_functions.py tests
.venv/bin/python -m ruff format --check loss_functions.py tests
.venv/bin/python -m pytest tests/ -v
