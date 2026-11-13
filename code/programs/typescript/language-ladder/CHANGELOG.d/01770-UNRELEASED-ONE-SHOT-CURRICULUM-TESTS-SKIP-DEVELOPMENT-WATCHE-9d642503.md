## Unreleased — one-shot curriculum tests skip development watches

`vitest run` still reads and validates the same canonical per-track ledgers,
lesson membership owners, and all seven curriculum assertions. It no longer
registers tens of thousands of source paths with the dev-server watcher during
that one-shot run. On Windows, the exact `tests/curriculum.test.ts` suite fell
from a 60-second `beforeAll` timeout with seven skipped tests to 15–23 seconds
with all seven passing, without changing the 60-second gate. Production builds
and interactive watch mode retain normal source watching.
