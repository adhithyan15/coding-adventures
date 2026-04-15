uv venv --quiet --clear
uv pip install -e ../logic-gates -e ../arithmetic -e ../clock -e ../intel8008-simulator -e ../simulator-protocol -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
