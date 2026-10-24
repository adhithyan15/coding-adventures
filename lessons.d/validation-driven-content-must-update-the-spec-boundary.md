---
category: Testing & coverage
---

# Validation-driven content must update the spec boundary

The Punjabi student-noun prerequisite originally forbade any etymology. A
later root-ledger validation required a visible, sourced borrowing note, so
the implementation and focused test gained that note while the written spec
kept the old prohibition. The generated book and narration were internally
consistent, but the specification and executable evidence contradicted each
other.

When a broader validation contract changes required lesson content, update the
feature spec in the same fix. Preserve the narrow boundary explicitly: here a
single cited borrowing note is allowed, while morphological decomposition and
productive derivation claims remain forbidden. Pin both the allowed evidence
and the forbidden generalization in the focused test, then rerun exact-head
review before publishing.
