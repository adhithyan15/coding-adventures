uv venv --quiet --clear
uv pip install -e ../grammar-tools -e ../state-machine -e ../lexer -e ../parser -e ../virtual-machine -e ../bytecode-compiler -e ../garbage-collector -e ../lisp-lexer -e ../lisp-parser -e ../lisp-vm -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
