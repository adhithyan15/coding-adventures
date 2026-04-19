uv venv --quiet --clear
uv pip install -e ../grammar-tools -e ../graph -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../dartmouth-basic-lexer -e ../dartmouth-basic-parser -e ../compiler-ir -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
