### Changed — the level-gate test builds its gap report at import

`tests/level-gate.test.ts` already built the whole-corpus gap report once per file, but lazily, inside the first test that asked for it. That charged the full load and build to one test's 30s budget: "separates what a track TOUCHES from what it has ATTAINED" took 22.8s locally, about 20s of it this build. The report is now built at import, so no single test pays for it, and that test drops below a second.

The change came out of a whole-suite timing audit that followed two CI timeouts, on the Telugu and Kannada A1 PRs. No assertion changes, and `vitest.config.ts` is unchanged. The other tests the audit found near the budget are the `cli` gap-report runs (about 19s each), `narration-cli` (16s) and `track-progress` (14s). They are recorded for follow-up.
