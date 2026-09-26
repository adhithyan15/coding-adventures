---
category: Testing & coverage
---

# Vitest's default timeouts are too tight for a suite that shares a CI runner with a whole-repo build

`build (macos-latest)` failed on the repo-wide dependency bump with

    Error: Hook timed out in 10000ms.
     ❯ tests/curriculum.test.ts:17:1
    Total: 5225 packages | 476 built | 4748 skipped | 1 failed

The hook is `beforeAll(async () => { await loadCurriculumPlans(); })` in
`language-ladder`. Nothing about it is broken.

### Why the load matters

Vitest's defaults are 5000 ms per test and 10000 ms per hook, and they assume
the suite has a machine to itself. A change that touches every lockfile makes
the affected closure the whole repository, so ~476 packages build in parallel
on one runner and every suite in that run is competing for CPU.

### Do NOT use the transform-to-wall-clock ratio to diagnose this

The obvious-looking signal is vitest's summary line, where aggregate transform
time dwarfs wall clock:

    Duration 75.99s (transform 20531.80s, ... environment 28.67s)

20,531 s inside 76 s looks like a damning 270x. **It proves nothing.** That
figure is summed across parallel workers and counts re-transforms, so a big
ratio is normal for any suite with heavy transforms. Measured on the same
idle 4-core box, the same suite gives:

    run 1  28.92s wall, transform 19829.08s  ->  686x
    run 2  28.13s wall, transform  8299.92s  ->  295x

The idle machine scored **higher** than the starved one, and the two idle runs
disagree with each other by more than 2x. Anyone "confirming starvation" this
way will confirm it on a perfectly healthy machine — and then raise a timeout
over what might be a genuine regression.

This lesson exists because that is exactly the mistake that was made here: the
ratio was written up as the diagnostic signature before it was checked against
an idle baseline. A security review caught it.

### What actually discriminates

Compare the same suite's wall clock against its own solo baseline, and look
for evidence the machine was the variable:

- **Wall clock vs baseline.** 75.99 s on CI against 28-29 s idle: ~2.6x
  slower. That is the real signal, and it is the one number in the summary
  line that is not aggregated across workers.
- **Other lanes on the same commit.** `build (ubuntu-latest)` and
  `build (windows-2025)` both passed the exact commit that failed on macOS.
  A code defect does not pick one runner.
- **What else was on the box.** 476 packages building concurrently.

### The margin was always thin

On an idle 4-core box `tests/curriculum.test.ts` costs 5.0 s against a 10 s
hook budget; under 3x CPU oversubscription, 6.4 s. So the default left barely
2x headroom and any busy runner could cross it.

Worse, the *same* CI run had a test finish in 4977 ms against the 5000 ms
default — passing by 23 ms. The next slightly slower run would have failed
there too, in a different file, looking like an unrelated flake.

### What it looks like when it fires

A hook timeout does not report as one failed test. It reports as

    tests/curriculum.test.ts (7 tests | 7 skipped)

Seven assertions that never executed, in a state that reads like a skip rather
than a failure. Only the file-level `1 failed` gives it away. Do not read
"skipped" as "fine".

### What to do

Set `testTimeout` and `hookTimeout` explicitly in `vitest.config.ts` for any
suite with expensive setup. Size them from measurement: take the idle cost and
give it roughly an order of magnitude, which covers a loaded runner while
still failing fast enough to be useful.

Before doing that, check two things about the slow operation:

1. **Can it hang unboundedly?** Here it cannot — `loadCurriculumPlans()` is a
   single-attempt `Promise.all` over dynamic imports of build-time virtual
   modules. No network, no retry loop. Raising the budget cannot hide a
   stalled connection, because there is no connection. If the operation *is*
   a real fetch, give it an `AbortSignal` rather than a bigger timeout.
2. **Is the job itself bounded?** Every build job in `ci.yml` carries
   `timeout-minutes`, so a pathological hang is capped by the job regardless.

Raising a timeout is not skipping a test — every assertion still runs. But
confirm starvation properly first, by wall clock against baseline, never by
the transform ratio.

### Related

The same oversized run first exposed a `node_modules` race
(`code/specs/B07-build-shared-directory-safety.md`), which predicted this
failure in its "What this does not address" section. Both are consequences of
one repo-wide change turning into one enormous parallel build.
