uv venv --quiet --clear
uv pip install -e ../sir-runtime-pairs --quiet
uv pip install -e ../sir-runtime-exceptions --quiet
uv pip install -e .[dev] --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest tests/ -v
