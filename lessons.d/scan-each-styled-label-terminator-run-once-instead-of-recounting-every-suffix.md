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

The first regression build guessed source/target fields instead of inspecting
SwimlaneEdge.from/to and failed to compile. The receipt guard rejected that
exit101 with zero executed tests; preserve it as a compile failure, fix only
the test names, then require a named executed assertion failure and exact
operation evidence before accepting the red phase.

Strict all-target Clippy rejected an explicit Option-none return after the
cursor scan even though all531 affected tests passed. Use the equivalent
question-mark return rather than a lint allowance; retain the partial native
receipt at its real head and rerun validation after the final source edit.
