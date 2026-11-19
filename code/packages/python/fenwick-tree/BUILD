uv venv .venv --quiet --no-project
# build-tool: deps=python/test-bootstrap
uv pip install --python .venv --no-deps -e ../test-bootstrap --quiet
uv pip install --python .venv -e .[dev] --quiet
uv run --no-project python -m pytest tests/ -v
uv run --no-project python tool/mutation_check.py
