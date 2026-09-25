---
category: Testing & coverage
---

# A test that mutates a copy to a fixed sentinel and asserts the original differs is flaky whenever the original is random

`build (macos-latest)` failed on a repo-wide dependency bump with

    AssertionError: expected 255 not to be 255 // Object.is equality
     ❯ tests/uuid.test.ts:192:31
    Total: 5225 packages | 472 built | 4748 skipped | 1 failed

"expected 255 not to be 255" reads like nonsense. It is not: it is a
`.not.toBe()` that drew the forbidden value by chance.

### The shape

    it("returns a copy (mutation-safe)", () => {
      const id = v4();
      const b1 = id.bytes;
      b1[0] = 0xFF;                        // mutate the returned copy
      expect(id.bytes[0]).not.toBe(0xFF);  // original unchanged
    });

The intent is sound: `get bytes()` returns `this._bytes.slice()`, and the test
should prove the caller cannot reach through the copy. The implementation was
correct and stayed correct.

The test is not. It conflates two different claims:

1. the original was not mutated, and
2. the original's first byte does not happen to equal `0xFF`.

`v4()` fills byte 0 with random bits — the version and variant nibbles live in
bytes 6 and 8, not 0 — so claim 2 is false about once every 256 runs. Measured
over 200,000 generations: 788 hits, **1 in 253.8**.

### Why it surfaced on a dependency bump

It has nothing to do with the bump. A repo-wide lockfile change makes the
affected closure the whole repository, so ~472 packages build in one run
instead of the handful a normal PR touches. A 1-in-256 test that almost never
runs becomes a test that runs, and a 0.4% failure starts landing.

Wide changes are flake detectors. Expect to meet other packages' latent
coin-flips, and budget for fixing them.

### The fix

Snapshot the whole array before mutating, and compare against the snapshot:

    const id = v4();
    const before = Uint8Array.from(id.bytes);  // snapshot first
    const b1 = id.bytes;
    b1[0] ^= 0xFF;                             // mutate, whatever it held
    expect(id.bytes).toEqual(before);          // original untouched
    expect(id.bytes).not.toBe(b1);             // each read is a fresh copy

This tests the actual claim and is independent of what the random bytes are.
It also compares all 16 bytes rather than one, so any collateral change to the
original is caught. Over 200,000 generations the old assertion failed 771 times
and the new one zero times.

Be precise about what the strengthening is. Comparing the whole array catches
collateral damage; it does not catch sharing of a byte the test never writes.
That gap is closed by the assertions elsewhere in the file that pin actual byte
content.

Note `^= 0xFF` rather than `= 0xFF`: it guarantees the copy changes whatever it
started from, so the test cannot silently pass by mutating a byte to the value
it already held.

### The general rule

Never assert that random data differs from a constant. When checking that a
copy is independent of its source, compare the source against a snapshot of
itself, never against the sentinel you wrote into the copy. The same trap
applies to "assert the new random id != the old one" and "assert the shuffled
list != the original" — both fail at a rate equal to the collision
probability, and both are usually testing the wrong thing.

Raising a timeout is sometimes the right answer to a slow test. Nothing is ever
the right answer to a probabilistic assertion except making it deterministic.
