uv venv .venv --quiet --no-project --no-config
uv pip install --no-config --python .venv -e ".[dev]" --quiet
uv pip install --no-config --python .venv -e ../riscv-simulator --quiet
uv pip install --no-config --python .venv -e ../cpu-simulator --quiet
uv pip install --no-config --python .venv -e ../logic-gates --quiet
uv pip install --no-config --python .venv -e ../arithmetic --quiet
uv run --no-project --no-config python -m pytest tests/ -v
