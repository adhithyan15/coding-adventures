---
category: Security boundaries
---

# Require a current positive verdict prefix in artifact installers

Independent preflight found that the minima source installer and documentation
writer accepted the substring PASS anywhere in a report. A withheld or negative
report discussing an earlier PASS could satisfy that predicate. Neither refused
helper was executed. The source installer successor was independently reviewed
before its actual successful one-file application.

Require the exact current positive verdict opening, reject negative verdicts in
that opening, bind the full expected source head near the start, and pin the
report's exact bytes. The source installer also binds its filename and SHA256 in
the review. Refresh a conflict-checked union of source, helper, receipt, raw and
copied-artifact pins at the final write boundary. Label retained old claims as
historical; a new current paragraph does not repair an unmarked stale claim.
