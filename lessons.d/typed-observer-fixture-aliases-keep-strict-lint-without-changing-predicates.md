---
category: Rust
---

# Typed observer fixture aliases keep strict lint without changing predicates

CLOC34's focused tests all passed after the event fixture repair, but parser
all-target strict lint rejected the new typed visitor's nested tuple/Vec return
type. Production code was not the cause, and no strict success was claimed.

Name the exact tuple shapes TypedOccurrence and TypedLeaf. Keep every field,
predicate, ordering and assertion identical, then rerun the affected tests and
strict lint. Avoid suppressing the lint or removing identity/flag comparisons.
