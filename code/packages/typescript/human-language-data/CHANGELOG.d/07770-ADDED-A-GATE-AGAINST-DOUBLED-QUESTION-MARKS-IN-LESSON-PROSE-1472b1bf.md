### Added — a gate against doubled question marks in lesson prose

- **`tests/double-question-mark.test.ts`.** Review lessons turn each gloss into
  a spoken prompt by adding a question mark, so a gloss that is itself a
  question word ("Why?", "How much?") printed as "Why?? (**quārē**.)". Five
  lessons carried it, and they are fixed in their tracks' changelogs. The new
  test asserts zero doubled question marks in learner-facing prose: block bodies
  and preamble, without frontmatter, directives or fenced code. It also has an
  anti-vacuity case proving the detector fires on the shape it replaced and on
  "?"-for-script mojibake, and stays quiet on Spanish "¿…?" and French spaced
  " ?". The corpus is already clean, so this is an assertion of zero, not a
  ceiling.
