uv venv --quiet --clear
uv pip install --no-deps -e ../../../packages/python/progress-bar --quiet
uv pip install --no-deps -e . --quiet
uv pip install pytest pytest-cov ruff mypy --quiet
uv run --no-project python -m pytest tests/ -v
