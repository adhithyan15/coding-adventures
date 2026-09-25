---
category: Testing & coverage
---

# A test that grows its input from the whole curriculum must not run under vitest's default 5 s budget

HL-C443 made script-ductus's `filmstrip-ledger.test.ts` derive part of its input
from the curriculum: it now loads every lesson to find single-letter writing
lessons, and renders 21 letters instead of 3. Locally the file took about three
seconds and every test passed. On CI the first ledger-building test died at
`Test timed out in 5000ms` (`tests/filmstrip-ledger.test.ts:150`), because that
one test paid for the cold curriculum load plus the render, on a runner several
times slower than a laptop.

Nothing was wrong with the output. The failure was a wall-clock limit nobody had
chosen, which a test that asserts BYTES was never meant to carry.

The fix was two changes:

- **Load once.** The curriculum load moved into `beforeAll` with its own
  explicit budget, and the candidate list is cached for the file.
- **Size the budget to the work.** The ledger tests got a describe-level timeout
  sized to what they do, with a comment saying they assert bytes, not speed.

**What to do differently:** when a change makes a test read the whole
curriculum, or multiplies the work inside a test, give that test an explicit
timeout in the same change. Time the file locally and treat anything over about
a second as a CI timeout waiting to happen. Do not raise `testTimeout` globally;
that hides real hangs everywhere else.
