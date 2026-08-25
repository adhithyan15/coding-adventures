### Fixed - an ordering check that graded file order instead of reading order

Found while testing the above. `sequence` arrives from the frontmatter parser as
a **string**, and the first draft of `sequenceOf` tested `typeof raw ===
"number"`. Every lesson therefore fell through to `Infinity`, the sort became a
no-op, and the out-of-order check silently graded lessons in whatever order the
array happened to hold -- passing on any fixture that was already sorted.

Now coerced the way `continuity.ts`'s `declaredSequence` already did it, with a
regression test asserting both directions: array order wrong but `sequence`
right must not fire, and array order right but `sequence` wrong must.

