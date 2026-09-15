# Adding shared build-tool fixtures must update the pinned corpus-summary tests

Adding three valid conformance cases changed `validate-corpus` from 38 to 41,
but the fixture-specific validation command still passed because it reports the
new count rather than asserting it. CI later failed two
`test_build_tool_conformance_runner.py` assertions that deliberately pin the
checked-in corpus size. Whenever a shared case is added or removed, update both
the direct `validate_corpus` summary assertion and the CLI machine-readable
summary assertion, then run the full conformance-runner test module—not only
`build_tool_conformance.py validate-corpus`.
