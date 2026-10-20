## Chapters 1–6 — block-boundary prompt closure

- Declared introductions and assessments at every typed block boundary across
  all 51 schema-v2 lessons. Guided production and recall now fail validation if
  they ask for an atom outside the lesson's transitive knowledge frontier.
- Made the boundary order executable: a block is checked before its own
  introductions become available, and every lesson-level introduction and
  practice atom must be accounted for by the body AST.
- Removed early production of *muy bien*, *¿y usted?*, *el gusto es mío*, and
  later preview forms; promoted *te llamas* and *gusto* to explicit atoms; and
  completed the practice declarations found by the editorial pass.
- Regenerated all six chapter fingerprints without exposing metadata comments
  in the printed book or Language Ladder lesson copy.

