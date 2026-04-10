uv venv --quiet --clear
uv pip install -e ../virtual-machine -e ../lexer -e ../grammar-tools -e ../parser -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
