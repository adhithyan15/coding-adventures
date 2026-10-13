---
category: Testing & coverage
---

# When iterating within a pre-sliced section, the final part must end at the slice boundary rather than search for the section marker that was already removed.

The C2 test first sliced the Reading paper before `## Listening`, then tried to end Reading Part 4 by finding `## Listening` inside that slice. Because the outer slice excludes its end marker, the nested search necessarily returned `-1`. Use an undefined end for the final part within a bounded section, or slice every part from the original paper, and reason explicitly about whether helper boundaries are inclusive or exclusive.
