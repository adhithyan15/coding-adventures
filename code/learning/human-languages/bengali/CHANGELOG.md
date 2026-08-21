# Changelog

## [Unreleased]

### Fixed — three false forward-review claims

- `BN-C02-alaap` now records the earlier `BN-C02-amar-naam` lesson that its
  warm-up and knowledge directives actually rehearse.
- `BN-C02-ki` now records its real name-word review instead of pointing ahead
  to the not-yet-taught pronoun lesson.
- `BN-C05-kaj-kora` no longer claims to review the following `thaka` lesson;
  its exercises revisit `BN-C05-bola` exactly as the knowledge ledger says.

The authored reading order does not change. Bengali's order-integrity debt
falls from three false forward reviews to zero, so the five-minute ramp now
describes what the learner actually encounters.

### Added — Chapter 16, the first nine pieces of the script (HL-C222)

Ten lessons. **Nine teach one piece each; one introduces nothing** and assembles
the greeting from pieces the reader can already write.

`scriptLessons` 0 → 10, `taughtGlyphs` 0 → 9, `neverTaughtGlyphs` **48 → 39**.

**The inherent vowel is not *a*.** It is **ɔ**, the vowel of English *awe*. A bare
Bengali consonant says *kɔ* where a bare Devanagari one says *ka*, and that single
default is most of why Bengali does not sound like Hindi read aloud. It is taught
on the very first shape, because every letter after it inherits the difference.

**One letter is written *s* and said *sh*.** স descends from the Sanskrit *s* and
every transliteration writes it that way; Bengali normally pronounces it *sh*. The
spelling records the ancestry, the sound records what Bengali did afterwards, and
both are true — the same way English keeps *knight* and *through*.

The four abugida ideas are Marathi's, in Marathi's order, because the greeting
carries a conjunct and the virama lesson therefore has somewhere to land. Bengali
calls that mark the **hasanta**, and its conjuncts fuse more thoroughly than
Devanagari's — the principle is unchanged, the shapes take a moment longer to take
apart.


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

## Chapters 10–12 — the pre-A1 noun tranche — 2026-08-08

- Authored **twelve** schema-v2 lessons in **three** chapters of four,
  continuing the cross-track pre-A1 vocabulary program (Hindi, Arabic, Tamil,
  German, French, Portuguese, Italian) and confirming the same measured
  mechanism on Bengali: `vocabularyOf()` counts distinct `headword:` strings
  1:1 with lessons, so twelve new word lessons move Bengali's pre-A1
  vocabulary by exactly twelve. Measured before/after with the level gate:
  headwords at or below pre-A1 **21 → 33** (shortfall of 300, **279 → 267**);
  track-wide vocabulary (any level) **40 → 52**; `attained`/`inProgressAt`
  unchanged at `null`/`pre-A1` — vocabulary alone needs roughly 267 more
  lessons of this shape, and nothing about the mechanism makes that cheaper.
- **A second gain, not just vocabulary**: `SPINE-POLITE-REQUEST-REPAIR` was
  the one pre-A1 spine node Bengali had never realized (`"segments": []`),
  and was blocking pre-A1 on its own, independent of the vocabulary
  shortfall. Chapter 10 realizes it — not with a dedicated "please" word,
  which the track does not have, but by reusing Chapter 7's respectful
  imperative of খাওয়া (আপনি খান) as a polite-offer pattern: **চা খান / জল
  খান / দুধ খান / ভাত খান**. The `spine-nodes` blocker is gone from the
  level-gate report; only `vocabulary` remains.
- **Chapter 10 — Tea, Water, Milk, and Rice** (`SPINE-POLITE-REQUEST-REPAIR`),
  4 lessons, 6 new atoms: **চা** *chā* (a Chinese loan via Persian, the
  overland route, unlike the Hokkien-via-Dutch sea route behind English
  *tea*) → **জল** *jôl* (prised out of Chapter 7's `জল খাওয়া`; its own root
  is genuinely disputed, while the Bangladesh-register **পানি** traces
  cleanly to Chapter 7's √pā, "to drink") → **দুধ** *dudh* (√duh, PIE
  *dʰewgʰ-*; the one secure English cousin is **doughty**, not the
  look-alike **dough**, which is a different PIE root entirely — a
  correction against the false lead an earlier track in this program had
  to make explicitly) → **ভাত** *bhāt* (√bhaj "to divide, to share," PIE
  *bʰeh₂g-* — the payoff, and the root Chapter 11 picks back up).
- **Chapter 11 — Friend, Family, Brother, and Sister**
  (`SPINE-EXCHANGE-NAMES`), 4 lessons, 4 new atoms: **বন্ধু** *bôndhu*
  (Sanskrit bandhu kept whole, a **tatsama**; √bandh, PIE *bʰendʰ-* —
  English **bind**/**bond**/**band**, an unusually undisguised cousin) →
  **পরিবার** *pôribār* (a second tatsama, *pari-* + √vṛ, "what surrounds
  you"; *pari-* is the same prefix English itself borrowed as *peri-*, but
  √vṛ's secure cousins are Latin and Lithuanian, not English) → **ভাই**
  *bhāi* (a **tadbhava** this time — worn down through Prakrit rather than
  kept whole — and simply *is* English "brother," PIE *bʰréh₂tēr*, not a
  cousin standing in for it) → **বোন** *bon* (the payoff: not built on PIE
  *swésōr* at all, but on Sanskrit *bhaginī* ← *bhaga*, "a share" — the same
  √bhaj that named Chapter 10's ভাত two lessons earlier. Bengali's "sister"
  and "rice" are cousins; its "sister" and English's are not). Also states
  plainly, and demonstrates on বন্ধু, that **Bengali marks no grammatical
  gender on any of these words** — Chapter 7 already established this for
  verbs; this chapter is where a reader feels it on nouns. A light,
  non-systematic touch on address-term honorificity: দাদা/দিদি as respectful
  terms for non-relatives, named in the ভাই/বোন lessons and not built into a
  system.
- **Chapter 12 — Eye, Mouth, Nose, and Heart** (`SPINE-CHECK-WELLBEING`),
  4 lessons, 4 new atoms: **চোখ** *chokh* (an **ardhatatsama** — half-worn
  through Old/Middle Bengali *cakhu*/*coukh* — √cakṣ, PIE *kʷeḱ-*, "to see";
  no secure English cousin, only Persian *čašm*) → **মুখ** *mukh* (a full
  tatsama with a tadbhava twin, **মু**, alive only inside compounds; its own
  deepest root is a genuine, unresolved Dravidian-vs-Indo-European dispute
  among Sanskritists, reported rather than picked) → **নাক** *nāk* (a
  tadbhava, PIE *néh₂s-*, as secure as etymology gets — the direct ancestor
  of English **nose** and Latin *nāsus*) → **হৃদয়** *hridoy* (the payoff: a
  tatsama with its own tadbhava twin, **হিয়া**, alive in poetry rather than
  speech — the reverse of মুখ's pair — and root হৃদ্, PIE *ḱérd-*, the
  widest confirmed cousin family in the track: English **heart**, Greek
  *kardía*, Latin *cor*).
- A finding worth naming precisely rather than repeating the generic one:
  this program has independently confirmed, in earlier tracks, that the
  seven pre-A1 spine nodes have no concept for a concrete object and that
  household nouns (table, window, key) get shortlisted and dropped. That
  finding does not apply cleanly here in the direction the brief assumed —
  Bengali's own words split roughly evenly between **tatsama** (borrowed
  whole from Sanskrit: বন্ধু, পরিবার, মুখ, হৃদয়) and **tadbhava** (worn down
  by inherited sound change: জল, দুধ, ভাত, ভাই, বোন, নাক), with চোখ as an
  **ardhatatsama** astride both and চা a loan from neither. "Mostly
  tadbhava" undersells how much of this vocabulary Bengali kept unassimilated
  from Sanskrit rather than inheriting.
- **Reinforcement, closed at both cadences.** Each lesson's
  `practises.knowledge` names atoms from the 1–3 lessons immediately before
  it; each chapter's payoff reaches back further. All twelve new atoms are
  revisited at least twice within their window — the level gate's
  `reinforcement` criterion, which briefly flagged three thin atoms
  (`BN-SOUND-C10-DUDH-02` at zero revisits, `BN-LEX-C10-DUDH-01` and
  `BN-LEX-C11-BHAI-01` at one) during authoring, reports **zero** for
  pre-A1 in the final state. The three chapter payoffs also rescue every
  atom the level gate reported as under-reinforced anywhere in the earlier
  corpus: Chapter 6's five numbers atoms (via বোন's sibling count, "āmār ek
  bhāi. āmār dui bon."), three thin Chapter 7 grammar atoms and Chapter 9's
  doubled-letter sound rule (via ভাত's closing recall), and Chapter 8's
  flapped-ড় and causative-আনো atoms (via হৃদয়'s closing recall) — matching
  the discipline the Chapters 8–9 tranche set.
- **Caught and corrected before commit, the FONT CHECK's actual finding**: a
  first pass quoted Sanskrit roots in **Devanagari** script (बन्धु, दुग्ध,
  हृदय, and others), plus Persian, Gurmukhi and Han characters in etymology
  asides — none of which the book's fonts cover, and none of which this
  track's own Chapters 7–9 ever do; their established convention is to
  render every Sanskrit citation in **Bengali script** instead (see
  Chapter 7's জ্ঞা for √jñā). A forced two-pass XeLaTeX compile first
  surfaced 91 `Missing character` errors from this; converting every
  citation to Bengali script and dropping the non-Latin asides (Chinese 茶,
  Persian چا/چشم, Punjabi ਚਾਹ — kept only as romanizations, matching how
  German's Kaffee lesson handles its own loanword route) brought the count
  to **zero**. A stray unmapped romanization character, `ẏ`, introduced for
  য়, was also replaced with the track's existing convention of plain `y`
  (as in *sahāya*, *kolkātāy*). One further overfull `\hbox` in the
  generated পরিবার section heading was cleared by shortening its gloss.
- **Verification**: the forced two-pass XeLaTeX build of the 76-page book
  has zero `Missing character`, zero over/underfull boxes, and zero
  duplicate labels. `npx vitest run tests/integration.test.ts
  tests/cli.test.ts` passes (19/19); `check:modality`, `check:books` and
  `check:narration` all pass. All twelve new lessons compute well under the
  300-second ceiling (declared 255–290 s), and the shared duration report
  measures zero Bengali duration violations. Every table stays at 2 columns;
  no lesson trips the sight-cue scanner. The corpus-wide pinned-number tests
  (chapters, continuity, levels, modality-manifest, narration, ramp) shift
  with any authored content and are left failing per standing instruction.
- Wired via both required steps: `BN-PATH-014`–`BN-PATH-016` path segments
  (attaching to `SPINE-POLITE-REQUEST-REPAIR`, `SPINE-EXCHANGE-NAMES` and
  `SPINE-CHECK-WELLBEING` respectively) plus matching
  `BN-EXT-014`–`BN-EXT-016-LANGUAGE-SPECIFIC` extensions, `chapters.json`,
  `core/book-generation.json`, `book/book.tex`, and the generated narration.
  Verified after every edit that all lessons remain on a path, given this
  track's prior history of an orphaned `curriculum.json` segment.

## Chapters 8 and 9 — the eight-verb tranche — 2026-08-07

- Authored **eight** schema-v2 lessons in **two** chapters of four, realizing the
  canonical `SPINE-SAY-WHAT-I-DO` concepts `VERB-THINK`, `VERB-UNDERSTAND`,
  `VERB-READ`, `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP` and
  `VERB-LIKE-LOVE`. Each of the eight was taught by exactly **three** tracks
  before this (Spanish, Latin, Portuguese) and is now taught by **four**. Bengali
  goes from **6 of 40** core verbs to **14 of 40**.
- **Chapter 8 — The Mind and the Page**, 4 lessons, **8** new atoms:
  - **ভাবা** *bhābā* — Sanskrit *bhāvayati* is the **causative** of √bhū, which
    is Chapter 7's হওয়া. Thinking is making something be. The gear is still
    live in Bengali as **-আনো**: দেখা → দেখানো "show," খাওয়া → খাওয়ানো "feed."
  - **বোঝা** *bojhā* — √budh "to wake," the root that titled the **Buddha**;
    PIE *\*bʰewdʰ-* → English **bid**, **forbid**. Vowel harmony returns on a new
    vowel, and this time the **spelling moves**: বুঝি against বোঝে. Three
    knowings now stand where English has one — জানা, চেনা, বোঝা.
  - **পড়া** *pôṛā* — √paṭh "to recite aloud." A single intervocalic retroflex
    softens into **ড়**, which is why the letter exists at all; the same
    softening dragged √pat "to fall" onto the identical spelling, and it is the
    **falling** twin that owns **feather**, **petition** and **pterodactyl**.
  - **লেখা** *lekhā* — √likh "to scratch," beside Latin *scrībere* and Germanic
    *wrītan*, both also "scratch": three unrelated roots, one idea, named as
    **convergence and not kinship**. And every **-া** form is simultaneously a
    **noun**, which is what Chapter 4's *dækhā hôbe* had been doing all along.
- **Chapter 9 — Taking, Asking, Helping, Liking**, 4 lessons, **9** new atoms:
  - **নেওয়া** *neowā* — √nī "to lead." Its working life is as the verb that
    **closes a compound**: লিখে নেওয়া "write it down," নিয়ে আসা "bring"
    (Bengali has no separate word for it), নিয়ে যাওয়া "take away."
  - **জিজ্ঞাসা করা** *jijñāsā kôrā* — জিজ্ঞাসা is the Sanskrit **desiderative**
    of √jñā, so asking is literally *wanting to know*: Chapter 7's জানা with an
    appetite, on the same PIE *\*ǵneh₃-* that gives English **know**. And **noun
    + করা** is not a compound but *the* way Bengali makes verbs — the door this
    lesson opens is wider than the word.
  - **সাহায্য করা** *sāhājjo kôrā* — *sahāya*, "a companion," is **সহ-**
    "together" + **√i** "to go"; both cousins are secure (PIE *\*sem-* → **same**,
    Greek *homo-*; PIE *\*h₁ey-* → Latin *īre* → **exit**, **transit**). The
    doubled **য্য** finally demonstrates the word-final **inherent o** that
    Chapter 6 had to admit its five numerals could not show.
  - **ভালো লাগা** *bhālo lāgā* — √lag "to attach." *Āmār bhālo lāge* is "good
    sticks **to me**": the liker is not the subject, the same inversion Spanish
    makes with *gustar*. It wears the identical clothes as আমার … আছে, "I have."
    Set against **ভালোবাসা**, where you *are* the subject — the chapter's payoff
    is the contrast.
- **Honest dead ends, again named rather than papered over**: √nī left no living
  English descendant; √paṭh has no secure Indo-European pedigree past Sanskrit;
  and ভালোবাসা's *bāsā* half has a **disputed** origin, so the commonest proposal
  (√vas "to dwell," which would make English **was** its cousin) is reported as a
  proposal and left open.
- **Reinforcement at two cadences**, which is the point of splitting this into
  two chapters rather than one. Every lesson's `practises.knowledge` names atoms
  from the one to three lessons immediately before it, across the chapter seam;
  the two payoffs reach several chapters back. Measured result: Bengali's
  never-revisited atom count falls from **12 of 18** to **4 of 35** — and all
  twelve of the previously orphaned atoms (Chapter 6's six and Chapter 7's six)
  are now genuinely practised, not merely listed. The four that remain are the
  three introduced by the track's final lesson, which nothing can follow, and
  the doubled **য্য** of সাহায্য, which is recorded rather than claimed.
- Windows closed: **R1** for both দেখা atoms and both জানা atoms; **R2** for all
  six Chapter-6 atoms and all six Chapter-7 pairs. R1 for Chapters 6 and 7 was
  already out of reach — those windows close at reading positions 31–37, which
  are lessons this tranche does not edit — and that residue is left visible.
- Wired into `curriculum.json` (`BN-PATH-012`/`BN-PATH-013`,
  `BN-EXT-012-MIND-AND-PAGE`/`BN-EXT-013-CONJUNCT-VERBS`, and the eight concepts
  struck from the `SPINE-SAY-WHAT-I-DO` omission ledger), `chapters.json`,
  `core/book-generation.json`, `book/book.tex`, and the generated narration.
  All 45 Bengali lessons are on a path; none is orphaned.
- All eight use the canonical **`## The letters in this word`** heading, which
  types as a `script` block. That labels them `sight` and **detachable**, so
  every one has a `voice` core: the track's drivability rises to **98%**, with
  30 lessons rescued for the hands-free view. Every table is 2 or 3 columns; no
  lesson contains a sight cue. Computed durations **256–298 s**, all inside the
  300 s ceiling.
- The forced nine-chapter XeLaTeX build is **warning-free**: 54 pages, zero
  `Missing character`, zero over/underfull boxes. The new conjuncts — **ড়**,
  **জ্ঞ**, **য্য**, **দ্বার** — all render from the vendored Noto Sans Bengali
  with no preamble change.

## Chapter 7 — The Core Verbs — 2026-08-06

- Authored six schema-v2 lessons realizing the canonical `SPINE-SAY-WHAT-I-DO`
  concepts `VERB-BE`, `VERB-GO`, `VERB-COME`, `VERB-EAT`, `VERB-SEE` and
  `VERB-KNOW`. Before this the track realized **no** canonical verb concept: its
  only four verbs (*bôlā*, *thākā*, *kôrā*, *dækhā hôbe*) were all namespaced
  `BN-VERB-*` and none of them was on the shared spine.
- One idea per lesson, each one a thing Bengali does that its neighbours do not:
  - **হওয়া** *hôwā* — Bengali has **two** be-verbs, and আছ- is unfinished: it
    has a present and a past and nothing else, so the future falls to *hôbe* or
    to Chapter 5's থাকা. Root: Sanskrit √bhū, PIE *\*bʰuH-* → English **be**,
    **been**, **future**, **physics**.
  - **যাওয়া** *jāwā* — the honorific level lives in the **verb ending**:
    *jāsh* / *jāo* / *jān* for তুই / তুমি / আপনি, and *se jāy* against *tini
    jān* in the third person. Drop the pronoun and the register still stands.
  - **আসা** *āsā* — **no grammatical gender, anywhere**, set against Hindi
    *ātā/ātī*, Marathi *yeto/yete* and Gujarati's *āvyo/āvī* past. Not a
    beginner's simplification the grammar takes back later.
  - **খাওয়া** *khāwā* — Bengali **eats its drinks**: *jôl khāwā*, *chā khāwā*,
    where Hindi keeps *pīnā*. The formal পান করা carries √pā → English
    **potion**, **potable**.
  - **দেখা** *dækhā* — **vowel harmony**: *dekhi* closes where *dækhe* and
    *dækho* stay open, and the spelling দে never moves. Root: Sanskrit √dṛś, PIE
    *\*derḱ-* → Greek *drákōn* → English **dragon**.
  - **জানা** *jānā* — জানা for facts against চেনা for people, the *savoir* /
    *connaître* line English lost. Root: √jñā, PIE *\*ǵneh₃-* → **know**,
    **notice**, **diagnosis**.
- Flagged two dead ends honestly rather than inventing cousins: যাওয়া's PIE
  *\*yeh₂-* has no living English descendant, and খাওয়া's *khād-* has no secure
  Indo-European pedigree outside Indo-Aryan.
- All six derive as **`voice`** — the chapter's `drivablePrefix` is 6, every
  table is two columns, and no lesson leans on a sight cue. Computed durations
  257–281 s, all inside the 300 s ceiling.
- Wired the chapter into `curriculum.json` (`BN-PATH-011`,
  `BN-EXT-011-CORE-VERBS`, and six concepts struck from the
  `SPINE-SAY-WHAT-I-DO` omission ledger), `chapters.json` (payoff
  `BN-C07-jana`, 8/12 introduced atoms = 0.67, above the 0.5 floor),
  `core/book-generation.json`, and `book/book.tex`.
- Gave the book preamble an optional `grammarlens` title — the generator passes
  each lesson's own "Grammar Lens: …" heading through, which the old
  no-argument box could not accept — plus composed glyphs for the PIE palatals
  `ǵ` and `ḱ`. The seven-chapter build is still warning-free, with no missing
  characters and no over/underfull boxes.

## Chapter capability ledger — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, covering Chapter 6:
  the reader can count *ek, dui, tin, chār, pā̃ch* in Bengali script and say what
  **দুই** kept that Hindi *do* and Marathi *don* flattened away.
- Made `BN-C06-numbers-1-5` the chapter payoff — the chapter's only lesson, and
  its only schema-v2 one. It is typed `production`: the payoff is counting the
  five aloud, then placing *dui* in its family.
- Recorded `SPINE-COUNT-ONE-TO-FIVE` as the chapter's spine node, matching
  `BN-PATH-010` in `curriculum.json`.
- Omitted Chapters 1–5 rather than stubbing them: all 30 of their lessons are
  schema v1 and declare no `practises.knowledge`, so no payoff there could name
  atoms a lesson actually exercises. Their absence is the debt the HL05 gap
  report exists to measure.
- Measured payoff representativeness for Chapter 6 at 6/6 introduced atoms
  (1.00), comfortably above the 0.5 policy floor.

## Book warning cleanup — 2026-08-03

- Kept punctuation outside the Bengali-only font and replaced five duplicate
  recap anchors with stable chapter-qualified labels.
- Preserved Bengali in PDF bookmarks while suppressing the font-only command
  there, and mapped the vendored static font to every requested shape.
- Let short lesson pages end naturally and made the long farewell title
  breakable so the forced six-chapter build has no layout, bookmark, label,
  font, punctuation-glyph, or package warnings.

## Canonical Chapter 6 publication — 2026-08-03

- Migrated the numbers lesson to schema v2 with the shared
  `SPINE-COUNT-ONE-TO-FIVE` can-do node, a 290-second ceiling, and block-level
  knowledge closure.
- Generated the downloadable Chapter 6 from the same lesson AST and source hash
  that Language Ladder loads instead of maintaining a second content copy.
- Preserved Bengali numeral forms, the chandrabindu note, the qualified history
  of *dui*, and bookmark-safe romanization in the generated chapter; the book
  preamble now supplies the shared width-aware table renderer it uses.

## Sub-five-minute remediation — 2026-08-02

- Corrected eleven declared five-minute estimates whose computed durations were
  already between 121 and 290 seconds.
- Preserved every lesson body unchanged; no split or content reduction was
  necessary. The shared report now measures zero Bengali duration violations.
- The 290-second numbers lesson is the tightest Bengali budget and should be
  watched during later copy edits.

## Chapter 6 — Numbers 1–5, and the conservative "two"

- **Chapter 6 authored** (`BN-C06-numbers-1-5`): *ek, dui, tin, chār, pā̃ch*
  (using *ek*, not the *êk* of a first draft, which would have introduced a
  diacritic this track never defines — its established mark is **ô**).
- **দুই *dui* is the lesson.** Against Hindi *do* and Marathi *don*, Bengali
  **keeps a trace of the vowel that followed the old cluster**, which is why it
  has two syllables where its neighbours have one.
- **Two absolutes scoped back**, both of which were false as first written:
  - "No modern Indo-Aryan language kept the *dv-* cluster" is true of the
    **everyday numeral** only — *dv-* is alive in words re-borrowed straight from
    Sanskrit, like Hindi *dvār* "door."
  - "The vowel survives **only** here" ignores **Assamese, Odia and Nepali**,
    which all have *dui*. Bengali is unusual only among the four languages this
    chapter compares. (Maithili was in a first draft of that list and removed —
    it has *dū*, not *dui*.)
- **A claim removed rather than repaired.** A first draft said the numbers
  demonstrate Chapter 1's o-leaning inherent vowel. They don't — **এক** opens
  with the independent vowel এ, and none of the five contains a bare
  inherent-vowel syllable. The observation is still mentioned (it's true, and
  `BN-C01` does teach it), but now explicitly as something *not* visible in this
  data, so the learner doesn't go looking for it here.
- The **ঁ** on *pā̃ch* is named as the same **chandrabindu** the Devanagari
  tracks use.

## Chapters 2–5 — Introductions, How-are-you, Farewells, First Verbs

- Four new chapters carry Bengali from Chapter 1 to Chapter 5, matching the
  leading tracks' arc. One word per lesson, atom-first, Bengali script inline;
  every root traced (`lessons/BN-C0{2,3,4,5}-*`, `book/chapters/ch0{2,3,4,5}-*.tex`).
  Concept tags reuse the universal `HL01` taxonomy; verbs namespaced (`BN-VERB-*`).
  Two Bengali distinctives run throughout: the **zero copula** (no "is" in the
  present) and **no grammatical gender at all**.
- **Ch. 2 — Introducing Yourself**: *nām* (← *nāman* → *name*) → *āmār* (no
  gender, unlike *merā/merī*) → *āmār nām …* (the zero copula) → *tumi/āpni* (+
  *tui*: Bengali's three-way "you") → *ki* → *tomār nām ki?* → *ālāp kore bhālo
  lāglo* (*ālāp* ← Sanskrit, a rāga's opening) → practice.
- **Ch. 3 — How Are You**: *kemon* → *tumi kemon āchho?* (the verb *āchhā* — the
  copula returns for state) → *āmi* (← *asmi* → English **am**) → *bhālo* (←
  *bhadra*) → *kono bæpār nā* ("no matter" = you're welcome; *nā* ← PIE *ne) →
  practice.
- **Ch. 4 — Farewells**: *ābār* → *dækhā hôbe* (the impersonal "a seeing will
  happen") → *ābār dækhā hôbe* (the fuller form of Ch.1's *āshi*) → *kāl dækhā
  hôbe* (*kāl* = both tomorrow and yesterday ← *kāla*) → practice.
- **Ch. 5 — First Verbs**: *bôlā* → *āmi bānglā bôli* (*bôngo* → the Ganges
  delta) → *thākā* (to live ← *sthā* → English *stand/stay/state*) → *kāj kôrā*
  (to work; ← √kṛ, the root of *nômoshkar*) → practice. **The verb changes for
  person but never for gender.** Book compiles clean with XeLaTeX (0 missing
  chars, 0 undefined refs).

## Chapter 1 — Greetings (Bengali script taught inline)

- New Bengali track on the HL00 framework — Indo-Aryan, written in the Bengali
  script (vendored Noto Sans Bengali font). One word per lesson, slug ids,
  atom-first, derivations shown, LaTeX book. No reading course: the script is
  taught *inside* each word lesson.
- Chapter 1 (`lessons/BN-C01-*`):
  - **নমস্কার** nômoshkar ("hello/goodbye") — the *same* word as Sanskrit
    *namaskāra*, used to introduce Bengali's fingerprint shifts (*a→ô*, *s→sh*)
    plus the inherent-ô vowel and the স্ক conjunct.
  - **ধন্যবাদ** dhônyobad ("thank you") — Sanskrit *dhanya*+*vāda*; shows *a→ô*
    again and *v→b* (Bengali has no "v").
  - **হ্যাঁ / না** hyã / nā ("yes / no") — the *chandrabindu* nasal; *nā* on PIE
    *ne (English *no/not/none*).
  - **আচ্ছা** āchchhā ("okay / I see") — the standalone vowel-letter আ and the
    চ্ছ conjunct; the conversational workhorse.
  - **আসি** āshi ("I'll be going") — literally "I come," the "promise of return"
    goodbye shared with Tamil and Marathi; Bengali marks no gender on the verb.
  - **practice**.
- The recurring thread: Bengali's one sound-fingerprint (inherent **ô**, *s→sh*,
  *v→b*) that disguises familiar Sanskrit words, taught so the learner can
  un-shift any word back. Script facts documented in the appendix. Book compiles
  clean with XeLaTeX.
