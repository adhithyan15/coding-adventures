---
category: Repo policy / workflow reminders
---

# Replace the generated lesson placeholder instead of appending a completed explanation after it

The first pipeline documentation carrier retained five generator TODO lines
despite completed explanations nearby. Independent final review withheld the
carrier, and the actual lessons validator exited 1; the doc-shard check and old
plan exits remain separately recorded. Remove only those obsolete placeholders,
fill this new lesson in place, and rerun real documentation/lesson/diff checks
and final-head planning. No Rust source, assertion, oracle, lock or cap changes.
Writing explanatory prose below the scaffold does not satisfy the validator.

Removing a terminal scaffold line can leave a blank line at EOF. The recovery
precommit diff check refused that whitespace. A second preflight caught stripped
porcelain status indentation before writes. Parse raw NUL-delimited status,
normalize only changed lesson EOF, and preserve both failed helper outcomes.
