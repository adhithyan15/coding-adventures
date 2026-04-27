uv venv --quiet --clear
uv pip install -e ../intel-4004-packager -e ../wasm-leb128 -e ../wasm-types -e ../wasm-module-encoder -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=code_packager --cov-report=term-missing
