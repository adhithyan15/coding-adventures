uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../directed-graph --quiet
uv pip install --python .venv -e ../state-machine --quiet
uv pip install --python .venv -e ".[dev]" --quiet
uv pip install --python .venv -e "../cpu-pipeline[dev]" --quiet
uv pip install --python .venv -e "../cache[dev]" --quiet
uv pip install --python .venv -e "../branch-predictor[dev]" --quiet
uv pip install --python .venv -e "../hazard-detection[dev]" --quiet
uv pip install --python .venv -e "../clock[dev]" --quiet
uv pip install --python .venv -e "../cpu-simulator[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
