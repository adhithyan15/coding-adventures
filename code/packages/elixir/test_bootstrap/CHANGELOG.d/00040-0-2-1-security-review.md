## 0.2.1

- Fuzz engine security review: memory is polled every 5 ms (process memory plus
  deduplicated refc binaries) because `max_heap_size` ignores refc binaries, and
  `:off_heap` mailboxes are no longer used; the budget is documented as best effort.
- `Handle.exhausted/1` cross-checks the counters, so clearing the sticky cell cannot
  turn a breach into a pass; the token is documented as a 32-bit identity check.
- Result messages carry the pid; `Fuzz.run/3` and verdict re-verification run in a
  dedicated engine process that drains stray messages and replies through a bounded
  monitored receive.
- Tests for sticky sentinels across kinds, the 300 MB binary, binary and mailbox
  floods, forged results and a flooded parent.
