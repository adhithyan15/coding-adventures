uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../../../packages/python/directed-graph -e ../../../packages/python/state-machine -e ../../../packages/python/cli-builder -e .[dev] --quiet
uv run --no-project python -m pytest tests/ -v
