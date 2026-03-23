uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../directed-graph -e ../grammar-tools -e ../state-machine -e ../lexer -e ../parser -e ../virtual-machine -e ../bytecode-compiler -e ../garbage-collector -e ../lisp-lexer -e ../lisp-parser -e ../lisp-vm -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
