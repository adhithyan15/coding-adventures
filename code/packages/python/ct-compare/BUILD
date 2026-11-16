uv venv --quiet --clear
uv pip install -e ../warrant-corpus -e ".[dev]" --quiet
.venv/bin/python tests/warrant_evidence.py exec -m pytest tests/ -v --tb=short
.venv/bin/python tests/warrant_evidence.py check evidence/test-manifest.json
.venv/bin/python tests/warrant_evidence.py sabotage
.venv/bin/python tests/warrant_evidence.py validate-manifest evidence/test-manifest.json --registry ../../../specs/fixtures/ct-compare-warrant-v1/consumers.json
# needs-toolchain: rust
# WD-004b2 (VER04i 8.3): the Rust loader re-checks the lane (digests, source freshness, labels);
# --require-active makes an absent, planned or unloadable lane a build failure.
cargo run --quiet --manifest-path ../../rust/ca-assurance-report/Cargo.toml -- --check-lane python/ct-compare --repo-root ../../../.. --require-active
