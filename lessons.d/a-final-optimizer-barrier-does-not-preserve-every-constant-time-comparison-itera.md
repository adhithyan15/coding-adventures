---
category: Cryptography & security review
---

# A final optimizer barrier does not preserve every constant-time comparison iteration

A comparison loop accumulated byte differences normally and passed only the
final accumulator through `black_box`. That barrier hid the final value, but it
did not make each earlier input read observable. Once the accumulator reached
`0xff`, an optimizer could prove that later ordinary reads could not change the
result and remove them, reintroducing data-dependent work despite the
source-level full scan.

For repository constant-work comparisons, preserve the boundary at every
iteration. The fix uses volatile reads of both input bytes and the accumulator,
plus a volatile accumulator write, behind small audited safe wrappers. Add a
release-optimized LLVM-IR regression test that proves every loop cycle passes
through those operations and that loop exits do not depend on values derived
from the volatile reads. Merely counting `volatile` tokens or rejecting
`memcmp`/`bcmp` can false-pass when the operations execute once outside the
loop, a direct data-dependent loop exit remains, or a data-dependent preheader
bypasses the valid loop and converges on its return. Reject ordinary loads of
every width across the whole optimized function, not just byte loads inside the
loop component, and prove branch conditions transitively derive only from
public lengths and induction arithmetic. Functional tests alone cannot detect
this class of optimizer rewrite.

Keep the claim narrow. Volatile operations establish compiler-observable work;
they do not establish resistance to cache, speculative-execution, power, or
other hardware side channels. Before replacing a reviewed comparison
dependency, security-review the repository primitive and its optimized output
first, then migrate consumers in a separate slice.
