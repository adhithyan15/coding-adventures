uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m pytest tests/ -v --tb=short
.venv/bin/python -m coding_adventures_warrant_oracle verify-v1 --spec ../../../specs/ct-compare.warrant --corpus ../../../specs/fixtures/ct-compare-warrant-v1/corpus.json
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 tests/fixtures/synth-v2/corpus --warrant tests/fixtures/synth-v2/synth.warrant --profile tests/fixtures/synth-v2/corpus.profile
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 tests/fixtures/accept-blind-v2/corpus --warrant tests/fixtures/accept-blind-v2/blind.warrant --profile tests/fixtures/accept-blind-v2/corpus.profile
# CV2-S3d: the committed ct-compare v2 fixture. --expect-generator pins the generator name and
# version that corpus.json records ([D13], VER04r 16.8): a generator bump fails here until the
# fixture is regenerated and this pin is reviewed with it. --profile is the fixture's own
# hand-written corpus.profile, so an edit to it moves profile_sha256 and fails ([D14]).
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 ../../../specs/fixtures/ct-compare-warrant-v2 --warrant ../../../specs/ct-compare.warrant --profile ../../../specs/fixtures/ct-compare-warrant-v2/corpus.profile --bindings ../../rust/ct-compare/evidence.bindings --expect-generator warrant-corpus-gen@0.4.0
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/ct-compare.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/gf256.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/wasm-leb128.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/logic-gates.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/lzw.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/arithmetic.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/x25519.warrant
.venv/bin/python ../../rust/warrant-corpus-gen/tests/fixtures/dump_oracle_lowering.py --check
.venv/bin/python ../../rust/warrant-corpus-gen/tests/fixtures/dump_oracle_enumeration.py --check
