uv venv --quiet --clear
uv pip install -e ../register-vm -e ../tetrad-lexer -e ../tetrad-parser -e ../tetrad-type-checker -e ../tetrad-compiler -e ../interpreter-ir -e ../vm-core -e ../jit-core -e ../intel4004-simulator -e ../tetrad-jit -e ../tetrad-vm -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
