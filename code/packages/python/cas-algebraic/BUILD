uv venv --quiet --clear
uv pip install -e ../symbolic-ir -e ../polynomial -e ../grammar-tools -e ../graph -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../macsyma-lexer -e ../macsyma-parser -e ../macsyma-compiler -e ../cas-pattern-matching -e ../cas-substitution -e ../cas-simplify -e ../cas-factor -e ../symbolic-vm -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --tb=short
