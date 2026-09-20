## Chapters 13–15 — the second pre-A1 noun tranche — 2026-08-12

- Authored **thirteen** schema-v2 lessons in **three** chapters, continuing
  the cross-track pre-A1 vocabulary program's second round and confirming
  the same measured mechanism on Bengali a second time:
  `vocabularyOf()` counts distinct `headword:` strings 1:1 with lessons, so
  thirteen new word lessons move Bengali's pre-A1 vocabulary by exactly
  thirteen. Measured before/after with the level gate: headwords at or
  below pre-A1 **33 → 46** (shortfall of 300, **267 → 254**); track-wide
  vocabulary (any level) **52 → 65**; `attained`/`inProgressAt` unchanged at
  `null`/`pre-A1` — vocabulary alone still needs roughly 254 more lessons of
  this shape.
- **Confirmed, not found: no pre-A1 spine-node gap.** All seven pre-A1
  spine nodes already had at least one segment before this tranche
  (`SPINE-POLITE-REQUEST-REPAIR` was closed by the prior tranche); the
  level-gate report's only blocker at the start of this tranche was
  `vocabulary`, and `spine-nodes`, `atom-budget` and `reinforcement` were
  all clean at pre-A1. This tranche closed two *universal concepts* instead
  — `COURTESY-PLEASE` and `COURTESY-SORRY` on `SPINE-POLITE-REQUEST-REPAIR`,
  plus `GREETING-WELCOME` on `SPINE-MEET-GREET` — which is a different,
  finer-grained thing than a spine-node gap: the node already had a
  segment (via Chapter 10's polite-offer workaround), but two of its three
  named concepts, and one of `SPINE-MEET-GREET`'s four, had never been
  taught by any actual word. `curriculum.json`'s per-node `omits` ledgers
  are updated to match exactly what `validateCurriculum` computes from
  `concept_tag` matches, not hand-edited to a guess.
- **Chapter 13 — Please, Sorry, and Welcome**
  (`SPINE-POLITE-REQUEST-REPAIR`, `SPINE-MEET-GREET`), 4 lessons, 6 new
  atoms: **দয়া করে** *doya kore* (Sanskrit দয়া *dayā* "compassion," most
  Indo-Europeanists' PIE *\*deh₂-* "to divide" — the same proposed root
  behind Greek *dêmos* and English **democracy**/**epidemic** — plus করা's
  conjunctive-participle shape **করে**, the noun-plus-করা pattern's third
  demonstration) → **দুঃখিত** *dukkhito* (Sanskrit দুঃখিত *duḥkhita*; the
  secure half, দুঃ/*dus-*, is PIE *\*dus-*, the same prefix inside Greek
  **dys-**; the traditional "bad axle-hole" story for the other half, খ, is
  reported as contested rather than settled, per Mayrhofer) → **মাফ করবেন**
  *maf korben* (the identical Arabic-via-Persian loan Hindi's own maaf-based
  "sorry" phrase already uses, this time softened by করা's **future** tense
  rather than a present-habitual command — a third grammatical shape for
  করা in one chapter) → **স্বাগতম** *shbagotom* (the payoff — Sanskrit
  *svāgatam*, *su-* "good" [PIE *\*h₁su-*, cousin of Greek *eu-*] fused by
  ordinary sandhi onto *āgata* "arrived," which opens with the very আ-
  "hither" prefix আসা's own lesson already named, riding √gam this time,
  PIE *\*gʷem-*, as secure a root as this book has shown: English **come**,
  and via Latin *venīre*, **advent** and **convene**).
- **Chapter 14 — Five Colors** (`SPINE-CHECK-WELLBEING`), 5 lessons, 5 new
  atoms: **লাল** *lāl* (a Persian loan, *la'l*, which named a ruby or spinel
  before it named the color — the identical loan Hindi already teaches) →
  **নীল** *nīl* (Sanskrit *nīla*, "dark blue"/indigo, the same root as
  Hindi's *nīlā*, except Bengali marks no gender on it; English "indigo" is
  a separate word, Greek *indikón* "the Indian thing," linked by trade and
  not by descent) → **কালো** *kālo* (built on Sanskrit *kāla*, "time,"
  tied to Kālī and Yama, the same word and the same still-debated
  one-root-or-two question as Hindi's *kālā*) → **সাদা** *shādā* (a tadbhava
  of Sanskrit *śveta*, with a tatsama twin, শ্বেত, alive in compounds — the
  one color in this set where Bengali and Hindi genuinely diverge: Hindi
  replaced its native word with Persian *safed*, Bengali kept its own) →
  **সবুজ** *shôbuj* (the payoff — a second Persian loan, *sabz*, the same
  root behind Hindi and Urdu's *sabzī*, "vegetable"; the chapter's five
  colors split two Persian loans, two Sanskrit words, and one tadbhava, a
  more even mix of inherited and borrowed than food or family showed).
- **Chapter 15 — Cloth, Shirt, Sari, and Glasses** (`SPINE-CHECK-WELLBEING`),
  4 lessons, 4 new atoms: **কাপড়** *kāpoṛ* (Sanskrit *karpaṭa*, "rag" — but
  even Sanskrit's own dictionaries call *karpaṭa* a **deśī** word, homegrown
  rather than inherited from any reconstructed root, and the identical word
  gives Hindi *kapṛā*, Marathi *kapaḍā*, Gujarati *kapaḍũ* and Punjabi
  *kapṛā*, a genuinely pan-Indo-Aryan family) → **জামা** *jāmā* (a Persian
  loan, *jāma*, "robe" — identical to Hindi's *jāmā*, a third Persian loan
  in this small set beside লাল and সবুজ) → **শাড়ি** *shāṛi* (Sanskrit
  *śāṭī*, "a strip of cloth," worn down through Middle Indic — and, unlike
  every other English cousin this track has traced through millennia of
  sound change, this one reached English directly, in the modern era, as
  the ordinary loanword **"sari"**; also introduces **শ**, Bengali's second
  *s*-family letter, distinct in spelling from স though merged with it in
  speech) → **চশমা** *chôshmā* (the payoff — built on Persian *chashm*,
  "eye," the exact word চোখ's own lesson already named as its only cousin
  outside the family, turned into "eye-thing": spectacles).
- **A correction made against this tranche's own brief, not just this
  track's history.** The brief proposed teaching "please"/"sorry" as pure
  vocabulary padding. Checking the spine ledger first showed both concepts
  were genuine, still-open gaps (`COURTESY-PLEASE` and `COURTESY-SORRY` on
  `SPINE-POLITE-REQUEST-REPAIR`'s own `omits` list) — closing them was not
  optional filler but real, previously-unrealized track debt, and is
  reported as such rather than as ordinary vocabulary depth.
- **Etymological corrections made against a first draft, before commit**:
  a first draft of মাফ করবেন rendered the Arabic root *'afw* in a
  synthesized Bengali-script spelling that no dictionary uses; corrected to
  a plain romanization, matching চা's and চোখ's own established convention
  of never inventing non-Latin, non-Bengali script in lesson prose. A first
  draft of লাল also quoted the Persian source word in Perso-Arabic script,
  repeating — inside this very tranche — the exact mistake Chapters 10–12's
  own changelog names as a 91-error incident; caught and converted to a
  romanization before the font check, not after.
- **Reinforcement, closed at pre-A1 even after the track grew past it.**
  Adding thirteen lessons after Chapter 12 extended several older R2/R3
  reinforcement windows from "not yet reachable" (the track was too short
  to judge them) into "reachable and missed," newly exposing seven pre-A1
  atoms the level gate had never previously flagged: হৃদয় and নাক from
  Chapter 12, and five of this tranche's own atoms. All seven are closed
  with a second revisit apiece, threaded into natural recall lines (দয়া
  and হৃদয় share a warmth; সাদা কাপড়, "white cloth," gives সাদা a second
  home) rather than mechanically repeated. The level gate's `reinforcement`
  criterion, which briefly regressed to `7 atom(s) at or below pre-A1
  revisited fewer than twice` during authoring, reports **zero** for
  pre-A1 in the final state — matching the state before this tranche
  began. Five A2-level atoms from Chapter 9 and one A1-level atom from
  Chapter 6 remain thin under the same newly-reachable-window effect; they
  do not block pre-A1 and are left visible as debt for a tranche scoped to
  that level.
- **Wired via both required steps**: `BN-PATH-018`–`BN-PATH-021` path
  segments (`BN-PATH-018` and the pre-existing `BN-PATH-014` both now
  realize `SPINE-POLITE-REQUEST-REPAIR`; `BN-PATH-019` adds a second
  segment to `SPINE-MEET-GREET`; `BN-PATH-020`/`BN-PATH-021` add two more
  to `SPINE-CHECK-WELLBEING`) plus matching `BN-EXT-018`–`BN-EXT-021`
  extensions, `chapters.json`, `core/book-generation.json`, `book/book.tex`,
  and the generated narration. Verified after every edit that all lessons
  remain on a path.
- **Verification**: the forced two-pass XeLaTeX build of the 119-page book
  has zero `Missing character`, zero over/underfull boxes, and zero
  undefined references after the second pass. `npx vitest run
  tests/integration.test.ts tests/cli.test.ts tests/chapter-references.test.ts
  tests/track-progress.test.ts` passes; `check:modality`, `check:books`,
  `check:narration`, `check:figures` and `check:progress` all pass. All
  thirteen new lessons compute well under the 300-second ceiling (declared
  250–295 s, computed 172–291 s). Every table stays at 2–3 columns; no
  lesson trips the sight-cue scanner or the info-dump rule-statement gate.
  The corpus-wide pinned-number tests (chapters, continuity, levels,
  modality-manifest, narration, info-dump, metalanguage, root-ledger,
  chapter-modality-book) shift with any authored content and are left
  failing per standing instruction, for the orchestrator to re-measure once
  after all four wave-6 branches merge.

