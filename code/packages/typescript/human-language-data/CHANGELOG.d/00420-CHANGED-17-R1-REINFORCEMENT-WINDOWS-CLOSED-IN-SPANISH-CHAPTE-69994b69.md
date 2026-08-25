### Changed — 17 R1 reinforcement windows closed in Spanish chapters 3-6 (HL09 step 3)

- Records practice the lessons **already do**. 17 atoms across 11 lessons gain an entry
  in `practises.knowledge` **and** in the `assesses=[...]` directive of the specific
  body block that exercises them.

      corpus R1 misses      766 -> 749   (exactly the 17 wired)
      corpus never revisited 767 -> 755   (12; five already had a distant revisit)
      Spanish never revisited 102 -> 90 of 199

  Those are the figures on the corpus of the day. A verb tranche landed on main in
  parallel, so the committed pins read 1599 atoms / 745 never revisited / R1 778;
  what this change is accountable for is the 17 windows and the 12-atom move.

- **Only 17 of the 58 R1 misses in these chapters could be wired.** The other **41 are
  genuine absence** — no lesson in the window touches the atom at all, so there is no
  practice to record. That is what HL09 §7.2 predicted, and it is the honest result:
  a `practises` entry the prose does not back is worse than an open window.
- **A frontmatter-only edit was tried first and rejected by the schema.** Adding an
  atom to `practises.knowledge` without declaring it in a body block fails validation
  with `schema-v2-block-assessment-missing`. That rule is the schema enforcing HL09
  §7.2's honesty principle directly: **you cannot claim practice without pointing at
  where it happens.** The rejected attempt was reverted, not worked around.
- Placement is evidenced, not guessed. Each atom went to the block containing the
  drill or recall that exercises it — mostly `## Guided Practice` and
  `## The word, taken apart`. A "what you've learned" bullet was **rejected as
  practice** five times during the audit; a recall *task* ("order the three *hasta*
  goodbyes by time") was accepted, because it cannot be done without the words.
- **R2 is unchanged at 1107, and that is correct** — closing a near window does not
  close a far one. R2, R3 and R4 need dedicated `review` lessons, per §7.2.
- **Open question for the project owner: 15 of 18 `ES-ETYMON-*` atoms could not be
  wired.** This is systematic rather than eighteen oversights — an etymon is cited when
  introduced and never re-cited; only `hasta` comes back. Either etymon atoms should be
  exempt from the retrieval schedule, or lessons should re-cite earlier etymons the way
  they re-use vocabulary. Not decided here.

