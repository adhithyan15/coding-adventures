---
category: CI & GitHub Actions
---

# A new trace-refusal rule must be run repeatedly against the recorder's own probe traces before merge

KILN04 D46 (cwd-race by overlap, #19877) refused a *split* `unshare(CLONE_FS)`
whenever another thread shared the cwd, because the start line was thought
unknown. The sandbox helper does exactly that unshare in every probe and step,
and strace splits a call whenever another tracee's stop lands between its
halves. So `kiln record prepare` failed on a random fraction of kernel runs
(the `pipes` and `signals` probes, `cwd-race` about 140 lines in), only on the
real-kernel job, and unit tests passed because they pinned the refusal.

### What to do differently

- A refusal rule that depends on interleaving (split lines, thread timing) is
  nondeterministic by construction. Before merging it, run the recorder's own
  helper and probe traces through it many times on the real kernel, under load.
- Do not write a test that pins a refusal of something the recorder itself
  always does; that test documents the bug instead of catching it.
- Where the information is not at hand, check whether an earlier stage already
  holds it (here, the pending call's start line) before refusing.
