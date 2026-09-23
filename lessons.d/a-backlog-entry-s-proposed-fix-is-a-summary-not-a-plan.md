---
category: Repo policy / workflow reminders
---

# A backlog entry's proposed fix is a summary, not a plan; cost it before trusting it

`HL-C420` carried a clean "Proposed fix, not yet taken" table: re-file four
chapter-268 lessons from `SPINE-HANDLE-TRAVEL` onto nodes matching what they
teach. Implemented literally, it raised four validation errors, because the node
**declares the three concepts those lessons are tagged with**, and
`curriculum.ts:509` treats a content lesson off its concept's owning node as
local support that must hang from an extension.

So the real change was never "re-file four lessons". It was "re-own three
concepts on the shared 23-language spine, or teach the validator that a
`relocates` ledger blesses an off-node lesson" — and it bought exactly **one**
exam item. Reverted.

**Cost the fix before trusting the entry, and cost it against what it buys.**
A backlog entry is written at diagnosis time, by someone who had just understood
the *problem*. The fix paragraph is usually the least-verified part of it, and
it is the part that reads most like a plan.

**The tell is a structural dependency the entry never mentions.** HL-C420
discussed `spine_node` — what a lesson *names* — at length, and never mentioned
`concepts` — what a node *owns*. One `cat` of the spine node would have shown
the binding. This is the second time in the same pair of entries that a
recommendation came from a summary rather than from the file it was about; the
first is recorded in HL-C418's own method note.

**Reverting a measured attempt is a result, not a failure**, provided the
measurement is written down. The attempt produced the numbers the entry had only
estimated (`objectiveFailed` 48 → 47, A1 holding at 0 as the control) and the
error list that shows why the cheap version cannot work. Both are now in the
entry, so the next attempt starts from the real shape instead of re-deriving it.
