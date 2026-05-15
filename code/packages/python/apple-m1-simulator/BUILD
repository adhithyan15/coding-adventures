uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../simulator-protocol -e ".[dev]" --quiet
find .venv -name "*.pth" -exec chflags nohidden {} \;
.venv/bin/ruff check src/ tests/
.venv/bin/python -m pytest tests/ -v
