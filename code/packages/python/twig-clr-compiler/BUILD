uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../state-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../interpreter-ir -e ../vm-core -e ../twig -e ../compiler-ir -e ../ir-optimizer -e ../cil-bytecode-builder -e ../ir-to-cil-bytecode -e ../cli-assembly-writer -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
