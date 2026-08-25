### Fixed — `measureRamp` was called by nothing but its own test

- The gap report now carries a `ramp` section, so `maxNewAtomsPerLesson` and
  `maxNewAtomsPerChapter` are finally read by something a human sees. They had been
  declared in `core/chapter-policy.json` since HL08 and enforced by nobody — policy in the
  sense that a sign is policy. The first published figures: **40** lessons over the atom
  budget, **25** chapters, with **572 lessons (47%) unmeasurable** because schema-v1
  declares no atoms.

