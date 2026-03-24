uv venv --quiet --clear
uv pip install -e ../md5 --quiet
uv pip install -e ../sha1 --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
