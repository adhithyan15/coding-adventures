## Unreleased

- `tests/warrant_proofs.rs` records `obligation_sha256` per discharged proof entry: SHA-256 of the canonical text of the lowered, inlined WIR function plus the textual obligation (security review B4c). `ca-assurance-report --check` verifies it is present and well formed; recomputation is follow-up WD-009.
