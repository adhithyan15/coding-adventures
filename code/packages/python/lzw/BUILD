uv venv .venv --quiet --no-project --clear
uv pip install --python .venv -e . --quiet
uv pip install --python .venv pytest pytest-cov ruff mypy --quiet
.venv/bin/python -m pytest tests/ -v
