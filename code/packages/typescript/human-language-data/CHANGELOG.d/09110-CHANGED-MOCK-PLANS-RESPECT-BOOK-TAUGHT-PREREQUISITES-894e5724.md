### Changed — mock plans respect book-taught prerequisites

- Per-form `bookPrerequisites` now require real level-eligible lesson teaching
  and practice, or exact target-script token closure against learner and examiner
  stimuli, before a mock counts as complete.
- Marwadi A1 functional frames and Punjabi A2 Gurmukhi surface closure remain
  projected but blocked by stable book-teaching IDs until their prerequisite
  work lands. Missing unpublished stimuli cannot silently satisfy the
  declaration, and the planner does not print confidential mock text or token
  lists.
