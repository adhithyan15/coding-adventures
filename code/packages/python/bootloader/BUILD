uv venv --quiet --clear
uv pip install -e ../riscv-simulator -e ../cpu-simulator -e ../logic-gates -e ../arithmetic -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
