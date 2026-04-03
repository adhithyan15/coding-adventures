uv venv --quiet --clear
uv pip install --no-deps -e ../trig --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
