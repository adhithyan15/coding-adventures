---
category: Rust
---

# Scan each styled label terminator run once instead of recounting every suffix

The styled Swimlane label scanner restarted a repeated-character count at every
interior character. Valid flat labels with a nonterminating dot run and a later
valid terminator caused quadratic work despite a depth128 guard. The ordinary
526-test/eight-target parser/drawing suite passed, but independent review found
P2 #17335; its original native receipt remains successful while security
acceptance remains rejected.

Commit the work contract first. Observe actual predicates in the original
scanner, reproduce exact-work red failures, then use one monotone run cursor
and candidate prefixes with constant state. Preserve empty-label refusal,
first eligible terminator, prefix-inside-run and standard operator precedence;
Unicode label text must not be confused with ASCII delimiter byte positions.
Full downstream tests and an independent exact-final-head review remain
required; Miri DEFERRED/PENDING is never acceptance of a resource finding.
