uv venv --quiet --clear
uv pip install -e ../symbolic-ir -e ../cas-factor -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --tb=short
