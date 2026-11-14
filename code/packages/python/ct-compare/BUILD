uv venv --quiet --clear
uv pip install -e ../warrant-corpus -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --tb=short
.venv/bin/python tests/warrant_evidence.py check evidence/test-manifest.json
.venv/bin/python tests/warrant_evidence.py sabotage
.venv/bin/python -m coding_adventures_warrant_corpus validate-manifest evidence/test-manifest.json --repo-root ../../../.. --corpus ../../../specs/fixtures/ct-compare-warrant-v1 --registry ../../../specs/fixtures/ct-compare-warrant-v1/consumers.json
