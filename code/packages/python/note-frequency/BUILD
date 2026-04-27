uv venv --quiet --clear
uv pip install -e . pytest pytest-cov ruff --quiet
.venv/bin/ruff check src/ tests/
.venv/bin/python -m pytest tests/ --cov=note_frequency --cov-report=term-missing -v
