uv venv --quiet --clear
uv pip install -e ../compiler-ir -e ../interpreter-ir -e ../ir-optimizer -e ../codegen-core -e ../simulator-protocol -e ../virtual-machine -e ../vm-core -e ../intel4004-simulator -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
