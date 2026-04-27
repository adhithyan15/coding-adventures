uv venv --quiet --clear
uv pip install -e ../simulator-protocol -e ../jvm-class-file -e ../jvm-bytecode-disassembler -e ../jvm-simulator -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
