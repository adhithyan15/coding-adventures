uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m pytest tests/ -v --tb=short
.venv/bin/python -m coding_adventures_warrant_oracle verify-v1 --spec ../../../specs/ct-compare.warrant --corpus ../../../specs/fixtures/ct-compare-warrant-v1/corpus.json
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 tests/fixtures/synth-v2/corpus --warrant tests/fixtures/synth-v2/synth.warrant --profile tests/fixtures/synth-v2/corpus.profile --expect-generator warrant-corpus-gen@0.6.0
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 tests/fixtures/accept-blind-v2/corpus --warrant tests/fixtures/accept-blind-v2/blind.warrant --profile tests/fixtures/accept-blind-v2/corpus.profile
# CV2-S10-1: the omit_example fixture (three omitted examples, both reasons); make_fixture.py
# regenerates corpus/ from the oracle, verify-v2 gates it, and the generator's parity PR must match it.
.venv/bin/python tests/fixtures/omit-example-v2/make_fixture.py --check
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 tests/fixtures/omit-example-v2/corpus --warrant tests/fixtures/omit-example-v2/omit.warrant --profile tests/fixtures/omit-example-v2/corpus.profile --expect-generator warrant-corpus-gen@0.6.0
# CV2-S3d: the committed ct-compare v2 fixture. --expect-generator pins the generator name and
# version that corpus.json records ([D13], VER04r 16.8): a generator bump fails here until the
# fixture is regenerated and this pin is reviewed with it. --profile is the fixture's own
# hand-written corpus.profile, so an edit to it moves profile_sha256 and fails ([D14]).
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 ../../../specs/fixtures/ct-compare-warrant-v2 --warrant ../../../specs/ct-compare.warrant --profile ../../../specs/fixtures/ct-compare-warrant-v2/corpus.profile --bindings ../../rust/ct-compare/evidence.bindings --expect-generator warrant-corpus-gen@0.6.0
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/ct-compare.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/gf256.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/wasm-leb128.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/logic-gates.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/lzw.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/arithmetic.warrant
.venv/bin/python -m coding_adventures_warrant_oracle check --spec ../../../specs/x25519.warrant
# CV2-S10a: the synthetic ir.fixed document (every built-in at every width) and its committed
# lowering dump, the file the generator's slice S10b must reproduce byte for byte.
.venv/bin/python tests/fixtures/fixed-forms-v1/make_fixed_forms.py --check
.venv/bin/python -m coding_adventures_warrant_oracle dump-lowering --spec tests/fixtures/fixed-forms-v1/fixed_forms_v1.warrant --check tests/fixtures/fixed-forms-v1/fixed_forms_v1.oracle-lowering.json
# CV2-S10t-1: the synthetic ir.tuple document and its committed dumps (lowering, evaluation, faults,
# refusals), the files the generator's slice S10t-2 must reproduce byte for byte.  make_tuple_forms.py
# --check regenerates all six files; dump-lowering checks the lowering dump through the CLI too.
.venv/bin/python tests/fixtures/tuple-forms-v1/make_tuple_forms.py --check
.venv/bin/python -m coding_adventures_warrant_oracle dump-lowering --spec tests/fixtures/tuple-forms-v1/tuple_forms_v1.warrant --check tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-lowering.json
# CV2-S10c-1: the synthetic ir.fixed EVALUATION document and its committed dumps (lowering, evaluation, faults
# under the closed and a test-only open gate, the value table of every built-in at every width, and the
# domain table of D21/D25/D26), the files the generator's slice S10c-2 must reproduce byte for byte.
.venv/bin/python tests/fixtures/fixed-eval-v1/make_fixed_eval.py --check
.venv/bin/python -m coding_adventures_warrant_oracle dump-lowering --spec tests/fixtures/fixed-eval-v1/fixed_eval_v1.warrant --check tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-lowering.json
.venv/bin/python ../../rust/warrant-corpus-gen/tests/fixtures/dump_oracle_lowering.py --check
.venv/bin/python ../../rust/warrant-corpus-gen/tests/fixtures/dump_oracle_enumeration.py --check
