uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m pytest tests/ -v --tb=short
.venv/bin/python -m coding_adventures_warrant_oracle verify-v1 --spec ../../../specs/ct-compare.warrant --corpus ../../../specs/fixtures/ct-compare-warrant-v1/corpus.json
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/ct-compare.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/gf256.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/wasm-leb128.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/logic-gates.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/lzw.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/arithmetic.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/x25519.warrant
