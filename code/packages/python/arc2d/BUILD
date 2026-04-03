uv venv --quiet --clear
uv pip install --no-deps -e ../trig --quiet
uv pip install --no-deps -e ../point2d --quiet
uv pip install --no-deps -e ../bezier2d --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
