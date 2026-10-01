---
category: Testing & coverage
---

# A test hook that deletes a whole-corpus copy needs its own timeout budget, like the cases it cleans up after

**What happened:** `human-language-data/tests/plan-cli.test.ts` copies the whole
curriculum into a temp directory for each case, and deletes it again in `afterEach`.
That is about 100,000 files. Each case declares a 120s budget, but the cleanup hook
ran on the package-wide 30s `hookTimeout`.

The Sanskrit, Italian and French A2 tranches together added some 6,000 files.
On the Italian A2 PR, CI's `build (ubuntu-latest)` then failed with `Hook timed out
in 30000ms` at the `afterEach` line. Every assertion had passed (2563 passed, the
hook counted as 2 failures). Locally the file passed, both in isolation and in the
full suite.

**Fix:** give the hook the same explicit 120s budget as the cases it follows
(`afterEach(() => {...}, 120_000)`). This is the per-case override that
`vitest.config.ts` already sanctions for a case stating its own cost. It is not a
global or command-line timeout raise.

**What to do differently:**

1. When a content program adds thousands of files, check every test that copies or
   walks the whole corpus. Look at its hooks as well as its cases. A hook doing
   corpus-sized filesystem work is a test cost and should state its budget.
2. The durable fix is still for `plan-cli.test.ts` to copy only what the plan CLI
   reads. Until that lands, every authoring wave moves this hook closer to its
   budget.
