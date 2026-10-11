---
category: Testing & coverage
---

# Asserting a bounded queue refuses right after it first fills races the consumer taking its first item

**What went wrong.** `a_host_that_stops_reading_fills_its_queue_instead_of_blocking`
(P2.6b) filled a `sync_channel(8)` with `try_send` until it refused, then
asserted that the next `try_send` was refused too. The consumer thread blocks
forever on the first 1 MiB frame it writes into a pipe nobody reads, but
*when* it takes that first frame is up to the scheduler. On macOS CI it took
it between the two sends, freeing a slot, and the assertion saw `Ok(())`. It
passed on Linux and on the PR that introduced it; it failed later on an
unrelated PR whose change only rebuilt the crate.

**The fix.** Refill in a loop, with a deadline, until exactly capacity + 1
items have been accepted. That count is only reachable once the consumer
holds its one item and is stuck on it, so the following refusal is
deterministic.

**Do differently.** A "full" assertion on a queue with a live consumer is only
sound once the consumer's state is pinned. Count to the exact steady-state
total instead of trusting the first refusal. Sleeping before the assertion
only narrows the race.
