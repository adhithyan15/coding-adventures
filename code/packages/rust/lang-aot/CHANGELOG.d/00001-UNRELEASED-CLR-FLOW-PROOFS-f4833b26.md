## Unreleased — additional strict CLR branch execution proofs

Preserve independent short conditional i64 early-return execution for both
outcomes and an explicit if/else join with an unconditional branch. Production
lowering remains the landed CLR12 implementation.
