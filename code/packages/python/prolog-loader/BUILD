uv venv --quiet --clear
uv pip install -e ../symbol-core -e ../logic-core -e ../logic-engine -e ../lexer -e ../parser -e ../grammar-tools -e ../graph -e ../directed-graph -e ../state-machine -e ../prolog-core -e ../prolog-lexer -e ../iso-prolog-lexer -e ../swi-prolog-lexer -e ../prolog-parser -e ../prolog-operator-parser -e ../iso-prolog-parser -e ../swi-prolog-parser -e ../logic-builtins -e ".[dev]" --quiet
.venv/bin/python -c "import pathlib, platform; [p.unlink() for p in pathlib.Path('.venv/lib').glob('python*/site-packages/coverage/tracer*.so')] if platform.system() == 'Darwin' else None"
.venv/bin/python -m pytest tests/ -v
