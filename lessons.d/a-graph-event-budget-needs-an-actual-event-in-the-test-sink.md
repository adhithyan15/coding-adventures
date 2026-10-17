---
category: Testing & coverage
---

# A graph event budget needs an actual event in the test sink

CLOC34's first focused run built successfully but failed its max_events=0
refusal case. The new test sink only called try_merge, which buys node/edge
capacity; the contribution-event budget does not reject that bare merge when
chronology is not enabled. The old CLOC32 sink also contributed an event.

Keep all three max_nodes/max_edges/max_events cases. Add an explicit checked
contribution after merge and before publishing the test receipt, so each budget
is exercised by the operation it actually constrains. Preserve the failed run
and distinguish this fixture repair from a production safety change.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
