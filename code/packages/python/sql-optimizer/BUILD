uv venv --clear
uv pip install -e ../sql-backend -e ../sql-planner -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
