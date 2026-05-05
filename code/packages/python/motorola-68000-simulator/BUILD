uv venv --quiet --clear --no-project
uv pip install --python .venv -e ../simulator-protocol -e .[dev] --quiet
.venv/bin/python -m pytest tests/ -v
