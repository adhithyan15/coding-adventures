uv venv --quiet --clear
uv pip install -e ../compiler-ir -e ../simulator-protocol -e ../ge225-simulator -e ../interpreter-ir -e ../ir-optimizer -e ../codegen-core -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
