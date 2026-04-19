#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Virtualenv + leaf-to-root transitive deps. Every transitive `file:` dep must
# be installed editable in order — the VM package sits at the top of the SQL
# pipeline, so everything below must be installed first.
uv venv --clear --quiet
uv pip install -e ../sql-backend --quiet
uv pip install -e ../sql-planner --quiet
uv pip install -e ../sql-optimizer --quiet
uv pip install -e ../sql-codegen --quiet
uv pip install -e ".[dev]" --quiet

.venv/bin/ruff check src/ tests/
.venv/bin/python -m pytest tests/
