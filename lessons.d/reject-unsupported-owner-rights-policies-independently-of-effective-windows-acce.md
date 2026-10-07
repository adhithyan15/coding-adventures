---
category: Rust
---

# Reject unsupported OWNER RIGHTS policies independently of effective Windows access checks

The correctly gated native Windows compiler test run passed strict lint but
failed the promised pre-mutation rejection of OWNER RIGHTS deny-READ_CONTROL
policies. The inherited fixture rejected locally but publication succeeded in
CI. Independent native review reproduced success locally with a current-user
FullControl allow preceding an OWNER RIGHTS denial: exact policy readback and
a fresh verification open both succeeded, then the original changed.

Windows access checks evaluate ACEs in order; a successful fresh open is a
capability measurement, not a structural policy-admission decision. Enforce
the explicitly unsupported policy class before probing or mutating originals,
including generic masks and the supported denial layouts, without changing ACE
order. Keep the fresh-open proof for other policies. Use a deterministic
allow-before-deny native regression and report token/derived-policy/open
diagnostics on the inherited fixture. The exact reason for CI's different
access result remains unproven; do not attribute it to elevation without evidence.
