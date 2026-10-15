---
category: Testing & coverage
---

# A timing test's margin must dwarf timer slack; a 100 ms margin over a 300 ms recv_timeout flakes on macOS

`chief-of-staff-broker-launcher`'s relay test
`the_deadline_does_not_charge_the_broker_for_the_daemon_s_time` failed on
macOS CI with "the relay did not end". In it the fake broker answers 400 ms
after a request, and the relay's deadline is 300 ms, so the deadline must
fire first. But the relay starts its clock when it queues the request and
then waits with `recv_timeout(remaining)`. On a loaded runner the timed
wait can wake late (macOS coalesces timers), or the relay thread can start
waiting late, and 100 ms is not enough margin to absorb either. The late
response arrived first, was accepted, and the relay went on.

Fix: the slow broker now answers 1.5 s late, five deadlines rather than a
third of one. Nothing else in the test changed.

Do differently: when a test asserts that a timeout wins against a sleep,
give it a margin of several times the timeout, never a fraction of it.
Timer slack on CI is tens of milliseconds even when nothing is wrong.
