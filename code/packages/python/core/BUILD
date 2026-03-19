uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
uv pip install -e "../cpu-pipeline[dev]" --quiet
uv pip install -e "../cache[dev]" --quiet
uv pip install -e "../branch-predictor[dev]" --quiet
uv pip install -e "../hazard-detection[dev]" --quiet
uv pip install -e "../clock[dev]" --quiet
uv pip install -e "../cpu-simulator[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
