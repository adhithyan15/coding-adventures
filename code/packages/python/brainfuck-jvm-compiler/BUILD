uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../state-machine -e ../virtual-machine -e ../grammar-tools -e ../lexer -e ../parser -e ../brainfuck -e ../compiler-ir -e ../compiler-source-map -e ../brainfuck-ir-compiler -e ../ir-optimizer -e ../jvm-class-file -e ../ir-to-jvm-class-file -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
