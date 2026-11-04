# HL115 — Single-read modality owners with linear identity checks

## Status

Proposed child of #18941. It follows the bounded lesson-source work in HL114,
but addresses a different full-corpus I/O cost. Neither the direct-owner
security contract nor any test timeout changes.

## Problem

The committed modality manifest has one direct JSON owner per lesson. Its
reader currently calls the guarded ledger parser and then opens each owner
again to compare canonical formatting. The guarded parser already has an
existing read-with-source variant that returns both the validated value and
the exact text read. On Windows, the full reader took over 75 seconds in a
focused test after the lesson-source setup was repaired. The end-to-end check
also compares the generated outputs and reconstructs the owners, and still
exceeded its existing 60-second integration budget.

The identity-set validator sorts both lists, then uses `Array.includes` for
each expected and actual identity. That is quadratic for a track with thousands
of owners, despite the prior duplicate/case-fold checks already proving the
lists are sets. The check must keep its sorted missing/extra diagnostics while
using set membership rather than repeated scans.

## Contract

Use `readLedgerFileWithSource` for each modality meta and lesson owner. Parse
and validate its returned value exactly as before, and compare the canonical
serialization against the returned text from the same guarded read. Preserve
all layout, identity, symlink, dangerous-key, malformed-JSON, canonical-byte,
and error-message behavior. Do not replace source-byte validation with a stat
or cache shortcut, and do not skip the independent source/narration identity
checks. A second read solely for canonical comparison is not evidence of
greater safety; it introduces a mutation window between validation and check.
Build membership sets once after the existing duplicate/case-fold refusals;
filter the already-sorted arrays against those sets so diagnostics retain their
exact ordering and wording. Do not skip any identity or relax collision rules.

Measure the reader and end-to-end check after this single-read change. If
either still exceeds its budget, profile the remaining phases and keep #18941
open; a passing focused subcase is not full-suite acceptance.

## Acceptance

- Direct-owner tests retain every refusal and canonical-format assertion,
  including source edits and unsafe owner paths.
- Missing/extra identities and case-fold collisions retain their exact
  diagnostic ordering, with a large fixture preventing quadratic regressions.
- A test verifies that the value and canonical text are judged from one
  guarded source read, not two potentially different file versions.
- Focused modality tests, the unweakened Windows/macOS package suites, and
  Ubuntu CI pass on the exact final head before #18941 or its parents close.
