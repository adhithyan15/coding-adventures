# coding-adventures-in-memory-data-store-protocol

Protocol intermediate representation for the in-memory data store.

The package mirrors the Rust `in-memory-data-store-protocol` contract with
command frames, ASCII command normalization, and engine response variants.

## Development

From this package directory, the canonical BUILD recipe is:

```bash
uv venv .venv --quiet --no-project --clear --python 3.13
uv pip install --python .venv -e .[dev] --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m mypy --strict src tests
.venv/bin/python -m pytest tests/ -v --tb=short
```

On Windows, run each line in `BUILD_windows`. Both recipes clear and recreate
the package-local `.venv`, then invoke every tool through that environment's
interpreter. Repeating a build is therefore safe even after an interrupted
install.
