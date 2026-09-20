## Chapters 33-34 — The eight verbs (2026-08-07)

- Added eight schema-v2 lessons carrying the eight canonical concepts fifteen
  other tracks already teach — `VERB-THINK`, `VERB-UNDERSTAND`, `VERB-READ`,
  `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP`, `VERB-LIKE-LOVE` — making
  Malayalam the sixteenth track to hold all of them, and the fourth Dravidian
  contributor after Tamil, Kannada and Telugu.
- **Two chapters of four, never one of eight.** Chapter 33 introduces 9 atoms
  and Chapter 34 introduces 9, both under the `maxNewAtomsPerChapter` budget of
  12 that one chapter of eight would have blown. Each carries its own `canDo`
  and its own payoff, and both payoffs assess **every** atom their chapter
  introduces — 9/9 and 9/9, representativeness **1.00** against the 0.5 floor.
- **Ch. 33 (The Mind and the Page)**: ചിന്തിക്കുക, മനസ്സിലാക്കുക, വായിക്കുക,
  എഴുതുക. Its spine is a diagnostic the learner can hear: the **ഇ** of
  **‑ഇക്കുക**, and the **‑ഇച്ചു** past that goes with it, mark a Sanskrit
  borrowing. മനസ്സിലാക്കുക comes apart into *manassŭ* + locative *‑il* +
  *ākkuka* — "to put in the mind" — and its intransitive twin **എനിക്ക്
  മനസ്സിലായി** puts the understander in the dative, joining Ch. 6's *enikku
  malayāḷaṁ aṟiyāṁ*. എഴുതുക is Tamil **எழுது** unchanged, carrying the **ഴ**
  that Kannada and Telugu lost — and with it the verb, which is why they write
  with the line-drawing root Malayalam kept for വരയ്ക്കുക, "to draw."
- **Ch. 34 (Taking, Asking, Helping, Liking)**: എടുക്കുക, ചോദിക്കുക,
  സഹായിക്കുക, എനിക്ക് മലയാളം ഇഷ്ടമാണ്. എടുക്കുക corrects Ch. 32's rule rather
  than breaking it — the stamp is the **ഇ**, not the doubled **ക്ക** a native
  stem carries by itself. ചോദിക്കുക is Sanskrit *cud*, "to urge," and the
  reason a loan was needed at all is structural: Proto-Dravidian *\*kēḷ‑*
  covered hearing **and** asking, Tamil's கேள் still does both, and Malayalam
  narrowed കേൾക്കുക to hearing alone. The same shape recurs four times across
  the two chapters — ഓതുക, ഓർക്കുക, കേൾക്കുക, ഉതവി each gave up an everyday
  slot to a Sanskrit word — which is the Maṇipravāḷam era's lexical footprint.
- **Every etymology was checked against sources rather than taken on trust, and
  four briefed or inherited claims were corrected in the process.**
  - വായിക്കുക is **not** from വായ് "mouth". Gundert marks വായന a *tadbhava* of
    Sanskrit *vac* and gives the Tamil verb as *vācikka*; DEDR 5352 (*vāy*
    "mouth") carries no reading sense in any Dravidian language. The mouth story
    is folk etymology, and the lesson says so.
  - The *c* → *y* is **not** a Malayalam sound law. It happened in the middle
    Indo-Aryan stage the *tadbhava* passed through; Malayalam holds the same
    Sanskrit word borrowed straight as വാചകം, so the doublet is visible in the
    language itself.
  - എഴുതുക is **not** filed under എഴു "to rise". Burrow–Emeneau keep them as
    separate entries, so the lesson names the link and marks it unproven — while
    എടുക്കുക genuinely *is* filed inside the rise-family, which is where that
    fact belongs.
  - English *mind* descends from *\*ménti‑*, not from the *\*ménos* that gave
    Sanskrit मनस्; the lesson claims root-level cognacy only, and names मति and
    Greek *ménos* as the exact matches.
  Two further claims are hedged rather than asserted: Monier-Williams marks the
  *saha* + *aya* reading of सहाय "probable", and whether Kannada/Telugu *ettu*
  is the cognate of എടു is a point Gundert and DEDR disagree on.
- **Reinforced at two cadences.** Every lesson's `practises.knowledge` names
  atoms from the immediately preceding one to three lessons, across the chapter
  seam; each payoff reaches several chapters back. Malayalam's never-revisited
  atoms fall from **72 of 78 (92%)** to **46 of 96 (48%)** — 29 previously
  orphaned atoms rescued, spanning Chapters 6, 7, 8, 9, 10, 13, 15, 16, 18, 19,
  20, 24, 26 and 32. The three that remain from this tranche belong to the
  final lesson of the track, which no later lesson exists to retrieve.
- All eight use the canonical `## The letters in this word` heading. That block
  classifies as `script`, which is **detachable**, so every lesson derives
  `modality: sight` with `coreModality: voice` — the driving edition is intact.
  (`core/lesson-modality.json` reports `drivable` from the whole-lesson modality
  rather than the core, so the published manifest understates this; that is a
  known bug in `modality-manifest.ts`, not a property of these lessons.)
- All eight sit under the 300-second effective ceiling (285-299s computed).
- Wiring: `ML-PATH-027`/`ML-PATH-028` and `ML-EXT-027-MIND-VERBS`/
  `ML-EXT-028-DOING-VERBS` in [`curriculum.json`](./curriculum.json), all eight
  concepts dropped from the `SPINE-SAY-WHAT-I-DO` omission ledger (36 omits down
  to 28), Chapter 33 and 34 ledger entries in [`chapters.json`](./chapters.json),
  two `core/book-generation.json` targets, the generated
  `book/chapters/ch33-mind-and-page.tex` and
  `book/chapters/ch34-taking-asking-helping-liking.tex`, `\input` in `book.tex`,
  and generated narration for both chapters.
- The 136-page XeLaTeX build has **zero missing characters** and zero errors.
  Three underfull boxes remain, two of them pre-existing in Chapters 6 and 30;
  the third is the Chapter 34 payoff's section heading, where the Malayalam
  script at 14.4pt forces an awkward break. The track is `null` in
  `core/latex-warning-baseline.json`, so nothing is re-pinned.

