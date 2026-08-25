### Added — does the course have a memory of itself? (HL09 step 1)

- Add `src/continuity.ts`: `measureContinuity` measures the three things a
  per-lesson budget cannot see, published in the gap report's new `continuity`
  section. The ramp budgets measure how big each *step* is; this measures whether
  the steps hold together.

      order: 565 lessons with no declared sequence across 19 tracks;
             271 prerequisites and 331 reviews pointing forward
      reinforcement: 746 of 1469 atoms never revisited (51%);
             missed windows R1 745, R2 1068, R3 649, R4 132
      forward references: 509 uses of material a later lesson teaches

- **You cannot review a lesson that has not happened yet**, and **331** do. A
  forward `reviews_of` cannot close a reinforcement window — it names lessons, not
  atoms — but it is still an authored claim about order, and a claim pointing
  forward is wrong on its own terms. `ES-C07-beber` reviews `ES-C07-vivir`, which
  `curriculum.json` places *after* it.
- **Order comes first because everything else depends on it.** 565 lessons carry
  no `sequence`, so their reading order exists only inside hand-typed LaTeX —
  Spanish 56 of 146, French **64 of 73**. A ramp whose order is unknown cannot be
  verified at all, so every other number here is provisional until this is zero.
- **51% of taught atoms are never practised again.** HL00 specified the schedule
  (N+1, N+3, N+7, N+15), defined a `review` lesson type to carry it, and named
  `session-map.md` as the artifact that verifies it. The corpus has **zero**
  `review` lessons and a session map covering 3 chapters of 33. The schedule was
  specified and never built.
- The measurement reads `practises.knowledge`, **never `reviews_of`** — which 144
  of Spanish's 146 lessons set, and which cannot close a window because it names
  *lesson ids* while atoms live in another namespace. Measuring that field would
  report a corpus that reinforces beautifully and teaches nothing twice.
- Windows are judged **only where the track is long enough to contain them**. A
  25-lesson track missing R4 has not failed; it has not got there yet.
- **Forward references are proved, not guessed.** A word is reported only when a
  *later lesson's own headword* teaches it, so the finding carries its own
  evidence and cannot false-positive on ordinary English prose. It reproduces
  what a human reviewer found by reading: `ES-C07-beber` rewards the learner with
  *"Como pan y bebo agua"* while **`pan` and `agua` are chapter 26**, and
  `ES-C08-practice` drills `diecinueve` in a chapter that taught 1–10.
- Three false-positive classes were found by censusing the output rather than
  guessing, and each is excluded on principle: **single-character headwords** from
  `writing` lessons (a Cyrillic `е` or a Devanagari mātrā matched in every lesson
  of its script — five scripts' worth), **pattern notation** like `e→ie`, and
  English collisions like `once` (18 hits) — only lessons whose type is `word` or
  `phrase` create a matcher at all.
- Honest limit, stated because it changes how the number reads: a word the course
  **never** teaches anywhere is invisible here. Chapter 7's `¿Algo más?` and the
  untaught `un`/`una` do not appear, because nothing in the data says they are
  target language. 509 is a floor.
- Report-only, per the HL05 precedent: the debt predates the measurement.
- `readingOrder`, `frontmatterList` and `introducedAtoms` are now exported from
  `ramp.ts` and shared. Two independent orderings that drifted apart would make
  the two reports disagree about which lesson comes first, silently.

