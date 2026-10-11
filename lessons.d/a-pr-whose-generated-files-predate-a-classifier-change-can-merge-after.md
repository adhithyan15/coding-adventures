---
category: CI & GitHub Actions
---

# A PR whose generated files predate a classifier change can merge after it and land stale derivations on main

#17162 (a Punjabi R4 lesson, PA-R172-form-order-again) was built and checked
on a main from before the driver-safety classifier changes (#17203/#17204). It
merged afterwards with its committed `core/lesson-modality` record still saying
`coreDrivable: true`. Regenerating on the new main says `pen` — the lesson's own
authored reason is "inspect and copy printed Gurmukhi form labels". Main's
`check:modality` was red from that merge until the next PR regenerated it, and
for that window the audio announced a copy-the-form task as doable in the car.

Generated outputs here are DERIVED from both the sources and the generator code.
A branch that does not touch the generator can still be stale against a main
whose generator moved. CI on the branch passes (old code, old outputs agree);
the merge lands outputs main's code would not produce.

What to do: before pushing or re-pushing a branch that carries generated files,
rebase onto current main and regenerate (`generate:books`, `generate:narration`,
`generate:modality`) — not only when the branch's own sources changed. A required
"branch up to date" rule, or a post-merge regenerate check on main, would close
this for everyone.
