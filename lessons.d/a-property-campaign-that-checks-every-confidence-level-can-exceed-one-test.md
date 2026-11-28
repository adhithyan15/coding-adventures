---
category: Testing & coverage
---

# A property campaign that checks every confidence level can exceed one test's wall-time limit

**What happened.** On TypeScript rank-statistics source commit
`8e29b7a878a6acbf909ffaa08dc0256b183991c6`, Windows run `46087864541`, job
`140802429249` (`build (windows-2025)`), failed the test
`checks 2,048 seeded finite-binary64 arrays at all 99 confidences`. The test's
20,000 ms timeout fired after a reported 29,767 ms. The suite reported 20 of
21 tests passed; the other test did not fail an oracle assertion. The test
checked all 2,048 generated arrays and 202,752 interval/confidence pairs in one
Vitest case. The preserved job log is
`outputs/data-structures/typescript-rank-statistics/ci-20212/head-8e29b7a8/job-140802429249/job-log.bin`
(SHA-256 `b26b742f05d9481d51d6b8bf6a9ce74ae088f6a0a3cb680dcec2f982062f3fb7`);
its parsed receipt records the run, job, source blob, test, timeout, and counts.

**What to do differently.** Keep the same seed, generate the full accepted
campaign once, then run it in eight ordinary sequential test cases over
contiguous 256-array slices. Each slice still checks all 99 confidences for
each array; across the slices the campaign retains all median and interval
oracle checks and the full accepted/rejected counts. Keep the existing
20-second per-test limit. Do not drop checks, regenerate the campaign per
slice, run the shards concurrently, or infer a hardware cause from this single
timing result.
