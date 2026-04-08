uv venv --quiet --clear
uv pip install -e ../directed-graph -e ../grammar-tools -e ../state-machine -e ../lexer -e ../mosaic-lexer -e ../parser -e ../mosaic-parser -e ../mosaic-analyzer -e ../mosaic-vm -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
