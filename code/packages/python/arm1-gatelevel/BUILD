uv venv --quiet --clear
uv pip install -e ../clock -e ../logic-gates -e ../arithmetic -e ../arm1-simulator -e ".[dev]" --quiet
uv run python -m pytest tests/ -v
