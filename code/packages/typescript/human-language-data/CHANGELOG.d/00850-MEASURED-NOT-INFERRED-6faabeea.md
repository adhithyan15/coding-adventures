### Measured, not inferred

Every figure below is a set difference against the pre-change corpus, computed by
loading both with the same build.

- `totalLessons` 1680 → 1684, `pen` 58 → 62 — the four new lessons. They are `type:
  writing` and therefore `pen`, even though all four are reading-only.
- `voice` 1100 → 1106, `sight` 522 → 516, `drivableLessons` 1100 → 1106 — the same six
  lessons, no others.
- `drivablePrefixTotal` 913 → 918 and `unstartableChapters` 140 → 139. Chapter 3 gains
  6 — its first lesson needed eyes, and the whole chapter is now one ear-only run — and
  chapter 25 loses 1, because holding the 3:1 cadence puts `TA-W10` *between* its two
  speaking lessons rather than after them. `TA-W06` already sits mid-chapter in chapter
  18, so this is the established shape, not a new one. `unstartableChapters` is chapter
  3 alone.
- `fullyDrivableChapters` 327 → **324**, which moves the wrong way. Chapters 25, 27, 29
  and 31 each take a writing lesson and stop being fully drivable (−4); chapter 3
  becomes fully drivable (+1). That is the honest cost of paying the debt where it
  belongs. Corpus `coreDrivable` does not move, but not because these blocks detach —
  rule 1 classifies a `type: writing` lesson as `pen` without reading its body, so all
  four record `coreDrivable: false`. It holds because the six lessons that flipped were
  already core-drivable and the four new ones were never counted.
- `payoffsNotRepresentative` 27 → **29**. Tamil 25 (2/3 → 2/5) and 27 (2/2 → 2/5) fall
  below the 0.5 floor because they gained script atoms their speaking payoffs do not
  assess — the same trade chapter 13 already records, and both are now noted in
  `tamil/chapters.json`. Tamil 29 and 31 took the same hit and did **not** join: each
  landed on exactly 2/4 (0.50). The difference is one atom of arithmetic.
- `atomsTaught` 2631 → 2640 — 2 + 3 + 2 + 2 new atoms.
- `atomsNeverRevisited` rises 469 → 470. Three in — `TTA-01`, `U-SIGN-01` and
  `READ-QUESTION-02`; nothing follows `TA-W13`, so its two are orphans by construction.
  Two out: extending the strand finally re-uses ெ and ப, so `E-SIGN-02` and `PA-YA-01`
  leave the set. `READ-PEYAR-03` stays. Three in, two out. `AA-SIGN-01`, `II-SIGN-01`,
  `NGA-LLA-01` and `READ-NAAN-02` never become orphans, because each lesson declares —
  and genuinely assesses — the earlier letters its own word is built from: `TA-W11`
  credits the ந it builds **நீ** from, `TA-W13` credits the ள it re-reads in
  **ள்** and the **நான்** it holds against **நீங்கள்**.
- `missedByWindow.R1` 864 → 874. Nine are the new atoms. The cadence puts consecutive
  strand lessons **four** apart in reading order, and R1 is a 1-3 window — an early draft
  put `TA-W10` after the fourth speaking lesson instead of the third, which made that gap
  3 and quietly falsified this rationale; the sequence was moved to 785 so the claim and
  the corpus agree. The tenth miss is not new and is worth naming: interleaving at
  chapter 29 pushes `TA-LEX-AFTERNOON-BOUNDARY-01`'s reinforcement past R1.
- `missedByWindow.R2` 1799 → 1800. Four of the nine new atoms miss R2; the other five —
  `AA-SIGN-01`, `II-SIGN-01`, `NGA-LLA-01`, `READ-NAAN-02`, `READ-NIINGAL-03` — do not,
  because a later strand lesson practises them 5-15 lessons on, which is what threading
  them through `practises` was for. Three pre-existing atoms are pulled back inside R2
  against that: `PA-YA-01`, `ETYMON-KAALAI-01` and `ETYMON-MAALAI-01`. `E-SIGN-02` leaves
  the orphan set but not R2 — `TA-W10` re-uses it at a distance R2 does not count.
  Four in, three out.

