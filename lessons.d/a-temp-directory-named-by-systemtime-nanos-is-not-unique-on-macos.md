---
category: Testing & coverage
---

# A temp directory named by SystemTime nanos is not unique on macOS, so parallel tests can share one

**What happened.** `mosaic-compile`'s `compose_fixtures` failed intermittently
(4 of 22 local runs): `compose_without_emit_project_writes_only_the_component`
found a `src/` directory in the scratch directory it had just created. The
helper named directories `…-{pid}-{SystemTime nanos}`. On macOS `SystemTime`
has microsecond resolution, so `as_nanos()` is always a multiple of 1000, and
two tests starting in parallel threads within one microsecond got the same
name. `create_dir_all` happily "created" the existing directory, and one test
saw the other's `--emit-project` output. Twenty Mosaic test sites used the
same pattern.

**Fix.** Name scratch directories with a per-process `AtomicU64` counter plus
the clock (`unique_nonce()`), and pin it with a test that takes 1,000 names
back to back and requires them distinct. A clock-only name gives 668 of 1,000
there, so the test fails deterministically on the old code.

**Do differently.** A timestamp is not an identifier. Uniqueness within a
process needs a counter (or `tempfile`), and a "fresh" scratch directory
should be made with `create_dir`, which fails if it exists, not
`create_dir_all`, which hides the collision. When a test flakes, rerun it in a
loop and check the helper before blaming the code under test.
