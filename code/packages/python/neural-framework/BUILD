uv venv --quiet --no-project --clear
uv pip install --no-deps -e ../matrix --quiet
uv pip install --no-deps -e . --quiet
uv pip install pytest pytest-cov ruff --quiet
.venv/bin/python -m ruff check src tests examples
.venv/bin/python -m pytest
.venv/bin/python examples/xor.py
