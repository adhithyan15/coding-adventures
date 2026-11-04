# HL119: One guarded stat per book-generation owner

## Problem

The generic shard `--check` takes about 34 seconds under the unchanged
30-second Windows coverage-case budget. Its book-generation plan reads many
small direct owners. Each `readSection` and script-set iteration calls
`lstatSync` to reject links/non-files and then calls `readLedgerFile`, which
performs the same leaf `lstatSync` refusal before reading. Issue #18957 tracks
the all-plan check.

## Contract

* Remove only the redundant preliminary leaf stat where the immediately
  following `readLedgerFile` enforces the same real-regular-file refusal.
* Keep parent-component checks, exact owner-set and identity checks, missing
  and extra owner refusals, `X.d` sibling refusal, canonical output bytes,
  symlink/junction refusal, and cross-ledger completeness unchanged.
* Verify focused book-generation corruption/link tests and the complete
  unchanged generic shard `--check` case under Windows coverage. The 30-second
  case budget and 85% package-wide line-coverage gate stay intact.

## Out of scope

This does not remove owner validation, alter source formats, or change the
curriculum and book generation output.
