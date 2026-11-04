# HL114 — Bounded asynchronous lesson-source reads

## Status

Proposed for #18751. This is a loader and test-performance change, not a
curriculum, assessment, timeout, cache-validity, or security-policy change.

## Problem and evidence

The Windows human-language-data suite now has 63,257 authored lesson Markdown
files. A warmed, cache-hit `loadLessons()` still spends most of its time opening
and reading those sources before it can compute content-addressed cache keys.
On clean main `6879222441e6`, a read-only pass over the same 173,992,454 bytes
took 25.82 seconds with synchronous reads and 6.93 seconds with 16 bounded
asynchronous readers. This measures raw I/O, not a full loader or test pass.
The synchronous public loader must keep its existing behavior for callers.

## Contract

Add an asynchronous whole-corpus lesson loader for consumers that can await it.
It must traverse tracks in the same sorted order and apply the same track-id,
non-directory-track, script-declaration, and cache rules as `loadLessons()`.
Within a track, read at most 16 lesson files concurrently, place each result at
its sorted filename index, and compute the cache key from the exact bytes read.
Do not start one promise per corpus file. A cache hit may skip parsing, but may
not skip source-byte validation; a miss parses those same bytes, with no second
read window. Cache-key observers see the same language/key pairs and order as
the synchronous loader. A read or parse failure rejects the load; it must not
silently drop a lesson, track, or error. The existing synchronous API remains.

Use the async path only where it preserves test meaning and reduces repeated
whole-corpus I/O. The modality-manifest real-corpus setup is the initial target:
it must still derive and compare the complete manifest, retain every existing
assertion, and stay within the existing hook timeout. Other slow files require
separate measured changes; no global timeout, worker count, heap limit, or
corpus ceiling is raised to make this proposal pass.

## Acceptance

- Fixture tests prove sync/async lesson parity, ordering, cache-key parity,
  warm/cold cache behavior, source-edit invalidation, and error/refusal parity.
- A bounded-concurrency test records the maximum simultaneous reads and shows
  the limit applies to the whole active track, including a failure path.
- Focused modality tests pass on Windows without loosening assertions or
  timeouts; record before/after wall time on stable source and cache state.
- Run the unweakened full Windows suite and Ubuntu CI on the exact final head.
  Keep #18751 and its dependents open if either full-suite gate remains red.
