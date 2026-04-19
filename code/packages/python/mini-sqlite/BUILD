#!/usr/bin/env bash
# Install every transitive dep in leaf-to-root order so editable installs
# see all of each other's source trees.
uv venv --quiet --clear
uv pip install \
    -e ../grammar-tools \
    -e ../graph \
    -e ../directed-graph \
    -e ../state-machine \
    -e ../lexer \
    -e ../parser \
    -e ../sql-lexer \
    -e ../sql-parser \
    -e ../sql-backend \
    -e ../sql-planner \
    -e ../sql-optimizer \
    -e ../sql-codegen \
    -e ../sql-vm \
    -e . --quiet
uv pip install pytest pytest-cov ruff --quiet
.venv/bin/python -m pytest tests/
