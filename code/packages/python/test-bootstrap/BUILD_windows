uv venv --quiet --clear
uv pip install -e . --quiet
uv pip install pytest pytest-cov ruff --quiet
uv run --no-project ruff check src/ tests/
uv run --no-project python -m pytest tests/ -v --cov=coding_adventures_test_bootstrap --cov-fail-under=95
