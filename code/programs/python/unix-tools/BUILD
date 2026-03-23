uv venv .venv --quiet --no-project
uv pip install --python .venv -e "../../../packages/python/directed-graph" --quiet
uv pip install --python .venv -e "../../../packages/python/state-machine" --quiet
uv pip install --python .venv -e "../../../packages/python/cli-builder" --quiet
uv pip install --python .venv -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
