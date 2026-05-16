uv venv --quiet --clear --no-project
uv pip install --python .venv -e ../logic-gates -e ../arithmetic -e ../simulator-protocol -e ../intel-8086-simulator -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
