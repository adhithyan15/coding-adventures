uv venv --quiet --clear
uv pip install -e "../../../packages/python/directed-graph" --quiet
uv pip install -e "../../../packages/python/state-machine" --quiet
uv pip install -e "../../../packages/python/cli-builder" --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
