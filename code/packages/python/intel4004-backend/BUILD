uv venv --quiet --clear
uv pip install -e ../simulator-protocol -e ../virtual-machine -e ../interpreter-ir -e ../vm-core -e ../jit-core -e ../intel4004-simulator -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
