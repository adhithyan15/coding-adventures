# HL121 — Pinned modality owner child lifetime

## Status

Proposed for #19165, a security child of #18947 and a prerequisite for
#18941's anchored modality-owner acceptance. The source-derived comparison
and all existing corpus time and coverage gates stay unchanged.

## Boundary

The direct-owner reader must pin each track directory and read only the exact
expected regular JSON owners beneath that handle. On Unix, open a listed owner
without following links **and without blocking on a FIFO**; check the opened
descriptor's type before reading any bytes. A concurrent replacement after
directory enumeration must fail closed, not park a worker forever. On Windows,
retain the existing handle-relative, reparse-point-refusing open.

The TypeScript transport starts at most four independent track children at a
time. Give each child a finite deadline. On timeout or oversized output, kill
the child and wait for its close before settling that track's promise. Keep
stderr out of diagnostics, bound response bytes, and wait for every started
track in a batch before propagating any failure. A deadline is an error, never
a reason to return a partial manifest or skip an owner.

## Verification

- A Unix FIFO replacing an expected owner after listing is rejected within a
  finite subprocess test timeout. A regular file still reads exact UTF-8 bytes.
- A controlled child that never responds is killed at the deadline, and its
  promise settles only after close. Oversized and nonzero children retain the
  same fail-closed behavior.
- Focused Python and TypeScript tests, full unchanged Windows coverage, macOS
  and Ubuntu CI, and a fresh final-source security review pass before a PR is
  made ready. No hook, test, heap, worker, or coverage limit is relaxed.
