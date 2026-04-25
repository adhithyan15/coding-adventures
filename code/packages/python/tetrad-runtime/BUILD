uv venv --quiet --clear
uv pip install -e ../tetrad-lexer -e ../tetrad-parser -e ../tetrad-type-checker -e ../tetrad-compiler -e ../interpreter-ir -e ../vm-core -e ../jit-core -e ../intel4004-simulator -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
