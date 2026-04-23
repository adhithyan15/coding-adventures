uv venv --quiet --clear
uv pip install -e ../graph -e ../directed-graph -e ../grammar-tools -e ../lexer -e ../parser -e ../state-machine -e ../type-checker-protocol -e ../oct-lexer -e ../oct-parser -e ../oct-type-checker -e ../compiler-ir -e ../oct-ir-compiler -e ../jvm-class-file -e ../ir-to-jvm-class-file -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
