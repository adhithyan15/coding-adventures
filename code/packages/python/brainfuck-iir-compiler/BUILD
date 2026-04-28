uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../state-machine -e ../virtual-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../brainfuck -e ../interpreter-ir -e ../vm-core -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
