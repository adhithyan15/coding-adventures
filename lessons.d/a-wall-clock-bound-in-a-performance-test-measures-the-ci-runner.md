---
category: Testing & coverage
---

# A wall-clock bound in a performance test measures the CI runner, so compare two timings from the same run instead

`build (macos-latest)` failed on a repo-wide dependency bump with

    × does not get slower just because the canvas is taller  675ms
    AssertionError: expected 516 to be less than 400
     ❯ tests/raster.test.ts:388:18

The assertion was `expect(tall).toBeLessThan(Math.max(400, short * 8))`, guarding
`path-raster`'s edge-bucketing optimisation.

### Two separate faults, and the second hid the first

**The ratio clause never bound anything.** Measured on an idle 4-core box, the
real `tall / short` ratio is 14–22x, not "close" as the comment claimed. The
band spans the whole canvas, so the sweep genuinely visits every row and the
O(rows) term dominates however the edges are filed. `short * 8` with `short` at
3–5 ms is 24–40 ms — never close to binding.

**So the only live clause was the 400 ms stopwatch,** and a stopwatch measures
the box:

    idle                                 64–90 ms
    3x CPU oversubscription             129–160 ms
    CI, whole repo building in parallel     516 ms   -> RED

Nothing regressed. The runner was busy.

The file's own neighbouring test already documented this trap — "The first
version timed the render and required under 4 seconds. It passed locally at
700 ms and failed CI at 7,768 ms: a stopwatch measures the machine." The lesson
had been learned once in this very file and then reintroduced two tests later,
because the bound was written as a ratio and only *looked* self-normalising.

### The fix: two measurements from the same run

Vary the thing the optimisation changes, and let the machine cancel out. Instead
of one shape on a short and a tall canvas, hold the canvas tall and vary the
edge count:

    const few  = time(8);      // 8 edges over 8192 rows
    const many = time(8000);   // 1000x the edges, same rows
    expect(many).toBeLessThan(Math.max(few, 1) * 8);

Both halves run back to back on the same box under the same load, so a slow
runner scales both and the ratio holds:

    bucketed, idle                      1.3 – 1.5x
    bucketed, 3x oversubscribed         1.4 – 2.3x
    bucketing defeated, 2000 edges     49.7x   (and ~4x that at 8000)

An 8x bound sits in a wide gap, and the test can still fail — that 49.7x was
measured by editing the implementation to file every edge into every row, the
pre-optimisation behaviour, with a quarter of the edges the test uses.

### The rule

A performance test must compare two numbers gathered under the same conditions.
An absolute millisecond budget is a property of the hardware, not the code, and
it will go red on a loaded runner and green on a fast one regardless of whether
the algorithm is right.

Before trusting a ratio bound, check it actually binds. Print the real ratio on
an idle machine. If it is nowhere near the threshold, the clause is decoration
and some other clause — usually an absolute one — is carrying the test.

And when defending an optimisation, verify the test fails without it. Edit the
implementation to remove the optimisation and watch it go red. A performance
test that has never been seen to fail is asserting nothing.

### Why it surfaced here

A repo-wide lockfile change makes the affected closure the whole repository, so
~476 packages build at once instead of the handful a normal PR touches. Tests
that rarely run start running, and timing-sensitive ones start losing. Wide
changes are flake detectors; budget for fixing what they find.
