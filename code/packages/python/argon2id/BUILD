uv venv --quiet --clear
uv pip install --python .venv/bin/python -e ../blake2b --quiet
uv pip install --python .venv/bin/python -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
