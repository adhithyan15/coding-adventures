uv venv --quiet --clear
uv pip install -e ../wasm-leb128 --quiet
uv pip install -e ../wasm-types --quiet
uv pip install -e ../wasm-opcodes --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
