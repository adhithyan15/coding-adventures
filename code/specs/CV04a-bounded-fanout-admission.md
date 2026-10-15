# CV04a — bounded fan-out admission

Imported c351402c fan-out queues accepted sockets before the worker checks its
active connection cap. A blocked worker therefore retains an unbounded number
of owned sockets. This repair bounds pending adoption separately from active
connections, with no new crates.io dependency. Existing active limits remain.

Every mailbox has an admission limit, initially the owning reactor's normalized
max_connections (standalone Default uses1024). Synchronize closure, limit changes
and admission under the same queue mutex. Each accepted adoption command owns a
reservation counted until adoption completes or the command is dropped. Draining
the queue must not release its reservations: commands held in a temporary vector,
early errors, remaining iterator entries and unwinding retain/drop ownership.
Only pending adoption is bounded here; existing write/close command semantics
and the active cap remain. At stable policy, active<=cap and pending<=cap; there
is no claim of a combined single cap.

set_max_connections updates active and pending policies. Already admitted
reservations survive shrinking; refuse further admission while pending>=new
limit, then resume when it falls below. Increasing permits new admissions.
No previously owned socket is invalidated by a policy update.

Preserve the public unit-returning adopt_connection API. A private admission
helper can return accepted/refused for exact controls. Overflow or closed
admission drops the passed owned socket immediately, outside the lock. Queue
closure clears queued commands and reservations under the mutex. Future commands
are discarded. Stop, serve exit/error/unwind and reactor Drop close admission;
shutdown_all must close it before fallible or panicking application callbacks.
Stop uses the existing optional wake handle after enqueue/closure; Windows has
no wake_handle implementation and keeps its bounded polling fallback. Stop
cannot preempt a permanently blocked application callback.

Controls use actual socket pairs to prove cap/overflow closure, reservations
held across a drained vector and released on drop/unwind, concurrent cloned
mailbox admission, synchronized closure and retained stale handles. Pin setter
shrink/increase behavior and serve/Drop closure. Preserve all existing active
connection, write, resource and actual fan-out tests. Full stream-reactor,
transport and downstream tests/strict lint plus independent exact-source review
are required. Worker/acceptor fault signalling remains a separate required
repair; neither this contract nor passing mailbox controls closes it.

Miri DEFERRED/PENDING, never passed; frozen checkout/inputs/target/receipts remain
untouched. Current spacing finite-input defect, whole456/general P2/required
exact-head CI/native3OS/conflict/full Closure parity and lineage gates remain
OPEN. No publication before applicable repair and review acceptance.
