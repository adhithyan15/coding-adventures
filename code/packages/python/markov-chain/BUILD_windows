#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
(cd ../../python/graph && mise exec -- uv pip install -e .)
(cd ../../python/directed-graph && mise exec -- uv pip install -e .)
mise exec -- uv pip install -e ".[dev]"
mise exec -- uv run pytest
