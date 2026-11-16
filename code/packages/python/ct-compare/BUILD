uv venv --quiet --clear
uv pip install -e ../warrant-corpus -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --tb=short
.venv/bin/python tests/warrant_evidence.py check evidence/test-manifest.json
.venv/bin/python tests/warrant_evidence.py sabotage
.venv/bin/python -m coding_adventures_warrant_corpus validate-manifest evidence/test-manifest.json --repo-root ../../../.. --corpus ../../../specs/fixtures/ct-compare-warrant-v1 --registry ../../../specs/fixtures/ct-compare-warrant-v1/consumers.json
# needs-toolchain: rust
# WD-004b2 (VER04i 8.3): the Rust loader re-checks the lane (digests, source freshness, labels);
# --require-active makes an absent, planned or unloadable lane a build failure.
# The loader refuses bytecode (Python runs a .pyc instead of its source, VER04i 8.1), so remove the
# caches the steps above created in this package and in the runner before the check.
.venv/bin/python -c "import pathlib,shutil;[shutil.rmtree(p) for r in ('src','../warrant-corpus/src') for p in list(pathlib.Path(r).rglob('__pycache__'))]"
cargo run --quiet --manifest-path ../../rust/ca-assurance-report/Cargo.toml -- --check-lane python/ct-compare --repo-root ../../../.. --require-active
