uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../grammar-tools -e ../lexer -e ../parser -e ../state-machine -e ../nib-lexer -e ../nib-parser -e ../type-checker-protocol -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
