uv venv .venv --quiet --no-project --clear
uv pip install --python .venv ../heap ../huffman-tree ../lzss --quiet
uv pip install --python .venv -e .[dev] --quiet
.venv/bin/python -m pytest tests/ -v
