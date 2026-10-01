# build-tool: deps=typescript/forme-plugin-runner-conformance
(cd ../../typescript/document-ast && npm ci --quiet)
(cd ../../typescript/forme-types && npm ci --quiet && npm run build)
(cd ../../typescript/forme-errors && npm ci --quiet && npm run build)
(cd ../../typescript/forme-capability && npm ci --quiet)
(cd ../../typescript/forme-stage && npm ci --quiet && npm run build)
(cd ../../typescript/forme-plugin-runner-ts && npm ci --quiet && npm run build)
(cd ../../typescript/forme-plugin-runner-conformance && npm ci --quiet && npm run build)
uv venv .venv --quiet --no-project --clear --python 3.13
uv pip install --python .venv -e .[dev] --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m mypy src
.venv/bin/python -m coverage erase
.venv/bin/python -m coverage run --parallel-mode -m pytest tests -v
node tests/conformance.mjs .venv/bin/python
.venv/bin/python -m coverage combine
.venv/bin/python -m coverage report --fail-under=85
.venv/bin/python -m coverage erase
