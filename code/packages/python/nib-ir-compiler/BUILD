uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../state-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../nib-lexer -e ../nib-parser -e ../type-checker-protocol -e ../nib-type-checker -e ../compiler-ir -e ../compiler-source-map -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
