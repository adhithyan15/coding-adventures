### Fixed — the plan CLI test's cleanup hook states its own budget

- `tests/plan-cli.test.ts`: the `afterEach` hook now has an explicit 120s budget,
  the same budget the cases it follows already declare.
- The hook deletes a full copy of the curriculum, which is about 100,000 files.
- After the Sanskrit, Italian and French A2 tranches, that delete overran the
  package-wide 30s `hookTimeout` on a loaded CI runner. It failed `build
  (ubuntu-latest)` even though every assertion had passed.
- The durable fix is still for the cases to stop copying the whole corpus.
