## 0.2.1 - fuzz security review

### Fixed
- `fz_bitmap_record` refuses to grow its edge list past the 65,536-edge map (a racing target thread, which is
  misuse, could previously push `nedges` past `edges[]`). Regression: more than 65,536 colliding records, and a
  corrupted `nedges` at the limit.
- The sancov guard callback has a static re-entrancy guard, so an accidentally instrumented engine cannot recurse forever.
- `fz_crash_push` compares frames with a bounded `strncmp`.
- `fz_mutate_sequence` clamps `max_len` to `FZ_MAX_MUTATE_BYTES`. Regression: `SIZE_MAX` equals the clamp value.
- Mutators no longer pass a NULL source or a zero length to `memcpy` (tested with NULL/empty inputs under the sanitizers).
- `fz_tap_failure_line` returns `FZ_E_ARG` when `first_test + index` would overflow an int (or `first_test` is negative).

### Documented
- Targets must be single-threaded or join their threads before returning; the 4,096-slot handle aliasing window;
  `fz_hooks` is a mutable test-seam global; keeping `fz_crash *out` past the call is misuse.
