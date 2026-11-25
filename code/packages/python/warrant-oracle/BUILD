uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m pytest tests/ -v --tb=short
# CV2-S5b (VER04r 11, R23): the committed v2 conformance kit.  make_conformance.py --check rebuilds the seven v2-*.json
# files from the case tables and the oracle (it derives the 2**24 suite, about a minute, once) and requires the
# committed bytes and the v2 rows of index.json to be identical.  The v1 checker indexes and structurally checks the
# same files without importing the oracle, and its own tests are run here too: none of the three ran in CI before.
.venv/bin/python tests/fixtures/conformance-v2/make_conformance.py --check --fixture ../../../specs/fixtures/warrant-corpus-conformance-v1
.venv/bin/python ../../../scripts/warrant_corpus_conformance.py --fixture ../../../specs/fixtures/warrant-corpus-conformance-v1
.venv/bin/python -m unittest discover -s ../../../scripts/tests -p test_warrant_corpus_conformance.py
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
# CV2 S10 adopter (x25519): the committed x25519 v2 fixture, pinned and profile-checked as above.
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 ../../../specs/fixtures/x25519-warrant-v2 --warrant ../../../specs/x25519.warrant --profile ../../../specs/fixtures/x25519-warrant-v2/corpus.profile --bindings ../../rust/x25519/evidence.bindings --expect-generator warrant-corpus-gen@0.6.0
# CV2 S10 adopter (lzw): the committed lzw v2 fixture (six omitted examples, max_ir_cost 16384); the full derive
# of its 17,563,819 cases takes about 20 s, so one line suffices (VER04r 16.10 [D33]).
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 ../../../specs/fixtures/lzw-warrant-v2 --warrant ../../../specs/lzw.warrant --profile ../../../specs/fixtures/lzw-warrant-v2/corpus.profile --bindings ../../rust/lzw/evidence.bindings --expect-generator warrant-corpus-gen@0.6.0
# CV2 S10 adopter (wasm-leb128): the committed wasm-leb128 v2 fixture (44 suites, 20 faults); the full derive of
# its 46,566,930 cases takes 4 min 53 s on one core (15 times lzw's), the most expensive line of this package.
# One line, not sharded (VER04r 16.11 [D38]); sharding by chunk (D33) is the documented remedy if it stops fitting.
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 ../../../specs/fixtures/wasm-leb128-warrant-v2 --warrant ../../../specs/wasm-leb128.warrant --profile ../../../specs/fixtures/wasm-leb128-warrant-v2/corpus.profile --bindings ../../rust/wasm-leb128/evidence.bindings --expect-generator warrant-corpus-gen@0.6.0
# LGE-S1c (logic-gates-basic): the committed cross-lane basic-gates v2 fixture (590,350 cases), pinned as above; no --bindings (LG02 section 5.1).
.venv/bin/python -m coding_adventures_warrant_oracle verify-v2 ../../../specs/fixtures/logic-gates-basic-warrant-v2 --warrant ../../../specs/logic-gates-basic.warrant --profile ../../../specs/fixtures/logic-gates-basic-warrant-v2/corpus.profile --expect-generator warrant-corpus-gen@0.6.0
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
# CV2-S10o-1: the synthetic off-by-one-consumed document (VER04r 8.2, D36) and its committed faults and
# refusals dumps, the files the generator's slice S10o-2 must reproduce byte for byte.
.venv/bin/python tests/fixtures/off-by-one-consumed-v1/make_off_by_one_consumed.py --check
# CV2-S11a: the synthetic ir.data document (VER04r Appendix A.2: records, variants, Option, some / none / ctor /
# field, match on every scrutinee with the data patterns, the intermediate use of unwrap_or) and its committed
# dumps (lowering, refusals), the files the generator's slice S11b must reproduce byte for byte.  The directory is
# new on purpose: the generator's tuple_parity.rs and fixed_parity.rs reproduce the older directories exactly.
.venv/bin/python tests/fixtures/data-forms-v1/make_data_forms.py --check
.venv/bin/python -m coding_adventures_warrant_oracle dump-lowering --spec tests/fixtures/data-forms-v1/data_forms_v1.warrant --check tests/fixtures/data-forms-v1/data_forms_v1.oracle-lowering.json
# CV2-S11f-1: the synthetic scalar-faults document (saturate-add, saturate-sub, shift-mask, scalar missing-error;
# VER04r 8.2) and its committed faults and refusals dumps, the files the generator's slice S11f-2 must reproduce.
.venv/bin/python tests/fixtures/scalar-faults-v1/make_scalar_faults.py --check
# CV2-S11c-1: the synthetic ir.data EVALUATION document (records, variants and Option evaluated over their domains, data
# values in examples, field-zero and the record, Option and variant payloads of missing-error) and its committed dumps
# (lowering, evaluation, faults under the gate before and after the one this slice opens, values, domains, refusals), the
# files the generator's slice S11c-2 must reproduce byte for byte.  The directory is new on purpose: data-forms-v1 is what
# the generator's S11b parity test reproduces.  --check re-derives the whole document (about 25 s).
.venv/bin/python tests/fixtures/data-eval-v1/make_data_eval.py --check
.venv/bin/python -m coding_adventures_warrant_oracle dump-lowering --spec tests/fixtures/data-eval-v1/data_eval_v1.warrant --check tests/fixtures/data-eval-v1/data_eval_v1.oracle-lowering.json
.venv/bin/python ../../rust/warrant-corpus-gen/tests/fixtures/dump_oracle_lowering.py --check
.venv/bin/python ../../rust/warrant-corpus-gen/tests/fixtures/dump_oracle_enumeration.py --check
