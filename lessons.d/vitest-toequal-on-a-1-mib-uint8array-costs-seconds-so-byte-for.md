---
category: TypeScript / JavaScript
---

# vitest toEqual on a 1 MiB Uint8Array costs seconds, so byte-for-byte checks on large buffers need a linear comparison

**What happened:** `canonical-cbor`'s CBR01 conformance test kept overrunning
its timeout on loaded CI runners. It was first given an explicit 15s budget
on the theory that 55 fixture cases are simply a lot of work. A repo-wide
lockfile PR, which rebuilt ~530 packages in one run, pushed it past 15s too.

**Root cause, measured:** the codec was never the cost. On the 1 MiB
size-limit case:

    encode           ~72 ms
    toEqual(1 MiB)  ~6,329 ms   <- vitest deep equality, element by element
    decode            ~1 ms

`expect(a).toEqual(b)` on two `Uint8Array`s walks every index through the
generic deep-equality machinery. That is fine for a few hundred bytes and
seconds of CPU for a megabyte. The test's runtime was almost entirely the
assertion, and it scales with runner load.

**Do instead:** compare large byte buffers with a plain loop, or
`Buffer.compare`, and assert on the result:

- check the type, then the length, then scan for the first mismatching index;
- put the offset and both bytes in the failure message, which also beats
  vitest's megabyte-long diff;
- mutation-check it: flip one byte of the big output, and the test must still
  fail.

The conformance test went from ~6.4s to ~120ms with identical strictness.

**General rule:** before raising a timeout on a slow test, profile which line
spends the time. A timeout raised to cover an assertion's own cost hides the
next overrun instead of fixing this one.
