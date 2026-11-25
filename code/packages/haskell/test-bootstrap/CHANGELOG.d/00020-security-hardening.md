## 0.1.1 - security review (LOW findings)

- Every stanza builds with `-fno-omit-yields`, so `timeout` interrupts a non-allocating loop (pinned by a test).
- A corpus fixture seed outside `[0, 2^64-1]` is refused with a case error instead of being reduced modulo 2^64.
- `forkWithBudget` runs its child unmasked (`forkIOWithUnmask`) and drops the child's exceptions; the new
  `forkWithBudgetHandler` hands a handler only the exception type name.
- The corpus `showJson` takes a bounded lazy prefix before rendering a malformed node.
- Documented: `repro-fnv` is an unkeyed fingerprint (not a redaction guarantee), custom generators and shrinkers must
  return finite values, and children of `forkWithBudget` are not killed when the scope closes.
