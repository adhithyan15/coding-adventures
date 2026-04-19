uv venv --quiet --clear
uv pip install -e ../grammar-tools -e ../graph -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../macsyma-lexer -e ../macsyma-parser -e ../symbolic-ir -e ../polynomial -e ../macsyma-compiler -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
