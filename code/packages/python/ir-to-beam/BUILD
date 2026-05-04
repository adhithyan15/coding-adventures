uv venv --quiet --clear
uv pip install -e ../compiler-ir -e ../beam-opcode-metadata -e ../beam-bytes-decoder -e ../beam-bytecode-encoder -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
