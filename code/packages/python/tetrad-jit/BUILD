uv venv --quiet --clear
uv pip install \
    -e ../register-vm \
    -e ../virtual-machine \
    -e ../simulator-protocol \
    -e ../intel4004-simulator \
    -e ../tetrad-lexer \
    -e ../tetrad-parser \
    -e ../tetrad-type-checker \
    -e ../tetrad-compiler \
    -e ../tetrad-vm \
    -e ".[dev]" \
    --quiet
.venv/bin/python -m pytest tests/ -v --cov=tetrad_jit --cov-report=term-missing
