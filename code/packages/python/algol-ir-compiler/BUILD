uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../state-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../algol-lexer -e ../algol-parser -e ../algol-type-checker -e ../compiler-ir -e .[dev] --quiet
.venv/bin/python -m pytest tests/ -v
