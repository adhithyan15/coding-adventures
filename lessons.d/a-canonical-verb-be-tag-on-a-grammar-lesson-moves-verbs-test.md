---
category: Testing & coverage
---

# A canonical VERB-BE tag on a grammar lesson moves verbs.test's tracksWithNoCoreVerb pin

The Marwadi A2 core put the canonical spine tags on its grammar lessons
(VERB-BE, VERB-PRESENT-HABITUAL, VERB-NEGATE, QUESTION-POLAR, VERB-PAST,
VERB-FUTURE) so the A2 nodes would be realized. The planner's pin list named
the lesson budget, the owned activities, the exam inventory, the letter
ceiling, the filmstrip count and levels.test, and every one of those was
updated. The full package suite still failed in a file nobody had listed:
`tests/verbs.test.ts` pins `tracksWithNoCoreVerb` at 2, and it fell to 1.

The cause is the cross-language verb join (`verbCoverage`). It only sees
canonical concept tags. Marwadi's forty-odd verb lessons all carry namespaced
`MW-VERB-*` tags, so the track counted as teaching no core verb at all. The
grammar lesson tagged VERB-BE (हूं · है) is the first canonical core verb on
the board, so the track left the list. Only VERB-BE counts there: the other
canonical tags are grammar concepts, not core verbs.

The fix was the pin, with a comment saying which lesson moved it and that the
mean (41) held. Do differently: when a core adds a canonical VERB-* tag to a
track whose verbs are namespaced, expect `verbs.test.ts` to move, and check
`verbCoverage(...).tracks` for that language before running the full suite.
Run the full suite, not a selection, before committing a core; a selection
built from the planner's pin list will not contain the file that moves.
