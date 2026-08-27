## 0.8.1 — carry etymological roots on each lesson (HL03 phase 3 prerequisite)

- `Lesson` now carries `roots: string[]` — the etymological roots a lesson cites
  (e.g. `["bonus", "dies"]`). `toLesson` maps them from the frontmatter the same
  way it maps `prerequisites`; the human-language-data parser already extracted
  them, they just were not threaded into the app's `Lesson`.
- This is the **join key for cross-language connections** (the next phase): two
  lessons in different languages that share a root are etymologically linked.
- Tests: roots parse through (a lesson citing none gets `[]`), and — against the
  real curriculum — the Sanskrit root `dhanya` is carried by lessons in more
  than one chain language (Hindi/Kannada/Telugu). Both fail if roots aren't
  plumbed, confirmed by injection.

