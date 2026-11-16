uv venv --quiet --clear
uv pip install -e . pytest --quiet
.venv/bin/python -m pytest tests/ -v
# The TRIG-REF-1 fixture (code/specs/fixtures/trig-reference-v1) is generated, not
# written. The build tool selects this package when the fixture, its generator, the
# oracle or the validator changes (repository_inputs.go), so these lines are what
# actually run the generator's byte-for-byte --check, its sensitivity gates and the
# validator tests (PHY00a 6.4 step 5). The C/Rust/bc cross-checks run inside the
# tests when gcc, rustc and bc exist (they are skipped, not faked, when absent).
.venv/bin/python ../../../scripts/generate_trig_reference_fixture.py --check --no-external
.venv/bin/python -m unittest discover -s ../../../scripts/tests -p test_trig_reference_fixture.py
.venv/bin/python ../../../scripts/generate_trig_reference_dart_fixtures.py --check
.venv/bin/python -m unittest discover -s ../../../scripts/tests -p test_trig_reference_dart_fixtures.py
