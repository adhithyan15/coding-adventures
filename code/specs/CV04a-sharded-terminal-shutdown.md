# CV04a — sharded terminal shutdown

Sequential joins can hang on the first healthy worker when a later worker
fails: its failure never reaches the supervisor to request shutdown. Every
worker and fan-out acceptor must request coordinated stop on return, error or
unwind, before destroying its owned runtime/provider. Keep the existing first
worker error and convert worker/acceptor panics to explicit ProviderFault.

An owned participant places its shutdown guard before its runtime/acceptor in
field destruction order. This ordering also applies if thread creation fails
and the unstarted closure is destroyed. Its run method consumes the entire
participant, preventing disjoint closure capture from discarding the guard.
The supervisor owns its own shutdown guard before starting any participant.
Use fallible Builder::spawn; on partial launch failure signal all participants
immediately, join already started workers and the acceptor, and return the launch
error. Never detach successfully launched threads. Preserve the acceptor's
connection-reset/interruption/backoff recovery and round-robin delivery.

Native two-worker controls make the later worker panic while the first stays
healthy, then require a bounded observed return with ProviderFault and closed
ingress. Guard controls verify shutdown is visible before owned destructor
execution on panic and on disposal of an unstarted participant, including
acceptor ownership. Do not claim these simulated disposal controls reproduced
an OS thread-creation failure. Re-run all four transport packages/strict lint
and downstream/standalone validation. Independent exact-source security review
remains required before publication.

Existing optional stop wake handles are used; Windows retains bounded poll
fallback. Coordinated stop cannot preempt an application callback that blocks
forever, nor prove an OS cancellation packet arrived before observing it.
Miri DEFERRED/PENDING, frozen run untouched. Numerical spacing, full37/standalone,
whole456/general P2/native3OS/exact-head CI/conflict/full Closure parity and
lineage gates remain OPEN; full objective incomplete and heartbeat ACTIVE.
