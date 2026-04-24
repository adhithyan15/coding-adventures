uv venv --quiet --clear
uv pip install -e ../register-vm -e ../virtual-machine -e ../simulator-protocol -e ../tetrad-lexer -e ../tetrad-parser -e ../tetrad-type-checker -e ../tetrad-compiler -e ../interpreter-ir -e ../vm-core -e ../jit-core -e ../intel4004-simulator -e ../tetrad-vm -e ../tetrad-jit -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
