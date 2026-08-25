### Changed — Spanish has a declared reading order (HL09 step 2)

- 50 Spanish lessons across chapters 8–18 gain a `sequence:`, recovered from
  evidence rather than invented: the `Next: …` sentence ending each lesson's
  Wrap-up Recall, corroborated by `prerequisites:` and `reviews_of`.
- **26 of Spanish's 31 "forward prerequisites" were never real.** With no declared
  order the walk fell back to sorting alphabetically inside a chapter, which put
  `beber` before `comer` and then reported `beber` as depending on a later lesson.
  Declaring the true order removed them:

      Spanish              before   after
      no sequence              56       6
      forward prerequisites    31       5
      forward references      143      99

  Corpus-wide: 565 → **515** unsequenced, 271 → **245** forward prerequisites,
  331 → **300** forward reviews. Spanish's atom figures are unchanged by the
  ordering itself, as they must be — ordering moved no content; the corpus totals
  moved only because a verb tranche landed on main in parallel.
- **Chapter 7's six lessons are deliberately left unsequenced.** `curriculum.json`
  says comer → beber → qué → vivir → dónde; the prose `Next:` chain **and**
  `ES-C07-beber`'s own `reviews_of` say comer → vivir → beber → qué → dónde. Under
  the ledger's order, `beber` reviews a lesson that has not happened. Guessing
  would bake a false ramp into every later measurement, so they wait for a ruling.
  A test pins exactly which six remain, so this cannot be forgotten.
- Chapter 18 is the weakest recovery: none of its ten lessons carries a `Next:`
  line, so its order rests solely on `prerequisites`/`reviews_of`. Those happen to
  form one clean chain, but with no prose corroboration.
- **Known remainder: the numbering is cramped.** Chapters 19–33 were already
  sequenced at 640–845, so chapters 7–18 had to fit between 510 and 640 — 129
  integers for 56 lessons. Spacing is therefore **2**, not the intended 10, leaving
  almost no insertion room in a track meant to grow from 146 lessons to thousands.
  Renumbering the whole track by 10s is mechanical and should follow.

