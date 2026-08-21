uv venv .venv --quiet --no-project --clear --python 3.13
uv pip install --python .venv ../pixel_container --quiet
uv pip install --python .venv ../lzss --quiet
uv pip install --python .venv ../zip --quiet
uv pip install --python .venv -e .[dev] --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m mypy src tests
.venv/bin/python -m pytest tests -v

