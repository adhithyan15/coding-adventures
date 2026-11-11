---
category: Rust
---

# A GlobalAlloc cannot fail an allocation on stable Rust without aborting the process, so an allocation cap must be enforced by containment

VER04j WX-H wants an over-cap allocation in the subject under test to *fail*, so a
decompression bomb is a failed instance and not an out-of-memory CI run.  A
counting `#[global_allocator]` cannot do that on stable Rust:

- the `GlobalAlloc` methods may not unwind (a panic out of them is undefined
  behaviour, and the allocator shims are `nounwind`, so it aborts anyway);
- returning null from `alloc` makes `Vec::with_capacity` call `handle_alloc_error`,
  which aborts the process (the alloc-error hook is nightly-only).

What worked is *containment*: run each observed call on a worker thread inside an
allocation window; when the allocator sees the over-cap request it records the
attempted size, wakes the caller and parks that thread forever (the request is
never made), and the caller fails the instance.  Three details matter: the
accounting must use `const` thread-locals without destructors (touching a lazily
initialised or `Drop` thread-local from inside the allocator can allocate or
register a destructor); a test that tripped once must fail its later observed
calls without running them (a shrinker would otherwise abandon one worker per
candidate); and the process must cap how many workers and how many bytes it will
ever abandon, because a parked worker is never freed.  A generated harness that
installs a global allocator also needs its own test binary: two includes of it in
one binary are a duplicate `#[global_allocator]`.
