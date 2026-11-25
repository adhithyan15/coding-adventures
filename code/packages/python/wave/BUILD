uv venv --quiet --clear
uv pip install --no-deps -e ../trig --quiet
uv pip install --no-deps -e . --quiet
uv pip install pytest pytest-cov ruff --quiet
uv run --no-project ruff check src/ tests/
uv run --no-project python -m pytest tests/ -v
