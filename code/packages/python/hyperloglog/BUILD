uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../hash-functions --quiet
uv pip install --python .venv -e .[dev] --quiet
uv run --no-project python -m pytest tests/ -v
