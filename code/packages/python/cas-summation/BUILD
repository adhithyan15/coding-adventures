uv venv --quiet --clear
uv pip install -e ../symbolic-ir -e ../cas-pattern-matching -e ../cas-substitution -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --tb=short
