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

