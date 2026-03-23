uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ../../../packages/python/directed-graph -e ../../../packages/python/state-machine -e ../../../packages/python/cli-builder -e .[dev] --quiet
uv run --no-project --no-config python -m pytest tests/ -v
