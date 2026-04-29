uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../state-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../interpreter-ir -e ../vm-core -e ../twig -e ../compiler-ir -e ../ir-optimizer -e ../beam-opcode-metadata -e ../beam-bytecode-encoder -e ../ir-to-beam -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
