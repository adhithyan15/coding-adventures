---
category: Testing & coverage
---

# A strace spawn can return on a resumed line

- When an unfinished `clone`, `clone3`, `fork`, or `vfork` is followed by
  interleaved trace activity, its child PID appears only on the matching
  `<... syscall resumed>` line. A test that detects PID reuse must pair that
  line with the pending parent/syscall and validate the result, or it can
  mistake a later process generation for the original one.
