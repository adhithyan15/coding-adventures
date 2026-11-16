## Unreleased

- `tests/warrant_evidence.py` re-runs itself under a fresh private `sys.pycache_prefix` (`isolate_bytecode_cache`) before importing anything from the checkout, so a forged `__pycache__/*.pyc` in the tree is never executed, whatever ran before (VER04i 8.1). New `tests/test_bytecode_isolation.py` proves it with a forged unchecked-hash pyc (and a control showing the forgery works without isolation). `evidence/test-manifest.json` and `evidence/domain-labels.json` regenerated: the manifest gains `pycache_isolated: true` and `runner.src_sha256` moved. Nothing else changed.
