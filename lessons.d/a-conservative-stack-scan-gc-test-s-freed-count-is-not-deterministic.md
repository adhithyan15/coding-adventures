# A conservative-stack-scan GC test's "freed" count is not deterministic in a debug build — assert on a non-conservative signal instead (gc-core-capi, AOT00-T8)

Writing a capi-level test for the new `__gc_collect_minor_precise` entry (AOT00-T8,
adaptive safepoint scheduling), I copied the exact pattern every other stack-scan
smoke test in `gc-core-capi/src/stack_scan.rs` already uses: allocate a `kept` object,
root it in a local; allocate a second object and immediately discard the pointer
(`let _ = __gc_alloc(16);`); call the collect entry; `assert!(freed >= 1, ...)`. It
failed **deterministically** (not flaky-sometimes — every run, `freed == 0`) even
though the underlying algorithm was already proven correct by a controlled
gc-core-only unit test with exact root slots. Swapping in the pre-existing,
already-shipped `__gc_collect_precise()` in the *exact same test function* reproduced
the identical `freed == 0` failure — proving the bug was not in my new code at all,
but in the test's own shape.

Root cause is the sibling of the "same-shaped `if` block" lesson directly above, one
layer down: in an unoptimized debug build, a discarded temporary's value (the dead
object's return value from `__gc_alloc`) is not guaranteed to be scrubbed from the
stack slot/register it transiently occupied — it can keep sitting there, byte-for-byte
identical to a real heap address, for the rest of the function's lifetime. A
conservative stack scan (`__gc_collect`/`__gc_collect_precise`/`__gc_collect_minor_precise`
with no stack maps registered — the exact case every existing unit test exercises)
reads *every* word in the scanned region as a *candidate* root, so that stale word
retains the "dead" object regardless of whether any live Rust binding still names it.
Whether a given test happens to dodge this is pure happenstance of that function's
specific local-variable layout and register allocation, not a property of the GC
algorithm — mine tripped it, several pre-existing tests apparently do not (this time).

**Fix:** don't assert `freed >= N` (or any positive lower bound) as the *sole* signal
in a new stack-scan integration test — it's inherently non-deterministic and doing so
just adds a coin-flip to CI without telling you anything actionable. If the property
under test has a **non-conservative, exact** way to check it (here: `__gc_kind_of`,
which does a real `find_header` lookup — no stack scanning involved — so it reports
"still live" vs "reclaimed" precisely), assert on that instead. Reserve the
`freed >= 1` pattern (as the *existing* precise/compacting smoke tests already do) for
cases with no better signal available, and don't add new tests that rely on it as the
primary proof of a specific new behavior.

**Update, having gone back to actually fix the 4 pre-existing flaky tests this entry
originally just characterized (not fixed):** `__gc_kind_of(dead)` is *not* a valid fix
for "prove this ONE specific unreferenced object was reclaimed" — it's a genuine
Catch-22. To read `dead`'s address back *after* the collect call, `dead` must still be
a live Rust local spanning that call — which means the compiler keeps its value in some
register or stack slot the conservative scan can see, and a conservative scan *correctly*
treats any value that looks like a live object's address as a possible root. So keeping
`dead` around long enough to check it is exactly what makes the scanner (correctly!)
retain it — confirmed empirically: adding the `__gc_kind_of` check turned an
occasionally-flaky `freed >= 1` into a *deterministically failing* "still alive" for
every one of the four affected tests. This is not a bug in the collector; it's what
conservative scanning is supposed to do.
