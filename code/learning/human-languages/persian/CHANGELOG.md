# Changelog

## 0.9.0 — 2026-08-07

- Added eight schema-v2 lessons across two new chapters, closing the eight-verb
  tranche that until now only Latin, Spanish and Portuguese taught. Persian is
  the fourth track to realise `VERB-THINK`, `VERB-UNDERSTAND`, `VERB-READ`,
  `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP` and `VERB-LIKE-LOVE`, and
  the first from the **Iranian** branch — nothing else in the corpus carries it.
- Chapter 7, *Four Verbs of the Mind* (`FA-PATH-008`): `FA-C07-fekr-kardan`
  (**فکر کردن**), `FA-C07-fahmidan` (**فهمیدن**), `FA-C07-khandan`
  (**خواندن**), `FA-C07-neveshtan` (**نوشتن**). Chapter 8, *Taking, Asking,
  Helping, Loving* (`FA-PATH-009`): `FA-C08-gereftan` (**گرفتن**),
  `FA-C08-porsidan` (**پرسیدن**), `FA-C08-komak-kardan` (**کمک کردن**),
  `FA-C08-dust-dashtan` (**دوست داشتن**). Split 4 + 4 rather than one chapter of
  eight: the tranche introduces 23 atoms, and one chapter would have doubled
  `maxNewAtomsPerChapter: 12`. Chapter 7 introduces 12, Chapter 8 introduces 11.
- The organising idea is that a Persian verb comes in **three shapes**, not one.
  `FA-GRAMMAR-COMPOUND-VERB-KARDAN` opens the noun-plus-light-verb pattern on
  *fekr kardan* and is proved twice more, on *komak kardan* and — with a
  different light verb — on *dust dâshtan*. `FA-GRAMMAR-IDAN-REGULAR-STEM` names
  the one predictable class, where stripping **-یدن** *-idan* yields the present
  stem (*fahmidan, fahm-*; *porsidan, pors-*). The inherited verbs keep the
  Chapter 6 bargain and state it plainly rather than hiding it: *khândan,
  khân-*; *neveshtan, nevis-*; *gereftan, gir-*.
- Chapter 7's first lesson says early and explicitly what Persian does not ask
  for — no grammatical gender, no noun cases, no adjective agreement, and a
  single set of personal endings for every verb. After Arabic's root system and
  Russian's aspect this is worth telling a learner up front, and it is true.
- Etymology is per lesson and verified against Wiktionary before use. *khândan*
  ← \**swenh₂-* “to sound” (Latin *sonus* → **sound**, **consonant**,
  **sonata**; English **swan**), which is why one verb covers read, recite, sing
  and study. *neveshtan* ← *ni-* “down” + \**peyḱ-* “to paint, mark” (Latin
  *pingere* → **paint**, **picture**, **pigment**). *gereftan* ← \**gʰrebh₂-*,
  the track's clearest inherited cousin: English **grab**, **grip**, **grasp**,
  German *greifen*. **دوست** *dust* ← \**ǵews-* “to taste, to choose,” which
  English inherited as **choose** and Latin took as *gustus* (**gusto**,
  **disgust**). *porsidan* ← \**preḱ-*, whose own English descendant (Old
  English *friġnan*) died out and was borrowed back from Latin as **pray**,
  **prayer**, **precarious**. Where the link is a borrowing rather than an
  inheritance — *kardan* ← \**kʷer-* and Sanskrit **karma** — the lesson says so
  instead of implying a family resemblance.
- Persian's vocabulary layers are named with a word from each: **خوب** *khub*
  inherited Iranian, **وقت** *vaqt* and **فهم** *fahm* Arabic, and **کمک**
  *komak* Turkic (Azerbaijani *kömək* ← Proto-Turkic \**kömek*). *fahmidan* is
  the showpiece — an Arabic noun turned into a native-shaped Persian verb.
- One new script atom, `FA-SCRIPT-SILENT-VAV`: the **و** of **خواندن** is
  written and never pronounced. It is taught as a fossil of the old *xw-*
  cluster, tied back to **خدا** *khodâ* ← Middle Persian *xwadây*, and
  contrasted against the **و** of **نوشتن**, where it is a plain consonant.
- Reinforcement was the point of the second cadence. Each lesson reaches back to
  the one to three lessons before it, across the chapter seam, and the two
  payoffs reach back several chapters. Persian's never-revisited atoms fall from
  **23 of 59** to **11 of 82**; eight of the eleven that remain are older
  script- and dialogue-shaped atoms that these verb lessons cannot honestly
  practise, and the other three belong to the final lesson, which nothing
  follows. Fifteen previously orphaned atoms are rescued, among them
  `FA-SCRIPT-GAF`, `FA-MORPH-KHOSH-VAQT-AM`, `FA-ETYMON-TO-THOU`,
  `FA-ETYMON-BUDAN-BE`, `FA-ETYMON-KHODA`, `FA-ETYMON-HAL`, `FA-ETYMON-HAFEZ`,
  `FA-ETYMON-VAQT-ARABIC`, `FA-ETYMON-KHUB`, `FA-SCRIPT-ALEF-MADDE`,
  `FA-GRAMMAR-NAME-QUESTION-ORDER`, and all three `dânestan` atoms.
- Both chapter payoffs assess **every** atom their chapter introduces (12/12 and
  11/11, against the 0.5 representativeness floor) and carry their reach-back
  atoms in the HL05 ledger as well. `FA-C08-dust-dashtan` runs all thirteen
  verbs the track now holds.
- `VERB-HAVE` and `VERB-DO-MAKE` deliberately stay in the `omits` list for
  `SPINE-SAY-WHAT-I-DO`. *kardan* and *dâshtan* are taught here as atoms because
  the compound verbs cannot be understood without them, but no lesson carries
  those concept tags, and claiming them would overstate what the track realises.
- All eight lessons stay under the five-minute gate on the **computed**
  estimate, not merely the declared one: 281–298 effective seconds against the
  300-second threshold. No lesson introduces more than three atoms, no lesson
  uses a table, and none trips a sight cue, so the track stays 100% drivable.
- The book gains Chapters 7 and 8 (`ch07-mind-verbs.tex`, `ch08-doing-verbs.tex`)
  and grows from 37 to 53 pages. Verified locally with XeLaTeX: zero
  `Missing character` lines, zero overfull or underfull boxes. Romanization uses
  the track's existing **â** convention throughout rather than a macron.

## 0.8.0 — 2026-08-06

- Added five schema-v2 Chapter 6 micro-lessons, the track's first verbs:
  `FA-C06-budan` (**بودن**, `VERB-BE`), `FA-C06-raftan` (**رفتن**, `VERB-GO`),
  `FA-C06-amadan` (**آمدن**, `VERB-COME`), `FA-C06-goftan` (**گفتن**,
  `VERB-SAY`), and `FA-C06-danestan` (**دانستن**, `VERB-KNOW`). Persian moves
  off zero core-verb coverage and realises its first `SPINE-SAY-WHAT-I-DO`
  segment, `FA-PATH-007`.
- The chapter's hook is the pairing, not the paradigm. `budan` establishes that
  every Persian infinitive ends in **-tan/-dan**; `raftan` introduces the
  present stem as the one unpredictable fact a verb carries, and each later
  lesson stores its verb as a single item — *raftan, rav-*; *âmadan, â-*;
  *goftan, gu-*; *dânestan, dân-*. No personal endings and no present tense are
  taught here; they are deliberately left to a later chapter that will find its
  stems already learned.
- Cousin webs are traced only where they are secure. *budan* is anchored to
  \**bheu-* (English **be**, Latin *fuī* → **future**), *âmadan* to \**gwem-*
  (**come**, *venīre* → **advent**, **event**), and *dânestan* to \**gnō-*
  (**know**, **can**, **cunning**, *gnosis* → **diagnosis**). *raftan* and
  *goftan* state plainly that no English cousin is established, and *goftan*
  flags *gab*/*gossip* as false friends; `budan` opens the chapter by flagging
  Persian **بد** *bad* against English *bad*. The **d** of *dânestan* is marked
  as a genuinely disputed sound change rather than smoothed over.
- Two letters arrive inline, inside the only words that need them: **آ**
  (alef with *madde*, in **آمدن**) and **گ** *gâf* (in **گفتن**, named as the
  second of Persian's four additions **پ چ ژ گ** after **چ** in Chapter 3). The
  other three verbs need no new letters at all. New sound ids `alef-madde` and
  `persian-gaf` are recorded in the pronunciation reference.
- Extended the exact N+1/N+3/N+7/N+15 ledger through S40 and added the S21–S25
  new-lesson rows; regenerated the six-chapter book and the modality manifest.
  All five lessons derive `voice` with `no-visual-dependency` — the chapter is
  fully drivable, and its widest structure is a two-item pair spoken aloud.
- Chapter 6 capability ledger added to `chapters.json`. Its payoff,
  `FA-C06-danestan`, assesses 12 of the 15 atoms the chapter introduces
  (0.80, against the 0.5 policy floor) because the closing lesson genuinely
  runs all five pairs back; the three it leaves are the etymon and script atoms
  belonging to the individual words.
- Measured durations (computed, threshold 300s): budan 281s, raftan 285s,
  âmadan 265s, goftan 274s, dânestan 270s. XeLaTeX compiles the six-chapter
  book with zero `Missing character` warnings and zero overfull boxes.

## 0.7.0 — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, for Chapters 2–5:
  each declares a first-person `canDo`, the spine nodes it realises, and the
  payoff lesson that proves the claim.
- Every `payoff.assesses` list is the payoff lesson's own
  `practises.knowledge` set verbatim — nothing is claimed that the lesson does
  not already practise, and nothing is padded to clear a threshold.
- Chapter 1 is deliberately omitted. Its four lessons are still schema v1 and
  declare no knowledge atoms, so any payoff written for it would be invented
  rather than derived. The gap is left visible as debt.
- Measured payoff representativeness (assessed ÷ chapter-introduced atoms)
  against the 0.5 policy threshold: ch2 3/3 = 1.00, ch3 7/14 = 0.50,
  ch4 6/16 = 0.375, ch5 4/11 = 0.364. Chapters 4 and 5 sit below threshold
  because their word lessons introduce script and etymon atoms that the
  consolidating dialogue does not re-exercise; that is a content gap for a
  later revision, not something to paper over here.

## 0.6.0 — 2026-08-04

- Added four schema-v2 Chapter 5 micro-lessons for **خدا**, **حافظ**, joined
  **خداحافظ**, and a start-versus-end interaction.
- Kept Middle Persian and Arabic root histories behind independently readable
  words, then introduced one broadly polite farewell without a hidden verb.
- Extended the exact N+1/N+3/N+7/N+15 ledger through S35, with objective
  activities and a generated five-chapter book.

## 0.5.0 — 2026-08-04

- Added six schema-v2 Chapter 4 micro-lessons for *hâl*, *chetor*, the careful
  respectful wellbeing question, *khub*, compact *khubam*, and cumulative
  practice.
- Reused ezafe before introducing only the first-person **-am** copula needed
  for the reply; colloquial contraction stays a labelled recognition preview.
- Extended the sound-id reference and exact N+1/N+3/N+7/N+15 ledger through
  S31, with objective activities and a generated four-chapter book.

## 0.4.0 — 2026-08-03

- Added five schema-v2 Chapter 3 micro-lessons for respectful/familiar “you,”
  *chist*, the full name question, *khoshvaghtam*, and cumulative practice.
- Added objective activity contracts and prerequisite-closed knowledge atoms for
  the migrated Chapter 2 name frame and every new lesson.
- Generated Chapter 3 for the downloadable book from the same canonical lesson
  AST used by Language Ladder and extended the review ledger through S25.

## 0.3.0 — 2026-08-03

- Added the authoritative five-lesson session map with exact N+1, N+3, N+7,
  and N+15 review placements.
- Added an on-demand Persian pronunciation and script reference keyed to the
  sound ids used by the starter lessons.

## 0.2.0 — 2026-08-02

- Added the first downloadable LaTeX edition.
- Published Chapter 1 (greetings and responses) and Chapter 2 (giving your name)
  from the five dependency-ordered starter lessons.
- Added a B1-oriented track roadmap with Persian-specific extension points.

## 0.1.0 — 2026-08-02

- Added the Persian shared-spine pilot with five under-five-minute lessons.
