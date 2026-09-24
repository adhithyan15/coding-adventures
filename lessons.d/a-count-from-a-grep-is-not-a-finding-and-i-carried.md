---
category: Testing & coverage
---

# A count from a grep is not a finding, and I carried one of my own forward for two tranches as if it were

Surveying which language tracks would be expensive to add review lessons to, I ran one command per
track:

```
grep -l "ordered\[\|toHaveLength(" tests/corpus/<track>/*.case.ts | wc -l
```

Punjabi returned **6**, every other track **0**. I wrote *"punjabi is the one remaining track with real
index pins (6 test files)"* into a survey file, then into a scheduled check-in, then into a pull
request description. Two tranches later I opened the files.

**Punjabi has no index pins at all.** Nothing in its corpus tests matches `ordered[`. The six files
matched on the other half of the alternation, `toHaveLength(`, and every one of them is a property
assertion over a *named* set of lessons — durations under a cap, `introduces.knowledge` empty, skills
present or absent — plus one `toEqual([])`. None can be disturbed by adding a lesson, and the
`toEqual([])` is monotone in the direction of the work.

The real constraint was something the grep never looked for: a **272-row session map** in prose,
asserted row-for-row against canonical sequence order.

**Two distinct errors, and the second is the one worth the entry.**

The first is ordinary: a regex alternation attributes every hit to whichever branch you had in mind.
`A\|B` returning 6 tells you nothing about how many matched `A`. The fix is to run the branches
separately, or to print the matching lines rather than count the files.

The second is that **I restated it with increasing confidence at each hop.** The survey file said "6
test files matching `ordered[` or `toHaveLength(`", which is true and hedged. The check-in I wrote
from that file said "the one remaining track with real index pins". The PR body said it flatly. Each
copy dropped the qualifier that made the previous one honest, and none of the three cost anything to
check — the files were on disk the whole time.

**A measurement's provenance decays faster than the measurement.** The number stayed 6; what rotted
was the sentence around it. So:

- When a survey number will steer later work, record **how it was obtained** in the same line, not
  just the value. "6 files matching `ordered[|toHaveLength(`" survives being copied; "6 index pins"
  does not.
- Before a claim derived from a survey becomes a *decision*, re-derive it from the source. The cost
  here was one `grep -rn "ordered\["`, which returned nothing.
- Treat your own notes with the same suspicion you would give a test's title. The
  [[a-test-s-title-is-not-its-assertion-deferring-work-on-the]] entry says read the body rather than
  the name; this is the same failure with my own writing in the role of the name.

Nothing broke. The tranche shipped, and the session map turned out cheaper to satisfy than the pins I
had imagined. That is the point: a wrong constraint that happens not to bind still gets planned
around, and the planning is the waste.
