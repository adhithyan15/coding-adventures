uv venv --quiet --clear
uv pip install -e ../compiler-ir -e ../intel-4004-ir-validator -e ../interpreter-ir -e ../ir-optimizer -e ../codegen-core -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
