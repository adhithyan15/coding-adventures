uv venv .venv --quiet --no-project
uv pip install --python .venv -e ".[dev]" --quiet
uv pip install --python .venv -e ../riscv-simulator --quiet
uv pip install --python .venv -e ../cpu-simulator --quiet
uv pip install --python .venv -e ../logic-gates --quiet
uv pip install --python .venv -e ../arithmetic --quiet
uv run --no-project python -m pytest tests/ -v
