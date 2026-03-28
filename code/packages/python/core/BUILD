uv venv --quiet --clear
uv pip install --no-deps -e ../directed-graph --quiet
uv pip install --no-deps -e ../state-machine --quiet
uv pip install --no-deps -e ../branch-predictor --quiet
uv pip install --no-deps -e ../hazard-detection --quiet
uv pip install --no-deps -e ../clock --quiet
uv pip install --no-deps -e ../cache --quiet
uv pip install --no-deps -e ../arithmetic --quiet
uv pip install --no-deps -e ../cpu-simulator --quiet
uv pip install --no-deps -e ../cpu-pipeline --quiet
uv pip install --no-deps -e .[dev] --quiet
uv pip install pytest pytest-cov ruff mypy --quiet
uv run --no-project python -m pytest tests/ -v
