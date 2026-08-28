### Changed - stable gentle-ramp metric and finding owners (#13376)

- Replace 23 generated language aggregates with 851 stable direct owners: immutable
  per-language metadata, 26 metric identities, and ten always-present finding kinds.
  Different agents can now change independent gentle-ramp dimensions in one language
  without rewriting the same generated file.
- Derive lesson counts from exact parsed/narration lesson identities and derive `next`,
  work-queue order, and summaries during the strict fold. Reject missing, extra,
  unsafe, noncanonical, contradictory, or resurrected aggregate state.
- Stage and validate the complete owner tree before replacing the prior snapshot
  directory, with recovery if installation or installed verification fails.
