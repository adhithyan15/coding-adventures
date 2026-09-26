---
category: Repo policy / workflow reminders
---

# Some tracks pin exactly one objective activity per lesson, so generated lessons there need an activity

Marwadi's corpus test asserts that every lesson compiles to exactly one
objective activity (`compileLessonActivities(lesson.blocks).length === 1`). It
also lists every activity id. The generated vocabulary lessons carried none, so
the pin failed. Urdu chapters 3-5 and Persian have the same kind of pin on a
smaller scale.

The fix: give each generated lesson a typed-recall activity. A word lesson asks
for its headword and accepts the romanization. A review asks for the first word
its Warm-up retrieves. Put the directive directly after the Wrap-up Recall
hl-knowledge line. Count the generated ids by pattern instead of adding
hundreds of literals to the hand-written list.

Before generating for a track, grep its corpus tests for
`compileLessonActivities` to see whether it needs this.
