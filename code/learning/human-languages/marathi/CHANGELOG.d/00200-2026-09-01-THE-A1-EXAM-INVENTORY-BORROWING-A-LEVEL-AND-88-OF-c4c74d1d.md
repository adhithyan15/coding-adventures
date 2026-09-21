## 2026-09-01 — The A1 exam inventory: borrowing a level, and 88 of 301

- **Marathi now has an A1 target list, and the corpus covers 88 of its 301 points
  (29%).** No awarding body publishes an A1 Marathi syllabus:
  `core/exam-levels.json` records this track as `exam: "no widely-sat ladder"`,
  `basis: "editorial"`. The parallel Hindi effort settled the search negatively
  against the best-placed South Asian candidate — DBHPS publishes examination
  names and prescribed readers and no syllabus, Kendriya Hindi Sansthan and the
  CHTI publish none, and no Council of Europe Reference Level Description exists
  for Hindi. **There is no South Asian equivalent of the Plan Curricular**, and
  the file records `THE SEARCH IS SETTLED, DO NOT REPEAT IT` so no later tranche
  spends hours confirming it.
- **So the file borrows a LEVEL rather than a LANGUAGE.** Spanish is the one
  track here whose A1 point set restates a real awarding body's published
  inventory, which makes its 273 points an *attributable* statement of what an A1
  learner must handle. Each was walked for what it **demands** — "give personal
  information: name, age, origin", "negate a statement", "ask a price" — and the
  Marathi point carrying the same load was written down. `derivedFrom` records
  the mapping per point, so the derivation is auditable line by line: **273 of
  the 301 points derive from Spanish, 28 are Marathi-specific**, and 266 of
  Spanish's 273 points are accounted for with the other 7 listed in
  `proxy.notTransferred` with reasons (three article points — Marathi has no
  articles; two capitalisation points — Devanagari is unicase; two written-accent
  points — Devanagari marks no stress). A test asserts that derivation is
  **total** in both directions, and writing it is what caught `A1-O1-06` going
  missing.
- **Nothing is attributed to anybody.** The `about` disclaims, in those words,
  both the Maharashtra Directorate of Languages and DELE / the Instituto
  Cervantes / the Plan Curricular — because "derived from the DELE A1 inventory"
  is one careless edit away from reading as "DELE says this about Marathi". Every
  Marathi exponent is this project's editorial judgement. Anchors now carry four
  kinds: `sourced-proxy`, `external-framework`, `project-owned`, `editorial`.
- **The rebuild moved the number from 55/131 to 88/301, and both halves are the
  result.** The numerator rose because walking a real inventory found taught
  material an editorially-chosen list had never asked about — evaluative
  notions, mental notions, the colon a form label uses. The denominator nearly
  trebled because it found **twenty thematic domains nobody had enumerated**:
  education, work, leisure, media, housing, services, shopping, health, travel,
  money, government, the arts, religion, the natural world. The corpus covers
  almost none of them. That is not the proxy being unfair; it is what an external
  boundary is for.
- **29% is a FLOOR, not an estimate, and the file says so twice over.** Coverage
  reads *declared* atoms. **SCHEMA-V1:** 26 of the track's 205 lessons — the whole
  of chapters 9 to 12 — declare none while teaching **मी**, **तू / तुम्ही**,
  **माझं**, **काय**, **कसा / कशी / कसं**, **तुमचं नाव काय आहे?**,
  **तुम्ही कसे आहात?**, **काम करणे** and **राहणे**. **EMPTY-INTRODUCES**, which is
  worse because it hides inside schema-v2 where everything *looks* declared: of 66
  v2 lessons carrying `introduces: []`, most are honest retrieval lessons, but
  `MR-R22-request-verbs` and `MR-R23-wellbeing-verbs` drill five polite
  imperatives (**द्या, प्या, आणा, ठेवा, बसा**) and a future (**चालेल**) while
  declaring only the *infinitive* atoms they review, and
  `MR-A1M17-guided-message` / `MR-A1M18-independent-message` teach the guided
  32-word and independent named-reader message — **the A1 writing paper's entire
  second task** — while declaring nothing at all. Thirteen points are marked
  **MEASUREMENT GAP**: taught, unmeasurable, and far cheaper to close than the
  content gaps beside them. The first draft recorded several as "untaught", which
  would have sent an author to rewrite chapters 9 and 24.
- **Every probe names an atom that exists today; nothing is aspirational.** A
  guessed id resolves to "not introduced" forever and is indistinguishable from
  real debt, so an uncovered point is `probe: null` plus a note saying what is
  present and what is missing — `MR-A1-OR-05` records that four of the five
  retroflex letters are taught and **ढ** is not. All four `scope` dimensions stay
  `partial`; a proxy does not close a dimension.
- **What is genuinely missing, ranked by what it unblocks.** The **oblique stem**
  stands between every taught noun and every postposition, so place, time,
  direction and accompaniment are blocked behind one rule. The **ergative -ने** is
  the gate on the whole past tense, not a refinement of it. The **interrogatives**
  कोण, कुठे, कधी are absent in both schemas, and the reading and listening papers
  are wh-item papers. Then the cheapest large win: the **ten Devanagari digits**,
  which no atom teaches and which the writing paper's "complete a practical form"
  needs for a phone number, a house number and a date.
- **`npm run plan` stops calling Marathi unmeasurable.** It now emits
  `exam-point — marathi: cover 213 of 301 A1 exam point(s) marathi does not
  teach`, the corpus-wide backlog moves 190 → 403 across five written
  inventories, and tracks that cannot be measured at all fall 20 → 19. Nothing
  regressed: the 213 were always there and were invisible.
- **Method recorded for the other nineteen editorial tracks** in
  `BACKLOG.d/02610-HL-C290-…`, so this does not have to be rediscovered.

