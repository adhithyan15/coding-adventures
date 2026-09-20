## 2026-09-07 — With, from, to whom, and how Marathi has things: 133 → 142 of 301 A1 points

- **Chapters 45–48 close the postposition column, and A1 exam coverage moves
  133/301 (44%) → 142/301 (47%).** Re-measured with `measureExamCoverage` on the
  merged tree.
- **Eleven items, nine points.** `MR-A1-P-03` (accompaniment and instrument),
  `MR-A1-P-04` (source), `MR-A1-PRON-08` (the pronoun's own oblique),
  `MR-A1-NG3-07` (origin), `MR-A1-VP-04` (the animate direct object),
  `MR-A1-VP-05` (the indirect object), `MR-A1-V-20` (possession),
  `MR-A1-NG1-01` (existence and non-existence) and `MR-A1-NG3-05` (orientation
  and direction). **Case and postpositions 3/6 → 5/6**, **Spatial notions 4/7 →
  6/7**, **The verb phrase 3/6 → 5/6**, **Existential notions 2/5 → 3/5**.
- **What is taught, one item per lesson:**

  | ch. | items |
  |---|---|
  | 45 | **बरोबर** alongside · **माझ्याबरोबर** the pronoun's oblique · **-शी** engaged with · **-ने** by means of |
  | 46 | **-हून** from · **पासून** from, since · **कुठून** from where |
  | 47 | **मित्राला** the animate object marker · **मित्राला … देतो** the recipient |
  | 48 | **-कडे** toward, at somebody's side · **माझ्याकडे … आहे** how Marathi has things |
- **Three English *with*s, three Marathi endings, and one question that sorts
  them.** *Is this a companion, a partner in the act, or a tool?* — **बरोबर**,
  **-शी**, **-ने**. English collapses all three; a learner who has only met
  *with* over-applies whichever one they met first, so the contrast is the
  lesson rather than a footnote to it.
- **Marathi has no verb for *to have*, and chapter 48 is what it uses instead.**
  **माझ्याकडे पाणी आहे** is *at-my-side water is*: the thing possessed is the
  SUBJECT, **आहे** agrees with it and not with the owner, and denial is the
  ordinary sentence negation of chapter 32. Every piece was already owned; only
  the assembly is new. That construction alone closed `MR-A1-V-20` and
  `MR-A1-NG1-01`.
- **The dative marker gets its other two jobs.** **-ला** has been in the book
  since chapter 16 and only ever on an experiencer (*मला चहा आवडतो*). Chapter 47
  names the two it also does: the **animate direct object** — Spanish's personal
  *a* under another marker, and a rule rather than a shade — and the
  **recipient**. One ending, three roles, and the verb decides which.
- **A side is not a direction, which is why they are four chapters apart.**
  **उजवा** and **डावा** were taught in chapter 44 as descriptions and explicitly
  left there; **-कडे** is what turns them into **उजवीकडे** and **डावीकडे**, on
  the FEMININE form, because the unspoken noun *बाजू* is feminine.
- **Zero new glyphs, zero closure violations, zero never-taught glyphs.** One
  draft example was rewritten for this: *by hand* wanted **हात**, which no
  lesson teaches, so the instrumental is demonstrated on **डोळा** instead — a
  noun the book has had since chapter nineteen.
- **`reinforcementWindowMisses` 352 → 348, and the tranche's own debt is zero.**
  R4 fell 84 → 80; R1, R2 and R3 did not move. Decomposed: **0 created**, **27
  exposed** by the added length (R2 for 3 older atoms, R3 for 11, R4 for 13) and
  all 27 retrieved on purpose, **4 pre-existing misses cleared**.
- **A forward reference this tranche created, and the fix.** The pronoun-oblique
  lesson was first headed **माझ्या-**, and chapter 41 had already written
  **माझ्या घरात** in bold sixteen lessons earlier — so a stem fragment as a
  headword retroactively made an existing lesson a forward reference.
  `forwardReferences` went 4 → 5 and is back at 4: the headword is now
  **माझ्याबरोबर**, a whole word the lesson actually teaches, which is a better
  headword on its own terms.
- **A rule statement, spent and then given back.** The chapter-47 review's
  wrap-up asked *which kind of direct object never takes it*, which
  `measureInfoDump` reads as a rule statement — and `ruleStatements` is a
  CEILING at 30 that may fall and never grow. The question was rewritten to ask
  about a sentence the reader has in front of them instead. 31 → 30.
- **A third regression, caught by reading the printed page rather than a
  number.** Four lessons in chapters 45 and 47 derived `sight` rather than
  `voice`, so the book printed *"Hands-free start: none of the 5 lessons"* on
  two chapters that teach nothing visual. Each was one incidental phrase
  tripping a `SIGHT_CUE` rule — *"Look at the base it shares"*, *"Read the
  middle column"*, *"I see the water"*, *"what you look at"*. None was
  load-bearing, and two of the replacements read better: Marathi has no
  articles, so **I see water** is the truer gloss of *mī pāṇī pāhto*. Both
  chapters open hands-free again.
- **`atomsTaught` 261 → 272; `atomsNeverRevisited` 1; `durationViolations` 0.**
- **What was deliberately left uncovered, with the reason written into the
  inventory:** `MR-A1-P-05` (**-कडे** is taught and **पर्यंत** is not, and
  writing **पर्यंत** needs the reph of `MR-A1-OR-21` — the two debts are one
  debt, and half a pair is not this point), `MR-A1-OR-21` (the rakaar has been
  SHOWN since chapter 18, in **मित्र**, and never named; two script lessons
  close it and `MR-A1-P-05` together), and `MR-A1-F1-03` (name and origin now
  work; AGE needs a number above five, so the interview's opening question is
  one number short of complete).

