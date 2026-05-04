uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../state-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../interpreter-ir -e ../vm-core -e ../twig -e ../compiler-ir -e ../ir-optimizer -e ../jvm-class-file -e ../ir-to-jvm-class-file -e ../jvm-jar-writer -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
