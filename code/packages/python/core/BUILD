uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ../directed-graph --quiet
uv pip install --no-config --python .venv -e ../state-machine --quiet
uv pip install --no-config --python .venv -e ".[dev]" --quiet
uv pip install --no-config --python .venv -e "../cpu-pipeline[dev]" --quiet
uv pip install --no-config --python .venv -e "../cache[dev]" --quiet
uv pip install --no-config --python .venv -e "../branch-predictor[dev]" --quiet
uv pip install --no-config --python .venv -e "../hazard-detection[dev]" --quiet
uv pip install --no-config --python .venv -e "../clock[dev]" --quiet
uv pip install --no-config --python .venv -e "../cpu-simulator[dev]" --quiet
uv run --no-project --no-config python -m pytest tests/ -v
