uv venv --quiet --clear
uv pip install -e ../simulator-protocol -e ../clr-pe-file -e ../clr-bytecode-disassembler -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
