# Trusted execution cases

Only `*.json` files in this directory belong to the trusted execution corpus.
The process-free policy tranche intentionally contains no cases, so its framed
corpus SHA-256 is the standard empty digest. Execution semantics enter this
directory only after an enforcing platform backend is reviewed.

The process-free validator captures one immutable snapshot before it computes
that digest or validates a case. Members are direct portable lowercase-`.json`
names, unique under NFC plus case folding, and exact raw bytes are retained
after a stable singly linked no-follow read. A typed selector returns only
those already-hashed bytes; it never reopens a requested pathname. Outside
paths, case or normalization aliases, links, reparses, hardlinks, identity
aliases, directory changes, and post-digest pathname substitution fail closed
or cannot affect the held selection. Runner-owned ceilings admit at most 4096
enumerated directory entries, 256 cases, 2000000 bytes per case, and 16777216
retained bytes in aggregate.

The bootstrap `cases/` directory remains process-free. Never move an execution
case there or add an execution-enabling flag to the bootstrap validator.

The Linux OCI backend identity schema and process-owning capability preflight
do not change this gate. They validate and probe runner-owned containment
inputs only; they never decode or execute a case from this directory.

Snapshot capture and selection likewise do not authorize execution, mark a
backend ready, or prove adapter conformance. The later trusted-execution
authority profile must bind the exact selected snapshot separately.
