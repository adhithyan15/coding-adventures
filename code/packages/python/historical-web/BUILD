set -eu
uv venv .venv --quiet --no-project --clear
uv pip install --python .venv -e ../test-bootstrap --quiet
uv pip install --python .venv -e ../ca-capability-analyzer --quiet
uv pip install --python .venv -e '.[dev]' --quiet
.venv/bin/ca-capability-analyzer --strict src --manifest required_capabilities.json
.venv/bin/python -m ruff check src tests
.venv/bin/python -m pytest
