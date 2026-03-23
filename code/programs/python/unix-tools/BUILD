uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e "../../../packages/python/directed-graph" --quiet
uv pip install --no-config --python .venv -e "../../../packages/python/state-machine" --quiet
uv pip install --no-config --python .venv -e "../../../packages/python/cli-builder" --quiet
uv pip install --no-config --python .venv -e ".[dev]" --quiet
uv run --no-project --no-config python -m pytest tests/ -v
