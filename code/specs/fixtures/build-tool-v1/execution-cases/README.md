# Trusted execution cases

Only `*.json` files in this directory belong to the trusted execution corpus.
The process-free policy tranche intentionally contains no cases, so its framed
corpus SHA-256 is the standard empty digest. Execution semantics enter this
directory only after an enforcing platform backend is reviewed.

The bootstrap `cases/` directory remains process-free. Never move an execution
case there or add an execution-enabling flag to the bootstrap validator.

The Linux OCI backend identity schema and process-owning capability preflight
do not change this gate. They validate and probe runner-owned containment
inputs only; they never decode or execute a case from this directory.
