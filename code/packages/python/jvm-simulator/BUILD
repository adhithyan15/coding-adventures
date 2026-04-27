uv venv --quiet --clear
uv pip install -e ../simulator-protocol -e ../jvm-bytecode-disassembler -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
