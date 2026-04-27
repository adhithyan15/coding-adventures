uv venv --quiet --clear
uv pip install -e ../paint-instructions --quiet
uv pip install -e ../barcode-2d --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
