uv venv --quiet --clear
uv pip install -e ../register-vm -e ../tetrad-lexer -e ../tetrad-parser -e ../tetrad-type-checker -e ../tetrad-compiler -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
