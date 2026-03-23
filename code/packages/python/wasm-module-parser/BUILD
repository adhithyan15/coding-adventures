uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ../wasm-leb128 --quiet
uv pip install --no-config --python .venv -e ../wasm-types --quiet
uv pip install --no-config --python .venv -e ../wasm-opcodes --quiet
uv pip install --no-config --python .venv -e ".[dev]" --quiet
uv run --no-project --no-config python -m pytest tests/ -v
