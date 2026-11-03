---
category: Testing & coverage
---

# Negative harness checks must retain the parent failure vocabulary

The CT01 L0 adapter invented adapter-baseline and adapter-mutation failure IDs
for its negative-check diagnostics. Normal runs exercised those checks only in
the passing direction, so all local tests were green. TF01 defines a closed
failure vocabulary; a future real failure would have produced an unregistered
TAP diagnostic even though its message and process exit were otherwise useful.

Use corpus-mismatch for both adapter checker failures and put the detail in the
message. Run the actual custom-harness binary with --sabotage always-pass and
check its nonzero exit, failing test lines and decoded failure IDs. TF01 prints
those IDs as quoted YAML scalars; the checking script must decode the scalar,
not compare quote characters as part of the ID. This produced three expected
negative-check failures, all using the registered corpus-mismatch ID.

The follow-up backlog item hardens the bootstrap boundary itself, since its
current string-taking Verdict API does not prevent consumers inventing IDs.
