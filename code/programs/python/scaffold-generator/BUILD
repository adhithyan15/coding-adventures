uv venv --quiet --clear
uv pip install -e ../../../packages/python/directed-graph -e ../../../packages/python/state-machine -e ../../../packages/python/cli-builder -e .[dev] --quiet
.venv/bin/python -m pytest tests/ -v
