uv venv --quiet --clear
uv pip install -e ../simulator-protocol -e ../clr-pe-file -e ../clr-bytecode-disassembler -e ../clr-simulator -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
