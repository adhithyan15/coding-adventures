---
category: Repo policy / workflow reminders
---

# Native validation checkouts need not materialize unrelated curriculum files

A full checkout during PR #18183 recovery exhausted C: despite an estimate
based on tracked payload bytes. More than 262,000 unrelated human-language
curriculum files account for substantial allocation and directory overhead;
payload size alone was not a sufficient capacity check.

Preserve authored edits and native test/coverage receipts first. Remove only
known regenerable output belonging to the current recovery. Materialize all
package/program sources, fixtures, scripts and configuration needed by the
native tests, while omitting unrelated curriculum files from this checkout.
Run the actual native suites and planner, and state the checkout boundary;
this is not evidence that the omitted curriculum was validated. Do not alter
another checkout or discard unpublished checkpoints to make room.
