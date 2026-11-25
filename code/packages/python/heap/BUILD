uv venv .venv --quiet --no-project --clear --seed --python 3.13
uv pip install --python .venv --no-deps -e .[dev] --quiet
uv pip install --python .venv pytest pytest-cov ruff mypy --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m mypy src tests
.venv/bin/python -m pytest tests/ -s -v
.venv/bin/python tests/verify_installed_wheel.py
