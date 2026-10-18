---
category: Testing & coverage
---

# Bind validation acceptance to actual targets and final physical identity

The unexecuted CLOC41 validator v1 rehashed a saved manifest list but did not
prove the current complete census, and its acceptance predicate allowed a
successful process with no expected target or test-summary coverage. It also
did not bind the final physical identity flag and helper pins into acceptance.
Independent preflight withheld it before any producer ran.

Keep that proposal and refusal as evidence. Recompute the current manifest
set and all dependency edges, derive the selected executable targets from
actual metadata, join successful builds and completed harness summaries to
those targets, and require final source, helper, lock and executable identity
in the acceptance predicate. A zero exit alone is not test coverage.
