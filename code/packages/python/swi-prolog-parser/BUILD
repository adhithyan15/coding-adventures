uv venv --quiet --clear
uv pip install -e ../symbol-core -e ../logic-core -e ../logic-engine -e ../grammar-tools -e ../graph -e ../directed-graph -e ../state-machine -e ../lexer -e ../parser -e ../prolog-core -e ../prolog-operator-parser -e ../prolog-lexer -e ../prolog-parser -e ../swi-prolog-lexer -e ".[dev]" --quiet
.venv/bin/python -c "import pathlib, platform; [p.unlink() for p in pathlib.Path('.venv/lib').glob('python*/site-packages/coverage/tracer*.so')] if platform.system() == 'Darwin' else None"
.venv/bin/python -m pytest tests/ -v
