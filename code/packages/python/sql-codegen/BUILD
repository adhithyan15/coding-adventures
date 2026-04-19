#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Virtualenv + leaf-to-root transitive deps (coding-adventures: each package
# is installed editable; order matters because the codegen package imports
# from both the planner and the optimizer).
uv venv --clear --quiet
uv pip install -e ../sql-backend --quiet
uv pip install -e ../sql-planner --quiet
uv pip install -e ../sql-optimizer --quiet
uv pip install -e ".[dev]" --quiet

.venv/bin/ruff check src/ tests/
.venv/bin/python -m pytest tests/
